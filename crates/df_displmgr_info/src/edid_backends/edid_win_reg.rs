use crate::edid_trait::EdidControl;
use crate::error::EdidError;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITORINFOEXW, HMONITOR, EnumDisplayDevicesW, DISPLAY_DEVICEW,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_READ, RegEnumKeyExW, REG_BINARY, REG_VALUE_TYPE,
};

pub struct WindowsRegBackend {
    pub handle: Option<isize>,
    pub device_id_override: Option<String>,
}

impl EdidControl for WindowsRegBackend {
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError> {
        // Resolve Device ID from override property or HMONITOR handle
        let device_id = if let Some(ref id) = self.device_id_override {
            id.clone()
        } else if let Some(h) = self.handle {
            self.get_device_id_from_handle(h)?
        } else {
            return Err(EdidError::NotFound);
        };

        // Normalize device path: strip kernel NT device prefix (\\?\ or \\.\) and uppercase,
        // then split on '#' or '\' to extract hardware ID and instance ID.
        // The Registry API exposes device paths like:
        //   \\?\DISPLAY#DELA123#5&1a2b3c4d&0&UID1#{<GUID>}
        let clean_id = device_id
            .trim_start_matches(r"\\.\")
            .trim_start_matches(r"\\?\")
            .to_uppercase();
        let parts: Vec<&str> = clean_id.split(|c| c == '#' || c == '\\').collect();
        if parts.len() < 3 {
            return Err(EdidError::NotFound);
        }

        let hw_id = parts[1];
        let inst_id = parts[2];

        // Strategy A: Direct path resolution using hw_id + inst_id
        let target_path = format!(
            r"SYSTEM\CurrentControlSet\Enum\DISPLAY\{}\{}\Device Parameters",
            hw_id, inst_id
        );
        if let Ok(data) = self.read_registry(&target_path) {
            return Ok(data);
        }

        // FIX: Strategy B previously hardcoded index 0000, meaning it only ever probed
        // the first Class GUID entry and silently failed for every other monitor.
        // Now we enumerate all indices under the Monitor Class GUID key until we find
        // a Device Parameters subkey that contains a valid EDID value.
        let class_guid_path =
            r"SYSTEM\CurrentControlSet\Control\Class\{4d36e96e-e325-11ce-bfc1-08002be10318}";
        if let Ok(data) = self.scan_class_guid_for_edid(class_guid_path, hw_id) {
            return Ok(data);
        }

        // Strategy C: Enumeration scan through DISPLAY root
        let base_path = r"SYSTEM\CurrentControlSet\Enum\DISPLAY";
        let mut hkey_base = HKEY::default();
        let sub_system_path = windows::core::HSTRING::from(base_path);

        if unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, &sub_system_path, 0, KEY_READ, &mut hkey_base)
        }
        .is_ok()
        {
            let mut index = 0;
            let mut name_buffer = vec![0u16; 256];

            loop {
                let mut name_len = name_buffer.len() as u32;

                if unsafe {
                    RegEnumKeyExW(
                        hkey_base,
                        index,
                        PWSTR(name_buffer.as_mut_ptr()),
                        &mut name_len,
                        None,
                        PWSTR(std::ptr::null_mut()),
                        None,
                        None,
                    )
                }
                .is_err()
                {
                    break;
                }

                let current_hw_id =
                    String::from_utf16_lossy(&name_buffer[0..name_len as usize]).to_uppercase();

                if current_hw_id.contains(hw_id) || hw_id.contains(&current_hw_id) {
                    // FIX: Previously scan_all_instances_for_edid returned the first EDID it
                    // found under a matching hw_id, regardless of whether the instance ID
                    // matched. With two identical monitors connected this returned the wrong
                    // unit's EDID. Now we pass inst_id and prefer an exact instance match,
                    // only falling back to the first available instance when inst_id is absent
                    // from the registry (e.g. after a driver reinstall changes the suffix).
                    let node_path = format!(r"{}\{}", base_path, current_hw_id);
                    if let Ok(data) =
                        self.scan_instances_for_edid(&node_path, inst_id)
                    {
                        let _ = unsafe { RegCloseKey(hkey_base) };
                        return Ok(data);
                    }
                }
                index += 1;
            }
            let _ = unsafe { RegCloseKey(hkey_base) };
        }

        Err(EdidError::NotFound)
    }
}

