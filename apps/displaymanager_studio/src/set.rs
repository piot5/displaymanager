use anyhow::{Result, Context};
use clap::{Parser, Args};

use df_displmgr::{NativeTopology, UniversalTopology};
use df_displmgr::backends::windows::displmgr_gdi::GdiTopology;
use df_displmgr::types::{Extent2D, Point2D};

use df_ddc::list_monitors;
use df_ddc::ddc_types::{PowerState, InputSource};

use crate::scan;

#[derive(Parser, Debug, Default)]
pub struct DemoArgs {
    #[arg(short = 's', long)]
    pub scan: bool,
    #[command(flatten)]
    pub output_config: OutputArgs,
    #[arg(long)]
    pub brightness: Option<u32>,
    #[arg(long)]
    pub contrast: Option<u32>,
    #[arg(long)]
    pub power: Option<String>,
    #[arg(long)]
    pub input: Option<String>,
    #[arg(long)]
    pub off: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct OutputArgs {
    #[arg(short = 'o', long)]
    pub output: Option<String>,
    #[arg(short = 'm', long)]
    pub mode: Option<String>,
    #[arg(short = 'p', long)]
    pub pos: Option<String>,
    #[arg(short = 'r', long)]
    pub rotate: Option<String>,
}

pub async fn apply_all_settings(target_id_val: u32, args: &DemoArgs) -> Result<()> {
    let monitor_data = scan::collect_monitor_data().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    
    let target = monitor_data.into_iter().find(|m| m.target_id == target_id_val)
        .context("The specified Monitor ID was not found in the current system scan.")?;

    if args.brightness.is_some() || args.contrast.is_some() || args.power.is_some() || args.input.is_some() {
        apply_ddc_settings(&target.gdi_name, &target.friendly_name, args)
            .context("Failed to apply hardware settings via DDC/CI bus")?;
    }

    if args.off {
        apply_gdi_fallback(&target.gdi_name, &args.output_config, true).await?;
    } else {
        // Try the native CCD (Windows Display Config) path first.
        // CCD supports advanced features like persistence across reboots.
        let ccd_res = apply_native_layout_settings(&target.device_path, &args.output_config).await;

        if let Err(e) = ccd_res {
            // Fall back to GDI (Windows Graphics Device Interface) when CCD fails.
            // GDI is more limited but works on older systems or for basic operations.
            eprintln!("CCD path failed ({}), falling back to GDI", e);
            apply_gdi_fallback(&target.gdi_name, &args.output_config, false).await?;
        }
    }

    Ok(())
}

fn apply_ddc_settings(gdi_name: &str, friendly_name: &str, args: &DemoArgs) -> Result<()> {
    let monitors = list_monitors();
    let gdi_lower = gdi_name.to_lowercase();
    let friendly_lower = friendly_name.to_lowercase();
    
    // Find the monitor matching GDI name or friendly name
    let mon = monitors.iter().find(|m| {
        let info = m.info.to_lowercase();
        info.contains(&gdi_lower) || 
        (!friendly_lower.is_empty() && info.contains(&friendly_lower))
    })
    .with_context(|| {
        let available: Vec<String> = monitors.iter().map(|m| m.info.clone()).collect();
        format!("DDC device not found. Targets: GDI='{}', Name='{}'\nAvailable: {:?}", gdi_name, friendly_name, available)
    })?;

    if let Some(val) = args.brightness { 
        mon.inner.set_vcp_feature(0x10, val).map_err(|e| anyhow::anyhow!("Brightness VCP error: {:?}", e))?; 
    }
    
    if let Some(val) = args.contrast { 
        mon.inner.set_vcp_feature(0x12, val).map_err(|e| anyhow::anyhow!("Contrast VCP error: {:?}", e))?; 
    }
    
    if let Some(ref p_str) = args.power {
        let state = if p_str.to_lowercase() == "on" { PowerState::On } else { PowerState::Off };
        let _ = mon.inner.set_power(state);
    }
    
    if let Some(ref i_str) = args.input {
        let source = match i_str.to_lowercase().as_str() {
            "hdmi1" => InputSource::Hdmi1,
            "hdmi2" => InputSource::Hdmi2,
            "dp1" => InputSource::DisplayPort1,
            _ => InputSource::DisplayPort2,
        };
        let _ = mon.inner.set_input(source);
    }
    Ok(())
}

async fn apply_native_layout_settings(dev_path: &str, config: &OutputArgs) -> Result<()> {
    let mut topology = NativeTopology::acquire().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    topology.set_persistence(true);

    let internal_id = topology.get_outputs().iter()
        .find(|o| o.identity.id.0 == dev_path || dev_path.contains(&o.identity.id.0))
        .map(|o| o.identity.id.clone())
        .context("Device path not found in CCD stack.")?;

    {
        let mut editor = topology.edit_output(&internal_id).context("Could not open CCD editor.")?;
        editor.set_enabled(true).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        
        if let Some(ref mode_str) = config.mode {
            let parts: Vec<&str> = mode_str.split('@').collect();
            let res_parts: Vec<u32> = parts[0].split('x').filter_map(|s| s.parse().ok()).collect();
            if res_parts.len() == 2 { 
                let _ = editor.set_resolution(Extent2D { width: res_parts[0], height: res_parts[1] }); 
            }
            if parts.len() > 1 {
                if let Ok(mut hz) = parts[1].parse::<u32>() { 
                    if hz > 1000 { hz /= 1000; } 
                    let _ = editor.set_refresh_rate(hz * 1000); 
                }
            }
        }
        if let Some(ref pos_str) = config.pos {
            let p: Vec<i32> = pos_str.split('x').filter_map(|s| s.parse().ok()).collect();
            if p.len() == 2 { let _ = editor.set_position(Point2D { x: p[0], y: p[1] }); }
        }
    }
    topology.commit().await.map_err(|e| anyhow::anyhow!("{:?}", e))
}

async fn apply_gdi_fallback(gdi_name: &str, config: &OutputArgs, off: bool) -> Result<()> {
    let mut topology: GdiTopology = UniversalTopology::acquire()
        .map_err(|e| anyhow::anyhow!("GDI backend init failed: {:?}", e))?;

    let internal_id = topology.get_outputs().iter()
        .find(|o| o.identity.id.0.to_lowercase() == gdi_name.to_lowercase())
        .map(|o| o.identity.id.clone())
        .context("Monitor not found in GDI list.")?;

    {
        let mut editor = topology.edit_output(&internal_id).context("Could not open GDI editor.")?;

        if off {
            let _ = editor.set_enabled(false);
        } else {
            let _ = editor.set_enabled(true);
            
            if let Some(ref mode_str) = config.mode {
                let parts: Vec<&str> = mode_str.split('@').collect();
                let res_parts: Vec<u32> = parts[0].split('x').filter_map(|s| s.parse().ok()).collect();
                if res_parts.len() == 2 { let _ = editor.set_resolution(Extent2D { width: res_parts[0], height: res_parts[1] }); }
                if parts.len() > 1 {
                    if let Ok(mut hz) = parts[1].parse::<u32>() {
                        if hz > 1000 { hz /= 1000; } 
                        let _ = editor.set_refresh_rate(hz * 1000); 
                    }
                }
            }
            if let Some(ref pos_str) = config.pos {
                let p: Vec<i32> = pos_str.split('x').filter_map(|s| s.parse().ok()).collect();
                if p.len() == 2 { let _ = editor.set_position(Point2D { x: p[0], y: p[1] }); }
            }
        }
    }

    topology.commit().await.map_err(|e| anyhow::anyhow!("GDI commit failed: {:?}", e))?;
    Ok(())
}