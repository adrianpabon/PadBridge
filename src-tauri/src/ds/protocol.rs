//! DualSense / DualSense Edge HID protocol: input report parsing and output report building, over USB and Bluetooth.
//!
//! Layouts follow Linux's hid-playstation driver (`dualsense_input_report`, `dualsense_output_report_common`); the
//! adaptive trigger effect encodings follow Nielk1's public DualSense trigger effect notes.

use serde::Serialize;

pub const SONY_VENDOR: u16 = 0x054C;
pub const DUALSENSE_PID: u16 = 0x0CE6;
pub const DUALSENSE_EDGE_PID: u16 = 0x0DF2;

pub const USB_INPUT_ID: u8 = 0x01;
pub const BT_INPUT_ID: u8 = 0x31;
pub const USB_OUTPUT_SIZE: usize = 48;
pub const BT_OUTPUT_SIZE: usize = 78;
/// Reading it switches a Bluetooth DualSense from its reduced 0x01 report to the full 0x31 one.
pub const FEATURE_CALIBRATION: u8 = 0x05;
pub const FEATURE_CALIBRATION_SIZE: usize = 41;
/// Pairing info: the controller's MAC address at bytes 1..7 (little endian).
pub const FEATURE_PAIRING: u8 = 0x09;
pub const FEATURE_PAIRING_SIZE: usize = 20;


