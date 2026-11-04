//! Finds controllers as they connect, owns their threads and profiles, and keeps the UI informed.

use std::collections::HashMap;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hidapi::{BusType, HidApi, HidDevice};
use parking_lot::Mutex;

use crate::device::{self, Cmd, ControllerInfo, Handle, InputView};
use crate::ds::protocol as p;
use crate::mapping;
use crate::settings::{AppPrefs, ControllerConfig, GameProfile, OutputMode, Profile, Store};
use crate::sys;
use crate::virt;
use crate::xinput;

/// A real Xbox controller in an XInput slot. Its virtual DualShock 4, if any, is extra: the physical
/// pad stays visible to games.
struct XboxPad {
    info: ControllerInfo,
    view: InputView,
    profile: Profile,
    ds4: Option<virt::Ds4Virt>,
    next_ds4: Instant,
}

fn xbox_id(slot: u32) -> String {
    format!("xbox-{}", slot + 1)
}

fn xbox_slot(id: &str) -> Option<u32> {
    id.strip_prefix("xbox-")?.parse::<u32>().ok()?.checked_sub(1)
}

fn is_xbox_id(id: &str) -> bool {
    id.starts_with("xbox-")
}

/// An Xbox pad is already Xbox. Asking for another virtual Xbox would only duplicate it.
fn sanitize(id: &str, mut profile: Profile) -> Profile {
    if is_xbox_id(id) && profile.output == OutputMode::Xbox360 {
        profile.output = OutputMode::Native;
    }
    profile
}

