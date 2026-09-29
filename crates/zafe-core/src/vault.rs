//! Vault descriptor, vault-log events, and replay of the log into current state
//! (spec §6.3, §7.3, §9.1, §9.6).
//!
//! The relay only orders encrypted entries; every rule about what an entry may do is
//! enforced here, by every member, when replaying the log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zafe_proto::{IdentityPublic, LogEntry, LogKey};

use crate::session::ProposalId;

const PERSONAL_DESCRIPTOR: &[u8; 16] = b"Zafe_VaultDescr_";
const PERSONAL_EVENT_SIG: &[u8] = b"Zafe descriptor signature v1";

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum VaultError {
    #[error("encoding")]
    Encoding,
    #[error("log entry {0} cannot be decrypted with the current log key")]
    Undecryptable(u64),
    #[error("the log must start with a VaultCreated event")]
    NotCreatedFirst,
    #[error("VaultCreated appears more than once")]
    DuplicateCreated,
    #[error("descriptor is missing a valid signature from a member")]
    MissingDescriptorSignature,
    #[error("entry {0} is authored by a non-member")]
    NotAMember(u64),
    #[error("entry {0} refers to an unknown proposal")]
    UnknownProposal(u64),
    #[error("entry {0}: vote for a proposal that is no longer open")]
    ProposalClosed(u64),
    #[error("entry {0}: vote references a different PCZT than the proposal")]
    PcztMismatch(u64),
    #[error("entry {0}: duplicate proposal id")]
    DuplicateProposal(u64),
    #[error("entry {0}: only the author can cancel a proposal")]
    NotAuthor(u64),
}

/// A member as recorded in the descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberInfo {
    pub identity: IdentityPublic,
    /// Serialized FROST identifier.
    pub frost_id: Vec<u8>,
    pub name: String,
}

/// Default proposal lifetime: 7 days at the 75-second block target (1152 blocks per day).
pub const DEFAULT_PROPOSAL_EXPIRY_BLOCKS: u32 = 7 * 1152;

/// Extra blocks a member accepts beyond the vault's expiry window, because the member's
/// synced tip can lag the proposer's (about two hours).
pub const EXPIRY_TIP_SLACK_BLOCKS: u32 = 96;

/// What every member signs at the end of vault creation (spec §7.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultDescriptor {
    pub vault_id: [u8; 16],
    pub version: u8,
    pub name: String,
    /// "main", "test" or "regtest".
    pub network: String,
    pub threshold: u16,
    pub members: Vec<MemberInfo>,
    /// `I2LEOSP_256(ak)`.
    pub group_public_key: [u8; 32],
    pub ufvk: String,
    pub address: String,
    pub use_qsk: bool,
    pub birthday_height: u32,
    /// How long a payment proposal stays valid, in blocks after the height it is built for
    /// (the transaction's expiry height). Approvals and signing are asynchronous, so this is
    /// days rather than the 40-block wallet default. Expiry stays a safety feature: a signed
    /// but unsent transaction must not remain valid forever.
    pub proposal_expiry_blocks: u32,
    pub epoch: u32,
    /// Hash of the key-generation transcript (binds the descriptor to the DKG run).
    pub transcript_hash: [u8; 32],
}

impl VaultDescriptor {
    pub fn hash(&self) -> Result<[u8; 32], VaultError> {
        let bytes = postcard::to_allocvec(self).map_err(|_| VaultError::Encoding)?;
        Ok(blake2b_simd::Params::new()
            .hash_length(32)
            .personal(PERSONAL_DESCRIPTOR)
            .hash(&bytes)
            .as_bytes()
            .try_into()
            .expect("32 bytes"))
    }

    /// The message each member signs with their identity key.
    pub fn signing_message(&self) -> Result<Vec<u8>, VaultError> {
        let mut msg = PERSONAL_EVENT_SIG.to_vec();
        msg.extend_from_slice(&self.hash()?);
        Ok(msg)
    }

    pub fn member(&self, sig_pk: &[u8; 32]) -> Option<&MemberInfo> {
        self.members.iter().find(|m| &m.identity.sig_pk == sig_pk)
    }

    /// Rejections needed to make a proposal impossible to approve: n − t + 1.
    pub fn rejection_threshold(&self) -> usize {
        self.members.len() - usize::from(self.threshold) + 1
    }
}