/// Our button bits (not the controller's layout).
pub mod btn {
    pub const CROSS: u32 = 1 << 0;
    pub const CIRCLE: u32 = 1 << 1;
    pub const SQUARE: u32 = 1 << 2;
    pub const TRIANGLE: u32 = 1 << 3;
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const L2: u32 = 1 << 6;
    pub const R2: u32 = 1 << 7;
    pub const CREATE: u32 = 1 << 8;
    pub const OPTIONS: u32 = 1 << 9;
    pub const L3: u32 = 1 << 10;
    pub const R3: u32 = 1 << 11;
    pub const PS: u32 = 1 << 12;
    pub const TOUCHPAD: u32 = 1 << 13;
    pub const MUTE: u32 = 1 << 14;
    pub const UP: u32 = 1 << 15;
    pub const DOWN: u32 = 1 << 16;
    pub const LEFT: u32 = 1 << 17;
    pub const RIGHT: u32 = 1 << 18;
    // DualSense Edge
    pub const FN_L: u32 = 1 << 19;
    pub const FN_R: u32 = 1 << 20;
    pub const PADDLE_L: u32 = 1 << 21;
    pub const PADDLE_R: u32 = 1 << 22;
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TouchPoint {
    pub active: bool,
    pub id: u8,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Charging {
    #[default]
    Discharging,
    Charging,
    Full,
    Error,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputState {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub buttons: u32,
    pub touch: [TouchPoint; 2],
    pub gyro: [i16; 3],
    pub accel: [i16; 3],
    /// 0-100
    pub battery: u8,
    pub charging: Charging,
}

fn le16(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}

/// Parses a full input report (USB 0x01 with 64 bytes, Bluetooth 0x31). Bluetooth's reduced 0x01 report, sent
/// before the calibration feature report is read, is ignored.
pub fn parse_input(report: &[u8], bluetooth: bool) -> Option<InputState> {
    let s = if bluetooth {
        if report.len() < 78 || report[0] != BT_INPUT_ID {
            return None;
        }
        &report[2..]
    } else {
        if report.len() < 64 || report[0] != USB_INPUT_ID {
            return None;
        }
        &report[1..]
    };

    let mut st = InputState { lx: s[0], ly: s[1], rx: s[2], ry: s[3], l2: s[4], r2: s[5], ..Default::default() };

    let (b0, b1, b2) = (s[7], s[8], s[9]);
    let mut buttons = 0u32;
    let mut set = |cond: bool, bit: u32| {
        if cond {
            buttons |= bit;
        }
    };
    set(b0 & 0x10 != 0, btn::SQUARE);
    set(b0 & 0x20 != 0, btn::CROSS);
    set(b0 & 0x40 != 0, btn::CIRCLE);
    set(b0 & 0x80 != 0, btn::TRIANGLE);
    set(b1 & 0x01 != 0, btn::L1);
    set(b1 & 0x02 != 0, btn::R1);
    set(b1 & 0x04 != 0, btn::L2);
    set(b1 & 0x08 != 0, btn::R2);
    set(b1 & 0x10 != 0, btn::CREATE);
    set(b1 & 0x20 != 0, btn::OPTIONS);
    set(b1 & 0x40 != 0, btn::L3);
    set(b1 & 0x80 != 0, btn::R3);
    set(b2 & 0x01 != 0, btn::PS);
    set(b2 & 0x02 != 0, btn::TOUCHPAD);
    set(b2 & 0x04 != 0, btn::MUTE);
    set(b2 & 0x10 != 0, btn::FN_L);
    set(b2 & 0x20 != 0, btn::FN_R);
    set(b2 & 0x40 != 0, btn::PADDLE_L);
    set(b2 & 0x80 != 0, btn::PADDLE_R);
    // D-pad hat: 0 = up, clockwise, 8 = released
    let (up, right, down, left) = match b0 & 0x0F {
        0 => (true, false, false, false),
        1 => (true, true, false, false),
        2 => (false, true, false, false),
        3 => (false, true, true, false),
        4 => (false, false, true, false),
        5 => (false, false, true, true),
        6 => (false, false, false, true),
        7 => (true, false, false, true),
        _ => (false, false, false, false),
    };
    set(up, btn::UP);
    set(right, btn::RIGHT);
    set(down, btn::DOWN);
    set(left, btn::LEFT);
    st.buttons = buttons;

    st.gyro = [le16(s, 15), le16(s, 17), le16(s, 19)];
    st.accel = [le16(s, 21), le16(s, 23), le16(s, 25)];

    for (i, p) in st.touch.iter_mut().enumerate() {
        let t = &s[32 + i * 4..36 + i * 4];
        p.active = t[0] & 0x80 == 0;
        p.id = t[0] & 0x7F;
        p.x = t[1] as u16 | ((t[2] as u16 & 0x0F) << 8);
        p.y = (t[2] as u16 >> 4) | ((t[3] as u16) << 4);
    }

    let status = s[52];
    st.battery = ((status & 0x0F) as u16 * 10 + 5).min(100) as u8;
    st.charging = match status >> 4 {
        0x0 => Charging::Discharging,
        0x1 => Charging::Charging,
        0x2 => Charging::Full,
        _ => Charging::Error,
    };
    if st.charging == Charging::Full {
        st.battery = 100;
    }
    Some(st)
}

/// Everything the controller is told, sent whole on every write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputState {
    /// Low frequency (left) motor.
    pub motor_large: u8,
    /// High frequency (right) motor.
    pub motor_small: u8,
    pub lightbar: [u8; 3],
    /// 5 bits, left to right.
    pub player_leds: u8,
    /// 0 off, 1 on, 2 pulse
    pub mic_led: u8,
    /// Player LED brightness: 0 high, 1 medium, 2 low
    pub led_brightness: u8,
    pub left_trigger: [u8; 11],
    pub right_trigger: [u8; 11],
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            motor_large: 0,
            motor_small: 0,
            lightbar: [0, 0, 64],
            player_leds: 0,
            mic_led: 0,
            led_brightness: 0,
            left_trigger: TRIGGER_OFF,
            right_trigger: TRIGGER_OFF,
        }
    }
}

/// The 47 bytes shared by the USB and Bluetooth output reports.
fn output_common(o: &OutputState, first: bool) -> [u8; 47] {
    let mut c = [0u8; 47];
    // flag0: compatible vibration, haptics select, right trigger, left trigger
    c[0] = 0x01 | 0x02 | 0x04 | 0x08;
    // flag1: mic LED, lightbar, player LEDs
    c[1] = 0x01 | 0x04 | 0x10;
    c[2] = o.motor_small;
    c[3] = o.motor_large;
    c[8] = o.mic_led;
    c[10..21].copy_from_slice(&o.right_trigger);
    c[21..32].copy_from_slice(&o.left_trigger);
    // flag2: LED brightness, (first write) lightbar setup, compatible vibration v2 (firmware 2.24+)
    c[38] = 0x01 | 0x04 | if first { 0x02 } else { 0 };
    // Fades out the blue light the controller shows until a host takes over the lightbar.
    c[41] = if first { 0x02 } else { 0 };
    c[42] = o.led_brightness;
    c[43] = (o.player_leds & 0x1F) | 0x20; // 0x20: no fade
    c[44..47].copy_from_slice(&o.lightbar);
    c
}

