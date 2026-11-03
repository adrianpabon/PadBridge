//! Real Xbox (XInput) controllers: games read them directly, PadBridge shows them (input, battery, number) and can
//! rumble them, so every controller in the room is in one list.

use windows_sys::Win32::UI::Input::XboxController::{
    XInputGetBatteryInformation, XInputGetState, XInputSetState, BATTERY_DEVTYPE_GAMEPAD, BATTERY_LEVEL_EMPTY,
    BATTERY_LEVEL_FULL, BATTERY_LEVEL_LOW, BATTERY_LEVEL_MEDIUM, BATTERY_TYPE_DISCONNECTED, BATTERY_TYPE_WIRED,
    XINPUT_BATTERY_INFORMATION, XINPUT_GAMEPAD, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
    XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP,
    XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER,
    XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y, XINPUT_STATE,
    XINPUT_VIBRATION,
};

use crate::device::InputView;
use crate::ds::protocol::{btn, Charging, InputState};

pub const SLOTS: u32 = 4;

pub fn state(slot: u32) -> Option<XINPUT_GAMEPAD> {
    let mut st: XINPUT_STATE = unsafe { std::mem::zeroed() };
    (unsafe { XInputGetState(slot, &mut st) } == 0).then_some(st.Gamepad)
}

pub struct Battery {
    pub level: u8,
    pub wired: bool,
}

pub fn battery(slot: u32) -> Option<Battery> {
    let mut info: XINPUT_BATTERY_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut info) } != 0
        || info.BatteryType == BATTERY_TYPE_DISCONNECTED
    {
        return None;
    }
    let wired = info.BatteryType == BATTERY_TYPE_WIRED;
    // XInput only reports four steps.
    let level = match info.BatteryLevel {
        _ if wired => 100,
        BATTERY_LEVEL_EMPTY => 5,
        BATTERY_LEVEL_LOW => 30,
        BATTERY_LEVEL_MEDIUM => 65,
        BATTERY_LEVEL_FULL => 100,
        _ => 0,
    };
    Some(Battery { level, wired })
}

pub fn rumble(slot: u32, large: u8, small: u8) {
    let v = XINPUT_VIBRATION { wLeftMotorSpeed: large as u16 * 257, wRightMotorSpeed: small as u16 * 257 };
    unsafe {
        XInputSetState(slot, &v);
    }
}

fn to_u8(v: i16, invert: bool) -> u8 {
    let v = if invert { -(v as i32) - 1 } else { v as i32 };
    ((v + 32768) >> 8).clamp(0, 255) as u8
}

/// An XInput state in the same shape as a DualSense's, for the input view: Xbox A is the bottom face button
/// (Cross's place), and so on.
pub fn view(g: &XINPUT_GAMEPAD, battery: u8) -> InputView {
    const MAP: [(u16, u32); 14] = [
        (XINPUT_GAMEPAD_A, btn::CROSS),
        (XINPUT_GAMEPAD_B, btn::CIRCLE),
        (XINPUT_GAMEPAD_X, btn::SQUARE),
        (XINPUT_GAMEPAD_Y, btn::TRIANGLE),
        (XINPUT_GAMEPAD_LEFT_SHOULDER, btn::L1),
        (XINPUT_GAMEPAD_RIGHT_SHOULDER, btn::R1),
        (XINPUT_GAMEPAD_BACK, btn::CREATE),
        (XINPUT_GAMEPAD_START, btn::OPTIONS),
        (XINPUT_GAMEPAD_LEFT_THUMB, btn::L3),
        (XINPUT_GAMEPAD_RIGHT_THUMB, btn::R3),
        (XINPUT_GAMEPAD_DPAD_UP, btn::UP),
        (XINPUT_GAMEPAD_DPAD_DOWN, btn::DOWN),
        (XINPUT_GAMEPAD_DPAD_LEFT, btn::LEFT),
        (XINPUT_GAMEPAD_DPAD_RIGHT, btn::RIGHT),
    ];
    let mut buttons = 0;
    for (from, to) in MAP {
        if g.wButtons & from != 0 {
            buttons |= to;
        }
    }
    if g.bLeftTrigger > 30 {
        buttons |= btn::L2;
    }
    if g.bRightTrigger > 30 {
        buttons |= btn::R2;
    }
    let raw = InputState {
        lx: to_u8(g.sThumbLX, false),
        ly: to_u8(g.sThumbLY, true), // the view's raw Y grows downwards, like a DualSense's
        rx: to_u8(g.sThumbRX, false),
        ry: to_u8(g.sThumbRY, true),
        l2: g.bLeftTrigger,
        r2: g.bRightTrigger,
        buttons,
        battery,
        charging: Charging::Discharging,
        ..Default::default()
    };
    // What the game gets is the controller itself.
    InputView {
        raw,
        out_left: [g.sThumbLX as f32 / 32767.0, g.sThumbLY as f32 / 32767.0],
        out_right: [g.sThumbRX as f32 / 32767.0, g.sThumbRY as f32 / 32767.0],
        out_l2: g.bLeftTrigger as f32 / 255.0,
        out_r2: g.bRightTrigger as f32 / 255.0,
    }
}
