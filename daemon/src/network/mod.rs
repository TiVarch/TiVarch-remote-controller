use crate::input::InputEngine;
use crate::protocol::{InputStateMachine, RemoteCommand, RemotePacket, RemoteResponse};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

pub struct AppState {
    pub input: Arc<InputEngine>,
    pub state_machine: Mutex<InputStateMachine>,
}

pub async fn start_server(port: u16, input: Arc<InputEngine>) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        input,
        state_machine: Mutex::new(InputStateMachine::new()),
    });

    let app = Router::new()
        .route("/", get(index_handler))
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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    info!("Client connected via WebSocket.");

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
                    let mut sm = state.state_machine.lock().await;

                    if !sm.validate_sequence(pkt.seq) {
                        warn!("Dropped out-of-order sequence packet: seq={}", pkt.seq);
                        continue;
                    }

                    match pkt.command {
                        RemoteCommand::DPad { action, state: key_state } => {
                            sm.update_dpad_state(action, key_state);
                            if let Err(e) = state.input.handle_dpad(action, key_state) {
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
                            for ch in text.chars() {
                                if ch == '\n' {
                                    let _ = state.input.click_key(evdev::Key::KEY_ENTER);
                                } else if ch == '\u{8}' {
                                    let _ = state.input.click_key(evdev::Key::KEY_BACKSPACE);
                                }
                            }
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