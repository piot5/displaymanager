use crate::cli::{DdcAction, DdcArgs};
use anyhow::anyhow;
use df_ddc::ddc_types::{InputSource, PowerState, VcpCode};
use df_ddc::list_monitors;

pub fn run(args: DdcArgs) -> anyhow::Result<()> {
    let devices = list_monitors();
    if devices.is_empty() {
        return Err(anyhow!("No DDC-capable monitors detected on this system."));
    }

    let device = devices.get(args.id).ok_or_else(|| {
        anyhow!(
            "DDC monitor index {} is out of bounds (0..{})",
            args.id,
            devices.len().saturating_sub(1)
        )
    })?;

    match args.action {
        DdcAction::List => {
            if args.json {
                let json_output = match device.inner.get_capabilities() {
                    Ok(caps) => serde_json::json!({
                        "monitor": device.info,
                        "brightness": caps.brightness,
                        "brightness_max": caps.brightness_max,
                        "contrast": caps.contrast,
                        "contrast_max": caps.contrast_max,
                        "status": "ok"
                    }),
                    Err(e) => serde_json::json!({
                        "monitor": device.info,
                        "error": format!("{:?}", e),
                        "status": "error"
                    }),
                };
                println!("{}", serde_json::to_string_pretty(&json_output)?);
            } else {
                match device.inner.get_capabilities() {
                    Ok(caps) => println!(
                        "Monitor: {} | Brightness: {}/{} | Contrast: {}/{}",
                        device.info,
                        caps.brightness,
                        caps.brightness_max,
                        caps.contrast,
                        caps.contrast_max
                    ),
                    Err(e) => println!(
                        "Monitor: {} | Capabilities unavailable: {:?}",
                        device.info, e
                    ),
                }
            }
        }
        DdcAction::Brightness { value } => {
            device
                .inner
                .set_vcp_feature(VcpCode::Brightness as u8, value)
                .map_err(|e| anyhow!("DDC brightness set failed: {:?}", e))?;
            println!("Set brightness to {value} on {}", device.info);
        }
        DdcAction::Contrast { value } => {
            device
                .inner
                .set_vcp_feature(VcpCode::Contrast as u8, value)
                .map_err(|e| anyhow!("DDC contrast set failed: {:?}", e))?;
            println!("Set contrast to {value} on {}", device.info);
        }
        DdcAction::Volume { value } => {
            device
                .inner
                .set_vcp_feature(VcpCode::Volume as u8, value)
                .map_err(|e| anyhow!("DDC volume set failed: {:?}", e))?;
            println!("Set volume to {value} on {}", device.info);
        }
        DdcAction::Power { state } => {
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
            println!("Set power {state} on {}", device.info);
        }
        DdcAction::Input { source } => {
            let src = parse_input_source(&source)?;
            device
                .inner
                .set_input(src)
                .map_err(|e| anyhow!("DDC input set failed on {}: {:?}", device.info, e))?;
            println!("Set input {source} on {}", device.info);
        }
        DdcAction::ColorGains { red, green, blue } => {
            device
                .inner
                .set_vcp_feature(VcpCode::RedGain as u8, red)
                .map_err(|e| anyhow!("DDC red gain set failed: {:?}", e))?;
            device
                .inner
                .set_vcp_feature(VcpCode::GreenGain as u8, green)
                .map_err(|e| anyhow!("DDC green gain set failed: {:?}", e))?;
            device
                .inner
                .set_vcp_feature(VcpCode::BlueGain as u8, blue)
                .map_err(|e| anyhow!("DDC blue gain set failed: {:?}", e))?;
            println!(
                "Set color gains to (R={red}, G={green}, B={blue}) on {}",
                device.info
            );
        }
    }

    Ok(())
}

fn parse_input_source(source: &str) -> anyhow::Result<InputSource> {
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
