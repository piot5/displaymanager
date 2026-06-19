use df_displmgr_info::edid_types::{DigitalInterfaceType, VideoInterfaceInfo};
use df_displmgr_info::{collect_monitor_data, MonitorDetails};

/// Maps native Windows video output technology enums to clean descriptive strings,
/// handling both raw numeric integers and formatted variations safely.
fn format_output_tech(tech_str: &str) -> String {
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

/// Maps native Windows display config rotation fields into clean degree formatting.
fn format_rotation(rot_str: &str) -> &str {
    if rot_str.contains("(1)") || rot_str.contains("IDENTITY") {
        "0° (Landscape)"
    } else if rot_str.contains("(2)") || rot_str.contains("90") {
        "90° (Portrait)"
    } else if rot_str.contains("(3)") || rot_str.contains("180") {
        "180° (Inverted Landscape)"
    } else if rot_str.contains("(4)") || rot_str.contains("270") {
        "270° (Inverted Portrait)"
    } else {
        "0°"
    }
}

/// Cleans padding artifacts, trailing whitespaces, and hidden null bytes from string text.
fn clean_display_string(input: &str) -> String {
    input
        .replace('\0', "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Prints a comprehensive summary of all available monitor data fields.
fn print_monitor_summary(idx: usize, monitor: &MonitorDetails) {
    let friendly_name = clean_display_string(&monitor.friendly_name);
    let printable_name = if friendly_name.is_empty() {
        "Generic Display"
    } else {
        &friendly_name
    };

    println!("======================================================");
    println!(" MONITOR #{idx} - {printable_name}");
    println!("======================================================");

    // 1. OS Layer Information
    println!("  [OS LAYER]");
    println!("  ├─ Target ID:        {}", monitor.target_id);
    println!("  ├─ GDI Name:         {}", monitor.gdi_name);
    println!(
        "  ├─ Active:           {}",
        if monitor.is_active { "Yes" } else { "No" }
    );
    println!(
        "  ├─ Output Tech:      {}",
        format_output_tech(&monitor.output_tech)
    );
    println!("  └─ Device Path:      {}", monitor.device_path);

    // 2. Monitor Topology Information
    if let Some(topo) = &monitor.topology {
        println!("\n  [TOPOLOGY]");
        println!("  ├─ Position:         X: {}, Y: {}", topo.x, topo.y);
        println!("  ├─ Dimensions:       {}x{}", topo.width, topo.height);
        println!("  └─ Rotation:         {}", format_rotation(&topo.rotation));
    }

    // 3. Hardware Identity (EDID)
    if let Some(data) = &monitor.edid {
        let clean_model = clean_display_string(&data.model_name);
        println!("\n  [HARDWARE IDENTITY (EDID)]");
        println!(
            "  ├─ Manufacturer:     {} (ID: {})",
            clean_model, data.manufacturer_id
        );
        println!("  ├─ Product Code:     {}", data.product_code);
        println!("  ├─ Serial (Binary):  {}", data.serial_number_binary);
        if let Some(serial_ascii) = &data.serial_number_ascii {
            println!(
                "  ├─ Serial (ASCII):   {}",
                clean_display_string(serial_ascii)
            );
        }
        println!(
            "  ├─ Manufacture Date: Week {}, Year {}",
            data.week_of_manufacture, data.year_of_manufacture
        );

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
                println!("  ├─ Interface:        Digital ({type_str}, {bit_depth}-bit)");
            }
            VideoInterfaceInfo::Analog {
                signal_level_v,
                setup_expected,
            } => {
                println!(
                    "  ├─ Interface:        Analog ({}V, Setup Expected: {})",
                    signal_level_v, setup_expected
                );
            }
            VideoInterfaceInfo::Unknown => {
                println!("  ├─ Interface:        Unknown");
            }
        }

        if let Some(chroma) = &data.chromaticity {
            println!("  ├─ Color Gamut:");
            println!(
                "  │  ├─ Red:           X={:.4}, Y={:.4}",
                chroma.red_x, chroma.red_y
            );
            println!(
                "  │  ├─ Green:         X={:.4}, Y={:.4}",
                chroma.green_x, chroma.green_y
            );
            println!(
                "  │  ├─ Blue:          X={:.4}, Y={:.4}",
                chroma.blue_x, chroma.blue_y
            );
            println!(
                "  │  └─ White Point:   X={:.4}, Y={:.4}",
                chroma.white_x, chroma.white_y
            );
        }

        if !data.audio_caps.short_audio_descriptors.is_empty() {
            println!("  ├─ Audio Capabilities:");
            for (a_idx, desc) in data.audio_caps.short_audio_descriptors.iter().enumerate() {
                println!("  │  └─ Codec Descriptor #{}: {}", a_idx + 1, desc);
            }
        }

        if !data.modes.is_empty() {
            println!("  ├─ Supported Display Modes:");
            for (m_idx, mode) in data.modes.iter().enumerate() {
                let interlaced_tag = if mode.interlaced { "i" } else { "" };
                println!(
                    "  │  └─ Mode #{}: {}x{}{} @ {}Hz",
                    m_idx + 1,
                    mode.width,
                    mode.height,
                    interlaced_tag,
                    mode.refresh_rate
                );
            }
        }

        // HDR capabilities — only printed when the monitor actually reports support
        let hdr = &data.hdr_caps;
        if hdr.supports_smpte_st2084 || hdr.supports_hlg || hdr.supports_hdr_traditional {
            println!("  ├─ HDR Capabilities:");
            if hdr.supports_smpte_st2084 {
                println!("  │  ├─ HDR10 (SMPTE ST 2084)");
            }
            if hdr.supports_hlg {
                println!("  │  ├─ Hybrid Log-Gamma (HLG)");
            }
            if hdr.supports_hdr_traditional {
                println!("  │  ├─ Traditional HDR");
            }
            if let Some(v) = hdr.max_luminance_cd_m2 {
                println!("  │  ├─ Max Luminance:  {:.0} cd/m²", v);
            }
            if let Some(v) = hdr.max_frame_average_luminance_cd_m2 {
                println!("  │  ├─ Max Avg Lum:    {:.0} cd/m²", v);
            }
            if let Some(v) = hdr.min_luminance_cd_m2 {
                println!("  │  └─ Min Luminance:  {:.4} cd/m²", v);
            }
        }

        println!("  └─ Extension Blocks: {}", data.extension_blocks);
    }

    // 4. Hardware Bus Stats (DDC/CI)
    println!("\n  [DDC/CI HARDWARE BUS]");
    if let Some(ddc) = &monitor.ddc_stats {
        println!(
            "  ├─ Brightness:       Current: {} / Max: {}",
            ddc.core_caps.brightness, ddc.core_caps.brightness_max
        );
        println!(
            "  ├─ Contrast:         Current: {} / Max: {}",
            ddc.core_caps.contrast, ddc.core_caps.contrast_max
        );
        println!("  ├─ Input Connection: {:?}", ddc.input_source);
        println!("  ├─ Power State:      {:?}", ddc.power_state);
        println!("  ├─ Audio Mute State: {:?}", ddc.audio_mute);

        if let Some(v) = ddc.volume {
            println!("  ├─ Audio Volume:     Current: {} / Max: {}", v.0, v.1);
        }
        if let Some((r, g, b)) = ddc.color_gains {
            println!("  ├─ RGB Hardware Gain:R:{} G:{} B:{}", r, g, b);
        }
        if let Some(h_freq) = ddc.horizontal_freq_hz {
            // Register 0xAC: value is in units of 1 Hz (no scaling needed)
            println!("  ├─ Horizontal Freq:  {:.2} kHz", h_freq as f64 / 1000.0);
        }
        // FIX: Field renamed from vertical_freq_mhz to vertical_freq_centihz.
        // Register 0xAE reports in units of 0.01 Hz → divide by 100 to get Hz.
        if let Some(v_freq) = ddc.vertical_freq_centihz {
            println!("  ├─ Vertical Freq:    {:.2} Hz", v_freq as f64 / 100.0);
        }
        if let Some(hours) = ddc.operating_hours {
            println!("  ├─ Operating Time:   {} Hours", hours);
        }
        if let Some(lang) = ddc.osd_language_code {
            println!("  ├─ OSD Language Code:0x{:X}", lang);
        }
        if let Some(panel) = ddc.panel_type_code {
            println!("  └─ Panel Type Code:  0x{:X}", panel);
        }
    } else {
        println!("  └─ Status:           Not available (Monitor inactive or DDC blocked)");
    }
}

fn main() {
    println!("Starting Full-Type Hardware Diagnostic...\n");

    match collect_monitor_data() {
        Ok(monitors) => {
            println!("Discovered {} monitors:\n", monitors.len());
            for (idx, monitor) in monitors.iter().enumerate() {
                print_monitor_summary(idx + 1, monitor);
                println!();
            }
        }
        Err(e) => {
            eprintln!("Failed to query monitor details: {:?}", e);
        }
    }
}
