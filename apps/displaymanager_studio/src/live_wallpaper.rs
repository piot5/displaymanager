use eframe::egui;
use crate::loader::AppState;
use crate::loader::FlowPackage;

pub fn render_wallpaper_engine(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Live Wallpaper Engine");
    ui.separator();

    // Package selection
    ui.horizontal(|ui| {
        if ui.button("📁 Browse Package").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Shader Package", &["zip"])
                .pick_file()
            {
                state.wallpaper_path = path.to_string_lossy().to_string();
            }
        }

        if !state.wallpaper_path.is_empty() {
            ui.label(&state.wallpaper_path);
        }
    });

    ui.separator();

    // Package info
    match FlowPackage::load(&state.wallpaper_path) {
        Some(pkg) => {
            ui.group(|ui| {
                ui.strong("Package Information");
                ui.label(format!("Stages: {}", pkg.config.sequence.len()));
                ui.label(format!("Shader: {} bytes", pkg.shader_src.len()));

                if !pkg.config.sequence.is_empty() {
                    ui.collapsing("Sequence Details", |ui| {
                        for (i, stage) in pkg.config.sequence.iter().enumerate() {
                            ui.label(format!("Stage {}: {:?}", i + 1, stage));
                        }
                    });
                }
            });

            ui.separator();

            // Controls
            ui.horizontal(|ui| {
                if ui.button("▶ Start Live Desktop").clicked() {
                    let _ = std::process::Command::new(std::env::current_exe().unwrap())
                        .arg("wallpaper")
                        .arg(&state.wallpaper_path)
                        .spawn();
                }

                if ui.button("⏹ Stop All").clicked() {
                    // Kill all wallpaper processes
                    let _ = std::process::Command::new("taskkill")
                        .args(["/F", "/IM", "displaymanager_studio.exe", "/FI", "WINDOWTITLE eq DisplayFlowWallpaper*"])
                        .output();
                }
            });

            ui.label("Tip: Start a .zip shader package to render as live desktop background.");
        }
        None => {
            if !state.wallpaper_path.is_empty() {
                ui.colored_label(egui::Color32::RED, "Failed to load package. Select a valid .zip file.");
            } else {
                ui.label("No package selected. Click 'Browse Package' to load a shader package (.zip).");
            }
        }
    }
}
