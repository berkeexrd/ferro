#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod backend;
mod chrome;
mod overlay;
mod shell;
mod diff;
mod graph;
mod highlight;
mod i18n;
mod icons;
mod screens;
mod theme;
mod ui;
mod util;

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Ferro")
            .with_icon(eframe::egui::IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 })
            .with_decorations(false)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native("Ferro", opts, Box::new(|cc| Ok(Box::new(shell::AppShell::new(cc)))))
}
