// apps/displaymanager_cli/src/logic.rs

use crate::cli::{DisplayArgs, DdcArgs, DdcAction};
use crate::utils::{FileBuffer, formatters};
use crate::synth::MonitorSynthesis;
use df_ddc::{list_monitors};
use df_ddc::ddc_types::{VcpCode, PowerState, InputSource};
// Ensure this crate name matches your Cargo.toml exactly
use df_displmgr::{NativeTopology, traits::UniversalTopology, types::DisplayId, types::Extent2D, types::Point2D};
use df_displmgr_info::collect_monitor_data;
use anyhow::{anyhow, Context};

/// Handles display topology operations, routing mutation requests to df_displmgr.
pub async fn handle_display(args: DisplayArgs) -> anyhow::Result<()> {
    // 1. Scan command: Simply list hardware info
    if args.scan {
        let monitors = collect_monitor_data().context("Hardware scan failed")?;
        for m in monitors {
            println!("ID {}: {} (Active: {})", m.target_id, m.friendly_name, m.is_active);
        }
    } 
    // 2. Mutation command: Use synth-aware resolution for stable identity
    else if let Some(input) = args.output {
        // Resolve the user-friendly name/id/path to a stable hardware identity
        let target = MonitorSynthesis::resolve(&input)?;
        
        // Acquire the platform-specific topology, explicitly typed to assist inference
        let mut topology: NativeTopology = NativeTopology::acquire()?;
        
        // CRITICAL FIX: Use device_path as the unique handle for the graphics subsystem
        // instead of target_id, which can change between reboots.
        let mut editor = topology.edit_output(&DisplayId(target.device_path.clone()))
            .context("Failed to acquire editor handle. The device path might be locked or invalid.")?;

        // Apply Resolution/Mode if provided
        if let Some(mode_str) = args.mode {
            let parts: Vec<&str> = mode_str.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse::<u32>()?;
                let h = parts[1].parse::<u32>()?;
                editor.set_resolution(Extent2D { width: w, height: h })?;
                println!("Resolution set to {}x{} for {}", w, h, target.friendly_name);
            }
        }

        // Apply position if provided (format: XxY or X,X)
        if let Some(pos_str) = args.pos {
            let parts: Vec<&str> = pos_str.split(|c| c == 'x' || c == 'X' || c == ',').collect();
            if parts.len() == 2 {
                let x = parts[0].parse::<i32>().context("Invalid X position")?;
                let y = parts[1].parse::<i32>().context("Invalid Y position")?;
                editor.set_position(Point2D { x, y })?;
                println!("Position set to {}x{} for {}", x, y, target.friendly_name);
            }
        }

        // Apply rotation if provided (accepted: 0,90,180,270)
        if let Some(rot_str) = args.rotate {
            let rot_val = rot_str.trim().trim_end_matches("deg");
            let rotation = match rot_val {
                "0" | "Rotate0" | "rotate0" => df_displmgr::types::DisplayRotation::Rotate0,
                "90" | "Rotate90" | "rotate90" => df_displmgr::types::DisplayRotation::Rotate90,
                "180" | "Rotate180" | "rotate180" => df_displmgr::types::DisplayRotation::Rotate180,
                "270" | "Rotate270" | "rotate270" => df_displmgr::types::DisplayRotation::Rotate270,
                _ => {
                    println!("Unrecognized rotation '{}', skipping", rot_str);
                    df_displmgr::types::DisplayRotation::Rotate0
                }
            };
            editor.set_rotation(rotation)?;
            println!("Rotation set to {:?} for {}", rotation, target.friendly_name);
        }
        // Handle power state
        if args.off {
            editor.set_enabled(false)?;
            println!("Disabled signal output for {}", target.friendly_name);
        } else {
            editor.set_enabled(true)?;
            println!("Enabled signal output for {}", target.friendly_name);
        }

        // Commit changes to the graphics subsystem
        drop(editor);
        topology.commit().await.context("Topology commit rejected by OS")?;
        println!("Configuration successfully applied.");
    }
    Ok(())
}

