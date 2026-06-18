//! Windows DDC/CI backend using the Win32 Monitor Configuration API.
//!
//! This backend uses `GetVCPFeatureAndVCPFeatureReply` and related
//! Win32 functions to read and write VCP codes on physical monitors.
//!
//! # Safety
//!
//! This module uses `unsafe` for Win32 FFI calls. All unsafe blocks are
//! audited with safety comments explaining why the operation is sound.
//!
//! The [`DestroyPhysicalMonitor`] call in [`Drop`] ensures handle
//! lifetimes are correctly managed.

use crate::ddc_trait::DdcControl;
use crate::error::DdcError;
use crate::ddc_types::MonitorCapabilities;
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::HANDLE;

/// Windows DDC/CI backend using the Win32 Monitor Configuration API.
///
/// Wraps a physical monitor handle obtained via
/// [`GetPhysicalMonitorsFromHMONITOR`]. The handle is automatically
/// released when this struct is dropped.
pub struct WindowsBackend {
    /// Physical monitor handle obtained from `GetPhysicalMonitorsFromHMONITOR`.
    pub handle: isize,
}

impl Drop for WindowsBackend {
    fn drop(&mut self) {
        // SAFETY: `DestroyPhysicalMonitor` is the correct deallocation
        // function for handles obtained via `GetPhysicalMonitorsFromHMONITOR`.
        // The handle is guaranteed to be valid at this point because it was
        // obtained in the same process context. After this call the handle
        // must not be used again, which is enforced by Rust's ownership model
        // — this is the last use of `self.handle`.
        unsafe {
            let _ = DestroyPhysicalMonitor(HANDLE(self.handle));
        }
    }
}

impl DdcControl for WindowsBackend {
    /// Reads a VCP feature value from the monitor.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::CommunicationFailed`] if the underlying
    /// `GetVCPFeatureAndVCPFeatureReply` call fails (e.g., the monitor
    /// does not support the requested VCP code, or the display has been
    /// disconnected).
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
        let h = HANDLE(self.handle);
        let (mut current, mut max) = (0, 0);

        // SAFETY: `GetVCPFeatureAndVCPFeatureReply` is called with properly
        // initialised stack-allocated output parameters. The monitor handle
        // is valid because it was obtained in the same process via
        // `GetPhysicalMonitorsFromHMONITOR` and has not been destroyed.
        unsafe {
            if GetVCPFeatureAndVCPFeatureReply(h, code, None, &mut current, Some(&mut max)) != 0 {
                Ok((current, max))
            } else {
                Err(DdcError::CommunicationFailed {
                    reason: format!("Win32 GetVCPFeature failed for code {:#04x}", code),
                })
            }
        }
    }

    /// Writes a VCP feature value to the monitor.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::CommunicationFailed`] if the underlying
    /// `SetVCPFeature` call fails.
    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
        let h = HANDLE(self.handle);

        // SAFETY: `SetVCPFeature` is called with a valid monitor handle
        // and well-defined parameter values. The handle is guaranteed to
        // be valid throughout this call.
        unsafe {
            if SetVCPFeature(h, code, value) != 0 {
                Ok(())
            } else {
                Err(DdcError::CommunicationFailed {
                    reason: format!("Win32 SetVCPFeature failed for code {:#04x}", code),
                })
            }
        }
    }

    /// Reads brightness and contrast capabilities from the monitor.
    ///
    /// Uses the dedicated Win32 `GetMonitorBrightness` and
    /// `GetMonitorContrast` functions which are often more reliable
    /// than generic VCP requests for these common controls.
    ///
    /// # Errors
    ///
    /// Returns [`DdcError::CommunicationFailed`] if the brightness
    /// query fails. Contrast is best-effort; if it fails, defaults
    /// of 50/100 are returned.
    fn get_capabilities(&self) -> Result<MonitorCapabilities, DdcError> {
        let h = HANDLE(self.handle);
        let (mut min_b, mut cur_b, mut max_b) = (0, 0, 0);
        let (mut min_c, mut cur_c, mut max_c) = (0, 0, 0);

        // SAFETY: `GetMonitorBrightness` and `GetMonitorContrast` are called
        // with properly initialised stack-allocated output parameters. The
        // monitor handle is guaranteed valid.
        unsafe {
            if GetMonitorBrightness(h, &mut min_b, &mut cur_b, &mut max_b) == 0 {
                return Err(DdcError::CommunicationFailed {
                    reason: "Win32 GetMonitorBrightness failed".into(),
                });
            }

            if GetMonitorContrast(h, &mut min_c, &mut cur_c, &mut max_c) == 0 {
                // Fallback: default to 50/100 if contrast is not supported directly
                cur_c = 50;
                max_c = 100;
            }

            Ok(MonitorCapabilities {
                brightness: cur_b,
                brightness_max: max_b,
                contrast: cur_c,
                contrast_max: max_c,
            })
        }
    }
}