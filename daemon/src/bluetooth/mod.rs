use crate::input::InputEngine;
use crate::protocol::{DPadAction, KeyState, TargetEnvironment};
use anyhow::Result;
use bluer::adv::{Advertisement, Type as AdvType};
use bluer::agent::{Agent, AgentHandle};
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
    CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod, Service,
};
use bluer::Uuid;
use futures_util::FutureExt;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

// Standard HID & GATT Characteristic UUIDs
pub const HID_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x00001812_0000_1000_8000_00805f9b34fb);
pub const REPORT_CHAR_UUID: Uuid =
    Uuid::from_u128(0x00002a4d_0000_1000_8000_00805f9b34fb);
pub const REPORT_MAP_CHAR_UUID: Uuid =
    Uuid::from_u128(0x00002a4b_0000_1000_8000_00805f9b34fb);
pub const HID_INFO_CHAR_UUID: Uuid =
    Uuid::from_u128(0x00002a4a_0000_1000_8000_00805f9b34fb);
pub const HID_CONTROL_POINT_CHAR_UUID: Uuid =
    Uuid::from_u128(0x00002a4c_0000_1000_8000_00805f9b34fb);

// Standard Generic Remote Control Appearance: 0x0180
pub const APPEARANCE_REMOTE_CONTROL: u16 = 0x0180;

/// Standard Consumer Control HID Report Descriptor (Media, DPad, Home).
pub const HID_REPORT_MAP: &[u8] = &[
    0x05, 0x0C, // Usage Page (Consumer Devices)
    0x09, 0x01, // Usage (Consumer Control)
    0xA1, 0x01, // Collection (Application)
    0x85, 0x01, //   Report ID (1)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x08, //   Report Count (8 bits)
    0x09, 0xE9, //   Usage (Volume Increment)
    0x09, 0xEA, //   Usage (Volume Decrement)
    0x09, 0xE2, //   Usage (Mute)
    0x09, 0xCD, //   Usage (Play/Pause)
    0x09, 0x24, //   Usage (Back)
    0x09, 0x21, //   Usage (Search)
    0x09, 0x23, //   Usage (Home)
    0x09, 0x40, //   Usage (Menu)
    0x81, 0x02, //   Input (Data, Var, Abs)
    0xC0,       // End Collection
];