/// Generates an EDID diagnostic report using gathered metadata.
pub fn handle_edid() -> anyhow::Result<()> {
    let mut file_engine = FileBuffer::new();
    let monitors = collect_monitor_data().context("Failed to collect monitor metadata")?;
    
    file_engine.write_line("--- Monitor Diagnostic Report ---");
    for (idx, m) in monitors.iter().enumerate() {
        // Use formatter utilities to resolve 'dead_code' warnings and clean output
        let name = formatters::clean_display_string(&m.friendly_name);
        let tech = formatters::format_output_tech(&m.output_tech);
        
        file_engine.write_line(&format!("Monitor {}: {}", idx + 1, name));
        file_engine.write_line(&format!("  Interface: {}", tech));
        file_engine.write_line(&format!("  Device Path: {}", m.device_path));
    }
    
    file_engine.save_to_file("edid_dump.txt");
    println!("Diagnostic report saved to 'edid_dump.txt'.");
    Ok(())
}

/// Handles DDC/CI commands, utilizing the deep telemetry provided by df_displmgr_info.
pub fn handle_ddc(args: DdcArgs) -> anyhow::Result<()> {
    let devices = list_monitors();
    if devices.is_empty() {
        return Err(anyhow!("No DDC-capable monitors detected on this system."));
    }

    let device = devices.get(args.id)
        .ok_or_else(|| anyhow!("DDC monitor index {} is out of bounds (0..{})", args.id, devices.len().saturating_sub(1)))?;

    match args.action {
        DdcAction::List => {
            // Query reported capabilities where available
            match device.inner.get_capabilities() {
                Ok(caps) => println!("Monitor: {} | Brightness: {}/{} | Contrast: {}/{}",
                    device.info, caps.brightness, caps.brightness_max, caps.contrast, caps.contrast_max),
                Err(e) => println!("Monitor: {} | Capabilities unavailable: {:?}", device.info, e),
            }
        }
        DdcAction::Brightness { value } => {
            device.inner.set_vcp_feature(VcpCode::Brightness as u8, value)
                .map_err(|e| anyhow!(format!("DDC brightness set failed: {:?}", e)))?;
            println!("Set brightness to {} on {}", value, device.info);
        }
        DdcAction::Contrast { value } => {
            device.inner.set_vcp_feature(VcpCode::Contrast as u8, value)
                .map_err(|e| anyhow!(format!("DDC contrast set failed: {:?}", e)))?;
            println!("Set contrast to {} on {}", value, device.info);
        }
        DdcAction::Volume { value } => {
            device.inner.set_vcp_feature(VcpCode::Volume as u8, value)
                .map_err(|e| anyhow!(format!("DDC volume set failed: {:?}", e)))?;
            println!("Set volume to {} on {}", value, device.info);
        }
        DdcAction::Power { state } => {
            let ps = match state.to_lowercase().as_str() {
                "on" | "1" => PowerState::On,
                "off" | "0" => PowerState::Off,
                _ => return Err(anyhow!("Unrecognized power state '{}', use 'on' or 'off'", state)),
            };
            device.inner.set_power(ps)
                .map_err(|e| anyhow!(format!("DDC power set failed: {:?}", e)))?;
            println!("Set power {} on {}", state, device.info);
        }
        DdcAction::Input { source } => {
            // Accept either friendly names or numeric VCP values (hex or decimal)
            let lowered = source.to_lowercase();
            let src = if let Ok(n) = if lowered.starts_with("0x") {
                u32::from_str_radix(lowered.trim_start_matches("0x"), 16)
            } else {
                lowered.parse::<u32>()
            } {
                match n {
                    0x0F => InputSource::DisplayPort1,
                    0x10 => InputSource::DisplayPort2,
                    0x11 => InputSource::Hdmi1,
                    0x12 => InputSource::Hdmi2,
                    _ => return Err(anyhow!("Unrecognized numeric input source 0x{:X}", n)),
                }
            } else {
                match lowered.as_str() {
                    "dp1" | "displayport1" | "displayport1.0" => InputSource::DisplayPort1,
                    "dp2" | "displayport2" => InputSource::DisplayPort2,
                    "hdmi1" | "hdmi-1" | "hdmi1.0" => InputSource::Hdmi1,
                    "hdmi2" | "hdmi-2" => InputSource::Hdmi2,
                    other => return Err(anyhow!("Unrecognized input source '{}'", other)),
                }
            };
            device.inner.set_input(src)
                .map_err(|e| anyhow!(format!("DDC input set failed on {}: {:?}", device.info, e)))?;
            println!("Set input {} on {}", source, device.info);
        }
    }

    Ok(())
}