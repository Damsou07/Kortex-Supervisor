#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use kortex_supervisor::app::KortexApp;
fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1500.0, 950.0])
        .with_min_inner_size([1000.0, 650.0]);
    if let Ok(image) = image::load_from_memory(include_bytes!("../../../logo-Kortex.png")) {
        let rgba = image.thumbnail(128, 128).to_rgba8();
        viewport = viewport.with_icon(egui::IconData {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
        });
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let open = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    eframe::run_native(
        "Kortex Supervisor",
        options,
        Box::new(move |cc| {
            let mut app = KortexApp::new(&cc.egui_ctx);
            if let Some(path) = open {
                app.open_sqlite_path(path);
            }
            Ok(Box::new(app))
        }),
    )
}
