//! Asynchronous FROST signing sessions for a proposal (spec §9.4–§9.5).
//!
//! - **Approve** (member): verify the PCZT, generate one nonce pair per spend to sign,
//!   store the secret nonces, and publish the commitments with the vote.
//! - **Signing request** (leader): once at least t members have approved, choose t of
//!   them and send one signing package per spend.
//! - **Sign** (member): re-verify everything, check each package against the local
//!   sighash and the member's own commitments, *delete* the nonces, then produce shares.
//! - **Aggregate** (leader): verify every share (cheater detection), aggregate one
//!   RedPallas signature per spend, and return them for injection into the PCZT.
//!
//! If a chosen member never answers, the leader picks another set of approvers whose
//! commitments are still unused and sends a new request.

use std::collections::{BTreeMap, BTreeSet};

use orchard::keys::FullViewingKey;
use pczt::Pczt;
use rand_core::{CryptoRng, RngCore};
use reddsa::frost::redpallas::{
    keys::{KeyPackage, PublicKeyPackage},
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
    Identifier, SigningPackage,
};

use crate::{
    signing::{self, SigningError},
    tx::SpendToSign,
    verify::{verify_pczt, Expectations, VerifiedTx, VerifyError},
};

const PERSONAL_PCZT_HASH: &[u8; 16] = b"Zafe_PCZT_Hash__";

pub type ProposalId = [u8; 16];

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Verify(#[from] VerifyError),
    #[error(transparent)]
    Signing(#[from] SigningError),
    #[error("PCZT serialization: {0}")]
    Encoding(String),
    #[error("the request is for a different PCZT than the proposal")]
    PcztMismatch,
    #[error("this member already approved this proposal")]
    AlreadyApproved,
    #[error("no unused nonces for this proposal (already signed, or never approved)")]
    NoNonces,
    #[error("request has {got} signing packages, but the PCZT has {expected} spends to sign")]
    WrongPackageCount { expected: usize, got: usize },
    #[error("signing package {0} does not sign the locally computed sighash")]
    WrongMessage(usize),
    #[error("signing package {0} does not contain this member's commitments")]
    NotOurCommitments(usize),
    #[error("need {needed} signers, got {got}")]
    NotEnoughSigners { needed: usize, got: usize },
    #[error("member has no commitments for this proposal")]
    UnknownApprover,
    #[error("share set from member does not cover every spend")]
    IncompleteShares,
}

/// `BLAKE2b-256("Zafe_PCZT_Hash__", serialized unsigned PCZT)`: binds votes and signing
/// requests to one exact proposal transaction.
pub fn pczt_hash(pczt: &Pczt) -> Result<[u8; 32], SessionError> {
    let bytes = pczt
        .clone()
        .serialize()
        .map_err(|e| SessionError::Encoding(format!("{e:?}")))?;
    let hash = blake2b_simd::Params::new()
        .hash_length(32)
        .personal(PERSONAL_PCZT_HASH)
        .hash(&bytes);
    Ok(hash.as_bytes().try_into().expect("32 bytes"))
}

/// Where a member keeps secret nonces between approving and signing.
///
/// Implementations on devices MUST use this-device-only storage excluded from backups
/// (spec §9.4). `take` must remove the nonces *before* returning them.
pub trait NonceStore {
    fn contains(&self, proposal: &ProposalId, pczt_hash: &[u8; 32]) -> bool;
    fn put(&mut self, proposal: ProposalId, pczt_hash: [u8; 32], nonces: Vec<SigningNonces>);
    fn take(&mut self, proposal: &ProposalId, pczt_hash: &[u8; 32]) -> Option<Vec<SigningNonces>>;
}

/// In-memory nonce store (tests and the CLI).
#[derive(Default)]
pub struct MemoryNonceStore(BTreeMap<(ProposalId, [u8; 32]), Vec<SigningNonces>>);

impl NonceStore for MemoryNonceStore {
    fn contains(&self, proposal: &ProposalId, pczt_hash: &[u8; 32]) -> bool {
        self.0.contains_key(&(*proposal, *pczt_hash))
    }
    fn put(&mut self, proposal: ProposalId, pczt_hash: [u8; 32], nonces: Vec<SigningNonces>) {
        self.0.insert((proposal, pczt_hash), nonces);
    }
    fn take(&mut self, proposal: &ProposalId, pczt_hash: &[u8; 32]) -> Option<Vec<SigningNonces>> {
        self.0.remove(&(*proposal, *pczt_hash))
    }
}

/// What a member publishes with an Approve vote: one commitment per spend to sign,
/// in `spends_to_sign` order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approval {
    pub member: Identifier,
    pub proposal: ProposalId,
    pub pczt_hash: [u8; 32],
    pub commitments: Vec<SigningCommitments>,
}

