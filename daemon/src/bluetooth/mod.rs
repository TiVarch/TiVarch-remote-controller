use crate::input::InputEngine;
use anyhow::Result;
use bluer::adv::{Advertisement, Type as AdvType};
use bluer::agent::{Agent, AgentHandle};
use bluer::Uuid;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub const HID_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x00001812_0000_1000_8000_00805f9b34fb);

// Standard Bluetooth Appearance for Generic Remote Control: 0x0180
pub const APPEARANCE_REMOTE_CONTROL: u16 = 0x0180;

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

pub fn create_auto_pairing_agent() -> Agent {
    Agent {
        request_default: true,
        request_pin_code: None,
        display_pin_code: None,
        display_passkey: Some(Box::new(|req| {
            Box::pin(async move {
                info!(
                    "Bluetooth pairing passkey display: passkey={}, entered={}",
                    req.passkey, req.entered
                );
                Ok(())
            })
        })),
        request_passkey: Some(Box::new(|_req| {
            Box::pin(async move {
                info!("Bluetooth requested passkey, providing default 000000");
                Ok(0)
            })
        })),
        request_confirmation: Some(Box::new(|req| {
            Box::pin(async move {
                info!(
                    "Bluetooth pairing confirmation request: device={}, passkey={}. Auto-confirming...",
                    req.device, req.passkey
                );
                Ok(())
            })
        })),
        request_authorization: Some(Box::new(|req| {
            Box::pin(async move {
                info!(
                    "Bluetooth device authorization request for {}. Auto-authorizing...",
                    req.device
                );
                Ok(())
            })
        })),
        authorize_service: Some(Box::new(|req| {
            Box::pin(async move {
                info!(
                    "Bluetooth service authorization for {} on {}. Auto-approving...",
                    req.service, req.device
                );
                Ok(())
            })
        })),
        ..Default::default()
    }
}

pub async fn start_ble_service(_input: Arc<InputEngine>) -> Result<()> {
    match bluer::Session::new().await {
        Ok(session) => {
            let _agent_handle: Option<AgentHandle> = match session.register_agent(create_auto_pairing_agent()).await {
                Ok(handle) => {
                    info!("Bluetooth pairing agent successfully registered.");
                    Some(handle)
                }
                Err(e) => {
                    warn!("Could not register Bluetooth agent (pairing may require manual confirmation): {:?}", e);
                    None
                }
            };

            match session.default_adapter().await {
                Ok(adapter) => {
                    adapter.set_powered(true).await?;
                    adapter.set_discoverable(true).await?;
                    adapter.set_discoverable_timeout(0).await?;
                    adapter.set_pairable(true).await?;

                    info!(
                        "Bluetooth adapter active on {}. Broadcasting as 'TiVarch Remote' (Appearance: Remote Control)",
                        adapter.name()
                    );

                    let adv = build_advertisement("TiVarch Remote");

                    match adapter.advertise(adv).await {
                        Ok(_adv_handle) => {
                            info!("BLE Peripheral advertisement active. Listening for connection events...");
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

#[cfg(test)]
mod tests {
    use super::*;
    use bluer::UuidExt;

    #[test]
    fn test_hid_service_uuid_matches_bluer_ext() {
        let expected = <Uuid as UuidExt>::from_u16(0x1812);
        assert_eq!(HID_SERVICE_UUID, expected);
    }

    #[test]
    fn test_advertisement_builder() {
        let adv = build_advertisement("TiVarch Remote");
        assert_eq!(adv.local_name, Some("TiVarch Remote".to_string()));
        assert_eq!(adv.discoverable, Some(true));
        assert_eq!(adv.appearance, Some(APPEARANCE_REMOTE_CONTROL));
        assert_eq!(adv.advertisement_type, AdvType::Peripheral);
        assert!(adv.service_uuids.contains(&HID_SERVICE_UUID));
    }
}