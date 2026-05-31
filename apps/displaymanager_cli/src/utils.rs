use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Buffer for accumulating diagnostic text before writing to disk.
pub struct FileBuffer {
    pub content: String,
}

impl FileBuffer {
    pub fn new() -> Self { Self { content: String::new() } }
    
    pub fn write_line(&mut self, text: &str) {
        self.content.push_str(text);
        self.content.push('\n');
    }
    
    pub fn save_to_file(&self, filename: &str) {
        if let Ok(mut file) = File::create(Path::new(filename)) {
            let _ = file.write_all(self.content.as_bytes());
        }
    }
}

pub mod formatters {
    /// Formats hardware technology strings into human-readable labels.
    pub fn format_output_tech(tech_str: &str) -> String {
        let clean = tech_str.trim();
        match clean {
            "5" | _ if clean.contains("DISPLAYPORT_EXTERNAL") => "DisplayPort (External)".to_string(),
            "4" | _ if clean.contains("HDMI") => "HDMI".to_string(),
            _ => tech_str.replace("DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY", "").trim_matches(|c| c == '(' || c == ')').to_string(),
        }
    }
    
    /// Removes null characters and normalizes whitespace in monitor names.
    pub fn clean_display_string(input: &str) -> String {
        input.replace('\0', "").split_whitespace().collect::<Vec<&str>>().join(" ")
    }
}