/// A member's context for one proposal.
pub struct Member<'a> {
    pub identifier: Identifier,
    pub key_package: &'a KeyPackage,
    pub vault_fvk: &'a FullViewingKey,
}

impl Member<'_> {
    /// Verifies the proposal and, if it passes, generates and stores nonces and returns
    /// the approval to publish, plus the verified facts for display.
    pub fn approve<R: RngCore + CryptoRng>(
        &self,
        proposal: ProposalId,
        pczt: &Pczt,
        expected: &Expectations,
        store: &mut impl NonceStore,
        rng: &mut R,
    ) -> Result<(Approval, VerifiedTx), SessionError> {
        let verified = verify_pczt(pczt, self.vault_fvk, expected)?;
        let hash = pczt_hash(pczt)?;
        if store.contains(&proposal, &hash) {
            return Err(SessionError::AlreadyApproved);
        }
        let (nonces, commitments): (Vec<_>, Vec<_>) = verified
            .spends_to_sign
            .iter()
            .map(|_| signing::commit(self.key_package, rng))
            .unzip();
        store.put(proposal, hash, nonces);
        Ok((
            Approval {
                member: self.identifier,
                proposal,
                pczt_hash: hash,
                commitments,
            },
            verified,
        ))
    }

    /// Round 2. Re-verifies the PCZT, checks every package, deletes the nonces, and
    /// returns one share per spend. The nonces are gone even if signing then fails:
    /// a nonce is never used twice.
    pub fn sign(
        &self,
        request: &SigningRequest,
        pczt: &Pczt,
        expected: &Expectations,
        store: &mut impl NonceStore,
    ) -> Result<Vec<SignatureShare>, SessionError> {
        let verified = verify_pczt(pczt, self.vault_fvk, expected)?;
        if pczt_hash(pczt)? != request.pczt_hash {
            return Err(SessionError::PcztMismatch);
        }
        let spends = &verified.spends_to_sign;
        if request.packages.len() != spends.len() {
            return Err(SessionError::WrongPackageCount {
                expected: spends.len(),
                got: request.packages.len(),
            });
        }
        for (i, package) in request.packages.iter().enumerate() {
            if package.message() != verified.sighash.as_slice() {
                return Err(SessionError::WrongMessage(i));
            }
        }

        let nonces = store
            .take(&request.proposal, &request.pczt_hash)
            .ok_or(SessionError::NoNonces)?;
        if nonces.len() != spends.len() {
            return Err(SessionError::NoNonces);
        }
        for (i, (package, nonce)) in request.packages.iter().zip(&nonces).enumerate() {
            if package.signing_commitment(&self.identifier) != Some(*nonce.commitments()) {
                return Err(SessionError::NotOurCommitments(i));
            }
        }

        // alpha comes from this member's own verification of the PCZT, never the request.
        let mut shares = Vec::with_capacity(spends.len());
        for ((package, nonce), spend) in request.packages.iter().zip(nonces).zip(spends) {
            shares.push(signing::sign(
                package,
                nonce,
                self.key_package,
                spend.alpha,
            )?);
        }
        Ok(shares)
    }
}

/// Sent by the leader to the chosen signers: one signing package per spend.
#[derive(Clone, Debug)]
pub struct SigningRequest {
    pub proposal: ProposalId,
    pub pczt_hash: [u8; 32],
    pub signers: BTreeSet<Identifier>,
    pub packages: Vec<SigningPackage>,
}

