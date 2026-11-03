//! DualSense input to Xbox 360 input, through the profile's stick and trigger settings.

use vigem_client::{XButtons, XGamepad};

use crate::ds::protocol::{btn, InputState};
use crate::settings::{Profile, StickSettings, TriggerRange};

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
    let mut raw = 0u16;
    for (from, to) in MAP {
        if input.buttons & from != 0 {
            raw |= to;
        }
    }
    if input.buttons & btn::TOUCHPAD != 0 && !profile.touchpad.mouse && profile.touchpad.click_as_back {
        raw |= XButtons::BACK;
    }

    // DualSense Y grows downwards, XInput's upwards.
    let (lx, ly) = shape_stick(axis(input.lx), -axis(input.ly), &profile.left_stick);
    let (rx, ry) = shape_stick(axis(input.rx), -axis(input.ry), &profile.right_stick);

    XGamepad {
        buttons: XButtons { raw },
        left_trigger: shape_trigger(input.l2, &profile.l2),
        right_trigger: shape_trigger(input.r2, &profile.r2),
        thumb_lx: to_i16(lx),
        thumb_ly: to_i16(ly),
        thumb_rx: to_i16(rx),
        thumb_ry: to_i16(ry),
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
}
