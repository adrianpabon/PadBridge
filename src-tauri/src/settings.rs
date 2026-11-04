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

/// What the game sees in addition to the physical controller, which always stays connected.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    /// The game talks to the real controller.
    Native,
    /// A virtual Xbox 360. The default for a DualSense.
    #[default]
    Xbox360,
    /// A virtual DualShock 4. Games that look for a PlayStation controller can use it.
    DualShock4,
}

impl OutputMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Xbox360 => "xbox360",
            Self::DualShock4 => "dualshock4",
        }
    }
}

/// Gyro used as mouse movement or added to the right stick of a virtual controller.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum GyroAim {
    #[default]
    Off,
    Mouse,
    RightStick,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GyroSettings {
    pub aim: GyroAim,
    /// Scales how far the gyro moves the mouse or the stick.
    pub sensitivity: f32,
    /// Raw gyro units ignored around rest. DualSense units, roughly 0-2000.
    pub deadzone: f32,
}

impl Default for GyroSettings {
    fn default() -> Self {
        Self { aim: GyroAim::Off, sensitivity: 1.0, deadzone: 120.0 }
    }
}

/// How a stick's movement becomes the virtual stick's.
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
    pub output: OutputMode,
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
    pub gyro: GyroSettings,
    /// Destination bit for each source bit 0..23. Zero unbinds that button.
    /// Only the virtual controller sees this; a native controller is read by the game as-is.
    pub buttons: [u32; 23],
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            output: OutputMode::Xbox360,
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
            gyro: GyroSettings::default(),
            buttons: identity_buttons(),
        }
    }
}

pub fn identity_buttons() -> [u32; 23] {
    let mut map = [0; 23];
    for (i, slot) in map.iter_mut().enumerate() {
        *slot = 1 << i;
    }
    map
}

/// A profile applied while a given executable is in the foreground.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameProfile {
    pub exe: String,
    pub profile: Profile,
}

