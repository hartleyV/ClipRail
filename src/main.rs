// 发布版不弹控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod archive;
mod clipboard;
mod hotkey;
mod icon;
mod model;
mod platform;
mod store;
mod ui;

use eframe::egui;

/// 崩溃时写入 data/crash.log，便于定位“闪退”原因（而不是默默退出）
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!(
            "[{}] {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            info
        );
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(store::base_dir().join("crash.log"))
        {
            let _ = f.write_all(msg.as_bytes());
        }
        default_hook(info);
    }));
}

fn main() -> eframe::Result<()> {
    store::ensure_dirs();
    install_panic_hook();

    // `ClipRail --toggle`：通知已运行实例显示 / 隐藏，然后立即退出。
    // 主要用于 Wayland 下将其绑定到系统快捷键。
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--toggle") {
        let _ = std::fs::write(store::toggle_file(), b"1");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("ClipRail - 剪贴板历史侧栏");
        println!("  ClipRail            启动程序");
        println!("  ClipRail --toggle   显示 / 隐藏已运行的竖栏");
        return Ok(());
    }

    let settings = store::load_settings();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (command_tx, command_rx) = crossbeam_channel::unbounded();
    clipboard::spawn(event_tx, command_rx);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("ClipRail")
        .with_app_id("cliprail")
        .with_icon(icon::clipboard_icon())
        .with_inner_size([settings.clamped_width(), settings.height])
        .with_min_inner_size([300.0, 240.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_transparent(false)
        // Windows：作为普通窗口显示在任务栏上
        .with_taskbar(true);

    if settings.x >= 0.0 {
        viewport = viewport.with_position([settings.x, settings.y]);
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: false,
        ..Default::default()
    };

    eframe::run_native(
        "ClipRail",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::App::new(cc, settings, command_tx, event_rx)))
        }),
    )
}