pub struct Manager {
    devices: Mutex<HashMap<String, Handle>>,
    xbox: Mutex<[Option<XboxPad>; xinput::SLOTS as usize]>,
    store: Mutex<Store>,
    /// Foreground executable that is not PadBridge itself. `None` until one is seen.
    active_exe: Mutex<Option<String>>,
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
            active_exe: Mutex::new(None),
            vigem_installed: AtomicBool::new(vigem_client::Client::connect().is_ok()),
            conflicts: Mutex::new(sys::conflicting_processes()),
            stop: AtomicBool::new(false),
        })
    }

    pub fn start(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::Builder::new().name("hotplug".into()).spawn(move || me.hotplug_loop()).unwrap();
        let me = self.clone();
        std::thread::Builder::new().name("xinput".into()).spawn(move || me.xinput_loop()).unwrap();
        let me = self.clone();
        std::thread::Builder::new().name("profiles".into()).spawn(move || me.profile_loop()).unwrap();
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let mut devices = self.devices.lock();
        for (_, mut h) in devices.drain() {
            h.stop();
        }
        // Dropping the virtual DualShock 4 unplugs it.
        *self.xbox.lock() = Default::default();
    }

    fn profile_loop(self: Arc<Self>) {
        while !self.stop.load(Ordering::Relaxed) {
            self.note_foreground(sys::foreground_exe());
            std::thread::sleep(Duration::from_millis(400));
        }
    }

    fn note_foreground(&self, exe: Option<String>) {
        let Some(name) = exe else { return };
        if name.eq_ignore_ascii_case("padbridge.exe") {
            return;
        }
        let changed = {
            let mut cur = self.active_exe.lock();
            if cur.as_ref() == Some(&name) {
                false
            } else {
                *cur = Some(name);
                true
            }
        };
        if changed {
            self.push_all();
        }
    }

    fn stored(&self, id: &str) -> ControllerConfig {
        let mut cfg = self.store.lock().config(id);
        cfg.base = sanitize(id, cfg.base);
        for game in &mut cfg.games {
            game.profile = sanitize(id, game.profile.clone());
        }
        cfg
    }

    pub fn config(&self, id: &str) -> ControllerConfig {
        self.stored(id)
    }

    /// The profile a connected controller should be running right now.
    pub fn effective_profile(&self, id: &str) -> Profile {
        let cfg = self.stored(id);
        let exe = self.active_exe.lock().clone();
        if let Some(exe) = exe {
            if let Some(game) = cfg.games.iter().find(|g| g.exe.eq_ignore_ascii_case(&exe)) {
                return game.profile.clone();
            }
        }
        cfg.base
    }

    /// `Some(exe)` when the foreground game has its own profile for this controller.
    pub fn editing_game(&self, id: &str) -> Option<String> {
        let exe = self.active_exe.lock().clone()?;
        let cfg = self.stored(id);
        cfg.games.into_iter().find(|g| g.exe.eq_ignore_ascii_case(&exe)).map(|g| g.exe)
    }

    pub fn editing_label(&self, id: &str) -> String {
        match self.editing_game(id) {
            Some(exe) => format!("Perfil de {exe}"),
            None => "Perfil base de este control".into(),
        }
    }

    fn push_device(&self, id: &str, profile: &Profile) {
        if is_xbox_id(id) {
            return;
        }
        if let Some(h) = self.devices.lock().get(id) {
            let _ = h.tx.send(Cmd::Profile(Box::new(profile.clone())));
        }
    }

    fn push_all(&self) {
        let ids: Vec<String> = self.devices.lock().keys().cloned().collect();
        for id in ids {
            let profile = self.effective_profile(&id);
            self.push_device(&id, &profile);
        }
    }

    fn write_target(&self, id: &str, game: Option<String>, profile: Profile) {
        let profile = sanitize(id, profile);
        {
            let mut store = self.store.lock();
            let cfg = store.data.profiles.entry(id.to_string()).or_default();
            if let Some(exe) = game {
                if let Some(slot) = cfg.games.iter_mut().find(|g| g.exe.eq_ignore_ascii_case(&exe)) {
                    slot.profile = profile.clone();
                }
            } else {
                cfg.base = profile.clone();
            }
            store.save();
        }
        self.push_device(id, &profile);
    }

    pub fn save_editor(&self, id: &str, profile: Profile) {
        let game = self.editing_game(id);
        self.write_target(id, game, profile);
    }

    pub fn add_game(&self, id: &str, exe: String, profile: Profile) {
        let exe = exe.trim().trim_start_matches(['\\', '/']).to_string();
        if exe.is_empty() {
            return;
        }
        let exe = if exe.to_ascii_lowercase().ends_with(".exe") { exe } else { format!("{exe}.exe") };
        let profile = sanitize(id, profile);
        {
            let mut store = self.store.lock();
            let cfg = store.data.profiles.entry(id.to_string()).or_default();
            if let Some(slot) = cfg.games.iter_mut().find(|g| g.exe.eq_ignore_ascii_case(&exe)) {
                slot.profile = profile;
            } else {
                cfg.games.push(GameProfile { exe, profile });
            }
            store.save();
        }
        self.push_all();
    }

    pub fn remove_game(&self, id: &str, exe: &str) {
        {
            let mut store = self.store.lock();
            if let Some(cfg) = store.data.profiles.get_mut(id) {
                cfg.games.retain(|g| !g.exe.eq_ignore_ascii_case(exe));
            }
            store.save();
        }
        self.push_all();
    }

    pub fn reset(&self, id: &str) -> Profile {
        let mut profile = Profile::default();
        if is_xbox_id(id) {
            profile.output = OutputMode::Native;
        }
        self.save_editor(id, profile.clone());
        profile
    }

    /// Copies the profile onto every other known controller, into the same slot (base or this game).
    pub fn apply_to_all(&self, id: &str, profile: Profile) {
        let game = self.editing_game(id);
        let mut ids: Vec<String> = self.store.lock().data.profiles.keys().cloned().collect();
        ids.extend(self.devices.lock().keys().cloned());
        ids.extend(self.xbox.lock().iter().flatten().map(|p| p.info.id.clone()));
        ids.sort();
        ids.dedup();
        for other in ids.into_iter().filter(|o| o != id) {
            if let Some(ref exe) = game {
                let has_game = self.stored(&other).games.iter().any(|g| g.exe.eq_ignore_ascii_case(exe));
                if !has_game {
                    self.add_game(&other, exe.clone(), profile.clone());
                    continue;
                }
            }
            self.write_target(&other, game.clone(), profile.clone());
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

    pub fn send(&self, id: &str, cmd: Cmd) {
        if let Some(slot) = xbox_slot(id) {
            match cmd {
                Cmd::TestRumble { large, small, ms } => Self::xbox_rumble(slot, &[(large, small, ms)]),
                Cmd::Identify => {
                    Self::xbox_rumble(slot, &[(0, 200, 200), (0, 0, 150), (0, 200, 200), (0, 0, 150), (0, 200, 200)])
                }
                Cmd::Profile(_) => {}
            }
            return;
        }
        if let Some(h) = self.devices.lock().get(id) {
            let _ = h.tx.send(cmd);
        }
    }

    pub fn controllers(&self) -> Vec<ControllerInfo> {
        let mut list: Vec<ControllerInfo> = self.devices.lock().values().map(|h| h.shared.info.lock().clone()).collect();
        list.extend(self.xbox.lock().iter().flatten().map(|x| x.info.clone()));
        list.sort_by_key(|c| (c.slot.unwrap_or(u32::MAX), c.id.clone()));
        list
    }

    pub fn input(&self, id: &str) -> Option<InputView> {
        if let Some(h) = self.devices.lock().get(id) {
            return Some(*h.shared.input.lock());
        }
        let slot = xbox_slot(id)?;
        self.xbox.lock()[slot as usize].as_ref().map(|pad| pad.view)
    }

    pub fn vigem_installed(&self) -> bool {
        self.vigem_installed.load(Ordering::Relaxed)
    }

    pub fn conflicts(&self) -> Vec<String> {
        self.conflicts.lock().clone()
    }

    pub fn settings_path(&self) -> String {
        self.store.lock().path().display().to_string()
    }

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

    fn xinput_loop(self: Arc<Self>) {
        let mut last_battery = Instant::now() - Duration::from_secs(60);
        while !self.stop.load(Ordering::Relaxed) {
            let (owned, pending) = {
                let devices = self.devices.lock();
                let mut owned = Vec::new();
                let mut pending = false;
                for h in devices.values() {
                    let info = h.shared.info.lock();
                    match (info.emulating && info.output == "xbox360", info.slot) {
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
            let profiles: Vec<Profile> = (0..xinput::SLOTS).map(|slot| self.effective_profile(&xbox_id(slot))).collect();

            let mut xbox = self.xbox.lock();
            for slot in 0..xinput::SLOTS {
                let entry = &mut xbox[slot as usize];
                let state = if owned.contains(&slot) { None } else { xinput::state(slot) };
                match (state, entry.as_mut()) {
                    (None, Some(_)) => *entry = None,
                    (Some(g), Some(pad)) => {
                        if refresh_battery {
                            pad.info.battery = xinput::battery(slot).map(|b| b.level);
                        }
                        pad.view = xinput::view(&g, pad.info.battery.unwrap_or(0));
                        Self::drive_xbox(pad, &profiles[slot as usize]);
                    }
                    (Some(g), None) if !pending => {
                        let b = xinput::battery(slot);
                        let level = b.as_ref().map(|b| b.level);
                        let id = xbox_id(slot);
                        let mut pad = XboxPad {
                            info: ControllerInfo {
                                id: id.clone(),
                                kind: "xbox".into(),
                                name: "Control de Xbox".into(),
                                connection: if b.as_ref().is_some_and(|b| b.wired) { "usb" } else { "wireless" }.into(),
                                battery: level,
                                charging: Default::default(),
                                slot: Some(slot),
                                emulating: false,
                                emulation_error: None,
                                output: "native".into(),
                            },
                            view: xinput::view(&g, level.unwrap_or(0)),
                            profile: profiles[slot as usize].clone(),
                            ds4: None,
                            next_ds4: Instant::now(),
                        };
                        Self::drive_xbox(&mut pad, &profiles[slot as usize]);
                        *entry = Some(pad);
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    fn drive_xbox(pad: &mut XboxPad, profile: &Profile) {
        pad.profile = profile.clone();
        if profile.output != OutputMode::DualShock4 {
            pad.ds4 = None;
            pad.info.emulating = false;
            pad.info.output = "native".into();
            pad.info.emulation_error = None;
            return;
        }
        pad.info.output = "dualshock4".into();
        if pad.ds4.is_none() && Instant::now() >= pad.next_ds4 {
            match virt::plug_ds4() {
                Ok(v) => {
                    pad.ds4 = Some(v);
                    pad.info.emulating = true;
                    pad.info.emulation_error = None;
                }
                Err(e) => {
                    pad.next_ds4 = Instant::now() + Duration::from_secs(5);
                    pad.info.emulating = false;
                    pad.info.emulation_error = Some(e);
                }
            }
        }
        let shaped = mapping::shape(&pad.view.raw, profile);
        pad.view.out_left = [shaped.lx, shaped.ly];
        pad.view.out_right = [shaped.rx, shaped.ry];
        pad.view.out_l2 = shaped.l2 as f32 / 255.0;
        pad.view.out_r2 = shaped.r2 as f32 / 255.0;
        if let Some(ds4) = pad.ds4.as_mut() {
            let report = mapping::to_ds4(&pad.view.raw, profile);
            if ds4.last != Some(report) && ds4.target.update(&report).is_ok() {
                ds4.last = Some(report);
            }
        }
    }

    fn hotplug_loop(self: Arc<Self>) {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                eprintln!("hidapi: {e}");
                return;
            }
        };
        let mut last_checks = Instant::now() - Duration::from_secs(60);
        while !self.stop.load(Ordering::Relaxed) {
            {
                let mut devices = self.devices.lock();
                let dead: Vec<String> = devices
                    .iter()
                    .filter(|(_, h)| !h.shared.alive.load(Ordering::Relaxed))
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in dead {
                    devices.remove(&id);
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
                        self.add_device(dev, path, bluetooth, pid);
                    }
                }
            }

            if last_checks.elapsed() >= Duration::from_secs(5) {
                last_checks = Instant::now();
                let vigem = vigem_client::Client::connect().is_ok();
                let conflicts = sys::conflicting_processes();
                self.vigem_installed.store(vigem, Ordering::Relaxed);
                *self.conflicts.lock() = conflicts;
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    }

    fn add_device(&self, dev: HidDevice, path: CString, bluetooth: bool, pid: u16) -> bool {
        let mut pairing = [0u8; p::FEATURE_PAIRING_SIZE];
        pairing[0] = p::FEATURE_PAIRING;
        let id = match dev.get_feature_report(&mut pairing) {
            Ok(n) if n >= 7 => format!(
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                pairing[6], pairing[5], pairing[4], pairing[3], pairing[2], pairing[1]
            ),
            _ => format!("{:08X}", crc32fast::hash(path.as_bytes())),
        };
        if self.devices.lock().contains_key(&id) {
            return false;
        }
        if bluetooth {
            let mut cal = [0u8; p::FEATURE_CALIBRATION_SIZE];
            cal[0] = p::FEATURE_CALIBRATION;
            let _ = dev.get_feature_report(&mut cal);
        }
        let name = if pid == p::DUALSENSE_EDGE_PID { "DualSense Edge" } else { "DualSense" }.to_string();
        let profile = self.effective_profile(&id);
        let handle = device::spawn(device::Spec { device: dev, path, id: id.clone(), name, bluetooth, profile });
        self.devices.lock().insert(id, handle);
        true
    }
}
