//! Relay API types (spec §6). Request and response bodies are postcard behind a
//! [`version::RELAY_API`] tag (envelopes and log entries carry their own tags); every
//! request is signed by a member identity, over the API version too. A relay answers a
//! body tagged with a version it doesn't speak with HTTP 426 and
//! [`UNSUPPORTED_VERSION_HEADER`] set to the version it supports. The relay never sees
//! plaintext vault data.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    envelope::{encode, Envelope, MailboxId},
    identity::{Identity, IdentityPublic},
    log::LogEntry,
    version::{self, Format},
    ProtoError,
};

const SIGNATURE_DOMAIN: &[u8] = b"Zafe relay request v1";

/// Response header of a 426 (Upgrade Required): the version of the rejected format that
/// the relay supports.
pub const UNSUPPORTED_VERSION_HEADER: &str = "zafe-supported-version";

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
        encode_body(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtoError> {
        decode_body(bytes)
    }
}

fn signed_bytes<T: Serialize>(payload: &T) -> Result<Vec<u8>, ProtoError> {
    let mut out = SIGNATURE_DOMAIN.to_vec();
    out.extend_from_slice(&version::RELAY_API.to_le_bytes());
    out.extend_from_slice(&encode(payload)?);
    Ok(out)
}

/// Opens a mailbox for a vault being created. Signed by the creator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateMailbox {
    pub mailbox: MailboxId,
    /// BLAKE2b hash of the invite's join token. The token is shared by all invited members
    /// and stops working when the creator seals membership.
    pub join_token_hash: [u8; 32],
    /// The vault's member count `n` (creator included); joins beyond it are refused.
    pub max_members: u16,
}

/// Removes a joined member before sealing (e.g. a stranger with a leaked invite). Signed by
/// the creator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remove {
    pub mailbox: MailboxId,
    pub member: [u8; 32],
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

/// The longest a relay holds a [`WaitRequest`] open; longer requests are cut to this. It
/// stays below common proxy idle timeouts (Caddy, Fly.io), so a quiet wait ends with an
/// answer rather than a dropped connection.
pub const MAX_WAIT_SECS: u32 = 25;

/// Long poll (`POST /v1/wait`): answers as soon as the vault log is longer than `log_len`
/// or an envelope for the signer is delivered past cursor `inbox_after`, and otherwise
/// after `max_wait_secs` (at most [`MAX_WAIT_SECS`]). Signed by a member. The answer says
/// only where the log and the inbox are now; the client reads them as usual. A relay
/// without this endpoint answers 404 (clients fall back to polling); an unknown mailbox
/// gets 403 here, never 404, so the two can't be confused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitRequest {
    pub mailbox: MailboxId,
    pub log_len: u64,
    pub inbox_after: u64,
    pub max_wait_secs: u32,
    pub timestamp: u64,
}

/// Where the mailbox is when a [`WaitRequest`] returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResponse {
    /// Entries in the vault log.
    pub log_len: u64,
    /// The newest delivery cursor for the signer (0: none stored).
    pub inbox_cursor: u64,
}

impl WaitResponse {
    /// Whether this is news for a client that has `log_len` entries and read its inbox up
    /// to `inbox_after`.
    pub fn is_news(&self, log_len: u64, inbox_after: u64) -> bool {
        self.log_len > log_len || self.inbox_cursor > inbox_after
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembersResponse {
    pub members: Vec<IdentityPublic>,
    pub sealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxResponse {
    /// `(cursor, Envelope::to_bytes)` in delivery order. Kept as bytes so one envelope in
    /// a version this client can't read doesn't make the whole page unreadable.
    pub envelopes: Vec<(u64, Vec<u8>)>,
}

impl InboxResponse {
    /// The envelopes this build can decode (others are dropped, like badly signed ones).
    pub fn decoded(&self) -> Vec<(u64, Envelope)> {
        self.envelopes
            .iter()
            .filter_map(|(cursor, bytes)| Some((*cursor, Envelope::from_bytes(bytes).ok()?)))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogResponse {
    /// `LogEntry::to_bytes` of each entry, in index order.
    pub entries: Vec<Vec<u8>>,
}

impl LogResponse {
    /// Decodes every entry. An entry in an unknown version stops here with
    /// [`ProtoError::UnsupportedVersion`]: the chain can't be followed past it.
    pub fn decoded(&self) -> Result<Vec<LogEntry>, ProtoError> {
        self.entries
            .iter()
            .map(|b| LogEntry::from_bytes(b))
            .collect()
    }
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

/// A relay API body: `version (u16) || postcard(value)`.
pub fn encode_body<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtoError> {
    Ok(version::encode(Format::RelayApi, value)?)
}

/// Decodes a relay API body, rejecting other API versions.
pub fn decode_body<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtoError> {
    Ok(version::decode(Format::RelayApi, bytes)?)
}

/// Push notification platform for a member device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PushPlatform {
    Apns,
    Fcm,
}

impl PushPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            PushPlatform::Apns => "apns",
            PushPlatform::Fcm => "fcm",
        }
    }
}

/// Registers (or replaces) this member's push token for a mailbox. Notifications carry no
/// content, only "vault activity" (spec §6.1). Signed by the member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterPush {
    pub mailbox: MailboxId,
    pub platform: PushPlatform,
    pub token: String,
}
