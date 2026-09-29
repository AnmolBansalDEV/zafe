//! `zafe-relay`: runs the relay on `ZAFE_RELAY_LISTEN` (default 127.0.0.1:8787).

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let addr = std::env::var("ZAFE_RELAY_LISTEN").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("zafe-relay listening on {addr}");
    axum::serve(listener, zafe_relay::Relay::new().router())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
