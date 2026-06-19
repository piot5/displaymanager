use df_displmgr_info::edid_types::{DigitalInterfaceType, VideoInterfaceInfo};
use df_displmgr_info::{collect_monitor_data, MonitorDetails};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// JSON data structures

#[derive(Serialize)]
struct JsonOutput {
    stream_type: String,
    total_discovered_targets: usize,
    monitors: Vec<MonitorJsonSchema>,
}

#[derive(Serialize)]
struct MonitorJsonSchema {
    index: usize,
    friendly_name: String,
    os_layer: OsLayerSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    topology: Option<TopologySchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hardware_identity_edid: Option<HardwareIdentitySchema>,
    ddc_ci_hardware_bus: DdcCiSchema,
}

#[derive(Serialize)]
struct OsLayerSchema {
    target_id: u32,
    gdi_name: String,
    active: String,
    output_tech: String,
    device_path: String,
}

#[derive(Serialize)]
struct TopologySchema {
    position: String,
    dimensions: String,
    rotation: String,
}

#[derive(Serialize)]
struct ChromaticitySchema {
    red: String,
    green: String,
    blue: String,
    white_point: String,
}

#[derive(Serialize)]
struct HardwareIdentitySchema {
    manufacturer: String,
    product_code: u16,
    serial_binary: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    serial_ascii: Option<String>,
    manufacture_date: String,
    interface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    color_gamut: Option<ChromaticitySchema>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    audio_capabilities: Vec<String>,
    supported_display_modes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    hdr_capabilities: Vec<String>,
    extension_blocks: u8,
}

#[derive(Serialize)]
struct DdcCiSchema {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    brightness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contrast: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_mute_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rgb_hardware_gain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizontal_freq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_freq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operating_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    osd_language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    panel_type_code: Option<String>,
}

// Formatters (mirrored from edid_file_txt.rs)

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

