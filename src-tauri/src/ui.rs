//! Slint window. The profile on screen is read back a few times a second and saved when it changes.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::device::Cmd;
use crate::manager::Manager;
use crate::mapping::{index_to_bit, remap_index};
use crate::settings::{
    effect_from_parts, effect_parts, presets, AppPrefs, ControllerConfig, GyroAim, GyroSettings, Lightbar,
    LightbarMode, MicLed, OutputMode, PlayerLedMode, PlayerLeds, Profile, Rumble, StickSettings, Touchpad, TriggerRange,
};
use crate::sys::Instance;

slint::include_modules!();

fn q(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

fn output_from_index(xbox: bool, index: i32) -> OutputMode {
    if xbox {
        if index == 1 { OutputMode::DualShock4 } else { OutputMode::Native }
    } else {
        match index {
            0 => OutputMode::Native,
            2 => OutputMode::DualShock4,
            _ => OutputMode::Xbox360,
        }
    }
}

fn index_from_output(xbox: bool, mode: OutputMode) -> i32 {
    if xbox {
        if mode == DualShock4 { 1 } else { 0 }
    } else {
        match mode {
            OutputMode::Native => 0,
            OutputMode::Xbox360 => 1,
            OutputMode::DualShock4 => 2,
        }
    }
}

fn light_mode(mode: i32) -> LightbarMode {
    match mode {
        1 => LightbarMode::Rainbow,
        2 => LightbarMode::Battery,
        3 => LightbarMode::Off,
        _ => LightbarMode::Static,
    }
}

fn light_index(mode: LightbarMode) -> i32 {
    match mode {
        LightbarMode::Static => 0,
        LightbarMode::Rainbow => 1,
        LightbarMode::Battery => 2,
        LightbarMode::Off => 3,
    }
}

fn led_mode(mode: i32) -> PlayerLedMode {
    match mode {
        1 => PlayerLedMode::Custom,
        2 => PlayerLedMode::Off,
        _ => PlayerLedMode::Auto,
    }
}

fn led_index(mode: PlayerLedMode) -> i32 {
    match mode {
        PlayerLedMode::Auto => 0,
        PlayerLedMode::Custom => 1,
        PlayerLedMode::Off => 2,
    }
}

fn mic_mode(mode: i32) -> MicLed {
    match mode {
        1 => MicLed::On,
        2 => MicLed::Pulse,
        _ => MicLed::Off,
    }
}

fn mic_index(mode: MicLed) -> i32 {
    match mode {
        MicLed::Off => 0,
        MicLed::On => 1,
        MicLed::Pulse => 2,
    }
}

fn gyro_mode(mode: i32) -> GyroAim {
    match mode {
        1 => GyroAim::Mouse,
        2 => GyroAim::RightStick,
        _ => GyroAim::Off,
    }
}

fn gyro_index(mode: GyroAim) -> i32 {
    match mode {
        GyroAim::Off => 0,
        GyroAim::Mouse => 1,
        GyroAim::RightStick => 2,
    }
}

fn led_bits(ui: &AppWindow) -> u8 {
    let mut bits = 0u8;
    if ui.get_led0() { bits |= 1 << 0; }
    if ui.get_led1() { bits |= 1 << 1; }
    if ui.get_led2() { bits |= 1 << 2; }
    if ui.get_led3() { bits |= 1 << 3; }
    if ui.get_led4() { bits |= 1 << 4; }
    bits
}

fn set_led_bits(ui: &AppWindow, bits: u8) {
    ui.set_led0(bits & 1 != 0);
    ui.set_led1(bits & 2 != 0);
    ui.set_led2(bits & 4 != 0);
    ui.set_led3(bits & 8 != 0);
    ui.set_led4(bits & 16 != 0);
}

struct Parts {
    mode: i32,
    p: [f32; 6],
    z: [f32; 10],
}

fn read_left(ui: &AppWindow) -> Parts {
    Parts {
        mode: ui.get_l_mode(),
        p: [ui.get_l_p0(), ui.get_l_p1(), ui.get_l_p2(), ui.get_l_p3(), ui.get_l_p4(), ui.get_l_p5()],
        z: [ui.get_l_z0(), ui.get_l_z1(), ui.get_l_z2(), ui.get_l_z3(), ui.get_l_z4(), ui.get_l_z5(), ui.get_l_z6(), ui.get_l_z7(), ui.get_l_z8(), ui.get_l_z9()],
    }
}

fn read_right(ui: &AppWindow) -> Parts {
    Parts {
        mode: ui.get_r_mode(),
        p: [ui.get_r_p0(), ui.get_r_p1(), ui.get_r_p2(), ui.get_r_p3(), ui.get_r_p4(), ui.get_r_p5()],
        z: [ui.get_r_z0(), ui.get_r_z1(), ui.get_r_z2(), ui.get_r_z3(), ui.get_r_z4(), ui.get_r_z5(), ui.get_r_z6(), ui.get_r_z7(), ui.get_r_z8(), ui.get_r_z9()],
    }
}

fn write_left(ui: &AppWindow, parts: Parts) {
    ui.set_l_mode(parts.mode);
    ui.set_l_p0(parts.p[0]); ui.set_l_p1(parts.p[1]); ui.set_l_p2(parts.p[2]);
    ui.set_l_p3(parts.p[3]); ui.set_l_p4(parts.p[4]); ui.set_l_p5(parts.p[5]);
    ui.set_l_z0(parts.z[0]); ui.set_l_z1(parts.z[1]); ui.set_l_z2(parts.z[2]); ui.set_l_z3(parts.z[3]); ui.set_l_z4(parts.z[4]);
    ui.set_l_z5(parts.z[5]); ui.set_l_z6(parts.z[6]); ui.set_l_z7(parts.z[7]); ui.set_l_z8(parts.z[8]); ui.set_l_z9(parts.z[9]);
}

fn write_right(ui: &AppWindow, parts: Parts) {
    ui.set_r_mode(parts.mode);
    ui.set_r_p0(parts.p[0]); ui.set_r_p1(parts.p[1]); ui.set_r_p2(parts.p[2]);
    ui.set_r_p3(parts.p[3]); ui.set_r_p4(parts.p[4]); ui.set_r_p5(parts.p[5]);
    ui.set_r_z0(parts.z[0]); ui.set_r_z1(parts.z[1]); ui.set_r_z2(parts.z[2]); ui.set_r_z3(parts.z[3]); ui.set_r_z4(parts.z[4]);
    ui.set_r_z5(parts.z[5]); ui.set_r_z6(parts.z[6]); ui.set_r_z7(parts.z[7]); ui.set_r_z8(parts.z[8]); ui.set_r_z9(parts.z[9]);
}

fn parts_of(effect: &crate::settings::TriggerEffect) -> Parts {
    let (mode, p, z) = effect_parts(effect);
    Parts { mode, p, z }
}

fn stick_of(sens: f32, dz: f32, anti: f32, curve: f32, inv_x: bool, inv_y: bool) -> StickSettings {
    StickSettings { sensitivity: q(sens), deadzone: q(dz), anti_deadzone: q(anti), curve: q(curve), invert_x: inv_x, invert_y: inv_y }
}

fn pull(ui: &AppWindow, buttons: &[u32; 23]) -> Profile {
    let left = read_left(ui);
    let right = read_right(ui);
    Profile {
        output: output_from_index(ui.get_xbox(), ui.get_output_index()),
        lightbar: Lightbar {
            mode: light_mode(ui.get_light_mode()),
            color: [ui.get_light_r().round() as u8, ui.get_light_g().round() as u8, ui.get_light_b().round() as u8],
            brightness: q(ui.get_light_bright()),
            speed: q(ui.get_light_speed()),
        },
        player_leds: PlayerLeds { mode: led_mode(ui.get_led_mode()), custom: led_bits(ui), brightness: ui.get_led_bright().round().clamp(0.0, 2.0) as u8 },
        mic_led: mic_mode(ui.get_mic_mode()),
        left_trigger: effect_from_parts(left.mode, left.p, left.z),
        right_trigger: effect_from_parts(right.mode, right.p, right.z),
        rumble: Rumble { enabled: ui.get_rumble_on(), intensity: q(ui.get_rumble_gain()) },
        left_stick: stick_of(ui.get_l_sens(), ui.get_l_dz(), ui.get_l_anti(), ui.get_l_curve(), ui.get_l_inv_x(), ui.get_l_inv_y()),
        right_stick: stick_of(ui.get_r_sens(), ui.get_r_dz(), ui.get_r_anti(), ui.get_r_curve(), ui.get_r_inv_x(), ui.get_r_inv_y()),
        l2: TriggerRange { deadzone: q(ui.get_l2_dz()), max: q(ui.get_l2_max()) },
        r2: TriggerRange { deadzone: q(ui.get_r2_dz()), max: q(ui.get_r2_max()) },
        touchpad: Touchpad { mouse: ui.get_touch_mouse(), speed: q(ui.get_touch_speed()), click_as_back: ui.get_touch_back() },
        gyro: GyroSettings { aim: gyro_mode(ui.get_gyro_mode()), sensitivity: q(ui.get_gyro_sens()), deadzone: q(ui.get_gyro_dz()) },
        buttons: *buttons,
    }
}

fn push(ui: &AppWindow, profile: &Profile, buttons: &Rc<RefCell<[u32; 23]>>) {
    let xbox = ui.get_xbox();
    ui.set_output_index(index_from_output(xbox, profile.output));
    ui.set_l_sens(profile.left_stick.sensitivity);
    ui.set_l_dz(profile.left_stick.deadzone);
    ui.set_l_anti(profile.left_stick.anti_deadzone);
    ui.set_l_curve(profile.left_stick.curve);
    ui.set_l_inv_x(profile.left_stick.invert_x);
    ui.set_l_inv_y(profile.left_stick.invert_y);
    ui.set_r_sens(profile.right_stick.sensitivity);
    ui.set_r_dz(profile.right_stick.deadzone);
    ui.set_r_anti(profile.right_stick.anti_deadzone);
    ui.set_r_curve(profile.right_stick.curve);
    ui.set_r_inv_x(profile.right_stick.invert_x);
    ui.set_r_inv_y(profile.right_stick.invert_y);
    ui.set_l2_dz(profile.l2.deadzone);
    ui.set_l2_max(profile.l2.max);
    ui.set_r2_dz(profile.r2.deadzone);
    ui.set_r2_max(profile.r2.max);
    ui.set_rumble_on(profile.rumble.enabled);
    ui.set_rumble_gain(profile.rumble.intensity);
    ui.set_touch_mouse(profile.touchpad.mouse);
    ui.set_touch_speed(profile.touchpad.speed);
    ui.set_touch_back(profile.touchpad.click_as_back);
    ui.set_gyro_mode(gyro_index(profile.gyro.aim));
    ui.set_gyro_sens(profile.gyro.sensitivity);
    ui.set_gyro_dz(profile.gyro.deadzone);
    ui.set_light_mode(light_index(profile.lightbar.mode));
    ui.set_light_r(profile.lightbar.color[0] as f32);
    ui.set_light_g(profile.lightbar.color[1] as f32);
    ui.set_light_b(profile.lightbar.color[2] as f32);
    ui.set_light_bright(profile.lightbar.brightness);
    ui.set_light_speed(profile.lightbar.speed);
    ui.set_led_mode(led_index(profile.player_leds.mode));
    ui.set_led_bright(profile.player_leds.brightness as f32);
    set_led_bits(ui, profile.player_leds.custom);
    ui.set_mic_mode(mic_index(profile.mic_led));
    write_left(ui, parts_of(&profile.left_trigger));
    write_right(ui, parts_of(&profile.right_trigger));
    *buttons.borrow_mut() = profile.buttons;
    ui.set_remap_gen(ui.get_remap_gen().wrapping_add(1));
}

fn badge(kind: &str, output: &str, slot: Option<u32>, error: bool) -> String {
    if error {
        return "Error".into();
    }
    match (kind, output, slot) {
        ("xbox", "dualshock4", Some(s)) => format!("Xbox {} + DS4", s + 1),
        ("xbox", _, Some(s)) => format!("Xbox {}", s + 1),
        (_, "dualshock4", _) => "DualShock 4".into(),
        (_, "xbox360", Some(s)) => format!("Xbox {}", s + 1),
        (_, "native", _) => "Nativo".into(),
        (_, "xbox360", None) => "Xbox …".into(),
        _ => "…".into(),
    }
}

fn games_of(cfg: &ControllerConfig) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(cfg.games.iter().map(|g| SharedString::from(g.exe.as_str())).collect::<Vec<_>>()))
}

