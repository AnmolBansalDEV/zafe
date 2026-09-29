//! Relay API types (spec §6). All bodies are postcard-encoded; every request is signed by
//! a member identity. The relay never sees plaintext vault data.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    envelope::{encode, Envelope, MailboxId},
    identity::{Identity, IdentityPublic},
    log::LogEntry,
    ProtoError,
};

const SIGNATURE_DOMAIN: &[u8] = b"Zafe relay request v1";

/// Read requests must be at most this old (and not from the future) when they arrive.
pub const MAX_REQUEST_SKEW_SECS: u64 = 300;

/// A request payload signed by `signer`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signed<T> {
    pub payload: T,
    pub signer: IdentityPublic,
    pub signature: Vec<u8>,
}

impl<T: Serialize + DeserializeOwned> Signed<T> {
    pub fn new(identity: &Identity, payload: T) -> Result<Self, ProtoError> {
        let signature = identity.sign(&signed_bytes(&payload)?).to_vec();
        Ok(Self {
            payload,
            signer: *identity.public(),
            signature,
        })
    }

    /// Checks the signature against the embedded signer. Callers must separately check
    /// that the signer is allowed to make the request.
    pub fn verify(&self) -> Result<&T, ProtoError> {
        self.signer
            .verify(&signed_bytes(&self.payload)?, &self.signature)?;
        Ok(&self.payload)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ProtoError> {
        encode(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtoError> {
        postcard::from_bytes(bytes).map_err(|_| ProtoError::Encoding)
    }
}

fn signed_bytes<T: Serialize>(payload: &T) -> Result<Vec<u8>, ProtoError> {
    let mut out = SIGNATURE_DOMAIN.to_vec();
    out.extend_from_slice(&encode(payload)?);
    Ok(out)
}

/// Opens a mailbox for a vault being created. Signed by the creator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMailbox {
    pub mailbox: MailboxId,
    /// BLAKE2b hash of the one-time join token carried in the invite.
    pub join_token_hash: [u8; 32],
}

/// Joins a mailbox using the invite's token. Signed by the joining member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Join {
    pub mailbox: MailboxId,
    pub join_token: [u8; 32],
}

/// Freezes membership to exactly `members` (their Ed25519 keys). Signed by the creator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    pub mailbox: MailboxId,
    pub members: Vec<[u8; 32]>,
}

/// Reads envelopes delivered to the signer after `after` (exclusive cursor).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxRead {
    pub mailbox: MailboxId,
    pub after: u64,
    pub timestamp: u64,
}

/// Reads log entries from index `from`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRead {
    pub mailbox: MailboxId,
    pub from: u64,
    pub timestamp: u64,
}

/// Lists the mailbox's current members.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembersRead {
    pub mailbox: MailboxId,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembersResponse {
    pub members: Vec<IdentityPublic>,
    pub sealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxResponse {
    /// `(cursor, envelope)` in delivery order.
    pub envelopes: Vec<(u64, Envelope)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogResponse {
    pub entries: Vec<LogEntry>,
}

/// Result of a log append.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppendResult {
    Appended {
        index: u64,
    },
    /// The entry did not extend the head; fetch from `len` and retry.
    Conflict {
        len: u64,
    },
}

/// `BLAKE2b-256("Zafe_JoinToken__", token)`.
pub fn join_token_hash(token: &[u8; 32]) -> [u8; 32] {
    blake2b_simd::Params::new()
        .hash_length(32)
        .personal(b"Zafe_JoinToken__")
        .hash(token)
        .as_bytes()
        .try_into()
        .expect("32 bytes")
}

pub fn encode_body<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtoError> {
    encode(value)
}

pub fn decode_body<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtoError> {
    postcard::from_bytes(bytes).map_err(|_| ProtoError::Encoding)
}