fn transform_monitor_data(idx: usize, monitor: &MonitorDetails) -> MonitorJsonSchema {
    let friendly_name = formatters::clean_display_string(&monitor.friendly_name);
    let printable_name = if friendly_name.is_empty() {
        "Generic Display".to_string()
    } else {
        friendly_name
    };

    let os_layer = OsLayerSchema {
        target_id: monitor.target_id,
        gdi_name: monitor.gdi_name.clone(),
        active: (if monitor.is_active { "Yes" } else { "No" }).to_string(),
        output_tech: formatters::format_output_tech(&monitor.output_tech),
        device_path: monitor.device_path.clone(),
    };

    let topology = monitor.topology.as_ref().map(|topo| TopologySchema {
        position: format!("X: {}, Y: {}", topo.x, topo.y),
        dimensions: format!("{}x{}", topo.width, topo.height),
        rotation: formatters::format_rotation(&topo.rotation).to_string(),
    });

    let hardware_identity_edid = monitor.edid.as_ref().map(|data| {
        let clean_model = formatters::clean_display_string(&data.model_name);
        let manufacturer = format!("{} (ID: {})", clean_model, data.manufacturer_id);

        let interface = match &data.video_interface {
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
                format!("Digital ({type_str}, {bit_depth}-bit)")
            }
            VideoInterfaceInfo::Analog {
                signal_level_v,
                setup_expected,
            } => {
                format!(
                    "Analog ({}V, Setup Expected: {})",
                    signal_level_v, setup_expected
                )
            }
            VideoInterfaceInfo::Unknown => "Unknown".to_string(),
        };

        let color_gamut = data.chromaticity.as_ref().map(|chroma| ChromaticitySchema {
            red: format!("X={:.4}, Y={:.4}", chroma.red_x, chroma.red_y),
            green: format!("X={:.4}, Y={:.4}", chroma.green_x, chroma.green_y),
            blue: format!("X={:.4}, Y={:.4}", chroma.blue_x, chroma.blue_y),
            white_point: format!("X={:.4}, Y={:.4}", chroma.white_x, chroma.white_y),
        });

        let mut audio_capabilities = Vec::new();
        for (a_idx, desc) in data.audio_caps.short_audio_descriptors.iter().enumerate() {
            audio_capabilities.push(format!("Codec Descriptor #{}: {}", a_idx + 1, desc));
        }

        let mut supported_display_modes = Vec::new();
        for (m_idx, mode) in data.modes.iter().enumerate() {
            let interlaced_tag = if mode.interlaced { "i" } else { "" };
            supported_display_modes.push(format!(
                "Mode #{}: {}x{}{} @ {}Hz",
                m_idx + 1,
                mode.width,
                mode.height,
                interlaced_tag,
                mode.refresh_rate
            ));
        }

        // Fall back to the topology resolution when EDID timings are absent.
        if supported_display_modes.is_empty() {
            if let Some(topo) = &monitor.topology {
                supported_display_modes
                    .push(format!("Mode #1: {}x{} @ 60Hz", topo.width, topo.height));
            }
        }

        let mut hdr_capabilities = Vec::new();
        let hdr = &data.hdr_caps;
        if hdr.supports_smpte_st2084 || hdr.supports_hlg || hdr.supports_hdr_traditional {
            if hdr.supports_smpte_st2084 {
                hdr_capabilities.push("HDR10 (SMPTE ST 2084)".to_string());
            }
            if hdr.supports_hlg {
                hdr_capabilities.push("Hybrid Log-Gamma (HLG)".to_string());
            }
            if hdr.supports_hdr_traditional {
                hdr_capabilities.push("Traditional HDR".to_string());
            }
            if let Some(v) = hdr.max_luminance_cd_m2 {
                hdr_capabilities.push(format!("Max Luminance:  {:.0} cd/m^2", v));
            }
            if let Some(v) = hdr.max_frame_average_luminance_cd_m2 {
                hdr_capabilities.push(format!("Max Avg Lum:    {:.0} cd/m^2", v));
            }
            if let Some(v) = hdr.min_luminance_cd_m2 {
                hdr_capabilities.push(format!("Min Luminance:  {:.4} cd/m^2", v));
            }
        }

        HardwareIdentitySchema {
            manufacturer,
            product_code: data.product_code,
            serial_binary: data.serial_number_binary,
            serial_ascii: data
                .serial_number_ascii
                .as_ref()
                .map(|s| formatters::clean_display_string(s)),
            manufacture_date: format!(
                "Week {}, Year {}",
                data.week_of_manufacture, data.year_of_manufacture
            ),
            interface,
            color_gamut,
            audio_capabilities,
            supported_display_modes,
            hdr_capabilities,
            extension_blocks: data.extension_blocks,
        }
    });

    let ddc_ci_hardware_bus = if let Some(ddc) = &monitor.ddc_stats {
        let audio_volume = ddc
            .volume
            .as_ref()
            .map(|v| format!("Current: {} / Max: {}", v.0, v.1));
        let rgb_hardware_gain = ddc
            .color_gains
            .as_ref()
            .map(|(r, g, b)| format!("R:{} G:{} B:{}", r, g, b));

        // Frequency scaling matches edid_file_txt.rs (Hz -> kHz, centiHz -> Hz).
        let horizontal_freq = ddc
            .horizontal_freq_hz
            .map(|h_freq| format!("{:.2} kHz", h_freq as f64 / 1000.0));
        let vertical_freq = ddc
            .vertical_freq_centihz
            .map(|v_freq| format!("{:.2} Hz", v_freq as f64 / 100.0));

        let operating_time = ddc.operating_hours.map(|hours| format!("{} Hours", hours));
        let osd_language_code = ddc.osd_language_code.map(|lang| format!("0x{:X}", lang));
        let panel_type_code = ddc.panel_type_code.map(|panel| format!("0x{:X}", panel));

        DdcCiSchema {
            status: "Available".to_string(),
            brightness: Some(format!(
                "Current: {} / Max: {}",
                ddc.core_caps.brightness, ddc.core_caps.brightness_max
            )),
            contrast: Some(format!(
                "Current: {} / Max: {}",
                ddc.core_caps.contrast, ddc.core_caps.contrast_max
            )),
            input_connection: Some(format!("{:?}", ddc.input_source)),
            power_state: Some(format!("{:?}", ddc.power_state)),
            audio_mute_state: Some(format!("{:?}", ddc.audio_mute)),
            audio_volume,
            rgb_hardware_gain,
            horizontal_freq,
            vertical_freq,
            operating_time,
            osd_language_code,
            panel_type_code,
        }
    } else {
        DdcCiSchema {
            status: "Not available (monitor inactive or DDC blocked)".to_string(),
            brightness: None,
            contrast: None,
            input_connection: None,
            power_state: None,
            audio_mute_state: None,
            audio_volume: None,
            rgb_hardware_gain: None,
            horizontal_freq: None,
            vertical_freq: None,
            operating_time: None,
            osd_language_code: None,
            panel_type_code: None,
        }
    };

    MonitorJsonSchema {
        index: idx,
        friendly_name: printable_name,
        os_layer,
        topology,
        hardware_identity_edid,
        ddc_ci_hardware_bus,
    }
}

struct JsonDumper;

impl JsonDumper {
    pub fn dump_to_json_file(filename: &str) {
        println!("Scanning monitors...");

        match collect_monitor_data() {
            Ok(monitors) => {
                println!("Found {} display targets.", monitors.len());
                println!("Serializing to JSON...");

                let mut monitor_schemas = Vec::new();
                for (idx, monitor) in monitors.iter().enumerate() {
                    monitor_schemas.push(transform_monitor_data(idx + 1, monitor));
                }

                let output = JsonOutput {
                    stream_type: "monitor-dump".to_string(),
                    total_discovered_targets: monitors.len(),
                    monitors: monitor_schemas,
                };

                match serde_json::to_string_pretty(&output) {
                    Ok(json_string) => {
                        let path = Path::new(filename);
                        match File::create(path) {
                            Ok(mut file) => match file.write_all(json_string.as_bytes()) {
                                Ok(_) => println!("Wrote JSON to '{}'.", filename),
                                Err(e) => eprintln!("Write failed: {:?}", e),
                            },
                            Err(e) => eprintln!("Could not create '{}': {:?}", filename, e),
                        }
                    }
                    Err(e) => eprintln!("Serialization failed: {:?}", e),
                }
            }
            Err(e) => {
                eprintln!("Monitor query failed: {:?}", e);

                let path = Path::new(filename);
                if let Ok(mut file) = File::create(path) {
                    let error_text = format!(
                        "{{\n  \"status\": \"error\",\n  \"message\": \"Monitor query failed\",\n  \"error_context\": \"{:?}\"\n}}",
                        e
                    );
                    let _ = file.write_all(error_text.as_bytes());
                }
            }
        }
    }
}

fn main() {
    JsonDumper::dump_to_json_file("edid_dump.json");
}
