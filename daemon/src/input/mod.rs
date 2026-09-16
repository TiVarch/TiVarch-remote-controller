use crate::protocol::{DPadAction, KeyState, MediaAction, PowerAction};
use anyhow::{Context, Result};
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, Key, RelativeAxisType};
use std::sync::Mutex;
use tracing::{info, warn};

/// Low-level Linux `/dev/uinput` abstraction emitting kernel input events.
pub struct InputEngine {
    device: Mutex<VirtualDevice>,
}

impl InputEngine {
    /// Registers virtual controller with navigation, multimedia, and pointer capabilities.
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<Key>::new();

        // TV / Desktop navigation scancodes
        keys.insert(Key::KEY_UP);
        keys.insert(Key::KEY_DOWN);
        keys.insert(Key::KEY_LEFT);
        keys.insert(Key::KEY_RIGHT);
        keys.insert(Key::KEY_ENTER);
        keys.insert(Key::KEY_BACKSPACE);
        keys.insert(Key::KEY_ESC);
        keys.insert(Key::KEY_LEFTMETA);
        keys.insert(Key::KEY_HOMEPAGE);
        keys.insert(Key::KEY_MENU);

        // Multimedia controls
        keys.insert(Key::KEY_PLAYPAUSE);
        keys.insert(Key::KEY_NEXTSONG);
        keys.insert(Key::KEY_PREVIOUSSONG);
        keys.insert(Key::KEY_VOLUMEUP);
        keys.insert(Key::KEY_VOLUMEDOWN);
        keys.insert(Key::KEY_MUTE);

        // Power management
        keys.insert(Key::KEY_POWER);
        keys.insert(Key::KEY_SLEEP);

        // Pointer buttons
        keys.insert(Key::BTN_LEFT);
        keys.insert(Key::BTN_RIGHT);
        keys.insert(Key::BTN_MIDDLE);

        // Relative axes for mouse/touchpad emulation
        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);
        rel_axes.insert(RelativeAxisType::REL_WHEEL);

        let device = VirtualDeviceBuilder::new()?
            .name("TiVarch Ultra Virtual Controller")
            .with_keys(&keys)?
            .with_relative_axes(&rel_axes)?
            .build()
            .context("Failed to create uinput device. Check /dev/uinput permissions.")?;

        info!("Virtual input device successfully registered with Linux kernel.");
        Ok(Self {
            device: Mutex::new(device),
        })
    }

    /// Emits a batch of raw input events followed by kernel synchronization.
    fn emit(&self, events: &[InputEvent]) -> Result<()> {
        let mut dev = self.device.lock().unwrap();
        dev.emit(events).context("Failed to emit kernel input events")?;
        Ok(())
    }

    /// Emits single key event (press=1, release=0).
    pub fn send_key_event(&self, key: Key, val: i32) -> Result<()> {
        let ev = InputEvent::new(EventType::KEY, key.0, val);
        self.emit(&[ev])
    }

    /// Emits discrete click (press -> sync -> release -> sync).
    pub fn click_key(&self, key: Key) -> Result<()> {
        let press = InputEvent::new(EventType::KEY, key.0, 1);
        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
        let release = InputEvent::new(EventType::KEY, key.0, 0);
        self.emit(&[press, syn, release, syn])
    }

    /// Handles DPad state changes.
    pub fn handle_dpad(&self, action: DPadAction, state: KeyState) -> Result<()> {
        let key = match action {
            DPadAction::Up => Key::KEY_UP,
            DPadAction::Down => Key::KEY_DOWN,
            DPadAction::Left => Key::KEY_LEFT,
            DPadAction::Right => Key::KEY_RIGHT,
            DPadAction::Select => Key::KEY_ENTER,
            DPadAction::Back => Key::KEY_ESC,
            DPadAction::Home => Key::KEY_LEFTMETA,
            DPadAction::Menu => Key::KEY_MENU,
        };

        match state {
            KeyState::Press => self.send_key_event(key, 1)?,
            KeyState::Release => self.send_key_event(key, 0)?,
            KeyState::Click => self.click_key(key)?,
        }
        Ok(())
    }

    /// Handles consumer multimedia key clicks.
    pub fn handle_media(&self, action: MediaAction) -> Result<()> {
        let key = match action {
            MediaAction::PlayPause => Key::KEY_PLAYPAUSE,
            MediaAction::Next => Key::KEY_NEXTSONG,
            MediaAction::Previous => Key::KEY_PREVIOUSSONG,
            MediaAction::VolumeUp => Key::KEY_VOLUMEUP,
            MediaAction::VolumeDown => Key::KEY_VOLUMEDOWN,
            MediaAction::Mute => Key::KEY_MUTE,
        };
        self.click_key(key)
    }

    /// Handles system power key clicks.
    pub fn handle_power(&self, action: PowerAction) -> Result<()> {
        let key = match action {
            PowerAction::Suspend => Key::KEY_SLEEP,
            PowerAction::PowerOff => Key::KEY_POWER,
            PowerAction::Reboot => Key::KEY_POWER,
        };
        self.click_key(key)
    }

    /// Emits relative cursor displacement.
    pub fn handle_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        let ev_x = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, dx);
        let ev_y = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, dy);
        self.emit(&[ev_x, ev_y])
    }

    /// Emits pointer button clicks and states.
    pub fn handle_mouse_button(&self, button: u8, state: KeyState) -> Result<()> {
        let btn = match button {
            1 => Key::BTN_LEFT,
            2 => Key::BTN_MIDDLE,
            3 => Key::BTN_RIGHT,
            _ => return Ok(()),
        };
        match state {
            KeyState::Press => self.send_key_event(btn, 1)?,
            KeyState::Release => self.send_key_event(btn, 0)?,
            KeyState::Click => self.click_key(btn)?,
        }
        Ok(())
    }

    /// Emits relative vertical scrolling steps.
    pub fn handle_scroll(&self, dy: i32) -> Result<()> {
        let ev = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, dy);
        self.emit(&[ev])
    }

    /// Releases stuck/hanging keys during failover or disconnection.
    pub fn release_dpad_keys(&self, actions: &[DPadAction]) {
        for action in actions {
            let key = match action {
                DPadAction::Up => Key::KEY_UP,
                DPadAction::Down => Key::KEY_DOWN,
                DPadAction::Left => Key::KEY_LEFT,
                DPadAction::Right => Key::KEY_RIGHT,
                DPadAction::Select => Key::KEY_ENTER,
                DPadAction::Back => Key::KEY_ESC,
                DPadAction::Home => Key::KEY_LEFTMETA,
                DPadAction::Menu => Key::KEY_MENU,
            };
            if let Err(e) = self.send_key_event(key, 0) {
                warn!("Failed to release hanging key {:?}: {:?}", key, e);
            }
        }
    }
}