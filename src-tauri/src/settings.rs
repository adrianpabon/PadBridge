//! Per-controller profiles and app settings, saved as JSON in the app's config directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ds::protocol as p;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LightbarMode {
    #[default]
    Static,
    Rainbow,
    Battery,
    Off,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Lightbar {
    pub mode: LightbarMode,
    pub color: [u8; 3],
    /// 0-1
    pub brightness: f32,
    /// Rainbow: full turns per second.
    pub speed: f32,
}

impl Default for Lightbar {
    fn default() -> Self {
        Self { mode: LightbarMode::Static, color: [0, 90, 255], brightness: 1.0, speed: 0.2 }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PlayerLedMode {
    /// The Xbox controller number it got.
    #[default]
    Auto,
    Custom,
    Off,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PlayerLeds {
    pub mode: PlayerLedMode,
    /// 5 bits, left to right.
    pub custom: u8,
    /// 0 high, 1 medium, 2 low
    pub brightness: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MicLed {
    #[default]
    Off,
    On,
    Pulse,
}

/// An adaptive trigger effect; positions are in tenths of the trigger's travel (0 released - 9 pressed).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TriggerEffect {
    #[default]
    Off,
    Resistance { start: u8, strength: u8 },
    Slope { start: u8, end: u8, start_strength: u8, end_strength: u8 },
    Weapon { start: u8, end: u8, strength: u8 },
    Vibration { start: u8, amplitude: u8, frequency: u8 },
    Bow { start: u8, end: u8, strength: u8, snap: u8 },
    Galloping { start: u8, end: u8, first_foot: u8, second_foot: u8, frequency: u8 },
    Machine { start: u8, end: u8, amp_a: u8, amp_b: u8, frequency: u8, period: u8 },
    Zones { strengths: [u8; 10] },
}

impl TriggerEffect {
    pub fn encode(&self) -> [u8; 11] {
        match *self {
            TriggerEffect::Off => p::TRIGGER_OFF,
            TriggerEffect::Resistance { start, strength } => p::trigger_resistance(start, strength),
            TriggerEffect::Slope { start, end, start_strength, end_strength } => {
                p::trigger_slope(start, end, start_strength, end_strength)
            }
            TriggerEffect::Weapon { start, end, strength } => p::trigger_weapon(start, end, strength),
            TriggerEffect::Vibration { start, amplitude, frequency } => p::trigger_vibration(start, amplitude, frequency),
            TriggerEffect::Bow { start, end, strength, snap } => p::trigger_bow(start, end, strength, snap),
            TriggerEffect::Galloping { start, end, first_foot, second_foot, frequency } => {
                p::trigger_galloping(start, end, first_foot, second_foot, frequency)
            }
            TriggerEffect::Machine { start, end, amp_a, amp_b, frequency, period } => {
                p::trigger_machine(start, end, amp_a, amp_b, frequency, period)
            }
            TriggerEffect::Zones { strengths } => p::trigger_zones(&strengths),
        }
    }
}

/// How a stick's movement becomes the Xbox stick's.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StickSettings {
    /// Multiplier on the distance from centre (0.25-3).
    pub sensitivity: f32,
    /// Ignored around the centre (0-0.5 of the travel).
    pub deadzone: f32,
    /// Output starts here once out of the deadzone (0-0.5), for games with their own deadzone.
    pub anti_deadzone: f32,
    /// Response curve exponent: 1 linear, above 1 finer near the centre, below 1 faster.
    pub curve: f32,
    pub invert_x: bool,
    pub invert_y: bool,
}

impl Default for StickSettings {
    fn default() -> Self {
        Self { sensitivity: 1.0, deadzone: 0.06, anti_deadzone: 0.0, curve: 1.0, invert_x: false, invert_y: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TriggerRange {
    /// Pressed less than this reads as released (0-0.9).
    pub deadzone: f32,
    /// Pressed this much reads as fully pressed (0.1-1).
    pub max: f32,
}

impl Default for TriggerRange {
    fn default() -> Self {
        Self { deadzone: 0.0, max: 1.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Rumble {
    pub enabled: bool,
    /// 0-2
    pub intensity: f32,
}

impl Default for Rumble {
    fn default() -> Self {
        Self { enabled: true, intensity: 1.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Touchpad {
    /// Moves the mouse pointer; the touchpad click is then a left click.
    pub mouse: bool,
    pub speed: f32,
    /// Without mouse mode: the touchpad click is the Xbox Back button.
    pub click_as_back: bool,
}

impl Default for Touchpad {
    fn default() -> Self {
        Self { mouse: false, speed: 1.0, click_as_back: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Profile {
    /// Shows up as a virtual Xbox 360 controller.
    pub emulate: bool,
    pub lightbar: Lightbar,
    pub player_leds: PlayerLeds,
    pub mic_led: MicLed,
    pub left_trigger: TriggerEffect,
    pub right_trigger: TriggerEffect,
    pub rumble: Rumble,
    pub left_stick: StickSettings,
    pub right_stick: StickSettings,
    pub l2: TriggerRange,
    pub r2: TriggerRange,
    pub touchpad: Touchpad,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            emulate: true,
            lightbar: Lightbar::default(),
            player_leds: PlayerLeds::default(),
            mic_led: MicLed::Off,
            left_trigger: TriggerEffect::Off,
            right_trigger: TriggerEffect::Off,
            rumble: Rumble::default(),
            left_stick: StickSettings::default(),
            right_stick: StickSettings::default(),
            l2: TriggerRange::default(),
            r2: TriggerRange::default(),
            touchpad: Touchpad::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppPrefs {
    /// Closing the window keeps PadBridge in the tray, so the controllers keep working.
    pub close_to_tray: bool,
    pub start_minimized: bool,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self { close_to_tray: true, start_minimized: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub prefs: AppPrefs,
    /// By controller id (its MAC address).
    pub profiles: HashMap<String, Profile>,
}

pub struct Store {
    path: PathBuf,
    pub data: Settings,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, data }
    }

    pub fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.data) {
            // Written beside and renamed over, so a crash mid-write never leaves half a file.
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn profile(&self, id: &str) -> Profile {
        self.data.profiles.get(id).cloned().unwrap_or_default()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
