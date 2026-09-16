use std::time::{Duration, Instant};
use tivarch_remote_daemon::bluetooth::{
    build_advertisement, APPEARANCE_REMOTE_CONTROL, HID_REPORT_MAP, HID_SERVICE_UUID,
};
use tivarch_remote_daemon::protocol::*;

#[test]
fn test_01_initial_sequence_acceptance() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(1));
    assert_eq!(sm.last_sequence_id, 1);
}

#[test]
fn test_02_sequential_packet_progression() {
    let mut sm = InputStateMachine::new();
    for seq in 1..=50 {
        assert!(sm.validate_sequence(seq));
    }
    assert_eq!(sm.last_sequence_id, 50);
}

#[test]
fn test_03_reject_duplicate_sequence() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(10));
    assert!(!sm.validate_sequence(10));
}

#[test]
fn test_04_reject_regressive_sequence() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(20));
    assert!(!sm.validate_sequence(19));
    assert!(!sm.validate_sequence(1));
}

#[test]
fn test_05_sparse_sequence_jumps_allowed() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(1));
    assert!(sm.validate_sequence(100));
    assert!(sm.validate_sequence(10_000));
    assert!(!sm.validate_sequence(5_000));
}

#[test]
fn test_06_session_reset_allows_sequence_restart() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(500));
    sm.reset_session();
    assert_eq!(sm.last_sequence_id, 0);
    assert!(sm.validate_sequence(1));
}

#[test]
fn test_07_power_action_initial_pass() {
    let mut sm = InputStateMachine::new();
    let now = Instant::now();
    assert!(sm.validate_power_action(now));
}

#[test]
fn test_08_power_action_rapid_burst_rejection() {
    let mut sm = InputStateMachine::new();
    let t0 = Instant::now();
    assert!(sm.validate_power_action(t0));

    for offset_ms in [50, 150, 500, 1000, 2999] {
        let t = t0 + Duration::from_millis(offset_ms);
        assert!(!sm.validate_power_action(t), "Expected failure at offset {}ms", offset_ms);
    }
}

#[test]
fn test_09_power_action_passes_after_exact_cooldown() {
    let mut sm = InputStateMachine::new();
    let t0 = Instant::now();
    assert!(sm.validate_power_action(t0));

    let t1 = t0 + Duration::from_millis(3001);
    assert!(sm.validate_power_action(t1));
}

#[test]
fn test_10_single_key_press_and_release_lifecycle() {
    let mut sm = InputStateMachine::new();
    assert!(sm.update_dpad_state(DPadAction::Up, KeyState::Press));
    assert!(sm.active_dpad_keys.contains(&DPadAction::Up));

    assert!(sm.update_dpad_state(DPadAction::Up, KeyState::Release));
    assert!(!sm.active_dpad_keys.contains(&DPadAction::Up));
}

#[test]
fn test_11_duplicate_key_press_idempotency() {
    let mut sm = InputStateMachine::new();
    assert!(sm.update_dpad_state(DPadAction::Down, KeyState::Press));
    assert!(!sm.update_dpad_state(DPadAction::Down, KeyState::Press));
    assert_eq!(sm.active_dpad_keys.len(), 1);
}

