#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Allow some dead code during active development to preserve logical structure
// and avoid spurious warnings while we incrementally refactor features.
#![allow(dead_code)]

use eframe::egui;
use tokio::runtime::Runtime;
use std::sync::Arc;
use std::time::Instant;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Foundation::HINSTANCE;

mod loader;
mod scan;
mod set;
mod monitor_manager;
mod window_manager;
mod live_wallpaper;
mod anim_editor;

use loader::{AppState, FlowPackage};
use monitor_manager::render_monitor_ctrl;
use live_wallpaper::render_wallpaper_engine;
use window_manager::{render_window_manager, WindowManager};
use anim_editor::{GpuCore, init_windows, render_anim_editor};

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Monitor,
    Windows,
    Wallpaper,
    Editor,
    SystemLog,
}

struct DisplayFlowApp {
    state: AppState,
    current_tab: Tab,
    rt: Arc<Runtime>,
    window_engine: Arc<WindowManager>,
    last_hardware_scan: Instant,
    system_messages: Vec<String>,
}

impl DisplayFlowApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let rt = Arc::new(Runtime::new().expect("Tokio Runtime Error"));
        let wm = Arc::new(WindowManager::new());
        
        let wm_logic = wm.clone();
        std::thread::spawn(move || {
            wm_logic.setup_hooks();
            wm_logic.run_loop();
        });

        Self {
            state: AppState::default(),
            current_tab: Tab::Monitor,
            rt,
            window_engine: wm,
            last_hardware_scan: Instant::now(),
            system_messages: vec!["DisplayFlow Studio initialized...".to_string()],
        }
    }
}

impl eframe::App for DisplayFlowApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::Monitor, "🖥 Hardware");
                ui.selectable_value(&mut self.current_tab, Tab::Windows, "🔲 Windows");
                ui.selectable_value(&mut self.current_tab, Tab::Wallpaper, "🖼 Wallpaper");
                ui.selectable_value(&mut self.current_tab, Tab::Editor, "📝 Anim Editor");
                ui.selectable_value(&mut self.current_tab, Tab::SystemLog, "📋 Log");
            });

            ui.separator();

            match self.current_tab {
                Tab::Monitor => render_monitor_ctrl(ui, &mut self.state),
                Tab::Windows => render_window_manager(ui, &mut self.state),
                Tab::Wallpaper => render_wallpaper_engine(ui, &mut self.state),
                Tab::Editor => render_anim_editor(ui, &mut self.state),
                Tab::SystemLog => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for msg in &self.system_messages {
                            ui.label(msg);
                        }
                    });
                }
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 2 && args[1] == "wallpaper" {
        let path = args[2].clone();
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            if let Some(pkg) = FlowPackage::load(&path) {
                let instance = wgpu::Instance::default();
                if let Ok(gpu) = GpuCore::new(&instance, &pkg.shader_src, "fs_default").await {
                    unsafe {
                        let h_instance: HINSTANCE = GetModuleHandleW(None).unwrap_or_default().into();
                        let _windows = init_windows(
                            &gpu,
                            &instance,
                            windows::core::w!("DisplayFlowWallpaper"),
                            h_instance,
                            true,
                            &pkg
                        );
                        
                        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
                        while windows::Win32::UI::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0).as_bool() {
                            windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                            windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                        }
                    }
                }
            }
        });
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_title("DisplayFlow Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "DisplayFlow Studio",
        options,
        Box::new(|cc| {
            let app = DisplayFlowApp::new(cc);
            // FIX E0308: Return the Box directly, not a Result/Ok
            Box::new(app) as Box<dyn eframe::App>
        }),
    )
}