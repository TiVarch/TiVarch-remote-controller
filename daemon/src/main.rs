mod bluetooth;
mod config;
mod input;
mod network;
mod pairing;
mod protocol;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting TiVarch Ultra Remote Controller Daemon...");

    // 1. Load or initialize ~/.config/tivarch/remote-config.json
    let config_mgr = Arc::new(Mutex::new(config::ConfigManager::load_or_create()?));
    let cfg = config_mgr.lock().await.config.clone();

    // 2. Initialize virtual kernel input device
    let input_engine = Arc::new(input::InputEngine::new()?);

    // 3. Obtain Bluetooth adapter hardware MAC dynamically
    let ble_mac = match bluer::Session::new().await {
        Ok(s) => match s.default_adapter().await {
            Ok(a) => a.address().await.ok().map(|addr| addr.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    };

    // 4. Initialize pairing session with smart IP filtering and display ASCII QR
    let pairing = Arc::new(Mutex::new(pairing::PairingSession::new(
        &cfg.server_name,
        cfg.port,
        ble_mac,
    )?));
    pairing.lock().await.print_terminal_qr()?;

    // 5. Start BLE background service gracefully
    let ble_input = input_engine.clone();
    tokio::spawn(async move {
        if let Err(e) = bluetooth::start_ble_service(ble_input).await {
            tracing::warn!("BLE service initialization warning: {:?}", e);
        }
    });

    // 6. Start Web, OSD (/qr) and WebSocket server with TTL auth & config
    network::start_server(cfg.port, input_engine, config_mgr, pairing).await?;

    Ok(())
}