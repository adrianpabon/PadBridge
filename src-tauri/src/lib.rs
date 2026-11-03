mod device;
mod ds;
mod manager;
mod mapping;
mod settings;
mod sys;
mod xinput;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager as _, RunEvent, State, WindowEvent};

use device::Cmd;
use manager::{Manager, Overview};
use settings::{AppPrefs, Profile, Store};

type Mgr<'a> = State<'a, Arc<Manager>>;

#[tauri::command]
fn get_overview(mgr: Mgr) -> Overview {
    mgr.overview()
}

#[tauri::command]
fn get_profile(mgr: Mgr, id: String) -> Profile {
    mgr.profile(&id)
}

#[tauri::command]
fn set_profile(mgr: Mgr, id: String, profile: Profile) {
    mgr.set_profile(&id, profile);
}

#[tauri::command]
fn reset_profile(mgr: Mgr, id: String) -> Profile {
    let profile = Profile::default();
    mgr.set_profile(&id, profile.clone());
    profile
}

#[tauri::command]
fn apply_profile_to_all(mgr: Mgr, id: String) {
    mgr.apply_to_all(&id);
}

#[tauri::command]
fn watch_input(mgr: Mgr, id: Option<String>) {
    mgr.watch(id);
}

#[tauri::command]
fn test_rumble(mgr: Mgr, id: String, large: u8, small: u8) {
    mgr.send(&id, Cmd::TestRumble { large, small, ms: 700 });
}

#[tauri::command]
fn identify(mgr: Mgr, id: String) {
    mgr.send(&id, Cmd::Identify);
}

#[tauri::command]
fn set_prefs(mgr: Mgr, prefs: AppPrefs) {
    mgr.set_prefs(prefs);
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // A second PadBridge would fight the first over the controllers: it shows the first one's window instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .setup(|app| {
            let dir = app.path().app_config_dir().expect("config dir");
            let mgr = Manager::new(Store::load(dir.join("settings.json")));
            let start_minimized = mgr.prefs().start_minimized;
            mgr.start(app.handle().clone());
            app.manage(mgr);

            let show = MenuItem::with_id(app, "show", "Mostrar PadBridge", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().cloned().expect("app icon"))
                .tooltip("PadBridge")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if !start_minimized {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let mgr = window.state::<Arc<Manager>>();
                if mgr.prefs().close_to_tray {
                    // The controllers stay virtual only while PadBridge runs: hide instead of quitting.
                    api.prevent_close();
                    let _ = window.hide();
                    mgr.watch(None);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_overview,
            get_profile,
            set_profile,
            reset_profile,
            apply_profile_to_all,
            watch_input,
            test_rumble,
            identify,
            set_prefs
        ])
        .build(tauri::generate_context!())
        .expect("error while building PadBridge");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(mgr) = app.try_state::<Arc<Manager>>() {
                mgr.shutdown();
            }
        }
    });
}
