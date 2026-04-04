#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod settings;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tori markdown viewer")
            .with_inner_size([1024.0, 768.0])
            .with_drag_and_drop(true),
        persist_window: true,
        follow_system_theme: true,
        ..Default::default()
    };

    eframe::run_native(
        "tori markdown viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
