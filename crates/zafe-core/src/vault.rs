//! Vault descriptor, vault-log events, and replay of the log into current state
//! (spec §6.3, §7.3, §9.1, §9.6).
//!
//! The relay only orders encrypted entries; every rule about what an entry may do is
//! enforced here, by every member, when replaying the log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zafe_proto::{
    version::{self, DecodeError, Format, UnsupportedVersion},
    IdentityPublic, LogEntry, LogKey,
};

use crate::session::ProposalId;

const PERSONAL_DESCRIPTOR: &[u8; 16] = b"Zafe_VaultDescr_";
const PERSONAL_EVENT_SIG: &[u8] = b"Zafe descriptor signature v1";

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum VaultError {
    #[error("encoding")]
    Encoding,
    /// An event or descriptor in a version this build doesn't read. Fatal for the
    /// `Created` entry; any later entry is ignored like other invalid entries.
    #[error(transparent)]
    UnsupportedVersion(#[from] UnsupportedVersion),
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
    #[error("entry {0}: invalid commitment batch")]
    BadCommitments(u64),
    #[error("entry {0}: signature shares that don't fit the proposal's signer groups")]
    BadShares(u64),
    #[error("entry {0}: this member already signed; an approval with shares is final")]
    VoteFinal(u64),
}

/// Most commitments one `Commitments` event may carry.
pub const MAX_COMMITMENT_BATCH: usize = 256;
/// Most unused commitments a member may have outstanding in the log.
pub const MAX_POOL_OUTSTANDING: usize = 4096;
/// Most signer groups (t-subsets) a proposal may be preprocessed for; beyond this the
/// proposal uses interactive signing. C(n, t): 2-of-3 = 3, 3-of-5 = 10, 4-of-7 = 35.
pub const MAX_PREPROCESSED_SUBSETS: usize = 64;

/// A member's pre-published FROST round-1 commitments (spec §9.4, preprocessing). Every
/// proposal takes the next unused ones in log order, so each is assigned at most once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pool {
    /// Serialized `SigningCommitments`, in publication order.
    pub commitments: Vec<Vec<u8>>,
    /// Index of the next unassigned commitment.
    pub next: usize,
}

impl Pool {
    pub fn available(&self) -> usize {
        self.commitments.len() - self.next
    }
}

/// One-tap signing for a proposal: every signer group (t-subset of members, in descriptor
/// order) with the commitments fixed for it when the proposal was logged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preprocessed {
    /// Member `sig_pk`s of each group, in descriptor order.
    pub subsets: Vec<Vec<[u8; 32]>>,
    /// `commitments[group][spend][position in group]`, serialized.
    pub commitments: Vec<Vec<Vec<Vec<u8>>>>,
}

impl Preprocessed {
    /// The commitment assigned to `member` for `group` and `spend`, if it is in the group.
    pub fn commitment(&self, group: usize, spend: usize, member: &[u8; 32]) -> Option<&[u8]> {
        let pos = self.subsets.get(group)?.iter().position(|m| m == member)?;
        Some(self.commitments.get(group)?.get(spend)?.get(pos)?)
    }

    /// Groups that include `member`.
    pub fn groups_of(&self, member: &[u8; 32]) -> Vec<usize> {
        (0..self.subsets.len())
            .filter(|g| self.subsets[*g].contains(member))
            .collect()
    }
}

/// A member's signature shares for one signer group: one share per spend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupShares {
    pub group: u16,
    pub shares: Vec<Vec<u8>>,
}