fn tray_icon() -> slint::Image {
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(32, 32);
    for (i, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = i % 32;
        let y = i / 32;
        let edge = x < 2 || y < 2 || x > 29 || y > 29;
        *pixel = slint::Rgba8Pixel {
            r: if edge { 16 } else { 47 },
            g: if edge { 24 } else { 109 },
            b: if edge { 48 } else { 235 },
            a: 255,
        };
    }
    slint::Image::from_rgba8(buffer)
}

pub fn launch(mgr: std::sync::Arc<Manager>, instance: Instance) {
    let ui = AppWindow::new().expect("ventana");
    let prefs = mgr.prefs();
    ui.set_close_to_tray(prefs.close_to_tray);
    ui.set_start_minimized(prefs.start_minimized);
    ui.set_run_at_startup(prefs.run_at_startup);

    let tray = TrayIcon::new().ok();
    if let Some(tray) = &tray {
        tray.set_icon(tray_icon());
        let weak = ui.as_weak();
        tray.on_show_app(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.show();
            }
        });
        let mgr_quit = mgr.clone();
        tray.on_quit_app(move || {
            mgr_quit.shutdown();
            let _ = slint::quit_event_loop();
        });
    }

    let buttons = Rc::new(RefCell::new(crate::settings::identity_buttons()));
    let loaded_id = Rc::new(RefCell::new(String::new()));
    let loaded_game = Rc::new(RefCell::new(String::new()));
    let last_profile = Rc::new(RefCell::new(Profile::default()));
    let last_prefs = Rc::new(RefCell::new(prefs));
    let list_sig = Rc::new(RefCell::new(String::new()));

    let weak = ui.as_weak();
    ui.on_bit_on(|mask, bit| (mask as u32) & (bit as u32) != 0);
    {
        let buttons = buttons.clone();
        ui.on_remap_target(move |index, _gen| {
            let map = buttons.borrow();
            let bit = map.get(index as usize).copied().unwrap_or(0);
            remap_index(bit)
        });
    }
    {
        let buttons = buttons.clone();
        let weak = weak.clone();
        ui.on_set_remap(move |index, target| {
            if !(0..23).contains(&index) {
                return;
            }
            buttons.borrow_mut()[index as usize] = index_to_bit(target);
            if let Some(ui) = weak.upgrade() {
                ui.set_remap_gen(ui.get_remap_gen().wrapping_add(1));
            }
        });
    }
    {
        let weak = weak.clone();
        ui.on_select(move |id| {
            if let Some(ui) = weak.upgrade() {
                ui.set_selected_id(id);
                ui.set_settings_open(false);
                ui.set_tab(0);
            }
        });
    }

    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        ui.on_identify(move || {
            if let Some(ui) = weak.upgrade() {
                let id = ui.get_selected_id().to_string();
                if !id.is_empty() {
                    mgr.send(&id, Cmd::Identify);
                }
            }
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        ui.on_test_strong(move || {
            if let Some(ui) = weak.upgrade() {
                mgr.send(&ui.get_selected_id().to_string(), Cmd::TestRumble { large: 255, small: 0, ms: 700 });
            }
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        ui.on_test_soft(move || {
            if let Some(ui) = weak.upgrade() {
                mgr.send(&ui.get_selected_id().to_string(), Cmd::TestRumble { large: 0, small: 255, ms: 700 });
            }
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        let buttons = buttons.clone();
        let loaded_id = loaded_id.clone();
        let loaded_game = loaded_game.clone();
        let last_profile = last_profile.clone();
        ui.on_apply_all(move || {
            let Some(ui) = weak.upgrade() else { return };
            let id = ui.get_selected_id().to_string();
            if id.is_empty() { return; }
            let profile = pull(&ui, &buttons.borrow());
            mgr.apply_to_all(&id, profile);
            *loaded_id.borrow_mut() = String::new();
            *loaded_game.borrow_mut() = String::new();
            *last_profile.borrow_mut() = Profile::default();
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        let loaded_id = loaded_id.clone();
        ui.on_reset_profile(move || {
            let Some(ui) = weak.upgrade() else { return };
            let id = ui.get_selected_id().to_string();
            if id.is_empty() { return; }
            mgr.reset(&id);
            *loaded_id.borrow_mut() = String::new();
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        let buttons = buttons.clone();
        let loaded_id = loaded_id.clone();
        ui.on_preset(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            let id = ui.get_selected_id().to_string();
            if id.is_empty() { return; }
            let Some(preset) = presets().get(index as usize) else { return };
            let mut profile = pull(&ui, &buttons.borrow());
            profile.left_trigger = preset.left.clone();
            profile.right_trigger = preset.right.clone();
            mgr.save_editor(&id, profile);
            *loaded_id.borrow_mut() = String::new();
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        let buttons = buttons.clone();
        ui.on_add_game(move |exe| {
            let Some(ui) = weak.upgrade() else { return };
            let id = ui.get_selected_id().to_string();
            if id.is_empty() { return; }
            let profile = pull(&ui, &buttons.borrow());
            mgr.add_game(&id, exe.to_string(), profile);
            ui.set_new_game(SharedString::new());
            ui.set_games(games_of(&mgr.config(&id)));
        });
    }
    {
        let mgr = mgr.clone();
        let weak = weak.clone();
        ui.on_remove_game(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            let id = ui.get_selected_id().to_string();
            let cfg = mgr.config(&id);
            if let Some(game) = cfg.games.get(index as usize) {
                let exe = game.exe.clone();
                mgr.remove_game(&id, &exe);
                ui.set_games(games_of(&mgr.config(&id)));
            }
        });
    }

    let weak = ui.as_weak();
    let mgr_timer = mgr.clone();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(50), move || {
        let Some(ui) = weak.upgrade() else { return };
        let controllers = mgr_timer.controllers();
        let sig = controllers.iter().map(|c| format!("{}:{}:{}:{}:{}", c.id, c.output, c.battery.unwrap_or(255), c.slot.unwrap_or(9), c.emulation_error.is_some())).collect::<String>();
        let list_changed = sig != *list_sig.borrow();
        if list_changed {
            *list_sig.borrow_mut() = sig;
            let rows = controllers.iter().map(|c| Ctrl {
                id: c.id.as_str().into(),
                name: c.name.as_str().into(),
                badge: badge(&c.kind, &c.output, c.slot, c.emulation_error.is_some()).into(),
                meta: format!("{} · {}", connection(&c.connection), battery_label(c.battery, &c.charging)).into(),
            }).collect::<Vec<_>>();
            ui.set_controllers(ModelRc::new(VecModel::from(rows)));
        }
        let mut id = ui.get_selected_id().to_string();
        if !controllers.iter().any(|c| c.id == id) {
            id = controllers.first().map(|c| c.id.clone()).unwrap_or_default();
            ui.set_selected_id(id.as_str().into());
        }
        let selected = controllers.iter().find(|c| c.id == id);
        let xbox = selected.is_some_and(|c| c.kind == "xbox");
        if ui.get_xbox() != xbox {
            ui.set_xbox(xbox);
            ui.set_show_ds(!xbox);
            *loaded_id.borrow_mut() = String::new();
        }
        if let Some(c) = selected {
            ui.set_heading(c.name.as_str().into());
            ui.set_subheading(c.id.as_str().into());
            ui.set_editing(mgr_timer.editing_label(&id).into());
        }
        let game = mgr_timer.editing_game(&id).unwrap_or_default();
        let switched = id != *loaded_id.borrow() || game != *loaded_game.borrow();
        if !id.is_empty() && switched {
            let profile = mgr_timer.effective_profile(&id);
            push(&ui, &profile, &buttons);
            *last_profile.borrow_mut() = profile;
            *loaded_id.borrow_mut() = id.clone();
            *loaded_game.borrow_mut() = game;
            ui.set_games(games_of(&mgr_timer.config(&id)));
        } else if !id.is_empty() && !ui.get_settings_open() {
            let profile = pull(&ui, &buttons.borrow());
            let profile_changed = profile != *last_profile.borrow();
            if profile_changed {
                mgr_timer.save_editor(&id, profile.clone());
                *last_profile.borrow_mut() = profile;
                ui.set_games(games_of(&mgr_timer.config(&id)));
            }
        }
        if let Some(view) = mgr_timer.input(&id) {
            let ax = |v: u8| (v as f32 - 127.5) / 127.5;
            ui.set_live_buttons(view.raw.buttons as i32);
            ui.set_live_lx(ax(view.raw.lx));
            ui.set_live_ly(-ax(view.raw.ly));
            ui.set_live_rx(ax(view.raw.rx));
            ui.set_live_ry(-ax(view.raw.ry));
            ui.set_live_olx(view.out_left[0]);
            ui.set_live_oly(view.out_left[1]);
            ui.set_live_orx(view.out_right[0]);
            ui.set_live_ory(view.out_right[1]);
            ui.set_live_l2(view.raw.l2 as f32 / 255.0);
            ui.set_live_r2(view.raw.r2 as f32 / 255.0);
            ui.set_gyro_text(format!("Giroscopio  X {}  Y {}  Z {}", view.raw.gyro[0], view.raw.gyro[1], view.raw.gyro[2]).into());
            ui.set_accel_text(format!("Acelerómetro  X {}  Y {}  Z {}", view.raw.accel[0], view.raw.accel[1], view.raw.accel[2]).into());
        }
        ui.set_vigem_banner(if mgr_timer.vigem_installed() {
            SharedString::new()
        } else {
            "No se encontró ViGEmBus. Sin ese driver no hay controles virtuales. Instálalo desde github.com/nefarius/ViGEmBus/releases y reinicia PadBridge.".into()
        });
        let conflicts = mgr_timer.conflicts();
        ui.set_conflict_banner(if conflicts.is_empty() {
            SharedString::new()
        } else {
            format!("{} está abierto. Ciérralo: si no, cada control puede aparecer dos veces.", conflicts.join(", ")).into()
        });
        ui.set_status_text(format!("ViGEmBus: {}. Configuración: {}", if mgr_timer.vigem_installed() { "instalado" } else { "no encontrado" }, mgr_timer.settings_path()).into());
        let prefs = AppPrefs {
            close_to_tray: ui.get_close_to_tray(),
            start_minimized: ui.get_start_minimized(),
            run_at_startup: ui.get_run_at_startup(),
        };
        let prefs_changed = prefs != *last_prefs.borrow();
        if prefs_changed {
            let startup = prefs.run_at_startup;
            mgr_timer.set_prefs(prefs.clone());
            crate::sys::set_run_at_startup(startup);
            *last_prefs.borrow_mut() = prefs;
        }
    });

    let mgr_close = mgr.clone();
    ui.window().on_close_requested(move || {
        if mgr_close.prefs().close_to_tray {
            slint::CloseRequestResponse::HideWindow
        } else {
            mgr_close.shutdown();
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        }
    });

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        loop {
            instance.wait_for_show();
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    let _ = ui.show();
                }
            });
        }
    });

    if !ui.get_start_minimized() {
        ui.show().expect("mostrar");
    }
    slint::run_event_loop().expect("bucle");
    drop(timer);
    drop(tray);
}

fn connection(value: &str) -> &'static str {
    match value {
        "usb" => "USB",
        "bluetooth" => "Bluetooth",
        "wireless" => "Inalámbrico",
        _ => "Conexión",
    }
}

fn battery_label(level: Option<u8>, charging: &crate::ds::protocol::Charging) -> String {
    let Some(level) = level else { return "Batería —".into() };
    let bolt = if *charging == crate::ds::protocol::Charging::Charging { " · cargando" } else { "" };
    format!("{level}%{bolt}")
}
