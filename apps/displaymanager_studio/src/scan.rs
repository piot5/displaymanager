use windows::core::Result;
use serde::Serialize;

pub use df_displmgr_info::edid_types::EdidData;

/// Studio-specific MonitorDetails wrapper with flattened topology fields.
/// This is a local adapter that wraps the library's MonitorDetails
/// to match the studio's expected data structure.
#[derive(Debug, Serialize, Clone)]
pub struct MonitorDetails {
    pub target_id: u32,
    pub friendly_name: String,
    pub is_active: bool,
    pub output_tech: String,
    pub gdi_name: String,
    pub native_path: String,
    pub device_path: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rotation: String,
    pub edid: Option<EdidData>,
    pub ddc_stats: Option<DdcData>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DdcData {
    pub brightness: (u32, u32, u32),
    pub contrast: Option<(u32, u32, u32)>,
}

/// Collects comprehensive monitor data from the library and adapts it to studio format.
/// 
/// This function calls the library's `collect_monitor_data()` to get the raw monitor data,
/// then converts it to the studio's expected `MonitorDetails` format with flattened
/// topology and DDC statistics fields.
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>> {
    let lib_monitors = df_displmgr_info::collect_monitor_data()
        .map_err(|_| windows::Win32::Foundation::E_FAIL)?;
    
    Ok(lib_monitors
        .into_iter()
        .map(|lib_mon| {
            let (x, y, width, height, rotation) = if let Some(topo) = &lib_mon.topology {
                (topo.x, topo.y, topo.width, topo.height, topo.rotation.clone())
            } else {
                (0, 0, 0, 0, "Unknown".to_string())
            };
            
            let ddc_stats = lib_mon.ddc_stats.as_ref().map(|deep_stats| {
                DdcData {
                    brightness: (0, deep_stats.core_caps.brightness, deep_stats.core_caps.brightness_max),
                    contrast: Some((0, deep_stats.core_caps.contrast, deep_stats.core_caps.contrast_max)),
                }
            });
            
            MonitorDetails {
                target_id: lib_mon.target_id,
                friendly_name: lib_mon.friendly_name,
                is_active: lib_mon.is_active,
                output_tech: lib_mon.output_tech,
                gdi_name: lib_mon.gdi_name,
                native_path: lib_mon.device_path.clone(),
                device_path: lib_mon.device_path,
                x,
                y,
                width,
                height,
                rotation,
                edid: lib_mon.edid,
                ddc_stats,
            }
        })
        .collect())
}