//! "Use Tor" (Settings): every relay request and lightwalletd connection goes through Tor,
//! fail-closed. The route is process-wide (`zafe_core::tor`), shared by the app and any
//! background check running in the same process.
//!
//! Order matters: with Tor on, call [`tor_request`] right after `RustLib.init()`, before
//! any other call can open a connection, then [`tor_enable`] to bootstrap.

use std::{path::PathBuf, time::Duration};

use zafe_core::tor::{self, TorError, TorStatus};

use super::{
    error::{ZafeError, ZafeErrorKind},
    vault::runtime,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TorState {
    /// Tor is off: connections are direct.
    Off,
    /// Tor is on and connecting: nothing is sent until it is.
    Connecting,
    /// Tor is on and connected.
    Connected,
    /// Tor is on but couldn't connect: nothing is sent until it does.
    Failed,
}

impl From<TorStatus> for TorState {
    fn from(s: TorStatus) -> Self {
        match s {
            TorStatus::Off => TorState::Off,
            TorStatus::Connecting => TorState::Connecting,
            TorStatus::Ready => TorState::Connected,
            TorStatus::Failed => TorState::Failed,
        }
    }
}

/// Switches the route to Tor at once (no bootstrap yet): from now on nothing connects
/// directly, and direct connections already open are closed. Does nothing if Tor is
/// already on.
#[flutter_rust_bridge::frb(sync)]
pub fn tor_request() {
    tor::request();
}

/// Turns Tor on and connects, giving up after `timeout_secs` (arti retries forever on a
/// network that blocks Tor). Tor keeps its state (guards, directory cache) in `tor_dir`.
/// Returns at once when Tor is already connected. On failure the route stays on Tor:
/// requests keep failing (typed `TorFailed`) until it connects or is turned off.
pub async fn tor_enable(tor_dir: String, timeout_secs: u32) -> Result<TorState, ZafeError> {
    // On the bridge runtime: arti spawns its background tasks on the runtime it starts in,
    // and every request runs there.
    let budget = Duration::from_secs(timeout_secs.into());
    runtime()
        .spawn(async move { tor::enable(&PathBuf::from(tor_dir), budget).await })
        .await
        .map_err(|e| ZafeError::new(ZafeErrorKind::Other, e.to_string()))?
        .map(TorState::from)
        .map_err(tor_error)
}

/// Switches back to direct connections.
#[flutter_rust_bridge::frb(sync)]
pub fn tor_disable() {
    tor::disable();
}

#[flutter_rust_bridge::frb(sync)]
pub fn tor_state() -> TorState {
    tor::status().into()
}

/// The app went to the background (`true`) or came back: a dormant Tor stops its idle
/// traffic. A request wakes it by itself, so background checks need nothing.
#[flutter_rust_bridge::frb(sync)]
pub fn tor_set_dormant(dormant: bool) {
    tor::set_dormant(dormant);
}

fn tor_error(e: TorError) -> ZafeError {
    let kind = match e {
        TorError::Abandoned => ZafeErrorKind::Other,
        _ => ZafeErrorKind::TorFailed,
    };
    ZafeError::new(kind, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_map_one_to_one() {
        assert_eq!(TorState::from(TorStatus::Off), TorState::Off);
        assert_eq!(TorState::from(TorStatus::Connecting), TorState::Connecting);
        assert_eq!(TorState::from(TorStatus::Ready), TorState::Connected);
        assert_eq!(TorState::from(TorStatus::Failed), TorState::Failed);
    }

    #[test]
    fn bootstrap_failures_are_typed() {
        assert_eq!(tor_error(TorError::Timeout).kind, ZafeErrorKind::TorFailed);
        assert_eq!(
            tor_error(TorError::Bootstrap("x".into())).kind,
            ZafeErrorKind::TorFailed
        );
    }
}
