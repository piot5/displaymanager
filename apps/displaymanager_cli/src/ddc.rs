//! DDC/CI monitor control integrated into the flat CLI.
//!
//! Resolves the monitor by name/ID/path (via `info::resolve_monitor`),
//! finds the matching DDC device, and applies the requested DDC operation.

use crate::info;
use anyhow::anyhow;
use df_ddc::ddc_types::{InputSource, PowerState, VcpCode};
use df_ddc::list_monitors;

/// Apply DDC operations to a monitor identified by `query`.
pub fn apply_ddc(
    query: &str,
    brightness: Option<u32>,
    contrast: Option<u32>,
    volume: Option<u32>,
    input: Option<&str>,
    power: Option<&str>,
) -> anyhow::Result<()> {
    // Resolve the monitor by name/ID/path to get its friendly name
    let target = info::resolve_monitor(query)?;
    let friendly = target.friendly_name.trim().to_string().to_lowercase();

    // Find matching DDC device by friendly name substring
    let devices = list_monitors();
    let device = devices
        .iter()
        .find(|d| d.info.to_lowercase().contains(&friendly))
        .ok_or_else(|| {
            anyhow!(
                "Monitor '{}' not found among DDC-capable devices. Available: {:?}",
                target.friendly_name.trim(),
                devices.iter().map(|d| &d.info).collect::<Vec<_>>()
            )
        })?;

    if let Some(val) = brightness {
        device
            .inner
            .set_vcp_feature(VcpCode::Brightness as u8, val)
            .map_err(|e| anyhow!("DDC brightness set failed: {:?}", e))?;
        println!("  DDC: brightness set to {} on {}", val, device.info);
    }

    if let Some(val) = contrast {
        device
            .inner
            .set_vcp_feature(VcpCode::Contrast as u8, val)
            .map_err(|e| anyhow!("DDC contrast set failed: {:?}", e))?;
        println!("  DDC: contrast set to {} on {}", val, device.info);
    }

    if let Some(val) = volume {
        device
            .inner
            .set_vcp_feature(VcpCode::Volume as u8, val)
            .map_err(|e| anyhow!("DDC volume set failed: {:?}", e))?;
        println!("  DDC: volume set to {} on {}", val, device.info);
    }

    if let Some(source) = input {
        let src = parse_input_source(source)?;
        device
            .inner
            .set_input(src)
            .map_err(|e| anyhow!("DDC input set failed: {:?}", e))?;
        println!("  DDC: input set to {} on {}", source, device.info);
    }

    if let Some(state) = power {
        let ps = match state.to_lowercase().as_str() {
            "on" | "1" => PowerState::On,
            "off" | "0" => PowerState::Off,
            _ => {
                return Err(anyhow!(
                    "Unrecognized power state '{state}', use 'on' or 'off'"
                ))
            }
        };
        device
            .inner
            .set_power(ps)
            .map_err(|e| anyhow!("DDC power set failed: {:?}", e))?;
        println!("  DDC: power set to {} on {}", state, device.info);
    }

    Ok(())
}

pub fn parse_input_source(source: &str) -> anyhow::Result<InputSource> {
    let lowered = source.to_lowercase();
    let numeric = if lowered.starts_with("0x") {
        u32::from_str_radix(lowered.trim_start_matches("0x"), 16)
    } else {
        lowered.parse::<u32>()
    };

    if let Ok(n) = numeric {
        return match n {
            0x0F => Ok(InputSource::DisplayPort1),
            0x10 => Ok(InputSource::DisplayPort2),
            0x11 => Ok(InputSource::Hdmi1),
            0x12 => Ok(InputSource::Hdmi2),
            _ => Err(anyhow!("Unrecognized numeric input source 0x{n:X}")),
        };
    }

    match lowered.as_str() {
        "dp1" | "displayport1" | "displayport1.0" => Ok(InputSource::DisplayPort1),
        "dp2" | "displayport2" => Ok(InputSource::DisplayPort2),
        "hdmi1" | "hdmi-1" | "hdmi1.0" => Ok(InputSource::Hdmi1),
        "hdmi2" | "hdmi-2" => Ok(InputSource::Hdmi2),
        other => Err(anyhow!("Unrecognized input source '{other}'")),
    }
}
