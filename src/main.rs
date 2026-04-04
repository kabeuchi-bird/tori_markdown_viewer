#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod settings;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Optional: open a file passed on the command line
    //   tori_markdown_viewer [path/to/file.md]
    let cli_file: Option<PathBuf> = std::env::args().nth(1).map(PathBuf::from);

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
        Box::new(move |cc| {
            let mut app = app::App::new(cc);
            // CLI argument overrides the last-opened file from persisted settings
            if let Some(path) = cli_file {
                if path.exists() {
                    app.open_file(path, &cc.egui_ctx);
                }
            }
            Ok(Box::new(app))
        }),
    )
}
