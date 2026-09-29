//! Zafe wire protocol: member identities, signed envelopes and HPKE sealing
//! (spec §5). Transport-agnostic and free of Zcash dependencies, so the relay can use it.

pub mod envelope;
pub mod identity;
pub mod log;

pub use envelope::{Envelope, Header, Kind, MailboxId, Recipient, ReplayGuard};
pub use identity::{safety_number, Identity, IdentityPublic, IdentitySeeds};
pub use log::{Chain, ChainError, LogEntry, LogKey};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProtoError {
    #[error("unsupported protocol version {0}")]
    UnsupportedVersion(u8),
    #[error("invalid public key")]
    BadKey,
    #[error("invalid signature")]
    BadSignature,
    #[error("envelope sender does not match the expected identity")]
    WrongSender,
    #[error("envelope is sealed to another member")]
    NotForMe,
    #[error("decryption failed")]
    Crypto,
    #[error("encoding error")]
    Encoding,
    #[error("replayed or reordered envelope (seq {seq}, last seen {last})")]
    Replay { seq: u64, last: u64 },
}
