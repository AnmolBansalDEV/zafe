//! Encrypted vault backups (spec §12.2): export one vault's secrets from a device and
//! restore them on another.
//!
//! Contents: the member's identity seeds, the vault material (FROST key package, vault
//! secret `sk`, descriptor, log key; `use_qsk` is part of the descriptor) and the invite.
//! **Never** FROST nonces or pre-published pool nonces: a restored device must never be
//! able to reuse a nonce, so it starts with none and publishes a fresh pool.
//!
//! Format (all integers little-endian):
//!
//! ```text
//! magic "ZAFEBAK" (7) | version u8 = version::BACKUP | m_cost_kib u32 | t_cost u32 | p_cost u32
//! | salt (16) | nonce (24) | XChaCha20-Poly1305 ciphertext of postcard(Contents)
//! ```
//!
//! The key is Argon2id(passphrase, salt) with the header's parameters (default 64 MiB,
//! 3 passes, 1 lane); the whole header is the AEAD's associated data, so tampering with
//! parameters or salt fails decryption. A text form (`zafe-backup-v1:` + base64url) carries
//! the same bytes for password managers and copy/paste.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use zafe_proto::{
    version::{self, Format, UnsupportedVersion},
    IdentitySeeds, ProtoError,
};

use crate::node::{NodeError, VaultMaterial};

const MAGIC: &[u8; 7] = b"ZAFEBAK";
const VERSION: u8 = version::BACKUP as u8;
const HEADER_LEN: usize = 7 + 1 + 4 * 3 + 16 + 24;
const TEXT_PREFIX: &str = "zafe-backup-v1:";

/// Argon2id cost: at least the spec's 64 MiB and 3 passes for real backups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl KdfParams {
    pub const DEFAULT: Self = Self {
        m_cost_kib: 64 * 1024,
        t_cost: 3,
        p_cost: 1,
    };
    /// Minimum accepted on import (an attacker-supplied file can't make restoring weak
    /// backups look normal, and can't request absurd costs either: see `MAX_M_COST_KIB`).
    const MIN_M_COST_KIB: u32 = 64 * 1024;
    const MIN_T_COST: u32 = 3;
    const MAX_M_COST_KIB: u32 = 1024 * 1024;
    const MAX_T_COST: u32 = 16;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum BackupError {
    #[error("not a Zafe vault backup")]
    NotABackup,
    /// The backup, or the identity or material inside it, is in another version.
    #[error(transparent)]
    UnsupportedVersion(#[from] UnsupportedVersion),
    #[error("wrong passphrase, or the backup is damaged")]
    WrongPassphraseOrDamaged,
    #[error("the backup's key-derivation settings are out of range")]
    BadParams,
    #[error("the backup's identity is not a member of its vault")]
    NotAMember,
    #[error("passphrase too weak: {0}")]
    WeakPassphrase(String),
    #[error("encoding: {0}")]
    Encoding(String),
}

/// What a backup holds.
#[derive(Clone, Serialize, Deserialize)]
pub struct Contents {
    /// `IdentitySeeds::to_bytes` (versioned), as stored on the device.
    pub identity_seeds: Vec<u8>,
    /// `VaultMaterial::to_bytes` (versioned), as stored on the device.
    pub material: Vec<u8>,
    pub invite: String,
    /// Unix seconds when the backup was made (display only).
    pub created_at: u64,
}

/// Redacted: backups hold secrets, so they never go to logs.
impl core::fmt::Debug for Contents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Contents")
            .field("created_at", &self.created_at)
            .finish_non_exhaustive()
    }
}

impl Contents {
    /// Checks that the contents are a usable vault: the material parses and the identity
    /// is one of the vault's members.
    pub fn validate(&self) -> Result<VaultMaterial, BackupError> {
        let material = VaultMaterial::from_bytes(&self.material).map_err(|e| match e {
            NodeError::UnsupportedVersion(v) => BackupError::UnsupportedVersion(v),
            _ => BackupError::Encoding("vault material".into()),
        })?;
        let seeds = IdentitySeeds::from_bytes(&self.identity_seeds).map_err(|e| match e {
            ProtoError::UnsupportedVersion(v) => BackupError::UnsupportedVersion(v),
            _ => BackupError::Encoding("identity".into()),
        })?;
        let me = zafe_proto::Identity::from_seeds(seeds);
        if material.descriptor.member(&me.public().sig_pk).is_none() {
            return Err(BackupError::NotAMember);
        }
        Ok(material)
    }
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    p: KdfParams,
) -> Result<Zeroizing<[u8; 32]>, BackupError> {
    let params = Params::new(p.m_cost_kib, p.t_cost, p.p_cost, Some(32))
        .map_err(|_| BackupError::BadParams)?;
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|_| BackupError::BadParams)?;
    Ok(key)
}

