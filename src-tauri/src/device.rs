//! One thread per connected controller: reads its reports, feeds its virtual Xbox 360 controller, and writes its
//! lights, rumble and trigger effects.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use hidapi::HidDevice;
use parking_lot::Mutex;
use serde::Serialize;
use vigem_client::{Client, TargetId, XGamepad, Xbox360Wired};

use crate::ds::protocol::{self as p, btn, Charging, InputState, OutputState};
use crate::mapping;
use crate::settings::{LightbarMode, MicLed, PlayerLedMode, Profile};
use crate::sys;

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ControllerInfo {
    pub id: String,
    /// "dualsense" or "xbox"
    pub kind: String,
    pub name: String,
    /// "usb" or "bluetooth"
    pub connection: String,
    /// 0-100, None when the controller doesn't report it.
    pub battery: Option<u8>,
    pub charging: Charging,
    /// XInput slot (0-3) of its virtual Xbox controller.
    pub slot: Option<u32>,
    pub emulating: bool,
    pub emulation_error: Option<String>,
}

/// What the input view shows: the controller's own state and what the game gets.
#[derive(Clone, Copy, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InputView {
    pub raw: InputState,
    /// -1..1, y up
    pub out_left: [f32; 2],
    pub out_right: [f32; 2],
    /// 0..1
    pub out_l2: f32,
    pub out_r2: f32,
}

pub enum Cmd {
    Profile(Box<Profile>),
    TestRumble { large: u8, small: u8, ms: u64 },
    Identify,
}

pub struct Shared {
    pub info: Mutex<ControllerInfo>,
    pub input: Mutex<InputView>,
    pub alive: AtomicBool,
    /// Set when `info` changed, cleared by the manager once it has told the UI.
    pub info_changed: AtomicBool,
}

pub struct Handle {
    pub path: std::ffi::CString,
    pub shared: Arc<Shared>,
    pub tx: Sender<Cmd>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Handle {
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct Spec {
    pub device: HidDevice,
    pub path: std::ffi::CString,
    pub id: String,
    pub name: String,
    pub bluetooth: bool,
    pub profile: Profile,
}

pub fn spawn(spec: Spec) -> Handle {
    let shared = Arc::new(Shared {
        info: Mutex::new(ControllerInfo {
            id: spec.id.clone(),
            kind: "dualsense".into(),
            name: spec.name.clone(),
            connection: if spec.bluetooth { "bluetooth" } else { "usb" }.into(),
            ..Default::default()
        }),
        input: Mutex::new(InputView::default()),
        alive: AtomicBool::new(true),
        info_changed: AtomicBool::new(true),
    });
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel();
    let path = spec.path.clone();
    let join = {
        let shared = shared.clone();
        let stop = stop.clone();
        std::thread::Builder::new()
            .name(format!("pad-{}", spec.id))
            .spawn(move || {
                Worker::new(spec, shared.clone(), stop, rx).run();
                shared.alive.store(false, Ordering::Relaxed);
            })
            .expect("spawn controller thread")
    };
    Handle { path, shared, tx, stop, join: Some(join) }
}

/// The virtual controller and its rumble listener.
struct Virtual {
    target: Xbox360Wired<Client>,
    last: Option<XGamepad>,
    slot: Option<u32>,
    /// Player LED number from the notifications, NO_LED until one arrives.
    led: Arc<AtomicU8>,
    _notify: Option<JoinHandle<()>>,
}

const NO_LED: u8 = 0xFF;

struct Worker {
    dev: HidDevice,
    bluetooth: bool,
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    rx: Receiver<Cmd>,
    profile: Profile,
    triggers: ([u8; 11], [u8; 11]),

    virt: Option<Virtual>,
    next_emulation_try: Instant,
    next_slot_check: Instant,
    /// Game rumble, large << 8 | small, written by the notification thread.
    rumble: Arc<AtomicU16>,
    test_rumble: Option<(u8, u8, Instant)>,
    identify_until: Option<Instant>,

    input: InputState,
    sent: Option<OutputState>,
    seq: u8,
    first_write: bool,
    last_write: Instant,
    started: Instant,

    touch_last: Option<(u8, u16, u16)>,
    touch_rest: (f32, f32),
    mouse_down: bool,
}

impl Worker {
    fn new(spec: Spec, shared: Arc<Shared>, stop: Arc<AtomicBool>, rx: Receiver<Cmd>) -> Self {
        let triggers = (spec.profile.left_trigger.encode(), spec.profile.right_trigger.encode());
        let now = Instant::now();
        Self {
            dev: spec.device,
            bluetooth: spec.bluetooth,
            shared,
            stop,
            rx,
            profile: spec.profile,
            triggers,
            virt: None,
            next_emulation_try: now,
            next_slot_check: now,
            rumble: Arc::new(AtomicU16::new(0)),
            test_rumble: None,
            identify_until: None,
            input: InputState::default(),
            sent: None,
            seq: 0,
            first_write: true,
            last_write: now,
            started: now,
            touch_last: None,
            touch_rest: (0.0, 0.0),
            mouse_down: false,
        }
    }

    fn run(&mut self) {
        let mut buf = [0u8; 128];
        while !self.stop.load(Ordering::Relaxed) {
            self.commands();
            self.manage_emulation();

            match self.dev.read_timeout(&mut buf, 4) {
                Ok(0) => {}
                Ok(n) => {
                    if let Some(st) = p::parse_input(&buf[..n], self.bluetooth) {
                        self.on_input(st);
                    }
                }
                Err(_) => break, // unplugged or switched off
            }

            self.track_slot();
            let out = self.desired_output();
            let due = self.last_write.elapsed() >= Duration::from_millis(5);
            if (self.sent != Some(out) || self.first_write) && due {
                self.write(&out);
            }
        }

        // Leave the controller as a game would find it: no rumble, no trigger resistance.
        if !self.first_write {
            let mut out = self.sent.unwrap_or_default();
            out.motor_large = 0;
            out.motor_small = 0;
            out.left_trigger = p::TRIGGER_OFF;
            out.right_trigger = p::TRIGGER_OFF;
            self.write(&out);
        }
        if self.mouse_down {
            sys::mouse_left(false);
        }
        self.virt = None; // unplugs the virtual controller
    }

    fn write(&mut self, out: &OutputState) {
        let report = p::build_output(out, self.bluetooth, &mut self.seq, self.first_write);
        if self.dev.write(&report).is_ok() {
            self.sent = Some(*out);
            self.first_write = false;
        }
        self.last_write = Instant::now();
    }

    fn commands(&mut self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                Cmd::Profile(profile) => {
                    self.triggers = (profile.left_trigger.encode(), profile.right_trigger.encode());
                    self.profile = *profile;
                    if !self.profile.touchpad.mouse && self.mouse_down {
                        sys::mouse_left(false);
                        self.mouse_down = false;
                    }
                }
                Cmd::TestRumble { large, small, ms } => {
                    self.test_rumble = Some((large, small, Instant::now() + Duration::from_millis(ms)))
                }
                Cmd::Identify => self.identify_until = Some(Instant::now() + Duration::from_millis(1500)),
            }
        }
    }

