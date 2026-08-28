use df_displmgr_info::collect_monitor_data;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning for monitors and collecting EDID data...");

    let monitors = collect_monitor_data()?;

    if monitors.is_empty() {
        println!("No monitors found.");
        return Ok(());
    }

    for (i, m) in monitors.iter().enumerate() {
        println!("\n--- Monitor {} ---", i);
        println!("ID:           {}", m.target_id);
        println!("Friendly Name:{}", m.friendly_name);
        println!("GDI Name:     {}", m.gdi_name);

        if let Some(edid) = &m.edid {
            println!("Model Name:   {}", edid.model_name);
            println!("Manufacturer ID: {}", edid.manufacturer_id);
            println!("Product Code: {}", edid.product_code);
            println!(
                "Serial Number: {}",
                edid.serial_number_ascii
                    .as_ref()
                    .unwrap_or(&"N/A".to_string())
            );
            println!("Year:         {}", edid.year_of_manufacture);
            println!("Week:         {}", edid.week_of_manufacture);
            println!("Interface:    {:?}", edid.video_interface);

            // HDR Support is not available in this version
            // if let Some(hdr) = &edid.hdr_caps {
            //     println!("HDR Support:  SDR: {}, HDR10: {}, HLG: {}",
            //         hdr.supports_sdr_eotf,
            //         hdr.supports_smpte_st2084,
            //         hdr.supports_hlg
            //     );
            //     if let Some(lum) = hdr.max_luminance_cd_m2 {
            //         println!("Max Luminance: {:.2} cd/m²", lum);
            //     }
            // }

            println!("Extension Blocks: {}", edid.extension_blocks);
            if !edid.modes.is_empty() {
                println!("Modes found: {}", edid.modes.len());
                for mode in &edid.modes {
                    println!(
                        "  - {}x{} @ {}Hz (interlaced: {})",
                        mode.width, mode.height, mode.refresh_rate, mode.interlaced
                    );
                }
            } else {
                println!("No modes found in EDID data");
            }
        }

        if let Some(ddc) = &m.ddc_stats {
            println!("DDC Stats:");
            println!("  Power State:  {:?}", ddc.power_state);
            println!("  Input Source: {:?}", ddc.input_source);
            if let Some((vol, max)) = ddc.volume {
                println!("  Volume:       {}/{}", vol, max);
            }
            println!("  Audio Mute:   {:?}", ddc.audio_mute);
        }
    }

    Ok(())
}
