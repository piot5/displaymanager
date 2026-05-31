use crate::edid_trait::EdidControl;
use crate::error::EdidError;
use windows::Win32::Devices::Display::*;
use std::mem::size_of;

/// Backend implementing EDID verification and extraction using the modern Windows CCD API.
pub struct WindowsCcdBackend {
    pub device_path: Option<String>,
}

impl EdidControl for WindowsCcdBackend {
    /// Validates the device target path against active hardware paths inside the CCD topology database.
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError> {
        let target_path = self.device_path.as_ref().ok_or(EdidError::NotFound)?;

        unsafe {
            let mut path_count = 0;
            let mut mode_count = 0;

            // 1. Fetch memory layout metrics required for active configuration data buffers
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count).is_err() {
                return Err(EdidError::CommunicationFailed);
            }

            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

            // 2. Query target operational active hardware topology maps
            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            ).is_err() {
                return Err(EdidError::CommunicationFailed);
            }

            // 3. Match device topology properties to isolate correct target identifiers
            for path in paths.iter().take(path_count as usize) {
                let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                target_name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                target_name.header.adapterId = path.targetInfo.adapterId;
                target_name.header.id = path.targetInfo.id;

                // Query deeper hardware metadata fields
                if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                    let current_path = String::from_utf16_lossy(&target_name.monitorDevicePath)
                        .trim_matches(char::from(0))
                        .to_string();

                    // If matches target path, delegate payload extraction to the registry backend sub-pipeline
                    if current_path == *target_path {
                        let reg_backend = crate::edid_backends::edid_win_reg::WindowsRegBackend {
                            handle: None,
                            device_id_override: Some(current_path),
                        };
                        return reg_backend.get_edid_raw();
                    }
                }
            }
        }

        Err(EdidError::NotFound)
    }
}