pub fn build_output(o: &OutputState, bluetooth: bool, seq: &mut u8, first: bool) -> Vec<u8> {
    let common = output_common(o, first);
    if bluetooth {
        let mut r = vec![0u8; BT_OUTPUT_SIZE];
        r[0] = BT_INPUT_ID; // 0x31, same id both ways
        r[1] = (*seq & 0x0F) << 4;
        r[2] = 0x10;
        *seq = seq.wrapping_add(1) & 0x0F;
        r[3..50].copy_from_slice(&common);
        let mut h = crc32fast::Hasher::new();
        h.update(&[0xA2]);
        h.update(&r[..74]);
        r[74..78].copy_from_slice(&h.finalize().to_le_bytes());
        r
    } else {
        let mut r = vec![0u8; USB_OUTPUT_SIZE];
        r[0] = 0x02;
        r[1..48].copy_from_slice(&common);
        r
    }
}

/// Linux's player numbers on the five LEDs.
pub fn player_leds_for_slot(slot: u32) -> u8 {
    const IDS: [u8; 5] = [0b00100, 0b01010, 0b10101, 0b11011, 0b11111];
    IDS[(slot as usize).min(4)]
}

// ---------------------------------------------------------------- adaptive triggers

pub const TRIGGER_OFF: [u8; 11] = [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

fn zones_bytes(zones: u16, out: &mut [u8; 11]) {
    out[1] = zones as u8;
    out[2] = (zones >> 8) as u8;
}

/// Resistance per zone (10 zones from released to fully pressed), each 0 (none) to 8 (strongest).
pub fn trigger_zones(strengths: &[u8; 10]) -> [u8; 11] {
    let mut active: u16 = 0;
    let mut forces: u32 = 0;
    for (i, &s) in strengths.iter().enumerate() {
        if s > 0 {
            let s = s.min(8);
            active |= 1 << i;
            forces |= ((s - 1) as u32 & 7) << (3 * i);
        }
    }
    if active == 0 {
        return TRIGGER_OFF;
    }
    let mut out = [0u8; 11];
    out[0] = 0x21;
    zones_bytes(active, &mut out);
    out[3..7].copy_from_slice(&forces.to_le_bytes());
    out
}

/// Even resistance from `start` (0-9) to the end of travel, strength 1-8.
pub fn trigger_resistance(start: u8, strength: u8) -> [u8; 11] {
    let (start, strength) = (start.min(9), strength.clamp(1, 8));
    let mut z = [0u8; 10];
    for s in z.iter_mut().skip(start as usize) {
        *s = strength;
    }
    trigger_zones(&z)
}

/// Resistance growing (or shrinking) from `start` to `end`, flat after `end`.
pub fn trigger_slope(start: u8, end: u8, start_strength: u8, end_strength: u8) -> [u8; 11] {
    let start = start.min(8);
    let end = end.clamp(start + 1, 9);
    let (a, b) = (start_strength.clamp(1, 8) as f32, end_strength.clamp(1, 8) as f32);
    let mut z = [0u8; 10];
    for (i, s) in z.iter_mut().enumerate() {
        let i = i as u8;
        if i < start {
            continue;
        }
        let t = if i >= end { 1.0 } else { (i - start) as f32 / (end - start) as f32 };
        *s = (a + (b - a) * t).round() as u8;
    }
    trigger_zones(&z)
}

/// A gun trigger: resistance from `start` (2-7) that gives way at `end` (start+1 - 8).
pub fn trigger_weapon(start: u8, end: u8, strength: u8) -> [u8; 11] {
    let start = start.clamp(2, 7);
    let end = end.clamp(start + 1, 8);
    let mut out = [0u8; 11];
    out[0] = 0x25;
    zones_bytes((1 << start) | (1 << end), &mut out);
    out[3] = strength.clamp(1, 8) - 1;
    out
}

/// Vibration from `start` (0-9) on, amplitude 1-8, frequency 1-255 Hz.
pub fn trigger_vibration(start: u8, amplitude: u8, frequency: u8) -> [u8; 11] {
    let start = start.min(9);
    let amp = (amplitude.clamp(1, 8) - 1) as u32;
    let mut active: u16 = 0;
    let mut amps: u32 = 0;
    for i in start..10 {
        active |= 1 << i;
        amps |= amp << (3 * i as u32);
    }
    let mut out = [0u8; 11];
    out[0] = 0x26;
    zones_bytes(active, &mut out);
    out[3..7].copy_from_slice(&amps.to_le_bytes());
    out[9] = frequency.max(1);
    out
}

/// A bow: tension from `start` (0-8) to `end` (start+1 - 8), then it snaps back with `snap` force.
pub fn trigger_bow(start: u8, end: u8, strength: u8, snap: u8) -> [u8; 11] {
    let start = start.min(8);
    let end = end.clamp(start + 1, 8);
    let pair = ((strength.clamp(1, 8) - 1) as u16 & 7) | (((snap.clamp(1, 8) - 1) as u16 & 7) << 3);
    let mut out = [0u8; 11];
    out[0] = 0x22;
    zones_bytes((1 << start) | (1 << end), &mut out);
    out[3] = pair as u8;
    out[4] = (pair >> 8) as u8;
    out
}

/// A galloping horse between `start` (0-8) and `end` (start+1 - 9); the feet land at `first_foot` (0-6) and
/// `second_foot` (first+1 - 7) of each cycle.
pub fn trigger_galloping(start: u8, end: u8, first_foot: u8, second_foot: u8, frequency: u8) -> [u8; 11] {
    let start = start.min(8);
    let end = end.clamp(start + 1, 9);
    let first = first_foot.min(6);
    let second = second_foot.clamp(first + 1, 7);
    let mut out = [0u8; 11];
    out[0] = 0x23;
    zones_bytes((1 << start) | (1 << end), &mut out);
    out[3] = (second & 7) | ((first & 7) << 3);
    out[4] = frequency.max(1);
    out
}

/// A machine gun between `start` (1-8) and `end` (start+1 - 9), alternating two amplitudes (0-7).
pub fn trigger_machine(start: u8, end: u8, amp_a: u8, amp_b: u8, frequency: u8, period: u8) -> [u8; 11] {
    let start = start.clamp(1, 8);
    let end = end.clamp(start + 1, 9);
    let mut out = [0u8; 11];
    out[0] = 0x27;
    zones_bytes((1 << start) | (1 << end), &mut out);
    out[3] = (amp_a.min(7) & 7) | ((amp_b.min(7) & 7) << 3);
    out[4] = frequency.max(1);
    out[5] = period;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_encoding() {
        // Zones 5-9 at strength 8: forces 7 in each of their 3-bit slots.
        let r = trigger_resistance(5, 8);
        assert_eq!(r[0], 0x21);
        assert_eq!(u16::from_le_bytes([r[1], r[2]]), 0b11111_00000);
        let forces = u32::from_le_bytes([r[3], r[4], r[5], r[6]]);
        assert_eq!(forces, 0o77777_00000);
    }

    #[test]
    fn weapon_encoding() {
        assert_eq!(&trigger_weapon(2, 5, 8)[..4], &[0x25, 0b0010_0100, 0, 7]);
    }

    #[test]
    fn bluetooth_crc() {
        let mut seq = 0;
        let r = build_output(&OutputState::default(), true, &mut seq, true);
        let mut h = crc32fast::Hasher::new();
        h.update(&[0xA2]);
        h.update(&r[..74]);
        assert_eq!(&r[74..], &h.finalize().to_le_bytes());
        assert_eq!(seq, 1);
    }

    #[test]
    fn usb_input_parse() {
        let mut r = [0u8; 64];
        r[0] = 0x01;
        r[1] = 128; // lx
        r[8] = 0x20 | 0x02; // cross + hat right
        r[9] = 0x01; // L1
        r[10] = 0x01; // PS
        r[33] = 0x80; // touch 0 not touching
        r[37] = 0x05; // touch 1 touching, id 5
        r[38] = 0x80; // x = 0x780
        r[39] = 0x07;
        r[53] = 0x18; // charging, level 8
        let s = parse_input(&r, false).unwrap();
        assert_eq!(s.lx, 128);
        assert_eq!(s.buttons, btn::CROSS | btn::RIGHT | btn::L1 | btn::PS);
        assert!(!s.touch[0].active);
        assert!(s.touch[1].active);
        assert_eq!(s.touch[1].x, 0x780);
        assert_eq!(s.charging, Charging::Charging);
        assert_eq!(s.battery, 85);
    }
}
