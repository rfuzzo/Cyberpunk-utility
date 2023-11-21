#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

const CARGO_NAME: &str = env!("CARGO_PKG_NAME");

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let _ = simple_logging::log_to_file(format!("{}.log", CARGO_NAME), log::LevelFilter::Info);

    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        ..Default::default()
    };
    eframe::run_native(
        "Red4 Conflict Checker",
        native_options,
        Box::new(|cc| Box::new(red4_conflicts::TemplateApp::new(cc))),
    )
}
