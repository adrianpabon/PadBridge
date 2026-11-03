//! Finds controllers as they connect, owns their threads and profiles, and keeps the UI informed.

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hidapi::{BusType, HidApi, HidDevice};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::device::{self, Cmd, ControllerInfo, Handle, InputView};
use crate::ds::protocol as p;
use crate::settings::{AppPrefs, Profile, Store};
use crate::sys;
use crate::xinput;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    pub id: String,
    #[serde(flatten)]
    pub view: InputView,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub vigem_installed: bool,
    pub conflicts: Vec<String>,
    pub controllers: Vec<ControllerInfo>,
    pub prefs: AppPrefs,
    pub settings_path: String,
}

/// A real Xbox controller in an XInput slot.
#[derive(Clone)]
struct XboxPad {
    info: ControllerInfo,
    view: InputView,
}

fn xbox_id(slot: u32) -> String {
    format!("xbox-{}", slot + 1)
}

fn xbox_slot(id: &str) -> Option<u32> {
    id.strip_prefix("xbox-")?.parse::<u32>().ok()?.checked_sub(1)
}

pub struct Manager {
    devices: Mutex<HashMap<String, Handle>>,
    xbox: Mutex<[Option<XboxPad>; xinput::SLOTS as usize]>,
    store: Mutex<Store>,
    watching: Mutex<Option<String>>,
    vigem_installed: AtomicBool,
    conflicts: Mutex<Vec<String>>,
    stop: AtomicBool,
}

impl Manager {
    pub fn new(store: Store) -> Arc<Self> {
        Arc::new(Self {
            devices: Mutex::new(HashMap::new()),
            xbox: Mutex::new(Default::default()),
            store: Mutex::new(store),
            watching: Mutex::new(None),
            vigem_installed: AtomicBool::new(vigem_client::Client::connect().is_ok()),
            conflicts: Mutex::new(sys::conflicting_processes()),
            stop: AtomicBool::new(false),
        })
    }

    pub fn start(self: &Arc<Self>, app: AppHandle) {
        let me = self.clone();
        let app2 = app.clone();
        std::thread::Builder::new().name("hotplug".into()).spawn(move || me.hotplug_loop(app2)).unwrap();
        let me = self.clone();
        let app2 = app.clone();
        std::thread::Builder::new().name("xinput".into()).spawn(move || me.xinput_loop(app2)).unwrap();
        let me = self.clone();
        std::thread::Builder::new().name("ui-feed".into()).spawn(move || me.feed_loop(app)).unwrap();
    }

