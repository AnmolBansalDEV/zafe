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

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
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
    /// Number of log entries applied.
    pub applied: u64,
}

impl VaultState {
    /// Replays verified log entries (already chain-checked by `zafe_proto::Chain`).
    pub fn replay(entries: &[LogEntry], key: &LogKey) -> Result<Self, VaultError> {
        let mut state: Option<VaultState> = None;
        for entry in entries {
            let index = entry.header.index;
            let event = VaultEvent::from_bytes(
                &entry
                    .decrypt(key)
                    .map_err(|_| VaultError::Undecryptable(index))?,
            )?;
            match (&mut state, event) {
                (
                    None,
                    VaultEvent::Created {
                        descriptor,
                        signatures,
                    },
                ) => {
                    if descriptor.member(&entry.header.author).is_none() {
                        return Err(VaultError::NotAMember(index));
                    }
                    check_descriptor_signatures(&descriptor, &signatures)?;
                    state = Some(VaultState {
                        descriptor,
                        proposals: BTreeMap::new(),
                        applied: 0,
                    });
                }
                (None, _) => return Err(VaultError::NotCreatedFirst),
                (Some(_), VaultEvent::Created { .. }) => return Err(VaultError::DuplicateCreated),
                (Some(s), event) => s.apply(index, entry.header.author, event)?,
            }
            if let Some(s) = &mut state {
                s.applied = index + 1;
            }
        }
        state.ok_or(VaultError::NotCreatedFirst)
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
