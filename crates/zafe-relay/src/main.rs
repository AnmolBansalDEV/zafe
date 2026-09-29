//! `zafe-relay`: runs the relay.
//!
//! - `ZAFE_RELAY_LISTEN` (default `127.0.0.1:8787`)
//! - `ZAFE_RELAY_DB`: SQLite file (default `zafe-relay.sqlite`; `:memory:` for none)

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
