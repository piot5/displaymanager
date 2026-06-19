use df_displmgr_info::edid_types::{DigitalInterfaceType, VideoInterfaceInfo};
use df_displmgr_info::{collect_monitor_data, MonitorDetails};
use std::fs::File;
use std::io::Write;
use std::path::Path;

struct FileBuffer {
    content: String,
}

impl FileBuffer {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn write_line(&mut self, text: &str) {
        self.content.push_str(text);
        self.content.push('\n');
    }

    fn save_to_file(&self, filename: &str) {
        let path = Path::new(filename);
        match File::create(path) {
            Ok(mut file) => match file.write_all(self.content.as_bytes()) {
                Ok(_) => println!("Wrote diagnostic output to '{}'.", filename),
                Err(e) => eprintln!("Write failed: {:?}", e),
            },
            Err(e) => eprintln!("Could not create '{}': {:?}", filename, e),
        }
    }
}

mod formatters {
    pub fn format_output_tech(tech_str: &str) -> String {
        let clean = tech_str.trim();
        if clean == "5"
            || clean.contains("(5)")
            || clean.to_uppercase().contains("DISPLAYPORT_EXTERNAL")
        {
            "DisplayPort (External)".to_string()
        } else if clean == "4" || clean.contains("(4)") || clean.to_uppercase().contains("HDMI") {
            "HDMI".to_string()
        } else if clean == "12"
            || clean.contains("(12)")
            || clean.to_uppercase().contains("DISPLAYPORT")
        {
            "DisplayPort (Embedded)".to_string()
        } else if clean == "0" || clean.contains("(0)") || clean.to_uppercase().contains("DVI") {
            "DVI".to_string()
        } else if clean == "1" || clean.contains("(1)") || clean.to_uppercase().contains("HD15") {
            "VGA (Analog)".to_string()
        } else {
            tech_str
                .replace("DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY", "")
                .trim_matches(|c| c == '(' || c == ')')
                .to_string()
        }
    }

    pub fn format_rotation(rot_str: &str) -> &'static str {
        if rot_str.contains("(1)") || rot_str.contains("IDENTITY") {
            "0 (Landscape)"
        } else if rot_str.contains("(2)") || rot_str.contains("90") {
            "90 (Portrait)"
        } else if rot_str.contains("(3)") || rot_str.contains("180") {
            "180 (Inverted Landscape)"
        } else if rot_str.contains("(4)") || rot_str.contains("270") {
            "270 (Inverted Portrait)"
        } else {
            "0"
        }
    }

    pub fn clean_display_string(input: &str) -> String {
        input
            .replace('\0', "")
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }
}

