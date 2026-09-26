//! DDC/CI monitor control integrated into the flat CLI.
//!
//! Resolves the monitor by name/ID/path (via `info::resolve_monitor`),
//! finds the matching DDC device, and applies the requested DDC operation.

use crate::info;
use anyhow::{anyhow, Context};
use df_ddc::ddc_types::{InputSource, PowerState, VcpCode};
use df_ddc::list_monitors;

/// Apply DDC operations to a monitor identified by `query`.
///
/// This function validates all VCP value ranges before applying changes
/// and provides dry-run support through the `--dry-run` CLI flag.
pub fn apply_ddc(
    query: &str,
    brightness: Option<u32>,
    contrast: Option<u32>,
    volume: Option<u32>,
    input: Option<&str>,
    power: Option<&str>,
    dry_run: bool,
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

    // Check if any DDC values are being set
    let has_changes = brightness.is_some()
        || contrast.is_some()
        || volume.is_some()
        || input.is_some()
        || power.is_some();

    if !has_changes {
        println!(
            "  ⚠ No DDC values to set (brightness, contrast, volume, input, or power required)"
        );
        return Ok(());
    }

    if dry_run {
        println!("  🔍 Dry-run mode: would apply the following changes:");
    }

    // Apply brightness with range validation
    if let Some(val) = brightness {
        if !(0..=100).contains(&val) {
            return Err(anyhow!("Brightness must be between 0 and 100, got {}", val));
        }
        if dry_run {
            println!("    → brightness: {}", val);
        } else {
            device
                .set_vcp_feature(VcpCode::Brightness as u8, val)
                .context("DDC brightness set failed")?;
            println!("  DDC: brightness set to {} on {}", val, device.info);
        }
    }

    // Apply contrast with range validation
    if let Some(val) = contrast {
        if !(0..=100).contains(&val) {
            return Err(anyhow!("Contrast must be between 0 and 100, got {}", val));
        }
        if dry_run {
            println!("    → contrast: {}", val);
        } else {
            device
                .set_vcp_feature(VcpCode::Contrast as u8, val)
                .context("DDC contrast set failed")?;
            println!("  DDC: contrast set to {} on {}", val, device.info);
        }
    }

    // Apply volume with range validation
    if let Some(val) = volume {
        if !(0..=100).contains(&val) {
            return Err(anyhow!("Volume must be between 0 and 100, got {}", val));
        }
        if dry_run {
            println!("    → volume: {}", val);
        } else {
            device
                .set_vcp_feature(VcpCode::Volume as u8, val)
                .context("DDC volume set failed")?;
            println!("  DDC: volume set to {} on {}", val, device.info);
        }
    }

    // Apply input source with validation
    if let Some(source) = input {
        let src = parse_input_source(source)?;
        if dry_run {
            println!("    → input: {}", source);
        } else {
            device.set_input(src).context("DDC input set failed")?;
            println!("  DDC: input set to {} on {}", source, device.info);
        }
    }

    // Apply power state with validation
    if let Some(state) = power {
        let ps = match state.to_lowercase().as_str() {
            "on" | "1" => PowerState::On,
            "off" | "0" => PowerState::Off,
            _ => {
                return Err(anyhow!(
                    "Unrecognized power state 'state', use 'on' or 'off'"
                ))
            }
        };
        if dry_run {
            println!("    → power: {}", state);
        } else {
            device.set_power(ps).context("DDC power set failed")?;
            println!("  DDC: power set to {} on {}", state, device.info);
        }
    }

    if dry_run {
        println!("  ℹ No changes applied (dry-run)");
    }

    Ok(())
}

/// Parse input source string to InputSource enum.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input_source_numeric() {
        assert_eq!(parse_input_source("15").unwrap(), InputSource::DisplayPort1);
    }

    #[test]
    fn test_parse_input_source_hex() {
        assert_eq!(parse_input_source("0x11").unwrap(), InputSource::Hdmi1);
    }

    #[test]
    fn test_parse_input_source_named_hdmi() {
        assert_eq!(parse_input_source("hdmi1").unwrap(), InputSource::Hdmi1);
        assert_eq!(parse_input_source("hdmi-1").unwrap(), InputSource::Hdmi1);
        assert_eq!(parse_input_source("hdmi1.0").unwrap(), InputSource::Hdmi1);
        assert_eq!(parse_input_source("HDMI1").unwrap(), InputSource::Hdmi1);
        assert_eq!(parse_input_source("hdmi2").unwrap(), InputSource::Hdmi2);
        assert_eq!(parse_input_source("hdmi-2").unwrap(), InputSource::Hdmi2);
    }

    #[test]
    fn test_parse_input_source_named_dp() {
        assert_eq!(
            parse_input_source("dp1").unwrap(),
            InputSource::DisplayPort1
        );
        assert_eq!(
            parse_input_source("DISPLAYPORT1").unwrap(),
            InputSource::DisplayPort1
        );
        assert_eq!(
            parse_input_source("displayport1.0").unwrap(),
            InputSource::DisplayPort1
        );
        assert_eq!(
            parse_input_source("dp2").unwrap(),
            InputSource::DisplayPort2
        );
        assert_eq!(
            parse_input_source("DISPLAYPORT2").unwrap(),
            InputSource::DisplayPort2
        );
    }

    #[test]
    fn test_parse_input_source_invalid() {
        assert!(parse_input_source("invalid").is_err());
        assert!(parse_input_source("999").is_err());
    }
}