    fn set_info(&self, f: impl FnOnce(&mut ControllerInfo)) {
        let mut info = self.shared.info.lock();
        f(&mut info);
        self.shared.info_changed.store(true, Ordering::Relaxed);
    }

    fn manage_emulation(&mut self) {
        if !self.profile.emulate {
            if self.virt.take().is_some() {
                self.rumble.store(0, Ordering::Relaxed);
                self.set_info(|i| {
                    i.emulating = false;
                    i.slot = None;
                    i.emulation_error = None;
                });
            }
            return;
        }
        if self.virt.is_some() || Instant::now() < self.next_emulation_try {
            return;
        }
        match self.plug_virtual() {
            Ok(v) => {
                self.virt = Some(v);
                self.set_info(|i| {
                    i.emulating = true;
                    i.emulation_error = None;
                });
            }
            Err(e) => {
                self.next_emulation_try = Instant::now() + Duration::from_secs(5);
                self.set_info(|i| {
                    i.emulating = false;
                    i.emulation_error = Some(e);
                });
            }
        }
    }

    fn plug_virtual(&self) -> Result<Virtual, String> {
        let client = Client::connect().map_err(|_| "ViGEmBus no está instalado".to_string())?;
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin().map_err(|e| format!("no se pudo crear el control virtual: {e:?}"))?;
        target.wait_ready().map_err(|e| format!("el control virtual no respondió: {e:?}"))?;
        let rumble = self.rumble.clone();
        let led = Arc::new(AtomicU8::new(NO_LED));
        let led2 = led.clone();
        let notify = target.request_notification().ok().map(|n| {
            n.spawn_thread(move |_, data| {
                rumble.store(((data.large_motor as u16) << 8) | data.small_motor as u16, Ordering::Relaxed);
                led2.store(data.led_number, Ordering::Relaxed);
            })
        });
        Ok(Virtual { target, last: None, slot: None, led, _notify: notify })
    }

    /// Which Xbox controller number it is: the player LED Windows lights on the virtual controller, sent with the
    /// rumble notifications (the bus's user index answers 0 for every target, so it isn't used). Until the LED
    /// arrives the slot stays unknown, which also keeps the manager from taking it for a real Xbox controller.
    fn track_slot(&mut self) {
        if Instant::now() < self.next_slot_check {
            return;
        }
        self.next_slot_check = Instant::now() + Duration::from_secs(1);
        let Some(v) = self.virt.as_mut() else { return };
        let slot = match v.led.load(Ordering::Relaxed) {
            n if n < 4 => n as u32,
            _ => return,
        };
        if v.slot != Some(slot) {
            v.slot = Some(slot);
            self.set_info(|i| i.slot = Some(slot));
        }
    }