fn process_monitor_to_buffer(idx: usize, monitor: &MonitorDetails, buffer: &mut FileBuffer) {
    let friendly_name = formatters::clean_display_string(&monitor.friendly_name);
    let printable_name = if friendly_name.is_empty() {
        "Generic Display"
    } else {
        &friendly_name
    };

    buffer.write_line("======================================================");
    buffer.write_line(&format!(" MONITOR #{idx} - {printable_name}"));
    buffer.write_line("======================================================");

    buffer.write_line("  [OS LAYER]");
    buffer.write_line(&format!("  |-- Target ID:        {}", monitor.target_id));
    buffer.write_line(&format!("  |-- GDI Name:         {}", monitor.gdi_name));
    buffer.write_line(&format!(
        "  |-- Active:           {}",
        if monitor.is_active { "Yes" } else { "No" }
    ));
    buffer.write_line(&format!(
        "  |-- Output Tech:      {}",
        formatters::format_output_tech(&monitor.output_tech)
    ));
    buffer.write_line(&format!("  `-- Device Path:      {}", monitor.device_path));

    if let Some(topo) = &monitor.topology {
        buffer.write_line("");
        buffer.write_line("  [TOPOLOGY]");
        buffer.write_line(&format!(
            "  |-- Position:         X: {}, Y: {}",
            topo.x, topo.y
        ));
        buffer.write_line(&format!(
            "  |-- Dimensions:       {}x{}",
            topo.width, topo.height
        ));
        buffer.write_line(&format!(
            "  `-- Rotation:         {}",
            formatters::format_rotation(&topo.rotation)
        ));
    }

    if let Some(data) = &monitor.edid {
        let clean_model = formatters::clean_display_string(&data.model_name);
        buffer.write_line("");
        buffer.write_line("  [HARDWARE IDENTITY (EDID)]");
        buffer.write_line(&format!(
            "  |-- Manufacturer:     {} (ID: {})",
            clean_model, data.manufacturer_id
        ));
        buffer.write_line(&format!("  |-- Product Code:     {}", data.product_code));
        buffer.write_line(&format!(
            "  |-- Serial (Binary):  {}",
            data.serial_number_binary
        ));
        if let Some(serial_ascii) = &data.serial_number_ascii {
            buffer.write_line(&format!(
                "  |-- Serial (ASCII):   {}",
                formatters::clean_display_string(serial_ascii)
            ));
        }
        buffer.write_line(&format!(
            "  |-- Manufacture Date: Week {}, Year {}",
            data.week_of_manufacture, data.year_of_manufacture
        ));

        match &data.video_interface {
            VideoInterfaceInfo::Digital {
                bit_depth,
                interface_type,
            } => {
                let type_str = match interface_type {
                    DigitalInterfaceType::Hdmi => "HDMI",
                    DigitalInterfaceType::DisplayPort => "DisplayPort",
                    DigitalInterfaceType::Dvi => "DVI",
                    DigitalInterfaceType::Unknown => "Unknown Digital",
                };
                buffer.write_line(&format!(
                    "  |-- Interface:        Digital ({type_str}, {bit_depth}-bit)"
                ));
            }
            VideoInterfaceInfo::Analog {
                signal_level_v,
                setup_expected,
            } => {
                buffer.write_line(&format!(
                    "  |-- Interface:        Analog ({}V, Setup Expected: {})",
                    signal_level_v, setup_expected
                ));
            }
            VideoInterfaceInfo::Unknown => {
                buffer.write_line("  |-- Interface:        Unknown");
            }
        }

        if let Some(chroma) = &data.chromaticity {
            buffer.write_line("  |-- Color Gamut:");
            buffer.write_line(&format!(
                "  |   |-- Red:           X={:.4}, Y={:.4}",
                chroma.red_x, chroma.red_y
            ));
            buffer.write_line(&format!(
                "  |   |-- Green:         X={:.4}, Y={:.4}",
                chroma.green_x, chroma.green_y
            ));
            buffer.write_line(&format!(
                "  |   |-- Blue:          X={:.4}, Y={:.4}",
                chroma.blue_x, chroma.blue_y
            ));
            buffer.write_line(&format!(
                "  |   `-- White Point:   X={:.4}, Y={:.4}",
                chroma.white_x, chroma.white_y
            ));
        }

        if !data.audio_caps.short_audio_descriptors.is_empty() {
            buffer.write_line("  |-- Audio Capabilities:");
            for (a_idx, desc) in data.audio_caps.short_audio_descriptors.iter().enumerate() {
                buffer.write_line(&format!(
                    "  |   `-- Codec Descriptor #{}: {}",
                    a_idx + 1,
                    desc
                ));
            }
        }

        if !data.modes.is_empty() {
            buffer.write_line("  |-- Supported Display Modes:");
            for (m_idx, mode) in data.modes.iter().enumerate() {
                let interlaced_tag = if mode.interlaced { "i" } else { "" };
                buffer.write_line(&format!(
                    "  |   `-- Mode #{}: {}x{}{} @ {}Hz",
                    m_idx + 1,
                    mode.width,
                    mode.height,
                    interlaced_tag,
                    mode.refresh_rate
                ));
            }
        }

        // HDR capabilities — only printed when the monitor actually reports support
        let hdr = &data.hdr_caps;
        if hdr.supports_smpte_st2084 || hdr.supports_hlg || hdr.supports_hdr_traditional {
            buffer.write_line("  |-- HDR Capabilities:");
            if hdr.supports_smpte_st2084 {
                buffer.write_line("  |   |-- HDR10 (SMPTE ST 2084)");
            }
            if hdr.supports_hlg {
                buffer.write_line("  |   |-- Hybrid Log-Gamma (HLG)");
            }
            if hdr.supports_hdr_traditional {
                buffer.write_line("  |   |-- Traditional HDR");
            }
            if let Some(v) = hdr.max_luminance_cd_m2 {
                buffer.write_line(&format!("  |   |-- Max Luminance:  {:.0} cd/m^2", v));
            }
            if let Some(v) = hdr.max_frame_average_luminance_cd_m2 {
                buffer.write_line(&format!("  |   |-- Max Avg Lum:    {:.0} cd/m^2", v));
            }
            if let Some(v) = hdr.min_luminance_cd_m2 {
                buffer.write_line(&format!("  |   `-- Min Luminance:  {:.4} cd/m^2", v));
            }
        }

        buffer.write_line(&format!(
            "  `-- Extension Blocks: {}",
            data.extension_blocks
        ));
    }

    buffer.write_line("");
    buffer.write_line("  [DDC/CI HARDWARE BUS]");
    if let Some(ddc) = &monitor.ddc_stats {
        buffer.write_line(&format!(
            "  |-- Brightness:       Current: {} / Max: {}",
            ddc.core_caps.brightness, ddc.core_caps.brightness_max
        ));
        buffer.write_line(&format!(
            "  |-- Contrast:         Current: {} / Max: {}",
            ddc.core_caps.contrast, ddc.core_caps.contrast_max
        ));
        buffer.write_line(&format!("  |-- Input Connection: {:?}", ddc.input_source));
        buffer.write_line(&format!("  |-- Power State:      {:?}", ddc.power_state));
        buffer.write_line(&format!("  |-- Audio Mute State: {:?}", ddc.audio_mute));

        if let Some(v) = ddc.volume {
            buffer.write_line(&format!(
                "  |-- Audio Volume:     Current: {} / Max: {}",
                v.0, v.1
            ));
        }
        if let Some((r, g, b)) = ddc.color_gains {
            buffer.write_line(&format!("  |-- RGB Hardware Gain:R:{} G:{} B:{}", r, g, b));
        }
        if let Some(h_freq) = ddc.horizontal_freq_hz {
            // Register 0xAC: value is in units of 1 Hz, divide by 1000 for kHz.
            buffer.write_line(&format!(
                "  |-- Horizontal Freq:  {:.2} kHz",
                h_freq as f64 / 1000.0
            ));
        }
        // Register 0xAE reports in units of 0.01 Hz, divide by 100 to get Hz.
        if let Some(v_freq) = ddc.vertical_freq_centihz {
            buffer.write_line(&format!(
                "  |-- Vertical Freq:    {:.2} Hz",
                v_freq as f64 / 100.0
            ));
        }
        if let Some(hours) = ddc.operating_hours {
            buffer.write_line(&format!("  |-- Operating Time:   {} Hours", hours));
        }
        if let Some(lang) = ddc.osd_language_code {
            buffer.write_line(&format!("  |-- OSD Language Code:0x{:X}", lang));
        }
        if let Some(panel) = ddc.panel_type_code {
            buffer.write_line(&format!("  `-- Panel Type Code:  0x{:X}", panel));
        }
    } else {
        buffer
            .write_line("  `-- Status:           Not available (monitor inactive or DDC blocked)");
    }
}

fn main() {
    let mut buffer = FileBuffer::new();
    buffer.write_line("Hardware diagnostic dump");

    match collect_monitor_data() {
        Ok(monitors) => {
            buffer.write_line(&format!("Found {} display targets:\n", monitors.len()));
            for (idx, monitor) in monitors.iter().enumerate() {
                process_monitor_to_buffer(idx + 1, monitor, &mut buffer);
                buffer.write_line("");
            }
            buffer.save_to_file("edid_dump.txt");
        }
        Err(e) => {
            let err_msg = format!("Monitor query failed: {:?}", e);
            buffer.write_line(&err_msg);
            buffer.save_to_file("edid_dump.txt");
        }
    }
}
