use crate::protocol::{DPadAction, KeyState, MediaAction, PowerAction};
use anyhow::{Context, Result};
use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, Key, RelativeAxisType};
use std::sync::Mutex;
use tracing::{info, warn};

pub struct InputEngine {
    device: Mutex<VirtualDevice>,
}

impl InputEngine {
    pub fn new() -> Result<Self> {
        let mut keys = AttributeSet::<Key>::new();

        // D-Pad and Desktop Navigation (Keyboard keys)
        keys.insert(Key::KEY_UP);
        keys.insert(Key::KEY_DOWN);
        keys.insert(Key::KEY_LEFT);
        keys.insert(Key::KEY_RIGHT);
        keys.insert(Key::KEY_ENTER);
        keys.insert(Key::KEY_BACKSPACE);
        keys.insert(Key::KEY_ESC);
        keys.insert(Key::KEY_LEFTMETA); // Super key to return to Desktop / Bigscreen home
        keys.insert(Key::KEY_MENU);

        // Gamepad buttons for TV interface compatibility
        keys.insert(Key::BTN_DPAD_UP);
        keys.insert(Key::BTN_DPAD_DOWN);
        keys.insert(Key::BTN_DPAD_LEFT);
        keys.insert(Key::BTN_DPAD_RIGHT);
        keys.insert(Key::BTN_SOUTH); // A / Cross button
        keys.insert(Key::BTN_EAST);  // B / Circle button
        keys.insert(Key::BTN_MODE);  // Guide / Home button

        // Multimedia & Audio
        keys.insert(Key::KEY_PLAYPAUSE);
        keys.insert(Key::KEY_NEXTSONG);
        keys.insert(Key::KEY_PREVIOUSSONG);
        keys.insert(Key::KEY_VOLUMEUP);
        keys.insert(Key::KEY_VOLUMEDOWN);
        keys.insert(Key::KEY_MUTE);

        // Power Management
        keys.insert(Key::KEY_POWER);
        keys.insert(Key::KEY_SLEEP);

        // Mouse Buttons
        keys.insert(Key::BTN_LEFT);
        keys.insert(Key::BTN_RIGHT);
        keys.insert(Key::BTN_MIDDLE);

        // Relative Axes (Touchpad / Mouse cursor)
        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);
        rel_axes.insert(RelativeAxisType::REL_WHEEL);

        let device = VirtualDeviceBuilder::new()?
            .name("TiVarch Ultra Virtual Controller")
            .with_keys(&keys)?
            .with_relative_axes(&rel_axes)?
            .build()
            .context("Failed to create uinput virtual device. Check /dev/uinput permissions.")?;

        info!("Virtual input device successfully registered with Linux kernel.");
        Ok(Self {
            device: Mutex::new(device),
        })
    }

    fn emit(&self, events: &[InputEvent]) -> Result<()> {
        let mut dev = self.device.lock().unwrap();
        dev.emit(events).context("Failed to emit kernel input events")?;
        Ok(())
    }

    pub fn send_key_event(&self, key: Key, val: i32) -> Result<()> {
        let ev = InputEvent::new(EventType::KEY, key.0, val);
        self.emit(&[ev])
    }

    pub fn click_key(&self, key: Key) -> Result<()> {
        let press = InputEvent::new(EventType::KEY, key.0, 1);
        let release = InputEvent::new(EventType::KEY, key.0, 0);
        self.emit(&[press, release])
    }

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

    pub fn handle_power(&self, action: PowerAction) -> Result<()> {
        let key = match action {
            PowerAction::Suspend => Key::KEY_SLEEP,
            PowerAction::PowerOff => Key::KEY_POWER,
            PowerAction::Reboot => Key::KEY_POWER,
        };
        self.click_key(key)
    }

    pub fn handle_mouse_move(&self, dx: i32, dy: i32) -> Result<()> {
        let ev_x = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, dx);
        let ev_y = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, dy);
        self.emit(&[ev_x, ev_y])
    }

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

    pub fn handle_scroll(&self, dy: i32) -> Result<()> {
        let ev = InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, dy);
        self.emit(&[ev])
    }

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