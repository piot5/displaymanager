use eframe::egui;
use crate::loader::AppState;
use crate::loader::FlowPackage;

pub fn render_wallpaper_engine(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Live Wallpaper Management");

    if ui.button("📁 Browse Packages").clicked() {
        if let Some(path) = rfd::FileDialog::new().add_filter("Shader Package", &["zip"]).pick_file() {
            state.wallpaper_path = path.to_string_lossy().to_string();
        }
    }

    if ui.button("▶ Start Live Desktop").clicked() {
        let _ = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("wallpaper")
            .arg(&state.wallpaper_path)
            .spawn();
    }

    if let Some(pkg) = FlowPackage::load(&state.wallpaper_path) {
        ui.colored_label(egui::Color32::GREEN, format!("Package loaded: {} stages found", pkg.config.sequence.len()));
    }
}