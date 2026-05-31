pub mod edid_types;
pub mod edid_trait;
pub mod edid_backends;
pub mod edid_parser;
pub mod error;

pub use crate::edid_trait::DisplayDevice;
use crate::edid_trait::EdidControl;
use crate::edid_types::{MonitorTopology, DeepDdcStats};
use std::collections::HashSet;

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows::Win32::Devices::Display::*;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{LPARAM, BOOL, RECT};
#[cfg(target_os = "windows")]
use std::mem::size_of;

/// High-level structure representing comprehensive monitor details.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorDetails {
    pub target_id: u32,
    pub friendly_name: String,
    pub is_active: bool,
    pub output_tech: String,
    pub gdi_name: String,
    pub device_path: String,
    pub topology: Option<MonitorTopology>,
    pub edid: Option<crate::edid_types::EdidData>,
    pub ddc_stats: Option<DeepDdcStats>,
}

/// Collects all monitor data by orchestrating CCD, GDI, and Registry backends on Windows targets.
#[cfg(target_os = "windows")]
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>, crate::error::EdidError> {
    let mut results = Vec::new();
    let mut seen_targets = HashSet::new();

    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;

        // Retrieve the required buffer sizes for the active display paths
        if GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count).is_err() {
            return Err(crate::error::EdidError::CommunicationFailed);
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        // Query the active layout configuration from the OS CCD subsystem
        if QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
        .is_err()
        {
            return Err(crate::error::EdidError::CommunicationFailed);
        }

        for path in paths.iter().take(path_count as usize) {
            let target_id = path.targetInfo.id;
            let adapter_luid = path.targetInfo.adapterId;
            let is_active = (path.flags & DISPLAYCONFIG_PATH_ACTIVE) != 0;

            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target_name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target_name.header.adapterId = adapter_luid;
            target_name.header.id = target_id;

            if DisplayConfigGetDeviceInfo(&mut target_name.header) != 0 {
                continue;
            }

            // Extract the persistent OS kernel device path
            let device_path = String::from_utf16_lossy(&target_name.monitorDevicePath)
                .trim_matches(char::from(0))
                .to_string();

            if device_path.is_empty() || !seen_targets.insert(target_id) {
                continue;
            }

            let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source_name.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            source_name.header.adapterId = adapter_luid;
            source_name.header.id = path.sourceInfo.id;

            // Resolve the logical GDI device surface interface handle
            let gdi_name = if DisplayConfigGetDeviceInfo(&mut source_name.header) == 0 {
                String::from_utf16_lossy(&source_name.viewGdiDeviceName)
                    .trim_matches(char::from(0))
                    .to_string()
            } else {
                "Unknown".to_string()
            };

            let mut topology = None;
            if is_active {
                let mode_idx = path.sourceInfo.Anonymous.modeInfoIdx as usize;
                if mode_idx < modes.len() {
                    let mode = &modes[mode_idx];
                    if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                        let source = mode.Anonymous.sourceMode;
                        topology = Some(MonitorTopology {
                            x: source.position.x,
                            y: source.position.y,
                            width: source.width,
                            height: source.height,
                            rotation: format!("{:?}", path.targetInfo.rotation),
                        });
                    }
                }
            }

            // Route execution payload to the robust registry fallback layer
            let edid_data = crate::edid_backends::edid_win_reg::WindowsRegBackend {
                handle: None,
                device_id_override: Some(device_path.clone()),
            }
            .get_edid_raw()
            .ok()
            .and_then(|raw| crate::edid_parser::EdidParser::parse(&raw).ok());

            // FIX: Only attempt DDC stats when the path is active AND gdi_name is valid.
            // Previously this was correct, but the inner helper swallowed all errors silently.
            // Now errors from the DDC backend surface as None rather than panicking.
            let ddc_stats = if is_active && gdi_name != "Unknown" {
                get_internal_deep_ddc_stats(&gdi_name)
            } else {
                None
            };

            results.push(MonitorDetails {
                target_id,
                friendly_name: String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                    .trim_matches(char::from(0))
                    .to_string(),
                is_active,
                output_tech: format!("{:?}", path.targetInfo.outputTechnology),
                gdi_name,
                device_path,
                topology,
                edid: edid_data,
                ddc_stats,
            });
        }
    }
    Ok(results)
}

