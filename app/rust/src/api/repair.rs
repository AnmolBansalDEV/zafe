//! Moving a lost member's seat to a new phone (spec §10.1): the new phone's recovery code,
//! co-signers' approvals, and the new phone collecting its repaired key.
//!
//! The helpers' side of the share repair runs inside `list_proposals` (every refresh).

use rand::rngs::OsRng;
use zafe_core::{
    relay_client::RelayClient,
    repair::{self, RecoveryRequest, RecoveryStatus},
};

use super::{
    error::{ZafeError, ZafeErrorKind},
    vault::{identity, material, runtime},
};

type Result<T, E = ZafeError> = std::result::Result<T, E>;

pub struct RecoveryCode {
    /// `zafe-recover-v1:...`: sent to a co-signer (text, link or QR code).
    pub code: String,
    /// Eight digits both phones show ("1234 5678"), read out to check the code.
    pub safety_code: String,
}

/// The recovery code of a new phone's identity (`seeds`, a fresh identity kept until the
/// seat has moved).
#[flutter_rust_bridge::frb(sync)]
pub fn recovery_code(seeds: Vec<u8>) -> Result<RecoveryCode> {
    let request = RecoveryRequest {
        identity: *identity(&seeds)?.public(),
    };
    Ok(RecoveryCode {
        code: request.encode(),
        safety_code: request.safety_code(),
    })
}

pub struct RecoveryCodeInfo {
    /// The new phone's signing key (hex).
    pub key_hex: String,
    pub safety_code: String,
}

/// Parses a recovery code a co-signer pasted or scanned (also inside a message or a
/// `zafe://recover?code=` link).
#[flutter_rust_bridge::frb(sync)]
pub fn parse_recovery_code(code: String) -> Result<RecoveryCodeInfo> {
    let request = decode(&code)?;
    Ok(RecoveryCodeInfo {
        key_hex: hex::encode(request.identity.sig_pk),
        safety_code: request.safety_code(),
    })
}

fn decode(text: &str) -> Result<RecoveryRequest> {
    let code = text
        .split(|c: char| c.is_whitespace() || c == '=' || c == '&')
        .find(|w| w.starts_with("zafe-recover-v"))
        .unwrap_or(text);
    RecoveryRequest::decode(code).map_err(|e| match e {
        zafe_core::node::NodeError::UnsupportedVersion(_) => e.into(),
        _ => ZafeError::invalid("That isn't a Zafe recovery code"),
    })
}

/// Approves moving the seat of `old_key_hex` (a signer who lost their phone) to the phone
/// that showed `code`. Returns whether the seat has moved (this was the last approval
/// needed); the share repair then starts on the helpers' next refresh.
pub fn approve_seat_move(
    relay_url: String,
    seeds: Vec<u8>,
    material: Vec<u8>,
    old_key_hex: String,
    code: String,
) -> Result<bool> {
    let me = identity(&seeds)?;
    let m = self::material(&material)?;
    let request = decode(&code)?;
    let old: [u8; 32] = hex::decode(old_key_hex.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| ZafeError::invalid("not a signer key"))?;
    // Whether `old` is a member is checked against the log (seats may have moved).
    if old == me.public().sig_pk {
        return Err(ZafeError::invalid("You can't approve moving your own seat"));
    }
    Ok(runtime().block_on(repair::approve_replacement(
        &RelayClient::new(relay_url),
        &me,
        &m,
        old,
        &request,
        &mut OsRng,
    ))?)
}

pub enum RecoveryStage {
    /// No vault has moved a seat to this phone yet: waiting for co-signers to approve.
    Waiting,
    /// The seat moved; the co-signers' phones are repairing the key.
    Repairing,
    /// Done: `material` and `invite` hold the vault (save them like a restored backup).
    Done,
}

pub struct RecoveryProgress {
    pub stage: RecoveryStage,
    /// Repair messages received and needed so far (`Repairing`; needed is 0 until known).
    pub received: u32,
    pub needed: u32,
    pub vault_id: String,
    pub name: String,
    /// `VaultMaterial::to_bytes` (secret; `Done` only).
    pub material: Vec<u8>,
    pub invite: String,
}

/// The new phone: checks whether its seat moved and the repaired key has arrived. On
/// `Done` the key has been checked against the vault (spec §10.4.2).
pub fn check_recovery(relay_url: String, seeds: Vec<u8>) -> Result<RecoveryProgress> {
    let me = identity(&seeds)?;
    let status = runtime().block_on(repair::try_recover(&RelayClient::new(relay_url), &me))?;
    let empty = |stage, received, needed| RecoveryProgress {
        stage,
        received,
        needed,
        vault_id: String::new(),
        name: String::new(),
        material: Vec::new(),
        invite: String::new(),
    };
    Ok(match status {
        RecoveryStatus::Waiting => empty(RecoveryStage::Waiting, 0, 0),
        RecoveryStatus::SeatMoved { received, needed } => {
            empty(RecoveryStage::Repairing, received as u32, needed as u32)
        }
        RecoveryStatus::Done { material, invite } => RecoveryProgress {
            stage: RecoveryStage::Done,
            received: 0,
            needed: 0,
            vault_id: hex::encode(material.descriptor.vault_id),
            name: material.descriptor.name.clone(),
            material: material
                .to_bytes()
                .map_err(|e| ZafeError::new(ZafeErrorKind::Other, e.to_string()))?,
            invite: invite.encode(),
        },
    })
}
