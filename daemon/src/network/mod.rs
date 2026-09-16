use crate::config::ConfigManager;
use crate::input::InputEngine;
use crate::pairing::PairingSession;
use crate::protocol::{InputStateMachine, RemoteCommand, RemotePacket, RemoteResponse};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::header,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

pub struct AppState {
    pub input: Arc<InputEngine>,
    pub state_machine: Mutex<InputStateMachine>,
    pub config_manager: Arc<Mutex<ConfigManager>>,
    pub pairing_session: Arc<Mutex<PairingSession>>,
}

#[derive(Deserialize, Default)]
pub struct WsAuthQuery {
    pub device_id: Option<String>,
    pub token: Option<String>,
}

fn register_mdns_service(port: u16, server_name: &str) {
    let name = server_name.to_string();
    tokio::task::spawn_blocking(move || {
        match ServiceDaemon::new() {
            Ok(mdns) => {
                let service_type = "_tivarch._tcp.local.";
                let instance_name = "tivarch-remote";
                let host_name = "tivarch.local.";
                let mut properties: HashMap<String, String> = HashMap::new();
                properties.insert("name".to_string(), name);
                properties.insert("ver".to_string(), "1".to_string());

                match ServiceInfo::new(
                    service_type,
                    instance_name,
                    host_name,
                    "",
                    port,
                    properties,
                ) {
                    Ok(service_info) => {
                        if let Err(e) = mdns.register(service_info) {
                            warn!("Failed to register mDNS service: {:?}", e);
                        } else {
                            info!("mDNS ZeroConf announced as _tivarch._tcp.local on port {}", port);
                        }
                    }
                    Err(e) => warn!("Failed to create mDNS ServiceInfo: {:?}", e),
                }
            }
            Err(e) => warn!("Failed to initialize mDNS daemon: {:?}", e),
        }
    });
}

fn launch_app(app_id: &str) {
    let app_id = app_id.to_lowercase();
    let command = match app_id.as_str() {
        "youtube" => "xdg-open https://www.youtube.com/tv",
        "kodi" => "kodi",
        "steam" => "steam steam://open/bigpicture",
        "retroarch" => "retroarch",
        "browser" => "xdg-open https://duckduckgo.com",
        _ => {
            warn!("Unknown app_id for launch: {}", app_id);
            return;
        }
    };

    info!("Launching application: {} via command: {}", app_id, command);
    let _ = Command::new("sh").arg("-c").arg(command).spawn();
}

pub async fn start_server(
    port: u16,
    input: Arc<InputEngine>,
    config_mgr: Arc<Mutex<ConfigManager>>,
    pairing_session: Arc<Mutex<PairingSession>>,
) -> anyhow::Result<()> {
    let server_name = config_mgr.lock().await.config.server_name.clone();

    let state = Arc::new(AppState {
        input,
        state_machine: Mutex::new(InputStateMachine::new()),
        config_manager: config_mgr,
        pairing_session,
    });

    register_mdns_service(port, &server_name);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/qr", get(qr_svg_handler))
        .route("/qr.svg", get(qr_svg_handler))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("TiVarch Remote listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../../../web-client/index.html"))
}

async fn qr_svg_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let session = state.pairing_session.lock().await;
    match session.render_svg() {
        Ok(svg) => (
            [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            svg,
        )
            .into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error rendering QR: {:?}", e),
        )
            .into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsAuthQuery>,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, query, state))
}

