use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::{Read, Cursor}};
use zip::ZipArchive;
use crate::scan::MonitorDetails;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Step {
    pub name: String,
    pub duration_ms: u64,
    pub shader_entry: String,
    pub sound: Option<String>,
    pub texture: Option<String>,
    pub easing: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub logic: HashMap<String, f32>,
    #[serde(default)]
    pub features: HashMap<String, bool>,
    #[serde(default)]
    pub sequence: Vec<Step>,
    #[serde(default)]
    pub z_order: String,
    #[serde(default)]
    pub screenshot_capture: bool,
}

#[allow(dead_code)]
pub struct FlowPackage {
    pub config: Config,
    pub sounds: HashMap<String, Vec<u8>>,
    pub textures: HashMap<String, (u32, u32, Vec<u8>)>,
    pub shader_src: String,
}

impl FlowPackage {
    pub fn load(path: &str) -> Option<Self> {
        let file = std::fs::File::open(path).ok()?;
        let mut archive = ZipArchive::new(file).ok()?;
        
        let mut config_str = String::new();
        archive.by_name("config.toml").ok()?.read_to_string(&mut config_str).ok()?;
        let config: Config = toml::from_str(&config_str).unwrap_or_default();
        
        let mut shader_src = String::new();
        archive.by_name("shader.wgsl").ok()?.read_to_string(&mut shader_src).ok()?;
        
        let mut sounds = HashMap::new();
        let mut textures = HashMap::new();

        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.ends_with(".wav") {
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).ok();
                    sounds.insert(name, buffer);
                } else if name.ends_with(".png") || name.ends_with(".jpg") {
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).ok();
                    if let Ok(img) = image::load_from_memory(&buffer) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        textures.insert(name, (w, h, rgba.into_raw()));
                    }
                }
            }
        }
        
        Some(Self { config, sounds, textures, shader_src })
    }

    #[allow(dead_code)]
    pub fn val(&self, key: &str, default: f32) -> f32 {
        *self.config.logic.get(key).unwrap_or(&default)
    }

    #[allow(dead_code)]
    pub fn play_sound(&self, sink: &rodio::Sink, name: &str) {
        if let Some(data) = self.sounds.get(name) {
            let cursor = Cursor::new(data.clone());
            if let Ok(source) = rodio::Decoder::new(cursor) {
                sink.append(source);
            }
        }
    }
}

pub struct AppState {
    pub monitors: Vec<MonitorDetails>,           // Speichert alle gefundenen Hardware-Infos
    pub selected_monitor_index: Option<usize>,
    pub brightness: u32,
    pub contrast: u32,
    pub wallpaper_path: String,
    pub editor_content: String,
    pub compile_status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            monitors: Vec::new(),
            selected_monitor_index: None,
            brightness: 50,
            contrast: 50,
            wallpaper_path: String::new(),
            editor_content: "{\n  \"logic\": {},\n  \"sequence\": []\n}".to_string(),
            compile_status: String::new(),
        }
    }
}
