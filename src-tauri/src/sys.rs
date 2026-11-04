//! Small Windows helpers: running-process checks and synthetic mouse input.

use std::mem::size_of;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
};
use windows_sys::Win32::System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEINPUT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

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

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn exe_of(pid: u32) -> Option<String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut ok = Process32FirstW(snapshot, &mut entry) != 0;
        let mut found = None;
        while ok {
            if entry.th32ProcessID == pid {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                found = Some(String::from_utf16_lossy(&entry.szExeFile[..len]));
                break;
            }
            ok = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        found
    }
}

/// Executable name of the foreground window, such as `halo.exe`.
pub fn foreground_exe() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            None
        } else {
            exe_of(pid)
        }
    }
}

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "PadBridge";

pub fn set_run_at_startup(enabled: bool) {
    unsafe {
        let subkey = wide(RUN_KEY);
        let mut key = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_SET_VALUE, &mut key) != ERROR_SUCCESS {
            return;
        }
        let name = wide(RUN_VALUE);
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let mut val = wide(&exe.display().to_string());
                let bytes = (val.len() * 2) as u32;
                let _ = RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, val.as_mut_ptr() as *const u8, bytes);
            }
        } else {
            let _ = RegDeleteValueW(key, name.as_ptr());
        }
        RegCloseKey(key);
    }
}

/// First process owns the mutex. A second one signals the event and leaves.
pub struct Instance {
    event: isize,
}

impl Instance {
    pub fn claim() -> Option<Self> {
        unsafe {
            let event_name = wide("Local\\PadBridgeShow");
            let mutex_name = wide("Local\\PadBridge");
            let event = CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr());
            let mutex = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
            if mutex.is_null() {
                return Some(Self { event: event as isize });
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                if !event.is_null() {
                    SetEvent(event);
                }
                return None;
            }
            Some(Self { event: event as isize })
        }
    }

    /// Blocks until another launch asks this one to show its window.
    pub fn wait_for_show(&self) {
        if self.event == 0 {
            return;
        }
        unsafe {
            WaitForSingleObject(self.event as HANDLE, INFINITE);
        }
    }
}