/// Encrypts `contents` under `passphrase` (checked with [`check_passphrase`] first).
pub fn encrypt<R: RngCore + CryptoRng>(
    contents: &Contents,
    passphrase: &str,
    params: KdfParams,
    rng: &mut R,
) -> Result<Vec<u8>, BackupError> {
    check_passphrase(passphrase)?;
    contents.validate()?;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce);
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.push(VERSION);
    for v in [params.m_cost_kib, params.t_cost, params.p_cost] {
        header.extend_from_slice(&v.to_le_bytes());
    }
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce);
    let key = derive_key(passphrase, &salt, params)?;
    let plain = Zeroizing::new(
        postcard::to_allocvec(contents).map_err(|e| BackupError::Encoding(e.to_string()))?,
    );
    let ciphertext = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(key.as_ref()))
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: &header,
            },
        )
        .map_err(|_| BackupError::Encoding("encryption".into()))?;
    let mut out = header;
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Whether `bytes` look like a Zafe backup (before asking for the passphrase).
pub fn is_backup(bytes: &[u8]) -> bool {
    bytes.len() > HEADER_LEN && bytes.starts_with(MAGIC)
}

/// Decrypts and validates a backup.
pub fn decrypt(bytes: &[u8], passphrase: &str) -> Result<Contents, BackupError> {
    if !is_backup(bytes) {
        return Err(BackupError::NotABackup);
    }
    version::check(Format::Backup, u16::from(bytes[7]))?;
    let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4"));
    let params = KdfParams {
        m_cost_kib: u32_at(8),
        t_cost: u32_at(12),
        p_cost: u32_at(16),
    };
    if params.m_cost_kib < KdfParams::MIN_M_COST_KIB
        || params.m_cost_kib > KdfParams::MAX_M_COST_KIB
        || params.t_cost < KdfParams::MIN_T_COST
        || params.t_cost > KdfParams::MAX_T_COST
        || params.p_cost == 0
        || params.p_cost > 8
    {
        return Err(BackupError::BadParams);
    }
    let (header, ciphertext) = bytes.split_at(HEADER_LEN);
    let salt = &header[20..36];
    let nonce = &header[36..60];
    let key = derive_key(passphrase, salt, params)?;
    let plain = Zeroizing::new(
        XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(key.as_ref()))
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad: header,
                },
            )
            .map_err(|_| BackupError::WrongPassphraseOrDamaged)?,
    );
    let contents: Contents = postcard::from_bytes(&plain)
        .map_err(|_| BackupError::Encoding("backup contents".into()))?;
    contents.validate()?;
    Ok(contents)
}

/// The text form of a backup (for password managers and copy/paste).
pub fn to_text(bytes: &[u8]) -> String {
    use base64::Engine;
    format!(
        "{TEXT_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

/// Parses the text form (whitespace and line breaks are ignored).
pub fn from_text(text: &str) -> Result<Vec<u8>, BackupError> {
    use base64::Engine;
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact
        .strip_prefix(TEXT_PREFIX)
        .ok_or(BackupError::NotABackup)?;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(body)
        .map_err(|_| BackupError::NotABackup)
}

/// A backup passphrase must be at least 12 words (e.g. a generated one) or rated
/// "very strong" (zxcvbn score 4) (spec §12.2).
pub fn check_passphrase(passphrase: &str) -> Result<(), BackupError> {
    let words = passphrase.split_whitespace().count();
    if words >= 12 {
        return Ok(());
    }
    let entropy = zxcvbn::zxcvbn(passphrase, &["zafe", "vault", "zcash"]);
    if entropy.score() >= zxcvbn::Score::Four {
        return Ok(());
    }
    let hint = entropy
        .feedback()
        .and_then(|f| f.warning().map(|w| w.to_string()))
        .unwrap_or_else(|| {
            "use 12 or more words, or a longer passphrase with unrelated words".into()
        });
    Err(BackupError::WeakPassphrase(hint))
}

/// A strong generated passphrase: 12 random words from the BIP-39 English list (128 bits).
pub fn suggest_passphrase<R: RngCore + CryptoRng>(rng: &mut R) -> String {
    let mut entropy = Zeroizing::new([0u8; 16]);
    rng.fill_bytes(entropy.as_mut());
    <bip0039::Mnemonic>::from_entropy(entropy.to_vec())
        .expect("16 bytes is a valid entropy size")
        .phrase()
        .to_owned()
}