/// All t-subsets of `items` (as index lists), in lexicographic order.
fn combinations(n: usize, t: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut idx: Vec<usize> = (0..t).collect();
    if t == 0 || t > n {
        return out;
    }
    loop {
        out.push(idx.clone());
        let mut i = t;
        while i > 0 && idx[i - 1] == n - t + i - 1 {
            i -= 1;
        }
        if i == 0 {
            return out;
        }
        idx[i - 1] += 1;
        for j in i..t {
            idx[j] = idx[j - 1] + 1;
        }
    }
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
    /// [`version::DESCRIPTOR`]. Part of the hash every member signs, so it can't be changed
    /// after creation.
    pub version: u16,
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
        /// Spends needing a FROST signature (members check it against the PCZT). With
        /// enough pre-published commitments, the proposal is signed at approval time.
        signing_spends: u16,
        /// Whether the member whose approval completes a signer group sends right away
        /// (otherwise any member sends when they choose).
        auto_send: bool,
    },
    Vote {
        proposal: ProposalId,
        pczt_hash: [u8; 32],
        approve: bool,
        /// Interactive signing: fresh round-1 commitments, one per spend (empty for
        /// rejections and one-tap approvals).
        commitments: Vec<Vec<u8>>,
        /// One-tap signing: this member's shares for the proposal's signer groups.
        shares: Vec<GroupShares>,
    },
    /// Pre-published FROST round-1 commitments of the author (preprocessing).
    Commitments {
        batch: Vec<Vec<u8>>,
    },
    Cancelled {
        proposal: ProposalId,
    },
    Broadcast {
        proposal: ProposalId,
        txid: [u8; 32],
    },
}

impl From<DecodeError> for VaultError {
    fn from(e: DecodeError) -> Self {
        match e {
            DecodeError::Unsupported(v) => VaultError::UnsupportedVersion(v),
            DecodeError::Malformed(_) => VaultError::Encoding,
        }
    }
}

