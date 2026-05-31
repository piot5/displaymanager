use crate::ddc_trait::DdcControl;
use crate::error::{DdcError, CommunicationFailedSnafu};
use crate::ddc_types::MonitorCapabilities;
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::HANDLE;

pub struct WindowsBackend {
    pub handle: isize,
}

impl Drop for WindowsBackend {
    fn drop(&mut self) {
        // Safety: Ensure physical monitor handle is released to prevent resource leaks
        unsafe { let _ = DestroyPhysicalMonitor(HANDLE(self.handle)); }
    }
}

impl DdcControl for WindowsBackend {
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
        let h = HANDLE(self.handle);
        let (mut current, mut max) = (0, 0);
        
        unsafe {
            // Win32 BOOL is 0 for failure, non-zero for success
            if GetVCPFeatureAndVCPFeatureReply(h, code, None, &mut current, Some(&mut max)) != 0 {
                Ok((current, max))
            } else {
                CommunicationFailedSnafu { 
                    reason: format!("Win32 GetVCP failed for code {:#04x}", code) 
                }.fail()
            }
        }
    }

    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
        let h = HANDLE(self.handle);
        
        unsafe {
            if SetVCPFeature(h, code, value) != 0 {
                Ok(())
            } else {
                CommunicationFailedSnafu { 
                    reason: format!("Win32 SetVCP failed for code {:#04x}", code) 
                }.fail()
            }
        }
    }

    fn get_capabilities(&self) -> Result<MonitorCapabilities, DdcError> {
        let h = HANDLE(self.handle);
        let (mut min_b, mut cur_b, mut max_b) = (0, 0, 0);
        let (mut min_c, mut cur_c, mut max_c) = (0, 0, 0);

        unsafe {
            // Direct API calls are often more reliable than generic VCP requests for these values
            if GetMonitorBrightness(h, &mut min_b, &mut cur_b, &mut max_b) == 0 {
                return CommunicationFailedSnafu { reason: "Win32 Brightness Read failed" }.fail();
            }

            if GetMonitorContrast(h, &mut min_c, &mut cur_c, &mut max_c) == 0 {
                // Fallback: Default to 50/100 if contrast reading is not supported directly
                cur_c = 50; 
                max_c = 100;
            }

            Ok(MonitorCapabilities { 
                brightness: cur_b, 
                brightness_max: max_b, 
                contrast: cur_c, 
                contrast_max: max_c 
            })
        }
    }
}