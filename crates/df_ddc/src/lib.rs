pub mod ddc_types;
pub mod ddc_trait;
pub mod ddc_backends;
pub mod error;

use crate::ddc_trait::DisplayDevice;

/// Enumerates all connected monitors supporting DDC/CI across different platforms.
/// 
/// This function filters out non-display I2C devices on Linux and 
/// manages physical monitor handles on Windows.
pub fn list_monitors() -> Vec<DisplayDevice> {
    let mut list = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::Devices::Display::*;
        use windows::Win32::Foundation::{LPARAM, BOOL, RECT};

        // Safety: The vector must outlive the EnumDisplayMonitors call.
        unsafe extern "system" fn callback(h: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
            let l = &mut *(lp.0 as *mut Vec<DisplayDevice>);
            let mut count = 0;
            
            if GetNumberOfPhysicalMonitorsFromHMONITOR(h, &mut count).is_ok() {
                let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); count as usize];
                
                if GetPhysicalMonitorsFromHMONITOR(h, &mut physical_monitors).is_ok() {
                    for monitor in physical_monitors {
                        let desc = monitor.szPhysicalMonitorDescription;
                        let len = desc.iter().position(|&c| c == 0).unwrap_or(desc.len());
                        
                        l.push(DisplayDevice {
                            info: String::from_utf16_lossy(&desc[..len]),
                            inner: Box::new(crate::ddc_backends::ddc_win::WindowsBackend { 
                                handle: monitor.hPhysicalMonitor.0 
                            }),
                        });
                    }
                }
            }
            BOOL::from(true)
        }
        
        unsafe { 
            let _ = EnumDisplayMonitors(None, None, Some(callback), LPARAM(&mut list as *mut _ as isize)); 
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use crate::ddc_trait::DdcControl;
        use crate::ddc_backends::ddc_linux::LinuxBackend;

        if let Ok(entries) = fs::read_dir("/dev") {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                
                // Only attempt to probe I2C buses
                if file_name.starts_with("i2c-") {
                    let path = format!("/dev/{}", file_name);
                    
                    // Skip common non-monitor internal buses to speed up enumeration
                    if file_name.contains("smbus") { continue; }

                    // Create backend with internal Mutex for thread-safety
                    let backend = LinuxBackend::new(path.clone());
                    
                    // Verification: Only add devices that respond to a basic DDC request.
                    // This is essential as /dev/i2c-* includes many non-monitor devices.
                    if backend.get_vcp_feature(0x10).is_ok() {
                        list.push(DisplayDevice {
                            info: format!("I2C Display Bus ({})", file_name),
                            inner: Box::new(backend),
                        });
                    }
                }
            }
        }
    }

    list
}