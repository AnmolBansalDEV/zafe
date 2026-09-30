//! `zafe-relay`: runs the relay.
//!
//! - `ZAFE_RELAY_LISTEN`: listen address (default `127.0.0.1:8787`; if unset and `PORT` is
//!   set, `0.0.0.0:$PORT`, as container platforms expect)
//! - `ZAFE_RELAY_DB`: SQLite file (default `zafe-relay.sqlite`; `:memory:` for none)
//! - `ZAFE_FCM_SERVICE_ACCOUNT`: path to a Firebase service-account JSON key; enables
//!   Android pushes (otherwise pushes are only logged)
//! - `ZAFE_FCM_SERVICE_ACCOUNT_JSON`: the same key inline (for secret env vars); wins
//! - `PORT`: see `ZAFE_RELAY_LISTEN`

use std::{path::Path, time::Duration};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // Plain text for Docker / systemd / Fly log collectors.
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();
    let addr = listen_addr(
        std::env::var("ZAFE_RELAY_LISTEN").ok(),
        std::env::var("PORT").ok(),
    );
    let db = std::env::var("ZAFE_RELAY_DB").unwrap_or_else(|_| "zafe-relay.sqlite".into());
    let relay = if db == ":memory:" {
        zafe_relay::Relay::new()
    } else {
        zafe_relay::Relay::open(Path::new(&db)).map_err(std::io::Error::other)?
    };

    // The key comes from a secret file, or inline from a secret env var (platforms such as
    // Fly.io expose secrets as env vars). Never bake it into an image.
    let fcm_json = match (
        std::env::var("ZAFE_FCM_SERVICE_ACCOUNT_JSON"),
        std::env::var("ZAFE_FCM_SERVICE_ACCOUNT"),
    ) {
        (Ok(json), _) if !json.trim().is_empty() => Ok(json),
        (_, Ok(path)) if !path.is_empty() => Ok(std::fs::read_to_string(&path)?),
        _ => Err(()),
    };
    let relay = match fcm_json {
        Ok(json) => {
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
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// `ZAFE_RELAY_LISTEN` wins; otherwise `PORT` (Fly.io, Cloud Run, ...) on all interfaces;
/// otherwise the local development default.
fn listen_addr(listen: Option<String>, port: Option<String>) -> String {
    match (listen, port) {
        (Some(listen), _) if !listen.is_empty() => listen,
        (_, Some(port)) if !port.is_empty() => format!("0.0.0.0:{port}"),
        _ => "127.0.0.1:8787".into(),
    }
}

/// Ctrl-C, or SIGTERM from Docker / systemd / Fly when stopping the service.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("shutting down");
}

#[cfg(test)]
mod tests {
    use super::listen_addr;

    #[test]
    fn listen_address_precedence() {
        assert_eq!(listen_addr(None, None), "127.0.0.1:8787");
        assert_eq!(listen_addr(None, Some("8080".into())), "0.0.0.0:8080");
        assert_eq!(
            listen_addr(Some("127.0.0.1:9000".into()), Some("8080".into())),
            "127.0.0.1:9000"
        );
        assert_eq!(listen_addr(Some(String::new()), None), "127.0.0.1:8787");
    }
}
