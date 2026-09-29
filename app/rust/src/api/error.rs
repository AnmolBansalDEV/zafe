//! Typed errors for the Dart side (improves on Vizor's substring matching): Dart switches on
//! `kind` for copy and recovery actions, and shows `message` only as detail.

use zafe_core::{node::NodeError, relay_client::RelayClientError, wallet::WalletError};

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
    /// Bad address, amount, memo or invite.
    InvalidInput,
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

impl From<NodeError> for ZafeError {
    fn from(e: NodeError) -> Self {
        let kind = match &e {
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

impl From<WalletError> for ZafeError {
    fn from(e: WalletError) -> Self {
        let kind = match &e {
            WalletError::Remote(_) | WalletError::Sync(_) => ZafeErrorKind::Network,
            WalletError::Payment(_) => ZafeErrorKind::InvalidInput,
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
