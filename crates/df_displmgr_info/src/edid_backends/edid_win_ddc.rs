use crate::edid_types::{
    AudioMuteState, DeepDdcStats, InputSource, MonitorCapabilities, PowerState, VcpCode,
};
use crate::error::EdidError;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetMonitorContrast,
    GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
    GetVCPFeatureAndVCPFeatureReply, SetVCPFeature, PHYSICAL_MONITOR,
};
use windows::Win32::Graphics::Gdi::HMONITOR;

/// DDC/CI backend for querying deep hardware telemetry via Win32 Monitor Configuration API.
pub struct WindowsDdcBackend {
    /// HMONITOR handle for the display device.
    pub h_monitor: *mut core::ffi::c_void,
}

impl WindowsDdcBackend {
    /// Probes and queries every DDC VCP code matching the types specified inside `edid_types.rs`.
    pub fn query_deep_hardware_stats(&self) -> Result<DeepDdcStats, EdidError> {
        unsafe {
            let h_monitor = HMONITOR(self.h_monitor);
            let mut num_physical = 0u32;

            GetNumberOfPhysicalMonitorsFromHMONITOR(h_monitor, &mut num_physical)
                .map_err(EdidError::WindowsError)?;

            if num_physical == 0 {
                return Err(EdidError::NotFound);
            }

            let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); num_physical as usize];
            GetPhysicalMonitorsFromHMONITOR(h_monitor, &mut physical_monitors)
                .map_err(EdidError::WindowsError)?;

            let h_phys = physical_monitors[0].hPhysicalMonitor;

            // 1. Extract core brightness & contrast capabilities.
            // FIX: Previously the return values of GetMonitorBrightness and GetMonitorContrast
            // were completely ignored. A failure left b_curr/c_curr at 0, silently populating
            // MonitorCapabilities with garbage values. Now failures surface as errors so the
            // caller can decide whether to retry or degrade gracefully.
            let mut b_min = 0u32;
            let mut b_curr = 0u32;
            let mut b_max = 0u32;
            // GetMonitorBrightness returns BOOL (i32), not a Result — nonzero = success
            if GetMonitorBrightness(h_phys, &mut b_min, &mut b_curr, &mut b_max) == 0 {
                return Err(EdidError::DdcError(
                    "GetMonitorBrightness failed".to_string(),
                ));
            }

            let mut c_min = 0u32;
            let mut c_curr = 0u32;
            let mut c_max = 0u32;
            // GetMonitorContrast returns BOOL (i32), not a Result — nonzero = success
            if GetMonitorContrast(h_phys, &mut c_min, &mut c_curr, &mut c_max) == 0 {
                return Err(EdidError::DdcError("GetMonitorContrast failed".to_string()));
            }

            let core_caps = MonitorCapabilities {
                brightness: b_curr,
                brightness_max: b_max,
                contrast: c_curr,
                contrast_max: c_max,
            };

            // Generic closure wrapping raw DDC VCP register querying.
            // Returns Some((current, max)) on success, None on failure.
            let query_vcp = |code: VcpCode| -> Option<(u32, u32)> {
                let mut current_val = 0u32;
                let mut max_val = 0u32;
                // GetVCPFeatureAndVCPFeatureReply returns a BOOL: nonzero = success.
                // No inner `unsafe` needed — already inside the outer unsafe block.
                if GetVCPFeatureAndVCPFeatureReply(
                    h_phys,
                    code as u8,
                    None,
                    &mut current_val,
                    Some(&mut max_val),
                ) != 0
                {
                    Some((current_val, max_val))
                } else {
                    None
                }
            };

