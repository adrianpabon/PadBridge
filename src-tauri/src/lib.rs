mod device;
mod ds;
mod manager;
mod mapping;
mod settings;
mod sys;
mod ui;
mod virt;
mod xinput;

use std::path::PathBuf;

use manager::Manager;
use settings::Store;

fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("PadBridge");
    let path = dir.join("settings.json");
    if !path.exists() {
        let old = base.join("com.adrianpabon.padbridge").join("settings.json");
        if old.exists() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::copy(&old, &path);
        }
    }
    path
}

pub fn run() {
    if std::env::var_os("SLINT_BACKEND").is_none() {
        // Direct3D through wgpu. FemtoVG stays the light renderer; Skia is not linked.
        std::env::set_var("SLINT_BACKEND", "winit-femtovg-wgpu");
    }
    let Some(instance) = sys::Instance::claim() else {
        return;
    };
    let mgr = Manager::new(Store::load(config_path()));
    sys::set_run_at_startup(mgr.prefs().run_at_startup);
    mgr.start();
    ui::launch(mgr.clone(), instance);
    mgr.shutdown();
}
