//! Controller input to a virtual Xbox 360 or DualShock 4, through the profile.

use vigem_client::{DS4Report, XButtons, XGamepad};

use crate::ds::protocol::{btn, InputState};
use crate::settings::{GyroAim, GyroSettings, Profile, StickSettings, TriggerRange};

pub const BUTTON_NAMES: [&str; 23] = [
    "Cruz / A",
    "Círculo / B",
    "Cuadrado / X",
    "Triángulo / Y",
    "L1 / LB",
    "R1 / RB",
    "L2 / LT",
    "R2 / RT",
    "Create / View",
    "Options / Menu",
    "L3",
    "R3",
    "PS / Xbox",
    "Panel táctil",
    "Mute",
    "Arriba",
    "Abajo",
    "Izquierda",
    "Derecha",
    "FN izquierdo",
    "FN derecho",
    "Paleta izquierda",
    "Paleta derecha",
];

/// Combo index 0 clears the binding. Index `bit + 1` maps to that button.
pub fn remap_index(bit: u32) -> i32 {
    if bit == 0 {
        0
    } else {
        bit.trailing_zeros() as i32 + 1
    }
}

pub fn index_to_bit(index: i32) -> u32 {
    if index <= 0 {
        0
    } else {
        1 << (index as u32 - 1).min(22)
    }
}

/// A raw stick axis (0-255, centre 127.5) to -1..1.
fn axis(v: u8) -> f32 {
    ((v as f32 - 127.5) / 127.5).clamp(-1.0, 1.0)
}

/// Applies deadzone, curve, sensitivity and anti-deadzone along the stick's direction (radially), so diagonals
/// keep their angle. `x`, `y` are -1..1 with y up.
pub fn shape_stick(x: f32, y: f32, s: &StickSettings) -> (f32, f32) {
    let (x, y) = (if s.invert_x { -x } else { x }, if s.invert_y { -y } else { y });
    let r = (x * x + y * y).sqrt();
    let dz = s.deadzone.clamp(0.0, 0.5);
    if r <= dz || r == 0.0 {
        return (0.0, 0.0);
    }
    let mut n = ((r - dz) / (1.0 - dz)).clamp(0.0, 1.0);
    n = n.powf(s.curve.clamp(0.2, 4.0));
    n = (n * s.sensitivity.clamp(0.1, 4.0)).min(1.0);
    let anti = s.anti_deadzone.clamp(0.0, 0.5);
    n = anti + (1.0 - anti) * n;
    let k = n / r;
    // Squared-off edges: a square's corner is past radius 1, so the components are clamped, not the radius.
    ((x * k).clamp(-1.0, 1.0), (y * k).clamp(-1.0, 1.0))
}

fn to_i16(v: f32) -> i16 {
    (v * 32767.0).round().clamp(-32768.0, 32767.0) as i16
}

pub fn shape_trigger(v: u8, r: &TriggerRange) -> u8 {
    let v = v as f32 / 255.0;
    let dz = r.deadzone.clamp(0.0, 0.9);
    let max = r.max.clamp(dz + 0.05, 1.0);
    let n = ((v - dz) / (max - dz)).clamp(0.0, 1.0);
    (n * 255.0).round() as u8
}

pub struct Shaped {
    pub buttons: u32,
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub l2: u8,
    pub r2: u8,
}

pub fn apply_remap(buttons: u32, map: &[u32; 23]) -> u32 {
    let mut out = 0;
    for i in 0..23 {
        if buttons & (1 << i) != 0 {
            out |= map[i];
        }
    }
    out
}

/// Sticks are -1..1 with y up. Button remap and gyro-to-stick are included.
pub fn shape(input: &InputState, profile: &Profile) -> Shaped {
    let buttons = apply_remap(input.buttons, &profile.buttons);
    // DualSense Y grows downwards. The shaped stick uses y up, like XInput.
    let (lx, ly) = shape_stick(axis(input.lx), -axis(input.ly), &profile.left_stick);
    let (mut rx, mut ry) = shape_stick(axis(input.rx), -axis(input.ry), &profile.right_stick);
    if profile.gyro.aim == GyroAim::RightStick {
        let (gx, gy) = gyro_stick(input.gyro, &profile.gyro);
        rx = (rx + gx).clamp(-1.0, 1.0);
        ry = (ry + gy).clamp(-1.0, 1.0);
    }
    Shaped {
        buttons,
        lx,
        ly,
        rx,
        ry,
        l2: shape_trigger(input.l2, &profile.l2),
        r2: shape_trigger(input.r2, &profile.r2),
    }
}

fn gyro_axis(v: i16, deadzone: f32) -> f32 {
    let v = v as f32;
    if v.abs() < deadzone { 0.0 } else { v / 8000.0 }
}

pub fn gyro_stick(gyro: [i16; 3], settings: &GyroSettings) -> (f32, f32) {
    let sens = settings.sensitivity.clamp(0.1, 6.0);
    (gyro_axis(gyro[2], settings.deadzone) * sens, -gyro_axis(gyro[0], settings.deadzone) * sens)
}

/// Pixel step for one input report. The caller keeps the fractional remainder.
pub fn gyro_mouse(gyro: [i16; 3], settings: &GyroSettings) -> (f32, f32) {
    let sens = settings.sensitivity.clamp(0.1, 8.0);
    let scale = 0.012 * sens;
    let ax = |v: i16| {
        let v = v as f32;
        if v.abs() < settings.deadzone { 0.0 } else { v * scale }
    };
    (ax(gyro[2]), ax(gyro[0]))
}