/// Base profile plus optional per-game overrides. Old files stored a bare [`Profile`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ControllerConfig {
    pub base: Profile,
    pub games: Vec<GameProfile>,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self { base: Profile::default(), games: Vec::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppPrefs {
    /// Closing the window keeps PadBridge in the tray, so the controllers keep working.
    pub close_to_tray: bool,
    pub start_minimized: bool,
    /// Register the executable in the current user's Run key.
    pub run_at_startup: bool,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self { close_to_tray: true, start_minimized: false, run_at_startup: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub prefs: AppPrefs,
    /// By controller id (its MAC address, or `xbox-N` for an XInput slot).
    pub profiles: HashMap<String, ControllerConfig>,
}

pub struct Store {
    path: PathBuf,
    pub data: Settings,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read_to_string(&path).ok().and_then(|s| migrate_settings(&s)).unwrap_or_default();
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

    pub fn config(&self, id: &str) -> ControllerConfig {
        self.data.profiles.get(id).cloned().unwrap_or_default()
    }

    pub fn profile(&self, id: &str) -> Profile {
        self.config(id).base
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Accepts the 0.2 file shape, where each profile was a bare object with `emulate: bool`.
fn migrate_settings(raw: &str) -> Option<Settings> {
    let mut value: serde_json::Value = serde_json::from_str(raw).ok()?;
    if let Some(profiles) = value.get_mut("profiles").and_then(|p| p.as_object_mut()) {
        let keys: Vec<String> = profiles.keys().cloned().collect();
        for key in keys {
            let entry = profiles.get_mut(&key)?;
            if entry.get("base").is_some() {
                continue;
            }
            let mut profile = entry.take();
            if let Some(obj) = profile.as_object_mut() {
                if !obj.contains_key("output") {
                    let emulate = obj.get("emulate").and_then(|e| e.as_bool()).unwrap_or(true);
                    obj.insert("output".into(), serde_json::json!(if emulate { "xbox360" } else { "native" }));
                }
            }
            *entry = serde_json::json!({ "base": profile, "games": [] });
        }
    }
    serde_json::from_value(value).ok()
}

pub fn default_trigger(mode: &str) -> TriggerEffect {
    match mode {
        "resistance" => TriggerEffect::Resistance { start: 2, strength: 5 },
        "slope" => TriggerEffect::Slope { start: 0, end: 9, start_strength: 1, end_strength: 8 },
        "weapon" => TriggerEffect::Weapon { start: 3, end: 6, strength: 7 },
        "vibration" => TriggerEffect::Vibration { start: 0, amplitude: 6, frequency: 30 },
        "bow" => TriggerEffect::Bow { start: 1, end: 6, strength: 6, snap: 6 },
        "galloping" => TriggerEffect::Galloping { start: 0, end: 9, first_foot: 2, second_foot: 5, frequency: 8 },
        "machine" => TriggerEffect::Machine { start: 1, end: 9, amp_a: 5, amp_b: 2, frequency: 12, period: 4 },
        "zones" => TriggerEffect::Zones { strengths: [0, 0, 1, 2, 3, 4, 5, 6, 7, 8] },
        _ => TriggerEffect::Off,
    }
}

pub struct Preset {
    pub name: &'static str,
    pub left: TriggerEffect,
    pub right: TriggerEffect,
}

pub fn presets() -> &'static [Preset] {
    use TriggerEffect::*;
    const PRESETS: &[Preset] = &[
        Preset { name: "Normal", left: Off, right: Off },
        Preset {
            name: "Rifle",
            left: Resistance { start: 2, strength: 3 },
            right: Weapon { start: 3, end: 6, strength: 7 },
        },
        Preset {
            name: "Pistola",
            left: Resistance { start: 2, strength: 3 },
            right: Weapon { start: 2, end: 4, strength: 8 },
        },
        Preset {
            name: "Escopeta",
            left: Resistance { start: 2, strength: 4 },
            right: Weapon { start: 4, end: 7, strength: 8 },
        },
        Preset {
            name: "Ametralladora",
            left: Resistance { start: 2, strength: 3 },
            right: Machine { start: 1, end: 9, amp_a: 6, amp_b: 2, frequency: 14, period: 3 },
        },
        Preset {
            name: "Arco",
            left: Off,
            right: Bow { start: 1, end: 6, strength: 7, snap: 7 },
        },
        Preset {
            name: "Carreras",
            left: Slope { start: 0, end: 9, start_strength: 2, end_strength: 7 },
            right: Slope { start: 0, end: 9, start_strength: 1, end_strength: 5 },
        },
        Preset {
            name: "Caballo",
            left: Off,
            right: Galloping { start: 0, end: 9, first_foot: 2, second_foot: 5, frequency: 8 },
        },
        Preset {
            name: "Motor",
            left: Vibration { start: 0, amplitude: 3, frequency: 25 },
            right: Vibration { start: 2, amplitude: 6, frequency: 40 },
        },
        Preset {
            name: "Máxima resistencia",
            left: Resistance { start: 0, strength: 8 },
            right: Resistance { start: 0, strength: 8 },
        },
    ];
    PRESETS
}

/// Trigger editor numbers: mode index, then p0..p5, then ten zone strengths.
pub fn effect_parts(effect: &TriggerEffect) -> (i32, [f32; 6], [f32; 10]) {
    let mut p = [0.0; 6];
    let mut z = [0.0; 10];
    let mode = match effect {
        TriggerEffect::Off => 0,
        TriggerEffect::Resistance { start, strength } => {
            p[0] = *start as f32;
            p[1] = *strength as f32;
            1
        }
        TriggerEffect::Slope { start, end, start_strength, end_strength } => {
            p[0] = *start as f32;
            p[1] = *end as f32;
            p[2] = *start_strength as f32;
            p[3] = *end_strength as f32;
            2
        }
        TriggerEffect::Weapon { start, end, strength } => {
            p[0] = *start as f32;
            p[1] = *end as f32;
            p[2] = *strength as f32;
            3
        }
        TriggerEffect::Vibration { start, amplitude, frequency } => {
            p[0] = *start as f32;
            p[1] = *amplitude as f32;
            p[2] = *frequency as f32;
            4
        }
        TriggerEffect::Bow { start, end, strength, snap } => {
            p[0] = *start as f32;
            p[1] = *end as f32;
            p[2] = *strength as f32;
            p[3] = *snap as f32;
            5
        }
        TriggerEffect::Galloping { start, end, first_foot, second_foot, frequency } => {
            p[0] = *start as f32;
            p[1] = *end as f32;
            p[2] = *first_foot as f32;
            p[3] = *second_foot as f32;
            p[4] = *frequency as f32;
            6
        }
        TriggerEffect::Machine { start, end, amp_a, amp_b, frequency, period } => {
            p[0] = *start as f32;
            p[1] = *end as f32;
            p[2] = *amp_a as f32;
            p[3] = *amp_b as f32;
            p[4] = *frequency as f32;
            p[5] = *period as f32;
            7
        }
        TriggerEffect::Zones { strengths } => {
            for (i, s) in strengths.iter().enumerate() {
                z[i] = *s as f32;
            }
            8
        }
    };
    (mode, p, z)
}

fn u8_at(v: f32, max: u8) -> u8 {
    v.round().clamp(0.0, max as f32) as u8
}

pub fn effect_from_parts(mode: i32, p: [f32; 6], z: [f32; 10]) -> TriggerEffect {
    match mode {
        1 => TriggerEffect::Resistance { start: u8_at(p[0], 9), strength: u8_at(p[1], 8) },
        2 => TriggerEffect::Slope {
            start: u8_at(p[0], 9),
            end: u8_at(p[1], 9),
            start_strength: u8_at(p[2], 8),
            end_strength: u8_at(p[3], 8),
        },
        3 => TriggerEffect::Weapon { start: u8_at(p[0], 9), end: u8_at(p[1], 9), strength: u8_at(p[2], 8) },
        4 => TriggerEffect::Vibration { start: u8_at(p[0], 9), amplitude: u8_at(p[1], 8), frequency: u8_at(p[2], 255) },
        5 => TriggerEffect::Bow {
            start: u8_at(p[0], 9),
            end: u8_at(p[1], 9),
            strength: u8_at(p[2], 8),
            snap: u8_at(p[3], 8),
        },
        6 => TriggerEffect::Galloping {
            start: u8_at(p[0], 9),
            end: u8_at(p[1], 9),
            first_foot: u8_at(p[2], 8),
            second_foot: u8_at(p[3], 8),
            frequency: u8_at(p[4], 255),
        },
        7 => TriggerEffect::Machine {
            start: u8_at(p[0], 9),
            end: u8_at(p[1], 9),
            amp_a: u8_at(p[2], 8),
            amp_b: u8_at(p[3], 8),
            frequency: u8_at(p[4], 255),
            period: u8_at(p[5], 255),
        },
        8 => TriggerEffect::Zones { strengths: std::array::from_fn(|i| u8_at(z[i], 8)) },
        _ => TriggerEffect::Off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_emulate_flag_becomes_output_mode() {
        let raw = r#"{"profiles":{"AA:BB":{"emulate":false}}}"#;
        let settings = migrate_settings(raw).unwrap();
        assert_eq!(settings.profiles["AA:BB"].base.output, OutputMode::Native);
        assert!(settings.profiles["AA:BB"].games.is_empty());
    }

    #[test]
    fn already_wrapped_config_is_kept() {
        let raw = r#"{"profiles":{"AA":{"base":{"output":"dualShock4"},"games":[{"exe":"halo.exe","profile":{"output":"native"}}]}}}"#;
        let settings = migrate_settings(raw).unwrap();
        assert_eq!(settings.profiles["AA"].base.output, OutputMode::DualShock4);
        assert_eq!(settings.profiles["AA"].games[0].exe, "halo.exe");
        assert_eq!(settings.profiles["AA"].games[0].profile.output, OutputMode::Native);
    }
}
