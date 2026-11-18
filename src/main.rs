//! HeartBridge: reads a GEOID HS500 strap (or any standard BLE heart-rate
//! sensor) and publishes the pulse: to VRChat over OSC, to OBS and to any app.
//!
//! The window (eframe/egui) and the tray icon run on the main thread;
//! Bluetooth and OSC run on a background tokio runtime. The window only
//! redraws when data arrives, and draws nothing while hidden in the tray.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ble;
mod channel;
mod config;
mod hr;
mod icon;
mod osc;
mod state;
mod theme;
mod tray;

use config::Config;
use state::Shared;

fn main() -> eframe::Result<()> {
    let config = Config::load();
    let compact = config.compact;

    let on_top = config.always_on_top;

    let window_icon = egui::IconData {
        rgba: icon::heart_rgba(64, [0xFF, 0x3B, 0x47]),
        width: 64,
        height: 64,
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("HeartBridge")
        .with_icon(window_icon)
        .with_decorations(false)
        .with_resizable(false)
        .with_inner_size(if compact {
            app::COMPACT_SIZE
        } else {
            app::FULL_SIZE
        });
    if on_top {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "HeartBridge",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            let shared = Shared::new(config, cc.egui_ctx.clone());
            spawn_background(shared.clone());
            let tray = tray::Tray::new(&cc.egui_ctx);
            Ok(Box::new(app::HeartBridgeApp::new(shared, tray)))
        }),
    )
}

fn spawn_background(shared: Shared) {
    std::thread::Builder::new()
        .name("background".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("tokio runtime");
            rt.block_on(async move {
                tokio::spawn(osc::run(shared.clone()));
                tokio::spawn(channel::run(shared.clone()));
                ble::run(shared).await;
            });
        })
        .expect("background thread");
}
