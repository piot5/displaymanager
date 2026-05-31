use windows::Win32::Devices::Display::{
    GetDisplayConfigBufferSizes, QueryDisplayConfig, SetDisplayConfig,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_MODE_INFO,
    QDC_ALL_PATHS, SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
};
use std::collections::HashSet;

// Constants for display configuration flags
const DISPLAYCONFIG_PATH_ACTIVE: u32 = 0x00000001;
const DISPLAYCONFIG_PATH_PRIMARY: u32 = 0x00000004;
const DISPLAYCONFIG_PATH_MODE_IDX_VALID: u32 = 0x00000002;
const MODE_IDX_INVALID: u32 = 0xFFFFFFFF;

fn main() -> Result<(), String> {
    println!("Scanning and enforcing display topology...");

    let mut num_path_elements = 0;
    let mut num_mode_elements = 0;

    // 1. Get current display topology buffer sizes
    unsafe {
        GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut num_path_elements, &mut num_mode_elements)
            .map_err(|e| format!("Buffer size error: {:?}", e))?;
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); num_path_elements as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); num_mode_elements as usize];

    // 2. Query the current display configuration from Windows
    unsafe {
        QueryDisplayConfig(QDC_ALL_PATHS, &mut num_path_elements, paths.as_mut_ptr(), 
                           &mut num_mode_elements, modes.as_mut_ptr(), None)
            .map_err(|e| format!("Query error: {:?}", e))?;
    }

    let target_ids_to_enable = [4352, 4356];
    let mut processed_targets = HashSet::new();
    let mut changed = false;

    // 3. Configure paths and enforce uniqueness to prevent ERROR 87
    for path in paths.iter_mut() {
        if target_ids_to_enable.contains(&path.targetInfo.id) {
            
            // Ensure each target ID is processed exactly once
            if !processed_targets.contains(&path.targetInfo.id) {
                path.flags |= DISPLAYCONFIG_PATH_ACTIVE | DISPLAYCONFIG_PATH_MODE_IDX_VALID;

                // Mark the first processed target as primary
                if processed_targets.is_empty() {
                    path.flags |= DISPLAYCONFIG_PATH_PRIMARY;
                }

                // Invalidate mode indices so the driver performs auto-negotiation
                // Accessing Anonymous fields is safe in modern windows-rs versions
                path.sourceInfo.Anonymous.modeInfoIdx = MODE_IDX_INVALID;
                path.targetInfo.Anonymous.modeInfoIdx = MODE_IDX_INVALID;

                processed_targets.insert(path.targetInfo.id);
                changed = true;
                println!("Successfully queued Target ID: {} for activation.", path.targetInfo.id);
            }
        }
    }

    // 4. Commit changes to the Windows Kernel
    if changed {
        unsafe {
            // SDC_USE_SUPPLIED_DISPLAY_CONFIG tells Windows to use the configuration provided in 'paths'
            // Passing 'None' for modes prevents parameter mismatch errors (ERROR 87)
            let result = SetDisplayConfig(
                Some(&paths), 
                None, 
                SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG
            );
            
            if result != 0 {
                return Err(format!("SetDisplayConfig failed with Win32 error code: {}. 
                    Ensure IDs are valid and not in a source conflict.", result));
            }
        }
        println!("Topology applied successfully.");
    } else {
        println!("No targets required activation.");
    }

    Ok(())
}