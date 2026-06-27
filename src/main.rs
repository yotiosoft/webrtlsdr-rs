mod api;
mod audio;
mod config;
mod device;
mod dsp;
mod sdr;
mod session;
mod stream;

use anyhow::Context;
use config::Config;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "webrtlsdr_rs=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env().context("failed to load configuration")?;
    let listener = TcpListener::bind(config.listen_addr())
        .await
        .with_context(|| format!("failed to bind {}", config.listen_addr()))?;

    info!(listen_addr = %config.listen_addr(), "starting WebRTLSDR server");

    axum::serve(listener, api::router())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server failed")?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!(%error, "failed to install Ctrl+C handler");
    }
}
