use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Directional pad and core navigation buttons.
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

/// Standard multimedia control actions.
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

/// System power states with rate-limit protections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Suspend,
    PowerOff,
    Reboot,
}

/// Input key physical switch states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyState {
    Press,
    Release,
    Click,
}

/// Supported target desktop shells and media center profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetEnvironment {
    PlasmaDesktop,
    PlasmaBigscreen,
    KodiMediaCenter,
}

/// High-level commands received from network/BLE transport layers.
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
    SetProfile { profile: TargetEnvironment },
    Ping,
}

/// Sequenced envelope ensuring packet ordering and deduplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePacket {
    pub seq: u64,
    pub command: RemoteCommand,
}

/// Structured responses sent back to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RemoteResponse {
    Pong,
    Ack,
    Error { message: String },
}

/// Core state machine tracking active keys, debounce timestamps, and session continuity.
#[derive(Debug)]
pub struct InputStateMachine {
    pub active_dpad_keys: HashSet<DPadAction>,
    pub last_power_event: Option<Instant>,
    pub power_cooldown: Duration,
    pub last_sequence_id: u64,
    pub environment: TargetEnvironment,
}

impl Default for InputStateMachine {
    fn default() -> Self {
        Self {
            active_dpad_keys: HashSet::new(),
            last_power_event: None,
            power_cooldown: Duration::from_millis(3000),
            last_sequence_id: 0,
            environment: TargetEnvironment::PlasmaBigscreen,
        }
    }
}

impl InputStateMachine {
    pub fn new() -> Self {
        Self::default()
    }
    
    #[allow(dead_code)]
    pub fn with_environment(env: TargetEnvironment) -> Self {
        Self {
            environment: env,
            ..Self::default()
        }
    }

    /// Resets sequence counter and clears active key states on a fresh connection.
    pub fn reset_session(&mut self) {
        self.last_sequence_id = 0;
        self.active_dpad_keys.clear();
    }

    /// Updates active environment mapping profile.
    pub fn set_environment(&mut self, env: TargetEnvironment) {
        self.environment = env;
    }

    /// Enforces monotonic sequence numbers to drop delayed or duplicated UDP/WS packets.
    pub fn validate_sequence(&mut self, seq: u64) -> bool {
        if seq > self.last_sequence_id {
            self.last_sequence_id = seq;
            true
        } else {
            false
        }
    }

    /// Enforces cooldown duration between sensitive system power events.
    pub fn validate_power_action(&mut self, now: Instant) -> bool {
        if let Some(last) = self.last_power_event {
            if now.duration_since(last) < self.power_cooldown {
                return false;
            }
        }
        self.last_power_event = Some(now);
        true
    }

    /// Updates internal track of physical button hold states.
    pub fn update_dpad_state(&mut self, action: DPadAction, state: KeyState) -> bool {
        match state {
            KeyState::Press => self.active_dpad_keys.insert(action),
            KeyState::Release => self.active_dpad_keys.remove(&action),
            KeyState::Click => false,
        }
    }

    /// Drains all held keys for the dead-man switch during connection loss.
    pub fn drain_active_keys(&mut self) -> Vec<DPadAction> {
        self.active_dpad_keys.drain().collect()
    }
}