impl VaultEvent {
    /// `version (u16) || postcard(event)`: the plaintext of a log entry.
    pub fn to_bytes(&self) -> Result<Vec<u8>, VaultError> {
        Ok(version::encode(Format::VaultEvent, self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VaultError> {
        Ok(version::decode(Format::VaultEvent, bytes)?)
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
    /// Latest approval per member: their serialized commitments (empty for one-tap).
    pub approvals: BTreeMap<[u8; 32], Vec<Vec<u8>>>,
    pub rejections: BTreeMap<[u8; 32], ()>,
    pub txid: Option<[u8; 32]>,
    pub signing_spends: u16,
    pub auto_send: bool,
    /// Set when the proposal is signed at approval time (enough commitments were in the
    /// members' pools when it was logged); `None` means interactive signing.
    pub preprocessed: Option<Preprocessed>,
    /// One-tap shares posted per member.
    pub shares: BTreeMap<[u8; 32], Vec<GroupShares>>,
    /// First signer group (in group order) with every member's shares: ready to send.
    pub ready_group: Option<u16>,
    /// The member whose approval completed `ready_group` (the one who auto-sends).
    pub completed_by: Option<[u8; 32]>,
}

impl ProposalState {
    fn find_ready_group(&self) -> Option<u16> {
        let pre = self.preprocessed.as_ref()?;
        (0..pre.subsets.len())
            .find(|g| {
                pre.subsets[*g].iter().all(|m| {
                    self.shares
                        .get(m)
                        .is_some_and(|gs| gs.iter().any(|s| usize::from(s.group) == *g))
                })
            })
            .map(|g| g as u16)
    }
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
    /// Pre-published commitments per member.
    pub pools: BTreeMap<[u8; 32], Pool>,
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
        version::check(Format::Descriptor, descriptor.version)?;
        if descriptor.member(&first.header.author).is_none() {
            return Err(VaultError::NotAMember(first.header.index));
        }
        check_descriptor_signatures(&descriptor, &signatures)?;
        let mut state = VaultState {
            descriptor,
            proposals: BTreeMap::new(),
            applied: first.header.index + 1,
            ignored: Vec::new(),
            pools: BTreeMap::new(),
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

    /// Entries skipped because they come from a newer version of Zafe: other members can
    /// see something this build can't, so the app should ask to update.
    pub fn newer_version_entries(&self) -> usize {
        self.ignored
            .iter()
            .filter(|(_, e)| matches!(e, VaultError::UnsupportedVersion(v) if v.is_newer()))
            .count()
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
                signing_spends,
                auto_send,
            } => {
                if self.proposals.contains_key(&id) {
                    return Err(VaultError::DuplicateProposal(index));
                }
                let preprocessed = self.assign_commitments(signing_spends);
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
                        signing_spends,
                        auto_send,
                        preprocessed,
                        shares: BTreeMap::new(),
                        ready_group: None,
                        completed_by: None,
                    },
                );
            }
            VaultEvent::Vote {
                proposal,
                pczt_hash,
                approve,
                commitments,
                shares,
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
                // Once a member has released signature shares, their approval cannot be
                // withdrawn or replaced (the shares are out).
                if p.shares.contains_key(&author) {
                    return Err(VaultError::VoteFinal(index));
                }
                if !shares.is_empty() {
                    let pre = p
                        .preprocessed
                        .as_ref()
                        .ok_or(VaultError::BadShares(index))?;
                    let mut seen = std::collections::BTreeSet::new();
                    let valid = approve
                        && commitments.is_empty()
                        && shares.iter().all(|gs| {
                            let g = usize::from(gs.group);
                            seen.insert(g)
                                && pre.subsets.get(g).is_some_and(|m| m.contains(&author))
                                && gs.shares.len() == usize::from(p.signing_spends)
                        });
                    if !valid {
                        return Err(VaultError::BadShares(index));
                    }
                    p.shares.insert(author, shares);
                    if p.ready_group.is_none() {
                        p.ready_group = p.find_ready_group();
                        if p.ready_group.is_some() {
                            p.completed_by = Some(author);
                        }
                    }
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
            VaultEvent::Commitments { batch } => {
                use reddsa::frost::redpallas::round1::SigningCommitments;
                let pool = self.pools.entry(author).or_default();
                if batch.is_empty()
                    || batch.len() > MAX_COMMITMENT_BATCH
                    || pool.available() + batch.len() > MAX_POOL_OUTSTANDING
                    || batch
                        .iter()
                        .any(|c| SigningCommitments::deserialize(c).is_err())
                {
                    return Err(VaultError::BadCommitments(index));
                }
                pool.commitments.extend(batch);
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

impl VaultState {
    /// Fixes the signing commitments of a new proposal: for every signer group (t-subset of
    /// members, descriptor order), every spend and every member of the group, the member's
    /// next unused pool commitment. Deterministic from the log, so every member computes the
    /// same assignment; each commitment is assigned at most once. `None` (interactive
    /// signing) if the vault has too many groups or any pool is short.
    fn assign_commitments(&mut self, spends: u16) -> Option<Preprocessed> {
        let members: Vec<[u8; 32]> = self
            .descriptor
            .members
            .iter()
            .map(|m| m.identity.sig_pk)
            .collect();
        let t = usize::from(self.descriptor.threshold);
        let groups = combinations(members.len(), t);
        let spends = usize::from(spends);
        if spends == 0 || groups.is_empty() || groups.len() > MAX_PREPROCESSED_SUBSETS {
            return None;
        }
        let per_member = groups.iter().filter(|g| g.contains(&0)).count() * spends;
        if members
            .iter()
            .any(|m| self.pools.get(m).map_or(0, Pool::available) < per_member)
        {
            return None;
        }
        let mut commitments = Vec::with_capacity(groups.len());
        for group in &groups {
            let mut per_spend = Vec::with_capacity(spends);
            for _ in 0..spends {
                let mut per_member = Vec::with_capacity(t);
                for &i in group {
                    let pool = self.pools.get_mut(&members[i]).expect("checked above");
                    per_member.push(pool.commitments[pool.next].clone());
                    pool.next += 1;
                }
                per_spend.push(per_member);
            }
            commitments.push(per_spend);
        }
        Some(Preprocessed {
            subsets: groups
                .iter()
                .map(|g| g.iter().map(|&i| members[i]).collect())
                .collect(),
            commitments,
        })
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