    fn on_input(&mut self, st: InputState) {
        if st.battery != self.input.battery || st.charging != self.input.charging {
            self.set_info(|i| {
                i.battery = Some(st.battery);
                i.charging = st.charging;
            });
        }
        self.input = st;

        let pad = mapping::to_xinput(&st, &self.profile);
        {
            let mut view = self.shared.input.lock();
            view.raw = st;
            view.out_left = [pad.thumb_lx as f32 / 32767.0, pad.thumb_ly as f32 / 32767.0];
            view.out_right = [pad.thumb_rx as f32 / 32767.0, pad.thumb_ry as f32 / 32767.0];
            view.out_l2 = pad.left_trigger as f32 / 255.0;
            view.out_r2 = pad.right_trigger as f32 / 255.0;
        }
        if let Some(v) = self.virt.as_mut() {
            if v.last != Some(pad) && v.target.update(&pad).is_ok() {
                v.last = Some(pad);
            }
        }
        if self.profile.touchpad.mouse {
            self.touch_mouse(&st);
        }
    }

    fn touch_mouse(&mut self, st: &InputState) {
        let t = st.touch[0];
        if t.active {
            if let Some((id, x, y)) = self.touch_last {
                if id == t.id {
                    // The pad is 1920 units wide: half a unit per pixel at speed 1.
                    let k = 0.5 * self.profile.touchpad.speed.clamp(0.1, 5.0);
                    let fx = (t.x as f32 - x as f32) * k + self.touch_rest.0;
                    let fy = (t.y as f32 - y as f32) * k + self.touch_rest.1;
                    let (dx, dy) = (fx.trunc(), fy.trunc());
                    self.touch_rest = (fx - dx, fy - dy);
                    sys::mouse_move(dx as i32, dy as i32);
                }
            }
            self.touch_last = Some((t.id, t.x, t.y));
        } else {
            self.touch_last = None;
            self.touch_rest = (0.0, 0.0);
        }
        let click = st.buttons & btn::TOUCHPAD != 0;
        if click != self.mouse_down {
            sys::mouse_left(click);
            self.mouse_down = click;
        }
    }

    fn desired_output(&mut self) -> OutputState {
        let now = Instant::now();
        let pr = &self.profile;
        let mut out = OutputState { left_trigger: self.triggers.0, right_trigger: self.triggers.1, ..Default::default() };

        // Rumble: a test from the UI, else the game's.
        if let Some((_, _, until)) = self.test_rumble {
            if now >= until {
                self.test_rumble = None;
            }
        }
        if let Some((large, small, _)) = self.test_rumble {
            out.motor_large = large;
            out.motor_small = small;
        } else if pr.rumble.enabled {
            let r = self.rumble.load(Ordering::Relaxed);
            let k = pr.rumble.intensity.clamp(0.0, 2.0);
            out.motor_large = ((r >> 8) as f32 * k).min(255.0) as u8;
            out.motor_small = ((r & 0xFF) as f32 * k).min(255.0) as u8;
        }

        // Lightbar
        let lb = &pr.lightbar;
        let base = match lb.mode {
            LightbarMode::Static => lb.color,
            LightbarMode::Off => [0, 0, 0],
            LightbarMode::Rainbow => {
                // 30 steps a second is smooth, and keeps writes from following every 4 ms loop.
                let t = (self.started.elapsed().as_millis() / 33) as f32 * 0.033;
                hsv((t * lb.speed.clamp(0.02, 3.0)).fract(), 1.0, 1.0)
            }
            LightbarMode::Battery => {
                let b = self.input.battery as f32 / 100.0;
                hsv(b * 0.33, 1.0, 1.0) // red empty, green full
            }
        };
        let k = lb.brightness.clamp(0.0, 1.0);
        out.lightbar = [(base[0] as f32 * k) as u8, (base[1] as f32 * k) as u8, (base[2] as f32 * k) as u8];

        if let Some(until) = self.identify_until {
            if now >= until {
                self.identify_until = None;
            } else {
                let on = (until - now).as_millis() / 150 % 2 == 0;
                out.lightbar = if on { [255, 255, 255] } else { [0, 0, 0] };
                out.player_leds = if on { 0x1F } else { 0 };
                out.motor_small = out.motor_small.max(if on { 120 } else { 0 });
                out.led_brightness = 0;
                out.mic_led = 0;
                return out;
            }
        }

        out.player_leds = match pr.player_leds.mode {
            PlayerLedMode::Auto => self.virt.as_ref().and_then(|v| v.slot).map(p::player_leds_for_slot).unwrap_or(0),
            PlayerLedMode::Custom => pr.player_leds.custom & 0x1F,
            PlayerLedMode::Off => 0,
        };
        out.led_brightness = pr.player_leds.brightness.min(2);
        out.mic_led = match pr.mic_led {
            MicLed::Off => 0,
            MicLed::On => 1,
            MicLed::Pulse => 2,
        };
        out
    }
}

fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - f * s), v * (1.0 - (1.0 - f) * s));
    let (r, g, b) = match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}