/// Constructs the peripheral LE advertisement payload.
pub fn build_advertisement(name: &str) -> Advertisement {
    Advertisement {
        advertisement_type: AdvType::Peripheral,
        service_uuids: vec![HID_SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some(name.to_string()),
        appearance: Some(APPEARANCE_REMOTE_CONTROL),
        timeout: Some(Duration::from_secs(0)),
        ..Default::default()
    }
}

/// Automatically authorizes pairing handshakes without requiring shell pin inputs.
pub fn create_auto_pairing_agent() -> Agent {
    Agent {
        request_default: true,
        request_pin_code: None,
        display_pin_code: None,
        display_passkey: Some(Box::new(|req| {
            Box::pin(async move {
                info!("Bluetooth pairing passkey display: {}", req.passkey);
                Ok(())
            })
        })),
        request_passkey: Some(Box::new(|_req| Box::pin(async move { Ok(0) }))),
        request_confirmation: Some(Box::new(|req| {
            Box::pin(async move {
                info!("Bluetooth pairing confirmation auto-accepted for {}", req.device);
                Ok(())
            })
        })),
        request_authorization: Some(Box::new(|req| {
            Box::pin(async move {
                info!("Bluetooth device authorization granted: {}", req.device);
                Ok(())
            })
        })),
        authorize_service: Some(Box::new(|req| {
            Box::pin(async move {
                info!("Bluetooth service authorization granted: {}", req.service);
                Ok(())
            })
        })),
        ..Default::default()
    }
}

/// Initializes and serves the GATT HID application tree and LE advertising.
pub async fn start_ble_service(input: Arc<InputEngine>) -> Result<()> {
    match bluer::Session::new().await {
        Ok(session) => {
            let _agent_handle: Option<AgentHandle> = match session.register_agent(create_auto_pairing_agent()).await {
                Ok(handle) => {
                    info!("Bluetooth pairing agent registered.");
                    Some(handle)
                }
                Err(e) => {
                    warn!("Could not register Bluetooth agent: {:?}", e);
                    None
                }
            };

            match session.default_adapter().await {
                Ok(adapter) => {
                    adapter.set_powered(true).await?;

                    // Reset any stale alias to preserve original system hostname
                    let _ = adapter.set_alias("".to_string()).await;

                    adapter.set_discoverable(true).await?;
                    adapter.set_discoverable_timeout(0).await?;
                    adapter.set_pairable(true).await?;

                    info!(
                        "Bluetooth adapter active on {} ({}). Starting GATT HID Application...",
                        adapter.name(),
                        adapter.address().await?
                    );

                    let input_engine_char = input.clone();
                    let app = Application {
                        services: vec![Service {
                            uuid: HID_SERVICE_UUID,
                            primary: true,
                            characteristics: vec![
                                Characteristic {
                                    uuid: HID_INFO_CHAR_UUID,
                                    read: Some(CharacteristicRead {
                                        read: true,
                                        fun: Box::new(|_| {
                                            async {
                                                // bcdHID (1.11), bCountryCode (0), Flags (0x02)
                                                Ok(vec![0x11, 0x01, 0x00, 0x02])
                                            }
                                            .boxed()
                                        }),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                Characteristic {
                                    uuid: REPORT_MAP_CHAR_UUID,
                                    read: Some(CharacteristicRead {
                                        read: true,
                                        fun: Box::new(|_| {
                                            async { Ok(HID_REPORT_MAP.to_vec()) }.boxed()
                                        }),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                Characteristic {
                                    uuid: HID_CONTROL_POINT_CHAR_UUID,
                                    write: Some(CharacteristicWrite {
                                        write: true,
                                        write_without_response: true,
                                        method: CharacteristicWriteMethod::Fun(Box::new(|val, _| {
                                            async move {
                                                info!("HID Control Point written: {:?}", val);
                                                Ok(())
                                            }
                                            .boxed()
                                        })),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                Characteristic {
                                    uuid: REPORT_CHAR_UUID,
                                    read: Some(CharacteristicRead {
                                        read: true,
                                        fun: Box::new(|_| async { Ok(vec![0x00]) }.boxed()),
                                        ..Default::default()
                                    }),
                                    write: Some(CharacteristicWrite {
                                        write: true,
                                        write_without_response: true,
                                        method: CharacteristicWriteMethod::Fun(Box::new(move |val, _| {
                                            let engine = input_engine_char.clone();
                                            async move {
                                                if !val.is_empty() {
                                                    let byte = val[0];
                                                    if byte & 0x01 != 0 {
                                                        let _ = engine.handle_media(crate::protocol::MediaAction::VolumeUp);
                                                    } else if byte & 0x02 != 0 {
                                                        let _ = engine.handle_media(crate::protocol::MediaAction::VolumeDown);
                                                    } else if byte & 0x04 != 0 {
                                                        let _ = engine.handle_media(crate::protocol::MediaAction::Mute);
                                                    } else if byte & 0x08 != 0 {
                                                        let _ = engine.handle_media(crate::protocol::MediaAction::PlayPause);
                                                    } else if byte & 0x10 != 0 {
                                                        let _ = engine.handle_dpad(
                                                            DPadAction::Back,
                                                            KeyState::Click,
                                                            TargetEnvironment::PlasmaBigscreen,
                                                        );
                                                    } else if byte & 0x40 != 0 {
                                                        let _ = engine.handle_dpad(
                                                            DPadAction::Home,
                                                            KeyState::Click,
                                                            TargetEnvironment::PlasmaBigscreen,
                                                        );
                                                    }
                                                }
                                                Ok(())
                                            }
                                            .boxed()
                                        })),
                                        ..Default::default()
                                    }),
                                    notify: Some(CharacteristicNotify {
                                        notify: true,
                                        method: CharacteristicNotifyMethod::Fun(Box::new(|_notifier| {
                                            async move {
                                                info!("Client subscribed to HID Report notifications.");
                                                futures_util::future::pending::<()>().await;
                                            }
                                            .boxed()
                                        })),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        }],
                        ..Default::default()
                    };

                    let _app_handle = match adapter.serve_gatt_application(app).await {
                        Ok(handle) => {
                            info!("BlueZ GATT HID Application registered successfully.");
                            Some(handle)
                        }
                        Err(e) => {
                            warn!("Failed to serve GATT HID application: {:?}", e);
                            None
                        }
                    };

                    let adv = build_advertisement("TiVarch Remote");

                    match adapter.advertise(adv).await {
                        Ok(_adv_handle) => {
                            info!("BLE Peripheral advertisement active with standard HID GATT.");
                            futures_util::future::pending::<()>().await;
                        }
                        Err(e) => {
                            error!("Failed to register BLE advertisement: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("No default Bluetooth adapter found: {:?}", e);
                }
            }
        }
        Err(e) => {
            warn!("Failed to initialize BlueZ D-Bus session: {:?}", e);
        }
    }
    Ok(())
}