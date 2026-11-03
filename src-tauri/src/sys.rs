//! Small Windows helpers: running-process checks and synthetic mouse input.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEINPUT,
};

/// Programs that also turn PlayStation controllers into Xbox ones: with one running, each controller shows up twice.
const CONFLICTING: [&str; 4] = ["DualSenseX.exe", "DSX.exe", "DS4Windows.exe", "DualSenseY.exe"];

pub fn conflicting_processes() -> Vec<String> {
    let mut found = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return found;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if let Some(c) = CONFLICTING.iter().find(|c| c.eq_ignore_ascii_case(&name)) {
                if !found.iter().any(|f: &String| f == c) {
                    found.push(c.to_string());
                }
            }
            ok = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    found
}

fn send_mouse(dx: i32, dy: i32, flags: u32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    };
    unsafe {
        SendInput(1, &input, size_of::<INPUT>() as i32);
    }
}

pub fn mouse_move(dx: i32, dy: i32) {
    if dx != 0 || dy != 0 {
        send_mouse(dx, dy, MOUSEEVENTF_MOVE);
    }
}

pub fn mouse_left(down: bool) {
    send_mouse(0, 0, if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP });
}
