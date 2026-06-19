//! Low-level GDI API functions for display enumeration (EnumDisplayDevicesW,
//! EnumDisplaySettingsW) and mode-setting (ChangeDisplaySettingsExW).
//!
//! # Safety
//!
//! This module uses `unsafe` for Win32 FFI calls. All unsafe blocks are
//! documented with SAFETY comments.

use super::displmgr_gdi_sys::{create_empty_devmode, from_wide, to_wide};
use crate::backends::windows::displmgr_ccd::displmgr_ccd_api::{
    ccd_wake_display, find_display_target,
};
use crate::backends::windows::displmgr_ccd::displmgr_ccd_sys::DISPLAYCONFIG_PATH_ACTIVE;
use crate::error::{DisplayError, DisplayResult};
use crate::types::{
    AdapterId, ConnectorId, DisplayId, DisplayIdentity, DisplayRotation, Extent2D, HdrMode,
    HdrState, OutputState, Point2D, Rect,
};
use std::collections::HashMap;
use windows::core::PCWSTR;
use windows::Win32::Devices::Display::{
    GetDisplayConfigBufferSizes, QueryDisplayConfig, SetDisplayConfig, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, QDC_ALL_PATHS, SDC_APPLY, SDC_SAVE_TO_DATABASE,
    SDC_USE_SUPPLIED_DISPLAY_CONFIG,
};
use windows::Win32::Graphics::Gdi::*;

/// GDI device state flags
const ATTACHED_TO_DESKTOP: u32 = 0x0000_0001;
const PRIMARY_DEVICE: u32 = 0x0000_0004;

/// Default resolution for forced activation of inactive displays
const DEFAULT_WIDTH: u32 = 1920;
const DEFAULT_HEIGHT: u32 = 1080;

/// Maximum number of display devices to enumerate
const MAX_DEVICES: u32 = 64;

/// Minimal representation of a GDI display device during a single scan pass.
/// `dev_string` and `is_primary` are stored for future extension but
/// currently not consumed by downstream callers.
#[derive(Clone)]
#[allow(dead_code)]
struct ScannedDevice {
    /// GDI device name, e.g. `\\.\DISPLAY1`
    gdi_name: String,
    /// Human-readable device string, e.g. "BenQ GL2450H"
    dev_string: String,
    /// Whether the device is currently attached to the desktop
    is_active: bool,
    /// Whether this is the primary display
    is_primary: bool,
}

/// Single-pass enumeration of all GDI display devices.
/// Returns every device found together with its active/inactive status.
fn scan_all_devices() -> Vec<ScannedDevice> {
    let mut devices = Vec::new();
    unsafe {
        for i in 0..MAX_DEVICES {
            let mut device = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() {
                break;
            }
            devices.push(ScannedDevice {
                gdi_name: from_wide(&device.DeviceName),
                dev_string: from_wide(&device.DeviceString),
                is_active: (device.StateFlags & ATTACHED_TO_DESKTOP) != 0,
                is_primary: (device.StateFlags & PRIMARY_DEVICE) != 0,
            });
        }
    }
    devices
}

// ── Public query API ──────────────────────────────────────────────────────