/// The leader's view of a proposal's signing session.
pub struct Leader {
    proposal: ProposalId,
    pczt_hash: [u8; 32],
    threshold: usize,
    sighash: [u8; 32],
    spends: Vec<SpendToSign>,
    approvals: BTreeMap<Identifier, Vec<SigningCommitments>>,
    /// Members whose commitments were already put in a request (not reusable).
    used: BTreeSet<Identifier>,
}

impl Leader {
    /// Starts from the leader's own verification of the proposal PCZT.
    pub fn new(
        proposal: ProposalId,
        pczt: &Pczt,
        verified: &VerifiedTx,
        threshold: u16,
    ) -> Result<Self, SessionError> {
        Ok(Self {
            proposal,
            pczt_hash: pczt_hash(pczt)?,
            threshold: usize::from(threshold),
            sighash: verified.sighash,
            spends: verified.spends_to_sign.clone(),
            approvals: BTreeMap::new(),
            used: BTreeSet::new(),
        })
    }

    /// Records an approval, or fresh commitments from a member re-approving after a
    /// failed round (their earlier nonces are gone). Ignores approvals for other PCZTs or
    /// with the wrong shape.
    pub fn add_approval(&mut self, approval: Approval) -> bool {
        let valid = approval.proposal == self.proposal
            && approval.pczt_hash == self.pczt_hash
            && approval.commitments.len() == self.spends.len();
        if valid && self.approvals.get(&approval.member) != Some(&approval.commitments) {
            self.used.remove(&approval.member);
            self.approvals.insert(approval.member, approval.commitments);
        }
        valid
    }

    /// Approvers whose commitments have not been used in a request yet.
    pub fn available_signers(&self) -> Vec<Identifier> {
        self.approvals
            .keys()
            .filter(|id| !self.used.contains(id))
            .copied()
            .collect()
    }

    /// Builds a signing request for exactly `threshold` approvers whose commitments are
    /// unused. Marks their commitments as used.
    pub fn request(&mut self, signers: &[Identifier]) -> Result<SigningRequest, SessionError> {
        let signers: BTreeSet<Identifier> = signers.iter().copied().collect();
        if signers.len() != self.threshold {
            return Err(SessionError::NotEnoughSigners {
                needed: self.threshold,
                got: signers.len(),
            });
        }
        if signers
            .iter()
            .any(|id| !self.approvals.contains_key(id) || self.used.contains(id))
        {
            return Err(SessionError::UnknownApprover);
        }
        let packages = (0..self.spends.len())
            .map(|spend| {
                let commitments = signers
                    .iter()
                    .map(|id| (*id, self.approvals[id][spend]))
                    .collect();
                signing::signing_package(commitments, &self.sighash)
            })
            .collect();
        self.used.extend(signers.iter().copied());
        Ok(SigningRequest {
            proposal: self.proposal,
            pczt_hash: self.pczt_hash,
            signers,
            packages,
        })
    }

    /// Aggregates one signature per spend. Every share is verified; an invalid share
    /// yields a FROST error naming the culprit.
    pub fn aggregate(
        &self,
        request: &SigningRequest,
        shares: &BTreeMap<Identifier, Vec<SignatureShare>>,
        public_key_package: &PublicKeyPackage,
    ) -> Result<Vec<(usize, [u8; 64])>, SessionError> {
        if shares.keys().copied().collect::<BTreeSet<_>>() != request.signers {
            return Err(SessionError::NotEnoughSigners {
                needed: request.signers.len(),
                got: shares.len(),
            });
        }
        let mut signatures = Vec::with_capacity(self.spends.len());
        for (i, (spend, package)) in self.spends.iter().zip(&request.packages).enumerate() {
            let per_spend = shares
                .iter()
                .map(|(id, s)| {
                    s.get(i)
                        .map(|share| (*id, *share))
                        .ok_or(SessionError::IncompleteShares)
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            let sig = signing::aggregate(package, &per_spend, public_key_package, spend.alpha)?;
            signatures.push((spend.action_index, sig));
        }
        Ok(signatures)
    }
}