    /// Real Xbox controllers: every XInput slot that isn't one of our virtual controllers.
    fn xinput_loop(self: Arc<Self>, app: AppHandle) {
        let mut last_battery = Instant::now() - Duration::from_secs(60);
        while !self.stop.load(Ordering::Relaxed) {
            // Our virtual controllers' slots; while one hasn't learnt its slot yet, a new controller in XInput
            // could be it, so none is added until it has.
            let (owned, pending) = {
                let devices = self.devices.lock();
                let mut owned = Vec::new();
                let mut pending = false;
                for h in devices.values() {
                    let info = h.shared.info.lock();
                    match (info.emulating, info.slot) {
                        (true, Some(s)) => owned.push(s),
                        (true, None) => pending = true,
                        _ => {}
                    }
                }
                (owned, pending)
            };
            let refresh_battery = last_battery.elapsed() >= Duration::from_secs(3);
            if refresh_battery {
                last_battery = Instant::now();
            }

            let mut changed = false;
            {
                let mut xbox = self.xbox.lock();
                for slot in 0..xinput::SLOTS {
                    let entry = &mut xbox[slot as usize];
                    let state = if owned.contains(&slot) { None } else { xinput::state(slot) };
                    match (state, entry.as_mut()) {
                        (None, Some(_)) => {
                            *entry = None;
                            changed = true;
                        }
                        (Some(g), Some(pad)) => {
                            if refresh_battery {
                                let level = xinput::battery(slot).map(|b| b.level);
                                if level != pad.info.battery {
                                    pad.info.battery = level;
                                    changed = true;
                                }
                            }
                            pad.view = xinput::view(&g, pad.info.battery.unwrap_or(0));
                        }
                        (Some(g), None) if !pending => {
                            let b = xinput::battery(slot);
                            let level = b.as_ref().map(|b| b.level);
                            *entry = Some(XboxPad {
                                info: ControllerInfo {
                                    id: xbox_id(slot),
                                    kind: "xbox".into(),
                                    name: "Control de Xbox".into(),
                                    connection: if b.as_ref().is_some_and(|b| b.wired) { "usb" } else { "wireless" }
                                        .into(),
                                    battery: level,
                                    charging: Default::default(),
                                    slot: Some(slot),
                                    emulating: false,
                                    emulation_error: None,
                                },
                                view: xinput::view(&g, level.unwrap_or(0)),
                            });
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            if changed {
                let _ = app.emit("controllers", self.controllers());
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    /// Rumble pulses on a real Xbox controller, off the caller's thread.
    fn xbox_rumble(slot: u32, pulses: &[(u8, u8, u64)]) {
        let pulses = pulses.to_vec();
        std::thread::spawn(move || {
            for (large, small, ms) in pulses {
                xinput::rumble(slot, large, small);
                std::thread::sleep(Duration::from_millis(ms));
            }
            xinput::rumble(slot, 0, 0);
        });
    }

    /// Stops every controller thread: each resets its controller's rumble and triggers on the way out.
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let mut devices = self.devices.lock();
        for (_, mut h) in devices.drain() {
            h.stop();
        }
    }

    pub fn controllers(&self) -> Vec<ControllerInfo> {
        let mut list: Vec<ControllerInfo> =
            self.devices.lock().values().map(|h| h.shared.info.lock().clone()).collect();
        list.extend(self.xbox.lock().iter().flatten().map(|x| x.info.clone()));
        list.sort_by_key(|c| (c.slot.unwrap_or(u32::MAX), c.id.clone()));
        list
    }

    pub fn overview(&self) -> Overview {
        let store = self.store.lock();
        Overview {
            vigem_installed: self.vigem_installed.load(Ordering::Relaxed),
            conflicts: self.conflicts.lock().clone(),
            controllers: self.controllers(),
            prefs: store.data.prefs.clone(),
            settings_path: store.path().display().to_string(),
        }
    }

    pub fn prefs(&self) -> AppPrefs {
        self.store.lock().data.prefs.clone()
    }

    pub fn set_prefs(&self, prefs: AppPrefs) {
        let mut store = self.store.lock();
        store.data.prefs = prefs;
        store.save();
    }

    pub fn profile(&self, id: &str) -> Profile {
        self.store.lock().profile(id)
    }

    pub fn set_profile(&self, id: &str, profile: Profile) {
        {
            let mut store = self.store.lock();
            store.data.profiles.insert(id.to_string(), profile.clone());
            store.save();
        }
        if let Some(h) = self.devices.lock().get(id) {
            let _ = h.tx.send(Cmd::Profile(Box::new(profile)));
        }
    }

    /// Every other known controller takes this one's profile.
    pub fn apply_to_all(&self, id: &str) {
        let profile = self.profile(id);
        let ids: Vec<String> = {
            let store = self.store.lock();
            let mut ids: Vec<String> = store.data.profiles.keys().cloned().collect();
            ids.extend(self.devices.lock().keys().cloned());
            ids.sort();
            ids.dedup();
            ids
        };
        for other in ids.iter().filter(|o| *o != id) {
            self.set_profile(other, profile.clone());
        }
    }

    pub fn send(&self, id: &str, cmd: Cmd) {
        if let Some(slot) = xbox_slot(id) {
            match cmd {
                Cmd::TestRumble { large, small, ms } => Self::xbox_rumble(slot, &[(large, small, ms)]),
                Cmd::Identify => Self::xbox_rumble(slot, &[(0, 200, 200), (0, 0, 150), (0, 200, 200), (0, 0, 150), (0, 200, 200)]),
                Cmd::Profile(_) => {}
            }
            return;
        }
        if let Some(h) = self.devices.lock().get(id) {
            let _ = h.tx.send(cmd);
        }
    }

    pub fn watch(&self, id: Option<String>) {
        *self.watching.lock() = id;
    }

    fn hotplug_loop(self: Arc<Self>, app: AppHandle) {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                eprintln!("hidapi: {e}");
                return;
            }
        };
        let mut last_checks = Instant::now() - Duration::from_secs(60);
        while !self.stop.load(Ordering::Relaxed) {
            let mut changed = false;

            // Threads whose controller went away
            {
                let mut devices = self.devices.lock();
                let dead: Vec<String> = devices
                    .iter()
                    .filter(|(_, h)| !h.shared.alive.load(Ordering::Relaxed))
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in dead {
                    devices.remove(&id);
                    changed = true;
                }
            }

            if api.refresh_devices().is_ok() {
                let known: Vec<CString> = self.devices.lock().values().map(|h| h.path.clone()).collect();
                let candidates: Vec<(CString, bool, u16)> = api
                    .device_list()
                    .filter(|d| {
                        d.vendor_id() == p::SONY_VENDOR
                            && matches!(d.product_id(), p::DUALSENSE_PID | p::DUALSENSE_EDGE_PID)
                            && d.usage_page() == 0x01
                            && d.usage() == 0x05
                    })
                    .filter(|d| !known.iter().any(|k| k.as_c_str() == d.path()))
                    .map(|d| (CString::from(d.path()), matches!(d.bus_type(), BusType::Bluetooth), d.product_id()))
                    .collect();
                for (path, bluetooth, pid) in candidates {
                    if let Ok(dev) = api.open_path(&path) {
                        changed |= self.add_device(dev, path, bluetooth, pid);
                    }
                }
            }

            if last_checks.elapsed() >= Duration::from_secs(5) {
                last_checks = Instant::now();
                let vigem = vigem_client::Client::connect().is_ok();
                let conflicts = sys::conflicting_processes();
                if self.vigem_installed.swap(vigem, Ordering::Relaxed) != vigem || *self.conflicts.lock() != conflicts {
                    *self.conflicts.lock() = conflicts;
                    let _ = app.emit("overview", self.overview());
                }
            }

            if changed {
                let _ = app.emit("controllers", self.controllers());
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    fn add_device(&self, dev: HidDevice, path: CString, bluetooth: bool, pid: u16) -> bool {
        // The MAC address names the controller the same over USB and Bluetooth.
        let mut pairing = [0u8; p::FEATURE_PAIRING_SIZE];
        pairing[0] = p::FEATURE_PAIRING;
        let id = match dev.get_feature_report(&mut pairing) {
            Ok(n) if n >= 7 => {
                format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", pairing[6], pairing[5], pairing[4], pairing[3], pairing[2], pairing[1])
            }
            _ => format!("{:08X}", crc32fast::hash(path.as_bytes())),
        };
        if self.devices.lock().contains_key(&id) {
            return false; // same controller on both USB and Bluetooth: the first one wins
        }
        if bluetooth {
            let mut cal = [0u8; p::FEATURE_CALIBRATION_SIZE];
            cal[0] = p::FEATURE_CALIBRATION;
            let _ = dev.get_feature_report(&mut cal); // switches it to full reports
        }
        let name = if pid == p::DUALSENSE_EDGE_PID { "DualSense Edge" } else { "DualSense" }.to_string();
        let profile = self.profile(&id);
        let handle = device::spawn(device::Spec { device: dev, path, id: id.clone(), name, bluetooth, profile });
        self.devices.lock().insert(id, handle);
        true
    }

    /// Live input for the controller the UI shows (30 a second), and controller changes (battery, slot...).
    fn feed_loop(self: Arc<Self>, app: AppHandle) {
        while !self.stop.load(Ordering::Relaxed) {
            let watching = self.watching.lock().clone();
            let mut info_changed = false;
            {
                let devices = self.devices.lock();
                for h in devices.values() {
                    info_changed |= h.shared.info_changed.swap(false, Ordering::Relaxed);
                }
                if let Some(id) = watching {
                    if let Some(h) = devices.get(&id) {
                        let view = *h.shared.input.lock();
                        let _ = app.emit("input", InputEvent { id, view });
                    } else if let Some(slot) = xbox_slot(&id) {
                        let view = self.xbox.lock()[slot as usize].as_ref().map(|x| x.view);
                        if let Some(view) = view {
                            let _ = app.emit("input", InputEvent { id, view });
                        }
                    }
                }
            }
            if info_changed {
                let _ = app.emit("controllers", self.controllers());
            }
            std::thread::sleep(Duration::from_millis(33));
        }
    }
}
