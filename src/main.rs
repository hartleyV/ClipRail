#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod clipboard;
mod hotkey;
mod model;
mod store;

use app::ClipRailApp;

fn main() -> eframe::Result<()> {
    let toggle_only = std::env::args().any(|a| a == "--toggle");
    if toggle_only {
        // Desktop environments may bind this command. A running-instance IPC hook can
        // replace this fallback without changing the public CLI.
        eprintln!("ClipRail --toggle: 请绑定主程序配置的全局快捷键");
        return Ok(());
    }
    let settings = store::Store::portable().load_settings();
    let viewport = egui::ViewportBuilder::default()
        .with_title("ClipRail")
        .with_inner_size([settings.width.clamp(300.0, 800.0), 760.0])
        .with_min_inner_size([300.0, 420.0])
        .with_decorations(false)
        .with_always_on_top();
    eframe::run_native("ClipRail", eframe::NativeOptions { viewport, ..Default::default() }, Box::new(|cc| Ok(Box::new(ClipRailApp::new(cc)))))
}