/// Internal utility for matching logical screens to structural handles mapping DDC pipelines.
#[cfg(target_os = "windows")]
unsafe fn get_internal_deep_ddc_stats(gdi_name: &str) -> Option<DeepDdcStats> {
    let gdi_name_pcwstr: Vec<u16> = gdi_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut context = (gdi_name_pcwstr, HMONITOR(0));

    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _: HDC,
        _: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let (target_gdi, found_hmon) = &mut *(lparam.0 as *mut (Vec<u16>, HMONITOR));
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmon, &mut mi.monitorInfo).0 != 0 {
            let current_name = String::from_utf16_lossy(&mi.szDevice);
            let target_name = String::from_utf16_lossy(target_gdi);
            if current_name.trim_matches('\0') == target_name.trim_matches('\0') {
                *found_hmon = hmon;
                return BOOL(0); // Terminate enumeration loop upon exact hit
            }
        }
        BOOL(1)
    }

    let _ = EnumDisplayMonitors(
        None,
        None,
        Some(enum_proc),
        LPARAM(&mut context as *mut _ as isize),
    );

    if context.1 .0 != 0 {
        let ddc_backend = crate::edid_backends::edid_win_ddc::WindowsDdcBackend {
            h_monitor: context.1 .0,
        };
        return ddc_backend.query_deep_hardware_stats().ok();
    }
    None
}

/// Collects monitor data on Linux platforms by orchestrating sysfs and ddcutil backends.
#[cfg(target_os = "linux")]
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>, crate::error::EdidError> {
    use crate::edid_backends::edid_linux_ddc::LinuxDdcBackend;
    use std::fs;

    let mut results = Vec::new();
    let drm_path = std::path::Path::new("/sys/class/drm/");

    // FIX: Build a bus-ID map from `ddcutil detect` once upfront rather than
    // blindly passing `bus_id: None` for every connector (which always probes
    // the first bus, potentially returning stats for the wrong monitor).
    let bus_map = LinuxDdcBackend::detect_connector_bus_map();

    if let Ok(entries) = fs::read_dir(drm_path) {
        let mut target_id = 0u32;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();

            // Filter out internal virtual execution nodes (render/control)
            if !name.contains('-') || name.contains("render") || name.contains("control") {
                continue;
            }

            let status_path = entry.path().join("status");
            let is_active = fs::read_to_string(&status_path)
                .map(|s| s.trim() == "connected")
                .unwrap_or(false);

            let sysfs_backend = crate::edid_backends::edid_linux_sys::LinuxSysfsBackend {
                connector_hint: Some(name.clone()),
            };

            let edid_data = sysfs_backend
                .get_edid_raw()
                .ok()
                .and_then(|raw| crate::edid_parser::EdidParser::parse(&raw).ok());

            // FIX: Previously the Linux DDC path fetched EDID (duplicating sysfs work),
            // discarded the result, and filled in hardcoded placeholder brightness values.
            // Now we look up the correct bus ID for this connector and query actual VCP
            // stats via `ddcutil getvcp`. Falls back to None on any error.
            let ddc_stats = if is_active {
                let bus_id = bus_map.get(&name).copied();
                LinuxDdcBackend { bus_id }.query_hardware_stats().ok()
            } else {
                None
            };

            results.push(MonitorDetails {
                target_id,
                friendly_name: edid_data
                    .as_ref()
                    .map(|e| e.model_name.clone())
                    .unwrap_or_else(|| "Generic Display".into()),
                is_active,
                output_tech: name.split('-').nth(1).unwrap_or("Unknown").to_string(),
                gdi_name: name.clone(),
                device_path: entry.path().to_string_lossy().into_owned(),
                topology: None,
                edid: edid_data,
                ddc_stats,
            });
            target_id += 1;
        }
    }

    if results.is_empty() {
        Err(crate::error::EdidError::NotFound)
    } else {
        Ok(results)
    }
}