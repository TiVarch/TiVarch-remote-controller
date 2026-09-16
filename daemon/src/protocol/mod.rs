use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DPadAction {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Home,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Suspend,
    PowerOff,
    Reboot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyState {
    Press,
    Release,
    Click,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RemoteCommand {
    DPad { action: DPadAction, state: KeyState },
    Media(MediaAction),
    Power(PowerAction),
    Keyboard { text: String },
    MouseMove { dx: i32, dy: i32 },
    MouseButton { button: u8, state: KeyState },
    Scroll { dy: i32 },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePacket {
    pub seq: u64,
    pub command: RemoteCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RemoteResponse {
    Pong,
    Ack,
    Error { message: String },
}

#[derive(Debug)]
pub struct InputStateMachine {
    pub active_dpad_keys: HashSet<DPadAction>,
    pub last_power_event: Option<Instant>,
    pub power_cooldown: Duration,
    pub last_sequence_id: u64,
}

impl Default for InputStateMachine {
    fn default() -> Self {
        Self {
            active_dpad_keys: HashSet::new(),
            last_power_event: None,
            power_cooldown: Duration::from_millis(3000),
            last_sequence_id: 0,
        }
    }
}

impl InputStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_session(&mut self) {
        self.last_sequence_id = 0;
        self.active_dpad_keys.clear();
    }

    pub fn validate_sequence(&mut self, seq: u64) -> bool {
        if seq > self.last_sequence_id {
            self.last_sequence_id = seq;
            true
        } else {
            false
        }
    }

    pub fn validate_power_action(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last_power_event {
            if now.duration_since(last) < self.power_cooldown {
                return false;
            }
        }
        self.last_power_event = Some(now);
        true
    }

    pub fn update_dpad_state(&mut self, action: DPadAction, state: KeyState) -> bool {
        match state {
            KeyState::Press => self.active_dpad_keys.insert(action),
            KeyState::Release => self.active_dpad_keys.remove(&action),
            KeyState::Click => false,
        }
    }

    pub fn drain_active_keys(&mut self) -> Vec<DPadAction> {
        self.active_dpad_keys.drain().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_order_validation() {
        let mut sm = InputStateMachine::new();
        assert!(sm.validate_sequence(1));
        assert!(sm.validate_sequence(2));
        assert!(!sm.validate_sequence(2), "Duplicate seq should be dropped");
        assert!(!sm.validate_sequence(1), "Older seq should be dropped");
        assert!(sm.validate_sequence(5));
    }

    #[test]
    fn test_session_reset_allows_reconnect() {
        let mut sm = InputStateMachine::new();
        assert!(sm.validate_sequence(10));
        assert!(sm.validate_sequence(15));

        sm.reset_session();
        assert_eq!(sm.last_sequence_id, 0);
        assert!(sm.validate_sequence(1), "First packet after reset must pass");
    }

    #[test]
    fn test_power_debounce_prevention() {
        let mut sm = InputStateMachine::new();
        let t0 = Instant::now();

        assert!(sm.validate_power_action(t0));

        let t_rapid = t0 + Duration::from_millis(500);
        assert!(!sm.validate_power_action(t_rapid), "Rapid trigger must be blocked");

        let t_valid = t0 + Duration::from_millis(3500);
        assert!(sm.validate_power_action(t_valid), "Trigger after cooldown must pass");
    }

    #[test]
    fn test_dpad_key_tracking_and_drain() {
        let mut sm = InputStateMachine::new();
        sm.update_dpad_state(DPadAction::Up, KeyState::Press);
        sm.update_dpad_state(DPadAction::Select, KeyState::Press);

        assert_eq!(sm.active_dpad_keys.len(), 2);

        sm.update_dpad_state(DPadAction::Up, KeyState::Release);
        assert_eq!(sm.active_dpad_keys.len(), 1);
        assert!(sm.active_dpad_keys.contains(&DPadAction::Select));

        let drained = sm.drain_active_keys();
        assert_eq!(drained, vec![DPadAction::Select]);
        assert!(sm.active_dpad_keys.is_empty());
    }

    #[test]
    fn test_session_reset_clears_active_keys() {
        let mut sm = InputStateMachine::new();
        sm.update_dpad_state(DPadAction::Right, KeyState::Press);
        assert!(!sm.active_dpad_keys.is_empty());

        sm.reset_session();
        assert!(sm.active_dpad_keys.is_empty(), "Session reset must clear hanging keys");
    }
}