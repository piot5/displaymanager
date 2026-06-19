//! Windows-specific monitor enumeration using the CCD (Connected and Configurable Display) API,
//! GDI device handles, Registry EDID fallback, and DDC/CI for hardware statistics.

use std::collections::HashSet;
use std::mem::size_of;
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::*;

use super::{MonitorDetails, MonitorEnumerator};
use crate::edid_backends::edid_win_ddc::WindowsDdcBackend;
use crate::edid_backends::edid_win_reg::WindowsRegBackend;
use crate::edid_parser::EdidParser;
use crate::edid_trait::EdidControl;
use crate::edid_types::{DeepDdcStats, MonitorTopology};
use crate::error::EdidError;

/// Windows monitor enumerator using CCD API and GDI.
pub struct WindowsMonitorEnumerator;

impl MonitorEnumerator for WindowsMonitorEnumerator {
    fn collect_monitors(&self) -> Result<Vec<MonitorDetails>, EdidError> {
        enumerate_windows_monitors()
    }
}

/// Collects all monitor data via the CCD, GDI, and Registry backends on Windows targets.
fn enumerate_windows_monitors() -> Result<Vec<MonitorDetails>, EdidError> {
    let mut results = Vec::new();
    let mut seen_targets = HashSet::new();

    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;

        if GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count).0 != 0 {
            return Err(EdidError::CommunicationFailed);
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
        .0 != 0
        {
            return Err(EdidError::CommunicationFailed);
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

            let edid_data = WindowsRegBackend {
                handle: None,
                device_id_override: Some(device_path.clone()),
            }
            .get_edid_raw()
            .ok()
            .and_then(|raw| EdidParser::parse(&raw).ok());

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

/// Find the HMONITOR that matches a GDI device name.
unsafe fn get_internal_deep_ddc_stats(gdi_name: &str) -> Option<DeepDdcStats> {
    let gdi_name_pcwstr: Vec<u16> = gdi_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut context = (gdi_name_pcwstr, HMONITOR(std::ptr::null_mut()));

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
                return BOOL(0); // Terminate enumeration on exact match
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

    if !context.1 .0.is_null() {
        let ddc_backend = WindowsDdcBackend {
            h_monitor: context.1 .0,
        };
        return ddc_backend.query_deep_hardware_stats().ok();
    }
    None
}