/// Queries all active GDI display devices and returns their current display modes
/// along with a `HashMap` of their raw `DEVMODEW` snapshots for later re-use during
/// commit cycles.
pub fn query_gdi_outputs() -> DisplayResult<(HashMap<String, DEVMODEW>, Vec<OutputState>)> {
    let mut staged_modes = HashMap::new();
    let mut outputs = Vec::new();

    unsafe {
        for i in 0..MAX_DEVICES {
            let mut device = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };

            if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() {
                break;
            }

            let name_str = from_wide(&device.DeviceName);
            let mut dev_mode = create_empty_devmode();
            let wide_name = to_wide(&name_str);

            let success = EnumDisplaySettingsW(
                PCWSTR(wide_name.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
            .as_bool();

            if !success {
                continue;
            }

            let (orient, pos_x, pos_y) = (
                dev_mode.Anonymous1.Anonymous2.dmDisplayOrientation,
                dev_mode.Anonymous1.Anonymous2.dmPosition.x,
                dev_mode.Anonymous1.Anonymous2.dmPosition.y,
            );

            let size = Extent2D {
                width: dev_mode.dmPelsWidth,
                height: dev_mode.dmPelsHeight,
            };

            outputs.push(OutputState {
                identity: DisplayIdentity {
                    id: DisplayId(name_str.clone()),
                    connector_id: ConnectorId(name_str.clone()),
                    adapter_id: AdapterId(from_wide(&device.DeviceID)),
                    hardware_uuid: None,
                    monitor_name: from_wide(&device.DeviceString),
                },
                geometry: Rect {
                    origin: Point2D { x: pos_x, y: pos_y },
                    size,
                },
                refresh_rate: dev_mode.dmDisplayFrequency * 1000,
                rotation: match orient {
                    DMDO_90 => DisplayRotation::Rotate90,
                    DMDO_180 => DisplayRotation::Rotate180,
                    DMDO_270 => DisplayRotation::Rotate270,
                    _ => DisplayRotation::Rotate0,
                },
                enabled: (device.StateFlags & ATTACHED_TO_DESKTOP) != 0,
                is_primary: (device.StateFlags & PRIMARY_DEVICE) != 0,
                hdr_state: HdrState::Disabled,
                hdr_mode: HdrMode::Default,
                scale: 1.0,
                native_resolution: Some(size),
                supported_modes: Vec::new(),
            });

            staged_modes.insert(name_str, dev_mode);
        }
    }

    Ok((staged_modes, outputs))
}

// ── Helper: apply resolution change to a single GDI device ────────────────

/// Applies a resolution change to a single GDI device by name.
/// Uses `ENUM_REGISTRY_SETTINGS` to read the base mode, then overwrites
/// width/height and writes via `ChangeDisplaySettingsExW`.
unsafe fn apply_resolution(gdi_name: &str, width: u32, height: u32) {
    let wide_name = to_wide(gdi_name);
    let mut dm = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    if EnumDisplaySettingsW(PCWSTR(wide_name.as_ptr()), ENUM_REGISTRY_SETTINGS, &mut dm).as_bool() {
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
        let _ = ChangeDisplaySettingsExW(
            PCWSTR(wide_name.as_ptr()),
            Some(&dm),
            None,
            CDS_UPDATEREGISTRY | CDS_NORESET,
            None,
        );
    }
}

/// Flushes all pending display setting changes by calling
/// `ChangeDisplaySettingsExW` with a null device name and `CDS_TYPE(0)`.
unsafe fn flush_display_settings() {
    let _ = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None);
}

// ── Force-all activation ──────────────────────────────────────────────────

/// Activates all disconnected GDI displays with a default 1920×1080 resolution.
///
/// Single-pass scan: enumerates devices once, then applies changes.
pub fn force_all() -> DisplayResult<()> {
    let devices = scan_all_devices();

    for dev in &devices {
        if !dev.is_active {
            unsafe {
                apply_resolution(&dev.gdi_name, DEFAULT_WIDTH, DEFAULT_HEIGHT);
            }
        }
    }

    unsafe {
        flush_display_settings();
    }
    Ok(())
}

// ── Selective activation by monitor name ──────────────────────────────────

