#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
#[cfg(target_os = "linux")]
mod file_dialog;
mod startup;
use eframe::egui;
use std::sync::{Arc, atomic::AtomicBool};

fn main() {
    let diagnostics = startup::StartupDiagnostics::begin();
    let ready = Arc::new(AtomicBool::new(false));
    diagnostics.install_panic_hook(&ready);
    diagnostics.spawn_watchdog(ready.clone());
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .ok()
        .map(Arc::new);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([960.0, 720.0])
        .with_min_inner_size([700.0, 560.0])
        .with_clamp_size_to_monitor_size(true);
    #[cfg(target_os = "windows")]
    {
        // 显式开启 winit 的 Windows 文件拖放接收。
        viewport = viewport.with_drag_and_drop(true);
    }
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let native = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    diagnostics.write_line("开始创建 eframe 窗口和 OpenGL 上下文");
    let log_path = diagnostics.log_path().map(|p| p.to_path_buf());
    if let Err(e) = eframe::run_native(
        "传 · 局域网文件传输",
        native,
        Box::new(move |cc| Ok(Box::new(app::ChuanApp::new(cc, ready, log_path)))),
    ) {
        diagnostics.report_failure(&e.to_string());
        std::process::exit(1);
    }
}
