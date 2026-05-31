use windows::core::Result;
use windows::Win32::Devices::Display::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use std::collections::HashSet;
use std::mem::size_of;
use serde::Serialize;

use df_displmgr_info::edid_backends::edid_win_reg::WindowsRegBackend;
use df_displmgr_info::edid_trait::EdidControl;
use df_displmgr_info::edid_parser::EdidParser;
pub use df_displmgr_info::edid_types::EdidData;

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

/// Collects comprehensive monitor data by bridging CCD API and GDI handles.
pub fn collect_monitor_data() -> Result<Vec<MonitorDetails>> {
    let mut results = Vec::new();
    let mut seen_targets = HashSet::new();

    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;

        // 1. Determine buffer sizes
        let _ = GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count);

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        // 2. Query current configuration
        let _ = QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        );

        // Sorting: Active paths first
        paths.sort_by_key(|p| (p.flags & DISPLAYCONFIG_PATH_ACTIVE) == 0);

        for path in paths.iter().take(path_count as usize) {
            let target_id = path.targetInfo.id;
            let adapter_luid = path.targetInfo.adapterId;

            // Resolve hardware identity
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

            // Resolve GDI Name (Logical Desktop Source)
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
                String::new()
            };

            // 3. Topology (Coordinates and Resolution)
            let (mut x, mut y, mut width, mut height) = (0, 0, 0, 0);
            let is_active = (path.flags & DISPLAYCONFIG_PATH_ACTIVE) != 0;

            if is_active {
                let mode_idx = path.sourceInfo.Anonymous.modeInfoIdx;
                if mode_idx != DISPLAYCONFIG_PATH_MODE_IDX_INVALID && (mode_idx as usize) < modes.len() {
                    let mode = &modes[mode_idx as usize];
                    if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                        let source_mode = mode.Anonymous.sourceMode;
                        x = source_mode.position.x;
                        y = source_mode.position.y;
                        width = source_mode.width;
                        height = source_mode.height;
                    }
                }
            }

            // EDID via Registry Backend
            let edid_data = WindowsRegBackend {
                handle: None,
                device_id_override: Some(device_path.clone()),
            }
            .get_edid_raw()
            .ok()
            .and_then(|raw| EdidParser::parse(&raw).ok());

            // DDC/CI Hardware Stats
            let ddc_info = if is_active && !gdi_name.is_empty() {
                get_ddc_info(&gdi_name)
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
                native_path: device_path.clone(),
                device_path,
                x,
                y,
                width,
                height,
                rotation: format!("{:?}", path.targetInfo.rotation),
                edid: edid_data,
                ddc_stats: ddc_info,
            });
        }
    }
    Ok(results)
}

/// Uses HMONITOR enumeration to access the physical monitor bus (DDC/CI).
unsafe fn get_ddc_info(gdi_name: &str) -> Option<DdcData> {
    let gdi_name_u16: Vec<u16> = gdi_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_monitor_found = HMONITOR(0);

    unsafe extern "system" fn enum_proc(hmon: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let (target_gdi_u16, found_hmon) = &mut *(lparam.0 as *mut (Vec<u16>, HMONITOR));
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(hmon, &mut mi.monitorInfo).as_bool() {
            let current_device = String::from_utf16_lossy(&mi.szDevice);
            let target_device = String::from_utf16_lossy(target_gdi_u16);

            if current_device.trim_matches('\0') == target_device.trim_matches('\0') {
                *found_hmon = hmon;
                return BOOL(0);
            }
        }
        BOOL(1)
    }

    let mut sync_context = (gdi_name_u16, h_monitor_found);
    let _ = EnumDisplayMonitors(
        None,
        None,
        Some(enum_proc),
        LPARAM(&mut sync_context as *mut _ as isize),
    );

    h_monitor_found = sync_context.1;

    if h_monitor_found.0 != 0 {
        let mut physical_count = 0;
        if GetNumberOfPhysicalMonitorsFromHMONITOR(h_monitor_found, &mut physical_count).is_ok() {
            let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); physical_count as usize];
            if GetPhysicalMonitorsFromHMONITOR(h_monitor_found, &mut physical_monitors).is_ok() {
                let phys = physical_monitors[0];
                let (mut b_min, mut b_curr, mut b_max) = (0, 0, 0);

                let ddc_res = if GetMonitorBrightness(phys.hPhysicalMonitor, &mut b_min, &mut b_curr, &mut b_max) != 0 {
                    let (mut c_min, mut c_curr, mut c_max) = (0, 0, 0);
                    let contrast = if GetMonitorContrast(phys.hPhysicalMonitor, &mut c_min, &mut c_curr, &mut c_max) != 0 {
                        Some((c_min, c_curr, c_max))
                    } else {
                        None
                    };
                    Some(DdcData {
                        brightness: (b_min, b_curr, b_max),
                        contrast,
                    })
                } else {
                    None
                };

                let _ = DestroyPhysicalMonitors(&physical_monitors);
                return ddc_res;
            }
        }
    }
    None
}