/// One payment as recorded in a proposal (what members are asked to approve).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedPayment {
    pub address: String,
    pub amount_zat: u64,
    /// Full 512-byte memo field.
    pub memo: Vec<u8>,
}

/// Events carried in the vault log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultEvent {
    Created {
        descriptor: VaultDescriptor,
        /// `(member sig_pk, signature over descriptor.signing_message())` for every member.
        signatures: Vec<([u8; 32], Vec<u8>)>,
    },
    Proposal {
        id: ProposalId,
        payments: Vec<ProposedPayment>,
        pczt: Vec<u8>,
        pczt_hash: [u8; 32],
        /// Chain tip the proposer built against (for expiry checks).
        tip_height: u32,
        /// Proposer's clock, unix seconds (display only; not trusted).
        created_at: u64,
    },
    Vote {
        proposal: ProposalId,
        pczt_hash: [u8; 32],
        approve: bool,
        /// Serialized FROST round-1 commitments, one per spend (empty for rejections).
        commitments: Vec<Vec<u8>>,
    },
    Cancelled {
        proposal: ProposalId,
    },
    Broadcast {
        proposal: ProposalId,
        txid: [u8; 32],
    },
}

impl VaultEvent {
    pub fn to_bytes(&self) -> Result<Vec<u8>, VaultError> {
        postcard::to_allocvec(self).map_err(|_| VaultError::Encoding)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VaultError> {
        postcard::from_bytes(bytes).map_err(|_| VaultError::Encoding)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    Open,
    Approved,
    Rejected,
    Cancelled,
    Broadcast,
}

#[derive(Clone, Debug)]
pub struct ProposalState {
    pub id: ProposalId,
    pub author: [u8; 32],
    pub payments: Vec<ProposedPayment>,
    pub pczt: Vec<u8>,
    pub pczt_hash: [u8; 32],
    pub tip_height: u32,
    /// Proposer-claimed creation time, unix seconds (display only).
    pub created_at: u64,
    /// Index of the proposal's log entry (orders proposals).
    pub log_index: u64,
    pub status: ProposalStatus,
    /// Latest approval per member: their serialized commitments.
    pub approvals: BTreeMap<[u8; 32], Vec<Vec<u8>>>,
    pub rejections: BTreeMap<[u8; 32], ()>,
    pub txid: Option<[u8; 32]>,
}

/// Current vault state, rebuilt from the log.
#[derive(Clone, Debug)]
pub struct VaultState {
    pub descriptor: VaultDescriptor,
    pub proposals: BTreeMap<ProposalId, ProposalState>,
    /// Number of log entries processed (applied or ignored).
    pub applied: u64,
    /// Entries that were validly signed and chained but semantically invalid (e.g. a vote
    /// racing a cancellation). Every member skips the same entries, so all members reach
    /// the same state; a bad entry can never make the log unreadable.
    pub ignored: Vec<(u64, VaultError)>,
}

impl VaultState {
    /// Replays verified log entries (already chain-checked by `zafe_proto::Chain`).
    ///
    /// Only structural problems are fatal: the first entry must be a `Created` event signed
    /// by every member. After that, entries that cannot be decrypted, decoded or applied are
    /// recorded in `ignored` and skipped deterministically.
    pub fn replay(entries: &[LogEntry], key: &LogKey) -> Result<Self, VaultError> {
        let (first, rest) = entries.split_first().ok_or(VaultError::NotCreatedFirst)?;
        let event = VaultEvent::from_bytes(
            &first
                .decrypt(key)
                .map_err(|_| VaultError::Undecryptable(first.header.index))?,
        )?;
        let VaultEvent::Created {
            descriptor,
            signatures,
        } = event
        else {
            return Err(VaultError::NotCreatedFirst);
        };
        if descriptor.member(&first.header.author).is_none() {
            return Err(VaultError::NotAMember(first.header.index));
        }
        check_descriptor_signatures(&descriptor, &signatures)?;
        let mut state = VaultState {
            descriptor,
            proposals: BTreeMap::new(),
            applied: first.header.index + 1,
            ignored: Vec::new(),
        };
        for entry in rest {
            state.apply_entry(entry, key);
        }
        Ok(state)
    }

    /// Applies one more log entry, recording it in `ignored` if it is invalid.
    pub fn apply_entry(&mut self, entry: &LogEntry, key: &LogKey) {
        let index = entry.header.index;
        let result = entry
            .decrypt(key)
            .map_err(|_| VaultError::Undecryptable(index))
            .and_then(|bytes| VaultEvent::from_bytes(&bytes))
            .and_then(|event| self.apply(index, entry.header.author, event));
        if let Err(e) = result {
            self.ignored.push((index, e));
        }
        self.applied = index + 1;
    }

    /// Whether `event` by `author` would be valid on top of the current state. Callers
    /// check this before appending, so they don't write entries everyone will ignore.
    pub fn check(&self, author: [u8; 32], event: &VaultEvent) -> Result<(), VaultError> {
        self.clone().apply(self.applied, author, event.clone())
    }

    /// Applies one event authored by `author`. Also used to apply new entries incrementally.
    pub fn apply(
        &mut self,
        index: u64,
        author: [u8; 32],
        event: VaultEvent,
    ) -> Result<(), VaultError> {
        if self.descriptor.member(&author).is_none() {
            return Err(VaultError::NotAMember(index));
        }
        let threshold = usize::from(self.descriptor.threshold);
        let rejection_threshold = self.descriptor.rejection_threshold();
        match event {
            VaultEvent::Created { .. } => return Err(VaultError::DuplicateCreated),
            VaultEvent::Proposal {
                id,
                payments,
                pczt,
                pczt_hash,
                tip_height,
                created_at,
            } => {
                if self.proposals.contains_key(&id) {
                    return Err(VaultError::DuplicateProposal(index));
                }
                self.proposals.insert(
                    id,
                    ProposalState {
                        id,
                        author,
                        payments,
                        pczt,
                        pczt_hash,
                        tip_height,
                        created_at,
                        log_index: index,
                        status: ProposalStatus::Open,
                        approvals: BTreeMap::new(),
                        rejections: BTreeMap::new(),
                        txid: None,
                    },
                );
            }
            VaultEvent::Vote {
                proposal,
                pczt_hash,
                approve,
                commitments,
            } => {
                let p = self
                    .proposals
                    .get_mut(&proposal)
                    .ok_or(VaultError::UnknownProposal(index))?;
                // Approved proposals still accept fresh commitments (re-approval after a
                // failed signing round); rejected, cancelled or broadcast ones do not.
                if !matches!(p.status, ProposalStatus::Open | ProposalStatus::Approved) {
                    return Err(VaultError::ProposalClosed(index));
                }
                if pczt_hash != p.pczt_hash {
                    return Err(VaultError::PcztMismatch(index));
                }
                if approve {
                    p.rejections.remove(&author);
                    p.approvals.insert(author, commitments);
                } else {
                    p.approvals.remove(&author);
                    p.rejections.insert(author, ());
                }
                p.status = if p.rejections.len() >= rejection_threshold {
                    ProposalStatus::Rejected
                } else if p.approvals.len() >= threshold {
                    ProposalStatus::Approved
                } else {
                    ProposalStatus::Open
                };
            }
            VaultEvent::Cancelled { proposal } => {
                let p = self
                    .proposals
                    .get_mut(&proposal)
                    .ok_or(VaultError::UnknownProposal(index))?;
                if p.author != author {
                    return Err(VaultError::NotAuthor(index));
                }
                if !matches!(p.status, ProposalStatus::Open | ProposalStatus::Approved) {
                    return Err(VaultError::ProposalClosed(index));
                }
                p.status = ProposalStatus::Cancelled;
            }
            VaultEvent::Broadcast { proposal, txid } => {
                let p = self
                    .proposals
                    .get_mut(&proposal)
                    .ok_or(VaultError::UnknownProposal(index))?;
                if p.status != ProposalStatus::Approved {
                    return Err(VaultError::ProposalClosed(index));
                }
                p.status = ProposalStatus::Broadcast;
                p.txid = Some(txid);
            }
        }
        Ok(())
    }
}

fn check_descriptor_signatures(
    descriptor: &VaultDescriptor,
    signatures: &[([u8; 32], Vec<u8>)],
) -> Result<(), VaultError> {
    let message = descriptor.signing_message()?;
    for member in &descriptor.members {
        let ok = signatures.iter().any(|(pk, sig)| {
            pk == &member.identity.sig_pk && member.identity.verify(&message, sig).is_ok()
        });
        if !ok {
            return Err(VaultError::MissingDescriptorSignature);
        }
    }
    Ok(())
}
