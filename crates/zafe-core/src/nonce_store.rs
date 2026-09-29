//! File-backed FROST nonce store (one file per proposal and PCZT).
//!
//! Nonces are secret and single-use. On devices the directory must be app-private and
//! excluded from backups and device transfer (spec §9.4): Android `allowBackup="false"`,
//! iOS `isExcludedFromBackup`. Restoring an old copy could make a member sign twice with the
//! same nonces, which leaks their key share.

use std::{fs, path::PathBuf};

use reddsa::frost::redpallas::round1::{SigningCommitments, SigningNonces};

use crate::session::{NonceStore, ProposalId, SessionError};

pub struct FileNonceStore(PathBuf);

impl FileNonceStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self(dir.into())
    }

    fn file(&self, proposal: &ProposalId, hash: &[u8; 32]) -> PathBuf {
        self.0.join(format!(
            "{}-{}.bin",
            hex::encode(proposal),
            hex::encode(hash)
        ))
    }

    fn read(&self, proposal: &ProposalId, hash: &[u8; 32]) -> Option<Vec<SigningNonces>> {
        let encoded: Vec<Vec<u8>> =
            postcard::from_bytes(&fs::read(self.file(proposal, hash)).ok()?).ok()?;
        encoded
            .iter()
            .map(|b| SigningNonces::deserialize(b).ok())
            .collect()
    }
}

impl NonceStore for FileNonceStore {
    fn contains(&self, proposal: &ProposalId, hash: &[u8; 32]) -> bool {
        self.file(proposal, hash).exists()
    }

    fn commitments(
        &self,
        proposal: &ProposalId,
        hash: &[u8; 32],
    ) -> Option<Vec<SigningCommitments>> {
        Some(
            self.read(proposal, hash)?
                .iter()
                .map(|n| *n.commitments())
                .collect(),
        )
    }

    fn put(
        &mut self,
        proposal: ProposalId,
        hash: [u8; 32],
        nonces: Vec<SigningNonces>,
    ) -> Result<(), SessionError> {
        let storage = |e: &dyn std::fmt::Debug| SessionError::Storage(format!("{e:?}"));
        let encoded = nonces
            .iter()
            .map(|n| n.serialize().map_err(|e| storage(&e)))
            .collect::<Result<Vec<_>, _>>()?;
        let bytes = postcard::to_allocvec(&encoded).map_err(|e| storage(&e))?;
        fs::create_dir_all(&self.0).map_err(|e| storage(&e))?;
        // Write then rename, so a crash never leaves a truncated nonce file.
        let path = self.file(&proposal, &hash);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, bytes).map_err(|e| storage(&e))?;
        fs::rename(&tmp, &path).map_err(|e| storage(&e))
    }

    fn take(&mut self, proposal: &ProposalId, hash: &[u8; 32]) -> Option<Vec<SigningNonces>> {
        let nonces = self.read(proposal, hash)?;
        fs::remove_file(self.file(proposal, hash)).ok()?; // delete before use: never reusable
        Some(nonces)
    }
}
