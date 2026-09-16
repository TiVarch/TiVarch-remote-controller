mod bluetooth;
mod input;
mod network;
mod protocol;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_PORT: u16 = 9000;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting TiVarch Ultra Remote Controller Daemon...");

    // 1. Initialize virtual kernel input device
    let input_engine = Arc::new(input::InputEngine::new()?);

    // 2. Start BLE background service gracefully
    let ble_input = input_engine.clone();
    tokio::spawn(async move {
        if let Err(e) = bluetooth::start_ble_service(ble_input).await {
            tracing::warn!("BLE service initialization warning: {:?}", e);
        }
    });

    // 3. Start Web and WebSocket server on designated port
    network::start_server(DEFAULT_PORT, input_engine).await?;

    Ok(())
}