/// Activates a specific display by its monitor friendly name (e.g. `"DTV"`)
/// while keeping other previously inactive displays off.
///
/// Uses the shared CCD API (`find_display_target`, `ccd_wake_display`) to:
/// 1. Find the target display by its friendly name across all CCD paths
/// 2. Activate only that target via the low-level CCD wake mechanism
/// 3. Then use GDI to set the resolution
///
/// This avoids duplicating the CCD query logic that lives in `displmgr_ccd_api.rs`.
pub fn force_activate_by_monitor_name(monitor_name: &str) -> DisplayResult<()> {
    // Step 1: Find the target using the shared CCD API
    let target = find_display_target(monitor_name).ok_or_else(|| {
        DisplayError::BackendError(format!("Monitor '{}' not found in CCD paths", monitor_name))
    })?;

    let target_id = target.target_id;

    // Step 2: Wake via CCD if inactive, or reposition if already active
    if target.is_active {
        // Already active — reposition via GDI to avoid CLONED mode
        unsafe {
            apply_resolution_for_target(target_id, DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }
    } else {
        // Perform CCD-level wake
        ccd_wake_display(target_id)?;
    }

    // Step 3: Flush GDI to apply
    unsafe {
        flush_display_settings();
    }
    Ok(())
}

/// Deactivates a specific display by its monitor friendly name using CCD.
/// Delegates to the shared CCD API rather than duplicating query logic.
pub fn force_deactivate_by_monitor_name(monitor_name: &str) -> DisplayResult<()> {
    // Find target via shared CCD API
    let target = find_display_target(monitor_name).ok_or_else(|| {
        DisplayError::BackendError(format!("Monitor '{}' not found in CCD paths", monitor_name))
    })?;

    if !target.is_active {
        return Ok(()); // Already inactive
    }

    // Query all CCD paths, find the target, clear its ACTIVE flag
    unsafe {
        let (mut paths, modes) = query_ccd_paths()?;
        let idx = paths
            .iter()
            .position(|p| p.targetInfo.id == target.target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(target.target_id.to_string())))?;

        // Clear active flag and mode indices
        paths[idx].flags &= !DISPLAYCONFIG_PATH_ACTIVE;
        paths[idx].sourceInfo.Anonymous.modeInfoIdx = 0xFFFFFFFF;
        paths[idx].targetInfo.Anonymous.modeInfoIdx = 0xFFFFFFFF;

        let flags = SDC_APPLY | SDC_SAVE_TO_DATABASE;
        let st = SetDisplayConfig(Some(&paths), Some(&modes), flags);
        if st != 0 {
            return Err(DisplayError::BackendError(format!(
                "SetDisplayConfig deactivate failed for target {}: 0x{:08X}",
                target.target_id, st as u32
            )));
        }
    }
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────

/// Queries all CCD paths (QDC_ALL_PATHS) and returns the raw buffers.
/// Consolidates the GetDisplayConfigBufferSizes + QueryDisplayConfig
/// pattern that was duplicated across multiple functions.
unsafe fn query_ccd_paths(
) -> DisplayResult<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>)> {
    let mut path_count = 0u32;
    let mut mode_count = 0u32;

    let result = GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count);
    if result.0 != 0 {
        return Err(DisplayError::BackendError(format!(
            "GetDisplayConfigBufferSizes failed: {}",
            result.0
        )));
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    let result = QueryDisplayConfig(
        QDC_ALL_PATHS,
        &mut path_count,
        paths.as_mut_ptr(),
        &mut mode_count,
        modes.as_mut_ptr(),
        None,
    );
    if result.0 != 0 {
        return Err(DisplayError::BackendError(format!(
            "QueryDisplayConfig failed: {}",
            result.0
        )));
    }

    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);
    Ok((paths, modes))
}

/// Applies resolution to a target display by finding it via CCD name lookup.
/// Used by `force_activate_by_monitor_name` for already-active display repositioning.
unsafe fn apply_resolution_for_target(target_id: u32, width: u32, _height: u32) {
    #[allow(unused_mut)]
    let (mut paths, mut modes) = match query_ccd_paths() {
        Ok(p) => p,
        Err(_) => return,
    };

    let idx = match paths.iter().position(|p| p.targetInfo.id == target_id) {
        Some(i) => i,
        None => return,
    };

    // Find rightmost X among other active paths
    let mut rightmost_x: i32 = 0;
    for (i2, p) in paths.iter().enumerate() {
        if i2 == idx || (p.flags & DISPLAYCONFIG_PATH_ACTIVE) == 0 {
            continue;
        }
        let si = p.sourceInfo.Anonymous.modeInfoIdx as usize;
        if si < modes.len() {
            let sm = &modes[si];
            let x = sm.Anonymous.sourceMode.position.x;
            let w = sm.Anonymous.sourceMode.width as i32;
            let x2 = x + w;
            if x2 > rightmost_x {
                rightmost_x = x2;
            }
        }
    }
    if rightmost_x == 0 {
        rightmost_x = width as i32;
    }

    let src_idx = paths[idx].sourceInfo.Anonymous.modeInfoIdx as usize;
    if src_idx < modes.len() {
        modes[src_idx].Anonymous.sourceMode.position.x = rightmost_x;
        modes[src_idx].Anonymous.sourceMode.position.y = 0;
    }

    let _ = SetDisplayConfig(
        Some(&paths),
        Some(&modes),
        SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_SAVE_TO_DATABASE,
    );
}
