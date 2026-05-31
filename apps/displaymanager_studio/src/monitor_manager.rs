use eframe::egui;
use egui::{Color32, Pos2, Rect, Vec2, Stroke, FontId, Align2};
use crate::loader::AppState;
use crate::scan::collect_monitor_data;
use crate::set::{apply_all_settings, DemoArgs, OutputArgs};

pub fn render_monitor_ctrl(ui: &mut egui::Ui, state: &mut AppState) {
    ui.heading("Hardware Discovery & Physical Layout");

    if ui.button("🔄 Scan system hardware").clicked() {
        if let Ok(data) = collect_monitor_data() {
            state.monitors = data;
        }
    }

    ui.separator();

    if state.monitors.is_empty() {
        ui.label("No hardware data available.");
        return;
    }

    // --- PHYSICS CANVAS ---
    let (response, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), 350.0),
        egui::Sense::hover(),
    );

    let canvas_rect = response.rect;
    let center = canvas_rect.center();
    
    // Zeichenbereich-Hintergrund (Dark Space)
    painter.rect_filled(canvas_rect, 8.0, Color32::from_gray(15));

    // Scaling factor for canvas: pixels per logical unit
    let scale = 0.1; // convert pixel resolution into smaller canvas units
    let mut monitor_bounds: Vec<Rect> = Vec::new();

    for (idx, m) in state.monitors.iter().enumerate() {
        // 1. Geometry definition (use reported pixel resolution as fallback)
        let size = Vec2::new(m.width as f32 * scale, m.height as f32 * scale);

        // 2. Snap & Glue Logic
        let rect = if idx == 0 {
            // PRIMARY: centered and fixed
            Rect::from_center_size(center, size)
        } else {
            // SECONDARY: glued to the right edge of the previous monitor
            let prev = monitor_bounds[idx - 1];
            let glue_pos = Pos2::new(prev.right() + 10.0, prev.center().y);
            Rect::from_center_size(glue_pos, size)
        };
        
        monitor_bounds.push(rect);

        // 3. Interaktion & Visualisierung
        let is_selected = state.selected_monitor_index == Some(idx);
        let id = egui::Id::new("mon_phys").with(idx);
        let interact = ui.interact(rect, id, egui::Sense::click());

        if interact.clicked() {
            state.selected_monitor_index = Some(idx);
            if let Some(ddc) = &m.ddc_stats {
                state.brightness = ddc.brightness.1;
            }
        }

        // Zeichnen der Hardware-Box
        let base_color = if m.is_active { Color32::from_gray(50) } else { Color32::from_gray(25) };
        let stroke = if is_selected {
            Stroke::new(2.5, Color32::LIGHT_BLUE)
        } else {
            Stroke::new(1.0, Color32::from_gray(100))
        };

        painter.rect_filled(rect, 4.0, base_color);
        painter.rect_stroke(rect, 4.0, stroke);

        // Labeling
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("#{}\n{}", idx + 1, m.friendly_name),
            FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    ui.separator();

    // --- CONTROL PANEL ---
    if let Some(idx) = state.selected_monitor_index {
        if let Some(m) = state.monitors.get(idx) {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(format!("Monitor: {}", m.friendly_name));
                        ui.label(format!("ID: {}", m.target_id));
                        
                        if m.ddc_stats.is_some() {
                            ui.add(egui::Slider::new(&mut state.brightness, 0..=100).text("Brightness"));
                            
                            if ui.button("🚀 Apply Physics to Hardware").clicked() {
                                let tid = m.target_id;
                                let b = state.brightness;
                                tokio::spawn(async move {
                                    let args = DemoArgs {
                                        brightness: Some(b),
                                        output_config: OutputArgs { output: Some(tid.to_string()), ..Default::default() },
                                        ..Default::default()
                                    };
                                    let _ = apply_all_settings(tid, &args).await;
                                });
                            }
                            } else {
                            ui.colored_label(Color32::KHAKI, "DDC bus not reachable.");
                        }
                    });
                });
            });
        }
    }
}