pub fn to_xinput(input: &InputState, profile: &Profile) -> XGamepad {
    const MAP: [(u32, u16); 15] = [
        (btn::CROSS, XButtons::A),
        (btn::CIRCLE, XButtons::B),
        (btn::SQUARE, XButtons::X),
        (btn::TRIANGLE, XButtons::Y),
        (btn::L1, XButtons::LB),
        (btn::R1, XButtons::RB),
        (btn::CREATE, XButtons::BACK),
        (btn::OPTIONS, XButtons::START),
        (btn::L3, XButtons::LTHUMB),
        (btn::R3, XButtons::RTHUMB),
        (btn::PS, XButtons::GUIDE),
        (btn::UP, XButtons::UP),
        (btn::DOWN, XButtons::DOWN),
        (btn::LEFT, XButtons::LEFT),
        (btn::RIGHT, XButtons::RIGHT),
    ];
    let shaped = shape(input, profile);
    let mut raw = 0u16;
    for (from, to) in MAP {
        if shaped.buttons & from != 0 {
            raw |= to;
        }
    }
    if shaped.buttons & btn::TOUCHPAD != 0 && !profile.touchpad.mouse && profile.touchpad.click_as_back {
        raw |= XButtons::BACK;
    }
    XGamepad {
        buttons: XButtons { raw },
        left_trigger: shaped.l2,
        right_trigger: shaped.r2,
        thumb_lx: to_i16(shaped.lx),
        thumb_ly: to_i16(shaped.ly),
        thumb_rx: to_i16(shaped.rx),
        thumb_ry: to_i16(shaped.ry),
    }
}

fn ds4_axis(v: f32) -> u8 {
    ((v.clamp(-1.0, 1.0) + 1.0) * 127.5).round() as u8
}

/// DS4 hat: 0 north, clockwise, 8 released. Same order as the DualSense.
fn dpad_nibble(buttons: u32) -> u16 {
    let (u, r, d, l) = (
        buttons & btn::UP != 0,
        buttons & btn::RIGHT != 0,
        buttons & btn::DOWN != 0,
        buttons & btn::LEFT != 0,
    );
    match (u, r, d, l) {
        (true, false, false, false) => 0,
        (true, true, false, false) => 1,
        (false, true, false, false) => 2,
        (false, true, true, false) => 3,
        (false, false, true, false) => 4,
        (false, false, true, true) => 5,
        (false, false, false, true) => 6,
        (true, false, false, true) => 7,
        _ => 8,
    }
}

pub fn to_ds4(input: &InputState, profile: &Profile) -> DS4Report {
    const MAP: [(u32, u16); 12] = [
        (btn::SQUARE, 1 << 4),
        (btn::CROSS, 1 << 5),
        (btn::CIRCLE, 1 << 6),
        (btn::TRIANGLE, 1 << 7),
        (btn::L1, 1 << 8),
        (btn::R1, 1 << 9),
        (btn::L2, 1 << 10),
        (btn::R2, 1 << 11),
        (btn::CREATE, 1 << 12),
        (btn::OPTIONS, 1 << 13),
        (btn::L3, 1 << 14),
        (btn::R3, 1 << 15),
    ];
    let shaped = shape(input, profile);
    let mut buttons = dpad_nibble(shaped.buttons);
    for (from, to) in MAP {
        if shaped.buttons & from != 0 {
            buttons |= to;
        }
    }
    if shaped.l2 > 0 {
        buttons |= 1 << 10;
    }
    if shaped.r2 > 0 {
        buttons |= 1 << 11;
    }
    if shaped.buttons & btn::TOUCHPAD != 0 && !profile.touchpad.mouse && profile.touchpad.click_as_back {
        buttons |= 1 << 12;
    }
    let mut special = 0u8;
    if shaped.buttons & btn::PS != 0 {
        special |= 1;
    }
    if shaped.buttons & btn::TOUCHPAD != 0 {
        special |= 1 << 1;
    }
    DS4Report {
        thumb_lx: ds4_axis(shaped.lx),
        thumb_ly: ds4_axis(-shaped.ly),
        thumb_rx: ds4_axis(shaped.rx),
        thumb_ry: ds4_axis(-shaped.ry),
        buttons,
        special,
        trigger_l: shaped.l2,
        trigger_r: shaped.r2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_and_full_tilt() {
        let s = StickSettings::default();
        assert_eq!(shape_stick(0.03, 0.0, &s), (0.0, 0.0));
        let (x, _) = shape_stick(1.0, 0.0, &s);
        assert!((x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sensitivity_scales_and_saturates() {
        let s = StickSettings { sensitivity: 2.0, deadzone: 0.0, ..Default::default() };
        let (x, _) = shape_stick(0.25, 0.0, &s);
        assert!((x - 0.5).abs() < 1e-5);
        let (x, _) = shape_stick(0.9, 0.0, &s);
        assert_eq!(x, 1.0);
    }

    #[test]
    fn trigger_range() {
        let r = TriggerRange { deadzone: 0.2, max: 0.8 };
        assert_eq!(shape_trigger(40, &r), 0);
        assert_eq!(shape_trigger(230, &r), 255);
    }

    #[test]
    fn edge_paddle_can_become_cross() {
        let mut profile = Profile::default();
        profile.buttons[21] = btn::CROSS;
        let mut input = InputState::default();
        input.buttons = btn::PADDLE_L;
        let pad = to_xinput(&input, &profile);
        assert_ne!(pad.buttons.raw & XButtons::A, 0);
    }

    #[test]
    fn neutral_ds4_hat_is_released() {
        let report = to_ds4(&InputState::default(), &Profile::default());
        assert_eq!(report.buttons & 0x0F, 8);
        assert_eq!(report.thumb_lx, 128);
    }
}
