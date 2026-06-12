use windows::Win32::Devices::Display as WinDisplay;
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME,
    QDC_ALL_PATHS,
    SDC_APPLY,
    SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SDC_SAVE_TO_DATABASE,
};
use crate::error::{DisplayError, DisplayResult};
use crate::types::{OutputState, DisplayRotation, HdrState, HdrMode, DisplayId, ConnectorId, AdapterId, DisplayIdentity, Rect, Point2D, Extent2D};
use super::displmgr_ccd_sys::{
    DISPLAYCONFIG_PATH_ACTIVE,
    DISPLAYCONFIG_PATH_MODE_IDX_INVALID,
    DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_MODE_INFO_TYPE_TARGET,
    CcdRawData,
};

/// Information about a display target found via CCD QDC_ALL_PATHS query.
#[derive(Debug, Clone)]
pub struct DisplayTargetInfo {
    /// The friendly monitor name (e.g., "DTV", "HDMI-1").
    pub friendly_name: String,
    /// The CCD target identifier.
    pub target_id: u32,
    /// Whether the display path is currently active.
    pub is_active: bool,
    /// The adapter LUID for this target.
    pub adapter_id: windows::Win32::Foundation::LUID,
}

/// Queries all CCD paths (including inactive) and returns a list of display targets.
/// This uses QDC_ALL_PATHS to find displays that may be physically connected but not active.
pub fn query_all_display_targets() -> DisplayResult<Vec<DisplayTargetInfo>> {
    let mut path_count = 0u32;
    let mut mode_count = 0u32;

    unsafe {
        WinDisplay::GetDisplayConfigBufferSizes(
            QDC_ALL_PATHS,
            &mut path_count,
            &mut mode_count,
        )
    }
    .map_err(|e| DisplayError::BackendError(format!("GetDisplayConfigBufferSizes failed: {}", e)))?;

    let mut paths = vec![WinDisplay::DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![WinDisplay::DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    unsafe {
        WinDisplay::QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    }
    .map_err(|e| DisplayError::BackendError(format!("QueryDisplayConfig failed: {}", e)))?;

    paths.truncate(path_count as usize);

    let mut targets = Vec::new();
    for path in &paths {
        let target_id = path.targetInfo.id;
        let adapter_id = path.targetInfo.adapterId;
        let is_active = (path.flags & DISPLAYCONFIG_PATH_ACTIVE) != 0;
        let friendly_name = get_target_name(adapter_id, target_id);

        targets.push(DisplayTargetInfo {
            friendly_name,
            target_id,
            is_active,
            adapter_id,
        });
    }

    Ok(targets)
}

/// Searches for a display target by name (case-insensitive substring match) across all
/// CCD paths including inactive ones. Returns the first match.
pub fn find_display_target(query: &str) -> Option<DisplayTargetInfo> {
    let qq = query.to_lowercase();
    let targets = query_all_display_targets().ok()?;
    targets.into_iter().find(|t| t.friendly_name.to_lowercase().contains(&qq))
}

/// Performs a CCD-level wake of an inactive display by setting the ACTIVE flag on its path.
/// Returns Ok(true) if the wake was performed, Ok(false) if already active, or Err on failure.
///
/// This is the low-level CCD mechanism: it finds the path for the given target_id in the
/// QDC_ALL_PATHS result, sets the ACTIVE flag, and calls SetDisplayConfig.
pub fn ccd_wake_display(target_id: u32) -> DisplayResult<bool> {
    unsafe {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;

        WinDisplay::GetDisplayConfigBufferSizes(
            QDC_ALL_PATHS,
            &mut path_count,
            &mut mode_count,
        )
        .map_err(|e| DisplayError::BackendError(format!("GetDisplayConfigBufferSizes failed: {}", e)))?;

        let mut paths = vec![WinDisplay::DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![WinDisplay::DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        WinDisplay::QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
        .map_err(|e| DisplayError::BackendError(format!("QueryDisplayConfig failed: {}", e)))?;

        paths.truncate(path_count as usize);

        // Find the path with the matching target_id
        let idx = paths.iter().position(|p| p.targetInfo.id == target_id)
            .ok_or_else(|| DisplayError::NotFound(DisplayId(target_id.to_string())))?;

        // Check if already active
        if (paths[idx].flags & DISPLAYCONFIG_PATH_ACTIVE) != 0 {
            return Ok(false);
        }

        // Verify the path has a valid source mode index (required for wake)
        let src_idx = paths[idx].sourceInfo.Anonymous.modeInfoIdx;
        if src_idx == DISPLAYCONFIG_PATH_MODE_IDX_INVALID {
            return Err(DisplayError::BackendError(
                format!("Target {} has no valid source mode index, cannot wake", target_id)
            ));
        }

        // Set the ACTIVE flag
        paths[idx].flags |= DISPLAYCONFIG_PATH_ACTIVE;

        // Apply the configuration
        let status = WinDisplay::SetDisplayConfig(
            Some(&paths),
            Some(&modes),
            SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_SAVE_TO_DATABASE,
        );

        if status != 0 {
            return Err(DisplayError::BackendError(
                format!("SetDisplayConfig wake failed for target {}: 0x{:08X}", target_id, status as u32)
            ));
        }

        Ok(true)
    }
}

/// Maps Win32 error codes to DisplayResult for consistent error handling across the CCD subsystem.
pub fn map_win32_code(result: windows::core::Result<()>) -> DisplayResult<()> {
    result.map_err(|e| DisplayError::BackendError(format!("Win32 Error: {}", e)))
}

/// Retrieves the friendly name of a display target from the Windows CCD subsystem.
/// Falls back to a synthesized ID if the monitor name cannot be queried.
pub fn get_target_name(
    adapter_id: windows::Win32::Foundation::LUID,
    target_id: u32,
) -> String {
    let mut payload = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: WinDisplay::DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
            adapterId: adapter_id,
            id: target_id,
        },
        ..Default::default()
    };

    let status = unsafe {
        WinDisplay::DisplayConfigGetDeviceInfo(
            &mut payload.header as *mut _ as *mut _,
        )
    };

    // Extract the friendly name if available
    if status == 0 && payload.monitorFriendlyDeviceName[0] != 0 {
        let len = payload
            .monitorFriendlyDeviceName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(64);
        String::from_utf16_lossy(&payload.monitorFriendlyDeviceName[..len]).to_string()
    } else {
        // Fallback to synthesized identifier using adapter and target IDs
        format!("Display_{:#010X}_{}", adapter_id.LowPart, target_id)
    }
}

/// Synchronously queries the current CCD configuration from the Windows display subsystem.
/// Returns both the raw CCD structures and derived OutputState snapshots.
pub fn query_config_sync() -> DisplayResult<(CcdRawData, Vec<OutputState>)> {
    let mut path_count = 0u32;
    let mut mode_count = 0u32;

    // Retrieve buffer sizes required for CCD structures
    unsafe {
        WinDisplay::GetDisplayConfigBufferSizes(
            WinDisplay::QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            &mut mode_count,
        )
    }
    .map_err(|e| DisplayError::BackendError(format!("GetDisplayConfigBufferSizes failed: {}", e)))?;

    // Allocate and populate path and mode buffers
    let mut paths = vec![WinDisplay::DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![WinDisplay::DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    unsafe {
        WinDisplay::QueryDisplayConfig(
            WinDisplay::QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    }
    .map_err(|e| DisplayError::BackendError(format!("QueryDisplayConfig failed: {}", e)))?;

    // Trim buffers to actual counts
    paths.truncate(path_count as usize);
    modes.truncate(mode_count as usize);

    let raw = CcdRawData {
        paths: paths.clone(),
        modes: modes.clone(),
    };

    // Map each path to an OutputState snapshot
    let mut outputs = paths
        .iter()
        .map(|p| map_path_to_output(p, &modes))
        .collect::<Vec<_>>();

    // Second pass: ensure exactly one primary monitor is marked
    finalize_primary_detection(&mut outputs);

    Ok((raw, outputs))
}

/// Converts a single CCD path and associated mode information into an OutputState snapshot.
/// Extracts position, resolution, refresh rate, and rotation from the CCD structures.
fn map_path_to_output(
    path: &WinDisplay::DISPLAYCONFIG_PATH_INFO,
    modes: &[WinDisplay::DISPLAYCONFIG_MODE_INFO],
) -> OutputState {
    let mut state = OutputState::default();

    let target_id = path.targetInfo.id;
    let adapter_id = path.targetInfo.adapterId;

    // Build the display identity
    state.identity = DisplayIdentity {
        id: DisplayId(target_id.to_string()),
        connector_id: ConnectorId(format!("{:#010X}", target_id)),
        adapter_id: AdapterId(format!("{:#010X}", adapter_id.LowPart)),
        hardware_uuid: None, // CCD does not expose EDID UUIDs directly
        monitor_name: get_target_name(adapter_id, target_id),
    };

    // Check if the path is active
    state.enabled = (path.flags & DISPLAYCONFIG_PATH_ACTIVE) != 0;

    // Extract source mode (position and resolution)
    let source_idx = unsafe { path.sourceInfo.Anonymous.modeInfoIdx };
    if source_idx != DISPLAYCONFIG_PATH_MODE_IDX_INVALID && (source_idx as usize) < modes.len() {
        let mode = &modes[source_idx as usize];
        if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
            unsafe {
                state.geometry = Rect {
                    origin: Point2D {
                        x: mode.Anonymous.sourceMode.position.x,
                        y: mode.Anonymous.sourceMode.position.y,
                    },
                    size: Extent2D {
                        width: mode.Anonymous.sourceMode.width,
                        height: mode.Anonymous.sourceMode.height,
                    },
                };
            }
        }
    }

    // Extract target mode (refresh rate and signal information)
    let target_idx = unsafe { path.targetInfo.Anonymous.modeInfoIdx };
    if target_idx != DISPLAYCONFIG_PATH_MODE_IDX_INVALID && (target_idx as usize) < modes.len() {
        let mode = &modes[target_idx as usize];
        if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_TARGET {
            unsafe {
                let v = mode.Anonymous.targetMode.targetVideoSignalInfo.vSyncFreq;
                if v.Denominator != 0 {
                    // Convert from fraction (Hz) to millihertz
                    state.refresh_rate = ((v.Numerator as u64 * 1000) / v.Denominator as u64) as u32;
                }
            }
        }
    }

    // Extract rotation from the target info
    state.rotation = match path.targetInfo.rotation {
        WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE90  => DisplayRotation::Rotate90,
        WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE180 => DisplayRotation::Rotate180,
        WinDisplay::DISPLAYCONFIG_ROTATION_ROTATE270 => DisplayRotation::Rotate270,
        _ => DisplayRotation::Rotate0,
    };

    // Set HDR and scaling defaults
    state.hdr_state = HdrState::Disabled;
    state.hdr_mode = HdrMode::Default;
    state.scale = 1.0;
    state.native_resolution = Some(state.geometry.size);

    // Detect primary monitor: typically the one at desktop origin (0,0)
    // This will be corrected in a second pass after all outputs are mapped
    state.is_primary = state.geometry.origin.x == 0 && state.geometry.origin.y == 0;

    state
}

/// Second pass to ensure exactly one primary monitor is marked.
/// If multiple monitors are at (0,0), the first one wins.
/// If none are at (0,0), the first enabled monitor becomes primary.
pub fn finalize_primary_detection(outputs: &mut [OutputState]) {
    let mut primary_found = false;
    for out in outputs.iter_mut() {
        if out.is_primary {
            if primary_found {
                out.is_primary = false; // Only one primary
            } else {
                primary_found = true;
            }
        }
    }
    if !primary_found {
        // Fallback: first enabled monitor becomes primary
        if let Some(first) = outputs.iter_mut().find(|o| o.enabled) {
            first.is_primary = true;
        }
    }
}