impl WindowsRegBackend {
    /// Enumerates all numeric indices under the Monitor Class GUID key and reads the
    /// first Device Parameters\EDID value whose parent key contains a matching hw_id
    /// in its DeviceInstance or MatchingDeviceId value.
    ///
    /// This replaces the original Strategy B which was hardcoded to index 0000.
    fn scan_class_guid_for_edid(
        &self,
        class_guid_path: &str,
        hw_id: &str,
    ) -> Result<Vec<u8>, EdidError> {
        let mut hkey_class = HKEY::default();
        let class_hstring = windows::core::HSTRING::from(class_guid_path);

        if unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, &class_hstring, 0, KEY_READ, &mut hkey_class)
        }
        .is_err()
        {
            return Err(EdidError::NotFound);
        }

        let mut index = 0u32;
        let mut name_buffer = vec![0u16; 256];

        loop {
            let mut name_len = name_buffer.len() as u32;

            if unsafe {
                RegEnumKeyExW(
                    hkey_class,
                    index,
                    PWSTR(name_buffer.as_mut_ptr()),
                    &mut name_len,
                    None,
                    PWSTR(std::ptr::null_mut()),
                    None,
                    None,
                )
            }
            .is_err()
            {
                break;
            }

            let entry_name =
                String::from_utf16_lossy(&name_buffer[0..name_len as usize]);
            let sub_path = format!(r"{}\{}", class_guid_path, entry_name);

            // Check whether this class entry belongs to our hw_id by reading
            // the MatchingDeviceId value stored next to Device Parameters
            if self.class_entry_matches_hw_id(&sub_path, hw_id) {
                let param_path = format!(r"{}\Device Parameters", sub_path);
                if let Ok(data) = self.read_registry(&param_path) {
                    let _ = unsafe { RegCloseKey(hkey_class) };
                    return Ok(data);
                }
            }

            index += 1;
        }

        let _ = unsafe { RegCloseKey(hkey_class) };
        Err(EdidError::NotFound)
    }

    /// Reads the MatchingDeviceId string value from a Class GUID subkey and checks
    /// whether it contains the target hardware ID.
    fn class_entry_matches_hw_id(&self, sub_path: &str, hw_id: &str) -> bool {
        let mut hkey = HKEY::default();
        let path_hstring = windows::core::HSTRING::from(sub_path);

        if unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, &path_hstring, 0, KEY_READ, &mut hkey)
        }
        .is_err()
        {
            return false;
        }

        let value_name = windows::core::HSTRING::from("MatchingDeviceId");
        let mut value_type = REG_BINARY;
        let mut data_len = 0u32;

        let matches = if unsafe {
            RegQueryValueExW(
                hkey,
                &value_name,
                None,
                Some(&mut value_type as *mut REG_VALUE_TYPE),
                None,
                Some(&mut data_len),
            )
        }
        .is_ok()
        {
            let mut buf = vec![0u8; data_len as usize];
            if unsafe {
                RegQueryValueExW(hkey, &value_name, None, None, Some(buf.as_mut_ptr()), Some(&mut data_len))
            }
            .is_ok()
            {
                // MatchingDeviceId is a REG_SZ (UTF-16)
                let wide: Vec<u16> = buf
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&wide)
                    .to_uppercase()
                    .contains(hw_id)
            } else {
                false
            }
        } else {
            false
        };

        let _ = unsafe { RegCloseKey(hkey) };
        matches
    }

    /// Iterates through instance subkeys under a hardware node, preferring the instance
    /// that matches `target_inst_id` exactly before falling back to the first valid EDID.
    ///
    /// This replaces the original implementation that always returned the first instance
    /// found, causing the wrong monitor's EDID to be returned when two identical models
    /// are connected simultaneously.
    fn scan_instances_for_edid(
        &self,
        node_path: &str,
        target_inst_id: &str,
    ) -> Result<Vec<u8>, EdidError> {
        let mut hkey_node = HKEY::default();
        let node_hstring = windows::core::HSTRING::from(node_path);

        if unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, &node_hstring, 0, KEY_READ, &mut hkey_node)
        }
        .is_err()
        {
            return Err(EdidError::NotFound);
        }

        let mut index = 0u32;
        let mut inst_buffer = vec![0u16; 256];
        let mut fallback: Option<Vec<u8>> = None;

        loop {
            let mut inst_len = inst_buffer.len() as u32;

            if unsafe {
                RegEnumKeyExW(
                    hkey_node,
                    index,
                    PWSTR(inst_buffer.as_mut_ptr()),
                    &mut inst_len,
                    None,
                    PWSTR(std::ptr::null_mut()),
                    None,
                    None,
                )
            }
            .is_err()
            {
                break;
            }

            let inst_name =
                String::from_utf16_lossy(&inst_buffer[0..inst_len as usize]);
            let sub_path = format!(r"{}\{}\Device Parameters", node_path, inst_name);

            if let Ok(data) = self.read_registry(&sub_path) {
                // Exact instance match — return immediately
                if inst_name.to_uppercase() == target_inst_id.to_uppercase() {
                    let _ = unsafe { RegCloseKey(hkey_node) };
                    return Ok(data);
                }
                // Keep first valid EDID as fallback in case no exact match exists
                if fallback.is_none() {
                    fallback = Some(data);
                }
            }

            index += 1;
        }

        let _ = unsafe { RegCloseKey(hkey_node) };
        fallback.ok_or(EdidError::NotFound)
    }

    /// Reads the EDID binary value from a Device Parameters registry key.
    fn read_registry(&self, path: &str) -> Result<Vec<u8>, EdidError> {
        let mut hkey = HKEY::default();
        let registry_path = windows::core::HSTRING::from(path);

        if unsafe {
            RegOpenKeyExW(HKEY_LOCAL_MACHINE, &registry_path, 0, KEY_READ, &mut hkey)
        }
        .is_ok()
        {
            let value_name = windows::core::HSTRING::from("EDID");
            let mut value_type = REG_BINARY;
            let mut data_len = 0u32;

            if unsafe {
                RegQueryValueExW(
                    hkey,
                    &value_name,
                    None,
                    Some(&mut value_type as *mut REG_VALUE_TYPE),
                    None,
                    Some(&mut data_len),
                )
            }
            .is_ok()
            {
                let mut buffer = vec![0u8; data_len as usize];
                if unsafe {
                    RegQueryValueExW(
                        hkey,
                        &value_name,
                        None,
                        None,
                        Some(buffer.as_mut_ptr()),
                        Some(&mut data_len),
                    )
                }
                .is_ok()
                {
                    let _ = unsafe { RegCloseKey(hkey) };
                    return Ok(buffer);
                }
            }
            let _ = unsafe { RegCloseKey(hkey) };
        }
        Err(EdidError::NotFound)
    }

    /// Resolves an HMONITOR handle via GDI structures to determine its Device ID.
    fn get_device_id_from_handle(&self, h_monitor: isize) -> Result<String, EdidError> {
        unsafe {
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

            if GetMonitorInfoW(HMONITOR(h_monitor), &mut mi.monitorInfo).as_bool() {
                let mut dev = DISPLAY_DEVICEW::default();
                dev.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

                let gdi_name = PCWSTR(mi.szDevice.as_ptr());
                if EnumDisplayDevicesW(gdi_name, 0, &mut dev, 0).as_bool() {
                    let device_id = String::from_utf16_lossy(&dev.DeviceID)
                        .trim_matches(char::from(0))
                        .to_string();
                    return Ok(device_id);
                }
            }
        }
        Err(EdidError::NotFound)
    }
}