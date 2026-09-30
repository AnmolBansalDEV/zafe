//! Typed errors for the Dart side (no substring matching): Dart switches on
//! `kind` for copy and recovery actions, and shows `message` only as detail.

use zafe_core::{node::NodeError, relay_client::RelayClientError, wallet::WalletError};
use zafe_proto::UnsupportedVersion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZafeErrorKind {
    /// The relay or lightwalletd could not be reached.
    Network,
    /// Waiting on other members or on sync; try again later.
    NotReady,
    /// Other members did not answer in time; safe to retry.
    Timeout,
    /// This device's independent check failed: never approve or sign.
    Verification,
    /// The vault cannot cover the amount plus the fee.
    InsufficientFunds,
    /// The vault holds enough, but part of it is held by open proposals.
    FundsReserved,
    /// Bad address, amount, memo or invite.
    InvalidInput,
    /// Data, a message or the relay comes from a newer version of Zafe: update the app.
    UpdateRequired,
    /// The relay is older than this app and must be updated by whoever runs it.
    RelayOutdated,
    Other,
}

#[derive(Clone, Debug)]
pub struct ZafeError {
    pub kind: ZafeErrorKind,
    pub message: String,
}

impl ZafeError {
    pub(crate) fn new(kind: ZafeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::new(ZafeErrorKind::InvalidInput, message)
    }
}

impl std::fmt::Display for ZafeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ZafeError {}

/// Newer data means "update the app"; older data this build no longer reads is `Other`
/// (pre-release formats have no migrations: reset or restore).
fn version_kind(v: &UnsupportedVersion) -> ZafeErrorKind {
    if v.is_newer() {
        ZafeErrorKind::UpdateRequired
    } else {
        ZafeErrorKind::Other
    }
}

impl From<UnsupportedVersion> for ZafeError {
    fn from(v: UnsupportedVersion) -> Self {
        Self::new(version_kind(&v), v.to_string())
    }
}

impl From<NodeError> for ZafeError {
    fn from(e: NodeError) -> Self {
        if let NodeError::Wallet(w) = e {
            return w.into();
        }
        let kind = match &e {
            NodeError::UnsupportedVersion(v)
            | NodeError::Relay(RelayClientError::UnsupportedVersion(v)) => version_kind(v),
            NodeError::Relay(r @ RelayClientError::VersionRejected { .. }) => {
                if r.app_outdated() {
                    ZafeErrorKind::UpdateRequired
                } else {
                    ZafeErrorKind::RelayOutdated
                }
            }
            NodeError::Relay(RelayClientError::Transport(_)) => ZafeErrorKind::Network,
            NodeError::NotReady(_) => ZafeErrorKind::NotReady,
            NodeError::Timeout(_) => ZafeErrorKind::Timeout,
            NodeError::Verification(_)
            | NodeError::SafetyNumberMismatch { .. }
            | NodeError::EchoMismatch => ZafeErrorKind::Verification,
            NodeError::BadInvite => ZafeErrorKind::InvalidInput,
            _ => ZafeErrorKind::Other,
        };
        Self::new(kind, e.to_string())
    }
}

impl From<RelayClientError> for ZafeError {
    fn from(e: RelayClientError) -> Self {
        NodeError::Relay(e).into()
    }
}

impl From<WalletError> for ZafeError {
    fn from(e: WalletError) -> Self {
        let kind = match &e {
            WalletError::Remote(_) | WalletError::Sync(_) => ZafeErrorKind::Network,
            WalletError::Payment(_) => ZafeErrorKind::InvalidInput,
            WalletError::FundsReserved => ZafeErrorKind::FundsReserved,
            // zcash_client_backend's error is only available as text here.
            WalletError::Proposal(m) if m.contains("InsufficientFunds") => {
                ZafeErrorKind::InsufficientFunds
            }
            _ => ZafeErrorKind::Other,
        };
        Self::new(kind, e.to_string())
    }
}

impl From<anyhow::Error> for ZafeError {
    fn from(e: anyhow::Error) -> Self {
        let e = match e.downcast::<NodeError>() {
            Ok(n) => return n.into(),
            Err(e) => e,
        };
        match e.downcast::<WalletError>() {
            Ok(w) => w.into(),
            Err(e) => Self::new(ZafeErrorKind::Other, format!("{e:#}")),
        }
    }
}