#[test]
fn test_12_releasing_non_pressed_key_is_safe() {
    let mut sm = InputStateMachine::new();
    assert!(!sm.update_dpad_state(DPadAction::Left, KeyState::Release));
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_13_multi_key_concurrent_press() {
    let mut sm = InputStateMachine::new();
    sm.update_dpad_state(DPadAction::Up, KeyState::Press);
    sm.update_dpad_state(DPadAction::Select, KeyState::Press);
    sm.update_dpad_state(DPadAction::Right, KeyState::Press);

    assert_eq!(sm.active_dpad_keys.len(), 3);
}

#[test]
fn test_14_partial_release_maintains_remaining_keys() {
    let mut sm = InputStateMachine::new();
    sm.update_dpad_state(DPadAction::Up, KeyState::Press);
    sm.update_dpad_state(DPadAction::Select, KeyState::Press);

    sm.update_dpad_state(DPadAction::Up, KeyState::Release);
    assert_eq!(sm.active_dpad_keys.len(), 1);
    assert!(sm.active_dpad_keys.contains(&DPadAction::Select));
}

#[test]
fn test_15_dead_man_drain_releases_all_active_keys() {
    let mut sm = InputStateMachine::new();
    sm.update_dpad_state(DPadAction::Up, KeyState::Press);
    sm.update_dpad_state(DPadAction::Down, KeyState::Press);
    sm.update_dpad_state(DPadAction::Select, KeyState::Press);

    let drained = sm.drain_active_keys();
    assert_eq!(drained.len(), 3);
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_16_drain_on_empty_state_returns_empty_vec() {
    let mut sm = InputStateMachine::new();
    let drained = sm.drain_active_keys();
    assert!(drained.is_empty());
}

#[test]
fn test_17_session_reset_wipes_active_held_keys() {
    let mut sm = InputStateMachine::new();
    sm.update_dpad_state(DPadAction::Menu, KeyState::Press);
    assert!(!sm.active_dpad_keys.is_empty());

    sm.reset_session();
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_18_environment_profile_switch() {
    let mut sm = InputStateMachine::with_environment(TargetEnvironment::PlasmaDesktop);
    assert_eq!(sm.environment, TargetEnvironment::PlasmaDesktop);

    sm.set_environment(TargetEnvironment::PlasmaBigscreen);
    assert_eq!(sm.environment, TargetEnvironment::PlasmaBigscreen);
}

#[test]
fn test_19_json_roundtrip_dpad_command() {
    let cmd = RemoteCommand::DPad {
        action: DPadAction::Home,
        state: KeyState::Press,
    };
    let packet = RemotePacket { seq: 12, command: cmd };
    let serialized = serde_json::to_string(&packet).expect("Must serialize");
    let deserialized: RemotePacket = serde_json::from_str(&serialized).expect("Must deserialize");

    assert_eq!(deserialized.seq, 12);
    match deserialized.command {
        RemoteCommand::DPad { action, state } => {
            assert_eq!(action, DPadAction::Home);
            assert_eq!(state, KeyState::Press);
        }
        _ => panic!("Wrong command type parsed"),
    }
}

#[test]
fn test_20_json_roundtrip_media_action() {
    let cmd = RemoteCommand::Media(MediaAction::VolumeUp);
    let packet = RemotePacket { seq: 42, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.seq, 42);
}

#[test]
fn test_21_json_roundtrip_power_action() {
    let cmd = RemoteCommand::Power(PowerAction::Suspend);
    let packet = RemotePacket { seq: 99, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.seq, 99);
}

#[test]
fn test_22_json_roundtrip_profile_switch() {
    let cmd = RemoteCommand::SetProfile {
        profile: TargetEnvironment::PlasmaBigscreen,
    };
    let packet = RemotePacket { seq: 100, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::SetProfile { profile } => {
            assert_eq!(profile, TargetEnvironment::PlasmaBigscreen)
        }
        _ => panic!("Expected SetProfile command"),
    }
}

#[test]
fn test_23_json_roundtrip_mouse_movement() {
    let cmd = RemoteCommand::MouseMove { dx: -15, dy: 30 };
    let packet = RemotePacket { seq: 7, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::MouseMove { dx, dy } => {
            assert_eq!(dx, -15);
            assert_eq!(dy, 30);
        }
        _ => panic!("Expected MouseMove command"),
    }
}

#[test]
fn test_24_json_roundtrip_mouse_button() {
    let cmd = RemoteCommand::MouseButton {
        button: 1,
        state: KeyState::Click,
    };
    let packet = RemotePacket { seq: 8, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::MouseButton { button, state } => {
            assert_eq!(button, 1);
            assert_eq!(state, KeyState::Click);
        }
        _ => panic!("Expected MouseButton command"),
    }
}

#[test]
fn test_25_json_roundtrip_scroll() {
    let cmd = RemoteCommand::Scroll { dy: -5 };
    let packet = RemotePacket { seq: 9, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::Scroll { dy } => assert_eq!(dy, -5),
        _ => panic!("Expected Scroll command"),
    }
}

#[test]
fn test_26_json_roundtrip_keyboard_unicode() {
    let cmd = RemoteCommand::Keyboard {
        text: "TiVarch Remote Test 123! 🎮".to_string(),
    };
    let packet = RemotePacket { seq: 10, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::Keyboard { text } => {
            assert_eq!(text, "TiVarch Remote Test 123! 🎮");
        }
        _ => panic!("Expected Keyboard command"),
    }
}

#[test]
fn test_27_json_roundtrip_ping_pong() {
    let cmd = RemoteCommand::Ping;
    let packet = RemotePacket { seq: 11, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::Ping => (),
        _ => panic!("Expected Ping command"),
    }

    let resp = RemoteResponse::Pong;
    let resp_json = serde_json::to_string(&resp).unwrap();
    let parsed_resp: RemoteResponse = serde_json::from_str(&resp_json).unwrap();
    match parsed_resp {
        RemoteResponse::Pong => (),
        _ => panic!("Expected Pong response"),
    }
}

#[test]
fn test_28_reject_malformed_json_missing_seq() {
    let invalid_json = r#"{"command":{"type":"ping"}}"#;
    let parsed: Result<RemotePacket, _> = serde_json::from_str(invalid_json);
    assert!(parsed.is_err());
}

#[test]
fn test_29_reject_malformed_json_invalid_action() {
    let invalid_json = r#"{"seq":1,"command":{"type":"d_pad","payload":{"action":"fly","state":"press"}}}"#;
    let parsed: Result<RemotePacket, _> = serde_json::from_str(invalid_json);
    assert!(parsed.is_err());
}

#[test]
fn test_30_reject_completely_unrelated_json() {
    let invalid_json = r#"{"user":"tester","action":"unknown"}"#;
    let parsed: Result<RemotePacket, _> = serde_json::from_str(invalid_json);
    assert!(parsed.is_err());
}

#[test]
fn test_31_click_key_state_does_not_pollute_active_keys() {
    let mut sm = InputStateMachine::new();
    let was_inserted = sm.update_dpad_state(DPadAction::Back, KeyState::Click);
    assert!(!was_inserted);
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_32_high_concurrency_out_of_order_stream() {
    let mut sm = InputStateMachine::new();
    let incoming_sequence = [1, 2, 4, 3, 5, 8, 6, 7, 9, 10];
    let mut accepted = Vec::new();

    for seq in incoming_sequence {
        if sm.validate_sequence(seq) {
            accepted.push(seq);
        }
    }

    assert_eq!(accepted, vec![1, 2, 4, 5, 8, 9, 10]);
}

#[test]
fn test_33_u64_max_boundary_sequence() {
    let mut sm = InputStateMachine::new();
    assert!(sm.validate_sequence(u64::MAX - 1));
    assert!(sm.validate_sequence(u64::MAX));
    assert!(!sm.validate_sequence(u64::MAX));
}

#[test]
fn test_34_empty_keyboard_string_handling() {
    let cmd = RemoteCommand::Keyboard { text: String::new() };
    let packet = RemotePacket { seq: 1, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();
    match parsed.command {
        RemoteCommand::Keyboard { text } => assert!(text.is_empty()),
        _ => panic!("Expected empty Keyboard text"),
    }
}

#[test]
fn test_35_all_dpad_keys_exhaustion() {
    let mut sm = InputStateMachine::new();
    let all_keys = [
        DPadAction::Up,
        DPadAction::Down,
        DPadAction::Left,
        DPadAction::Right,
        DPadAction::Select,
        DPadAction::Back,
        DPadAction::Home,
        DPadAction::Menu,
    ];

    for key in all_keys {
        sm.update_dpad_state(key, KeyState::Press);
    }
    assert_eq!(sm.active_dpad_keys.len(), 8);

    let drained = sm.drain_active_keys();
    assert_eq!(drained.len(), 8);
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_36_bluetooth_hid_uuid_constant() {
    use bluer::UuidExt;
    let expected = <bluer::Uuid as UuidExt>::from_u16(0x1812);
    assert_eq!(HID_SERVICE_UUID, expected);
}

#[test]
fn test_37_bluetooth_advertisement_builder() {
    let adv = build_advertisement("TiVarch Test");
    assert_eq!(adv.local_name, Some("TiVarch Test".to_string()));
    assert_eq!(adv.discoverable, Some(true));
    assert_eq!(adv.appearance, Some(APPEARANCE_REMOTE_CONTROL));
    assert!(adv.service_uuids.contains(&HID_SERVICE_UUID));
}

#[test]
fn test_38_bluetooth_report_map_structure() {
    assert!(!HID_REPORT_MAP.is_empty());
    assert_eq!(HID_REPORT_MAP[0], 0x05);
    assert_eq!(HID_REPORT_MAP[1], 0x0C);
}

#[test]
fn test_39_rapid_click_cycle_integrity() {
    let mut sm = InputStateMachine::new();
    for _ in 0..100 {
        sm.update_dpad_state(DPadAction::Select, KeyState::Press);
        sm.update_dpad_state(DPadAction::Select, KeyState::Release);
    }
    assert!(sm.active_dpad_keys.is_empty());
}

#[test]
fn test_40_repeated_session_resets() {
    let mut sm = InputStateMachine::new();
    for i in 1..=10 {
        sm.validate_sequence(i * 10);
        sm.update_dpad_state(DPadAction::Up, KeyState::Press);
        sm.reset_session();
        assert_eq!(sm.last_sequence_id, 0);
        assert!(sm.active_dpad_keys.is_empty());
    }
}

#[test]
fn test_41_json_roundtrip_launch_app() {
    let cmd = RemoteCommand::LaunchApp {
        app_id: "youtube".to_string(),
    };
    let packet = RemotePacket { seq: 15, command: cmd };
    let json = serde_json::to_string(&packet).unwrap();
    let parsed: RemotePacket = serde_json::from_str(&json).unwrap();

    match parsed.command {
        RemoteCommand::LaunchApp { app_id } => assert_eq!(app_id, "youtube"),
        _ => panic!("Expected LaunchApp command"),
    }
}

#[test]
fn test_42_bigscreen_home_key_behavior() {
    let mut sm = InputStateMachine::new();
    assert_eq!(sm.environment, TargetEnvironment::PlasmaBigscreen);
    sm.set_environment(TargetEnvironment::PlasmaDesktop);
    assert_eq!(sm.environment, TargetEnvironment::PlasmaDesktop);
}