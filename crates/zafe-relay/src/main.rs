//! `zafe-relay`: runs the relay.
//!
//! - `ZAFE_RELAY_LISTEN` (default `127.0.0.1:8787`)
//! - `ZAFE_RELAY_DB`: SQLite file (default `zafe-relay.sqlite`; `:memory:` for none)
//! - `ZAFE_FCM_SERVICE_ACCOUNT`: path to a Firebase service-account JSON key; enables
//!   Android pushes (otherwise pushes are only logged)

use std::{path::Path, time::Duration};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let addr = std::env::var("ZAFE_RELAY_LISTEN").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let db = std::env::var("ZAFE_RELAY_DB").unwrap_or_else(|_| "zafe-relay.sqlite".into());
    let relay = if db == ":memory:" {
        zafe_relay::Relay::new()
    } else {
        zafe_relay::Relay::open(Path::new(&db)).map_err(std::io::Error::other)?
    };

    let relay = match std::env::var("ZAFE_FCM_SERVICE_ACCOUNT") {
        Ok(path) => {
            let json = std::fs::read_to_string(&path)?;
            let account =
                zafe_relay::fcm::ServiceAccount::from_json(&json).map_err(std::io::Error::other)?;
            tracing::info!("FCM pushes enabled for project {}", account.project_id);
            relay.with_notifier(std::sync::Arc::new(zafe_relay::fcm::FcmNotifier::new(
                account,
            )))
        }
        Err(_) => relay,
    };

    // Hourly retention pruning of old undelivered envelopes.
    let pruner = relay.clone();
    tokio::spawn(async move {
        loop {
            match pruner.prune() {
                Ok(n) if n > 0 => tracing::info!("pruned {n} expired deliveries"),
                Ok(_) => {}
                Err(e) => tracing::warn!("prune failed: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("zafe-relay listening on {addr}, db {db}");
    axum::serve(listener, relay.router())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
