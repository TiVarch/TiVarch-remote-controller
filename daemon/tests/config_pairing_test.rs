use std::time::{Duration, Instant};
use tempfile::tempdir;
use tivarch_remote_daemon::config::ConfigManager;
use tivarch_remote_daemon::pairing::PairingSession;
use tivarch_remote_daemon::protocol::{RemoteCommand, RemotePacket, RemoteResponse};

#[test]
fn test_config_path_format() {
    let default_path = ConfigManager::default_path();
    assert!(default_path.ends_with("tivarch/remote-config.json"));
}

#[test]
fn test_config_creation_and_persistence() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("tivarch").join("remote-config.json");

    let mut mgr = ConfigManager::load_or_create_from_path(&config_path).unwrap();
    assert!(config_path.exists());
    assert_eq!(mgr.config.server_name, "TiVarch Ultra Remote");
    assert!(mgr.config.paired_devices.is_empty());

    mgr.register_device("device_abc_123".to_string(), "Galaxy A31".to_string())
        .unwrap();
    assert_eq!(mgr.config.paired_devices.len(), 1);
    assert!(mgr.is_device_paired("device_abc_123"));

    // Reload from disk to verify persistence
    let reloaded = ConfigManager::load_or_create_from_path(&config_path).unwrap();
    assert_eq!(reloaded.config.paired_devices.len(), 1);
    assert_eq!(
        reloaded.config.paired_devices[0].device_name,
        "Galaxy A31"
    );
}

#[test]
fn test_device_unpairing() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("tivarch").join("remote-config.json");
    let mut mgr = ConfigManager::load_or_create_from_path(&config_path).unwrap();

    mgr.register_device("dev_1".to_string(), "Phone 1".to_string())
        .unwrap();
    mgr.register_device("dev_2".to_string(), "Phone 2".to_string())
        .unwrap();
    assert_eq!(mgr.config.paired_devices.len(), 2);

    let removed = mgr.unpair_device("dev_1").unwrap();
    assert!(removed);
    assert!(!mgr.is_device_paired("dev_1"));
    assert!(mgr.is_device_paired("dev_2"));
}

#[test]
fn test_qr_session_payload_generation() {
    let session = PairingSession::with_ip(
        "Living Room Bigscreen",
        9000,
        "192.168.1.50".to_string(),
        Some("60:E9:AA:AF:55:A4".to_string()),
    )
    .unwrap();

    assert_eq!(session.payload.v, 1);
    assert_eq!(session.payload.name, "Living Room Bigscreen");
    assert_eq!(session.payload.ip, "192.168.1.50");
    assert_eq!(session.payload.port, 9000);
    assert_eq!(
        session.payload.ble_mac,
        Some("60:E9:AA:AF:55:A4".to_string())
    );
    assert!(!session.active_token.is_empty());
    assert!(!session.is_expired());
}

#[test]
fn test_qr_terminal_rendering_not_empty() {
    let session = PairingSession::with_ip(
        "Test TV",
        9000,
        "127.0.0.1".to_string(),
        None,
    )
    .unwrap();

    let rendered = session.render_terminal_qr().unwrap();
    assert!(!rendered.is_empty());
}

#[test]
fn test_qr_svg_rendering_format() {
    let session = PairingSession::with_ip(
        "Bigscreen OSD",
        9000,
        "192.168.1.25".to_string(),
        None,
    )
    .unwrap();

    let svg = session.render_svg().unwrap();
    assert!(svg.starts_with("<?xml") || svg.starts_with("<svg"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn test_pairing_token_invalidation_single_use() {
    let mut session = PairingSession::with_ip(
        "Secure TV",
        9000,
        "192.168.1.10".to_string(),
        None,
    )
    .unwrap();

    assert!(!session.is_expired());
    session.invalidate();
    assert!(session.is_expired());
}

#[test]
fn test_pairing_token_expiration_ttl() {
    let mut session = PairingSession::with_ip(
        "TTL TV",
        9000,
        "192.168.1.10".to_string(),
        None,
    )
    .unwrap();

    // Manipulate created_at to simulate 11 minutes in the past
    session.created_at = Instant::now() - Duration::from_secs(660);
    assert!(session.is_expired());
}

#[test]
fn test_primary_ip_detection_valid_v4() {
    let detected_ip = PairingSession::detect_primary_ip();
    assert!(!detected_ip.is_empty());
    // Should not select standard loopback
    assert_ne!(detected_ip, "0.0.0.0");
}

#[test]
fn test_pair_request_packet_roundtrip() {
    let cmd = RemoteCommand::PairRequest {
        device_id: "phone_uuid_99".to_string(),
        device_name: "Pixel 8".to_string(),
        token: "token_12345".to_string(),
    };
    let pkt = RemotePacket {
        seq: 1,
        command: cmd,
    };

    let json = serde_json::to_string(&pkt).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::PairRequest {
            device_id,
            device_name,
            token,
        } => {
            assert_eq!(device_id, "phone_uuid_99");
            assert_eq!(device_name, "Pixel 8");
            assert_eq!(token, "token_12345");
        }
        _ => panic!("Expected PairRequest command variant"),
    }
}

#[test]
fn test_pair_response_serialization() {
    let success = RemoteResponse::PairSuccess {
        server_name: "Living Room TV".to_string(),
    };
    let json = serde_json::to_string(&success).unwrap();
    assert!(json.contains("pair_success"));
    assert!(json.contains("Living Room TV"));

    let err = RemoteResponse::Error {
        message: "Token expired".to_string(),
    };
    let err_json = serde_json::to_string(&err).unwrap();
    assert!(err_json.contains("error"));
    assert!(err_json.contains("Token expired"));
}