async fn handle_socket(socket: WebSocket, query: WsAuthQuery, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    info!("Client connected via WebSocket.");

    let mut is_authenticated = false;

    // Check if device is persistently paired
    if let Some(ref dev_id) = query.device_id {
        let cm = state.config_manager.lock().await;
        if cm.is_device_paired(dev_id) {
            is_authenticated = true;
            info!("Device {} authenticated from saved configuration.", dev_id);
        }
    }

    // Check if pairing token is passed via query and valid
    if let Some(ref token) = query.token {
        let ps = state.pairing_session.lock().await;
        if !ps.is_expired() && *token == ps.active_token {
            is_authenticated = true;
            info!("Device authenticated via active pairing token query.");
        }
    }

    {
        let mut sm = state.state_machine.lock().await;
        let hanging = sm.drain_active_keys();
        state.input.release_dpad_keys(&hanging);
        sm.reset_session();
    }

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!("WebSocket read error: {:?}", e);
                break;
            }
        };

        if let Message::Text(text) = msg {
            let packet: Result<RemotePacket, _> = serde_json::from_str(&text);
            match packet {
                Ok(pkt) => {
                    // Handshake processing
                    if let RemoteCommand::PairRequest {
                        device_id,
                        device_name,
                        token,
                    } = pkt.command
                    {
                        let mut ps = state.pairing_session.lock().await;

                        if ps.is_expired() {
                            warn!("Rejected pairing request: Token expired or already used.");
                            let resp = RemoteResponse::Error {
                                message: "Pairing token expired. Check TV screen for a new code."
                                    .to_string(),
                            };
                            let _ = sender
                                .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                                .await;
                            continue;
                        }

                        if token == ps.active_token {
                            let mut cm = state.config_manager.lock().await;
                            let _ = cm.register_device(device_id.clone(), device_name.clone());
                            ps.invalidate();
                            is_authenticated = true;
                            info!(
                                "Successfully paired and registered device: {} ({})",
                                device_name, device_id
                            );

                            let resp = RemoteResponse::PairSuccess {
                                server_name: cm.config.server_name.clone(),
                            };
                            let _ = sender
                                .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                                .await;
                        } else {
                            warn!("Rejected pairing request: Invalid token provided.");
                            let resp = RemoteResponse::Error {
                                message: "Invalid pairing token".to_string(),
                            };
                            let _ = sender
                                .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                                .await;
                        }
                        continue;
                    }

                    if !is_authenticated {
                        warn!("Rejected command from unauthenticated connection.");
                        let resp = RemoteResponse::Error {
                            message: "Unauthorized device. Please pair first.".to_string(),
                        };
                        let _ = sender
                            .send(Message::Text(serde_json::to_string(&resp).unwrap().into()))
                            .await;
                        continue;
                    }

                    let mut sm = state.state_machine.lock().await;

                    if !sm.validate_sequence(pkt.seq) {
                        warn!("Dropped out-of-order sequence packet: seq={}", pkt.seq);
                        continue;
                    }

                    match pkt.command {
                        RemoteCommand::PairRequest { .. } => unreachable!(),
                        RemoteCommand::DPad { action, state: key_state } => {
                            sm.update_dpad_state(action, key_state);
                            if let Err(e) = state.input.handle_dpad(action, key_state, sm.environment) {
                                error!("Failed to process D-Pad action: {:?}", e);
                            }
                        }
                        RemoteCommand::Media(action) => {
                            if let Err(e) = state.input.handle_media(action) {
                                error!("Failed to process Media action: {:?}", e);
                            }
                        }
                        RemoteCommand::Power(action) => {
                            if sm.validate_power_action(Instant::now()) {
                                if let Err(e) = state.input.handle_power(action) {
                                    error!("Failed to process Power action: {:?}", e);
                                }
                            } else {
                                warn!("Power command suppressed due to debounce policy.");
                            }
                        }
                        RemoteCommand::MouseMove { dx, dy } => {
                            let _ = state.input.handle_mouse_move(dx, dy);
                        }
                        RemoteCommand::MouseButton { button, state: b_state } => {
                            let _ = state.input.handle_mouse_button(button, b_state);
                        }
                        RemoteCommand::Scroll { dy } => {
                            let _ = state.input.handle_scroll(dy);
                        }
                        RemoteCommand::Keyboard { text } => {
                            if let Err(e) = state.input.type_text(&text) {
                                error!("Failed to type keyboard text: {:?}", e);
                            }
                        }
                        RemoteCommand::SetProfile { profile } => {
                            info!("Switching target environment profile to {:?}", profile);
                            sm.set_environment(profile);
                        }
                        RemoteCommand::LaunchApp { app_id } => {
                            launch_app(&app_id);
                        }
                        RemoteCommand::Ping => {
                            let resp = serde_json::to_string(&RemoteResponse::Pong).unwrap();
                            let _ = sender.send(Message::Text(resp.into())).await;
                        }
                    }
                }
                Err(e) => warn!("Invalid packet format received: {:?}", e),
            }
        }
    }

    info!("Client disconnected. Releasing all held keys...");
    let mut sm = state.state_machine.lock().await;
    let hanging_keys = sm.drain_active_keys();
    state.input.release_dpad_keys(&hanging_keys);
}