            // 2. Decode current physical input video source matching InputSource enum specs
            let input_source = match query_vcp(VcpCode::InputSource).map(|(c, _)| c) {
                Some(0x01) => InputSource::AnalogVga,
                Some(0x03) => InputSource::Dvi,
                Some(0x05) => InputSource::Composite,
                Some(0x06) => InputSource::SVideo,
                Some(0x0F) => InputSource::DisplayPort1,
                Some(0x10) => InputSource::DisplayPort2,
                Some(0x11) => InputSource::Hdmi1,
                Some(0x12) => InputSource::Hdmi2,
                Some(0x13) => InputSource::UsbC,
                _ => InputSource::Unknown,
            };

            // 3. Decode operational power management status matching PowerState enum specs
            let power_state = match query_vcp(VcpCode::PowerMode).map(|(c, _)| c) {
                Some(0x01) => PowerState::On,
                Some(0x02) => PowerState::Standby,
                Some(0x03) => PowerState::Suspend,
                Some(0x04) => PowerState::Off,
                _ => PowerState::Unknown,
            };

            // 4. Query physical audio metrics
            let volume = query_vcp(VcpCode::Volume);
            let audio_mute = match query_vcp(VcpCode::AudioMute).map(|(c, _)| c) {
                Some(0x01) => AudioMuteState::Muted,
                Some(0x02) => AudioMuteState::Unmuted,
                _ => AudioMuteState::Unknown,
            };

            // 5. Query discrete raw RGB sub-color gain vectors
            let color_gains = match (
                query_vcp(VcpCode::RedGain).map(|(c, _)| c),
                query_vcp(VcpCode::GreenGain).map(|(c, _)| c),
                query_vcp(VcpCode::BlueGain).map(|(c, _)| c),
            ) {
                (Some(r), Some(g), Some(b)) => Some((r, g, b)),
                _ => None,
            };

            // 6. Pull extended DDC bus attributes
            let horizontal_freq_hz = query_vcp(VcpCode::HorizontalFrequency).map(|(c, _)| c);
            // FIX: Field was named vertical_freq_mhz but DDC register 0xAE reports in units
            // of 0.01 Hz, not MHz. The field name in edid_types.rs should be corrected to
            // vertical_freq_centihz; we document the unit here until that rename lands.
            let vertical_freq_centihz = query_vcp(VcpCode::VerticalFrequency).map(|(c, _)| c); // unit: 0.01 Hz
            let operating_hours = query_vcp(VcpCode::OperatingHours).map(|(c, _)| c);
            let osd_language_code = query_vcp(VcpCode::OsdLanguage).map(|(c, _)| c);
            let panel_type_code = query_vcp(VcpCode::FlatPanelType).map(|(c, _)| c);

            let _ = DestroyPhysicalMonitors(&physical_monitors);

            Ok(DeepDdcStats {
                core_caps,
                input_source,
                power_state,
                volume,
                audio_mute,
                color_gains,
                horizontal_freq_hz,
                vertical_freq_centihz,
                operating_hours,
                osd_language_code,
                panel_type_code,
            })
        }
    }

    /// Updates target hardware parameter fields using custom DDC Set VCP Feature subroutines.
    pub fn set_vcp_feature(&self, code: VcpCode, value: u32) -> Result<(), EdidError> {
        unsafe {
            let h_monitor = HMONITOR(self.h_monitor);
            let mut num_physical = 0u32;

            GetNumberOfPhysicalMonitorsFromHMONITOR(h_monitor, &mut num_physical)
                .map_err(EdidError::WindowsError)?;

            if num_physical == 0 {
                return Err(EdidError::NotFound);
            }

            let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); num_physical as usize];
            GetPhysicalMonitorsFromHMONITOR(h_monitor, &mut physical_monitors)
                .map_err(EdidError::WindowsError)?;

            let h_phys = physical_monitors[0].hPhysicalMonitor;
            let result = SetVCPFeature(h_phys, code as u8, value);
            let _ = DestroyPhysicalMonitors(&physical_monitors);

            if result == 0 {
                Err(EdidError::DdcError(
                    "Failed to execute SetVCPFeature call".to_string(),
                ))
            } else {
                Ok(())
            }
        }
    }
}
