//! Windows-specific DDC/CI monitor enumeration via Win32 GDI FFI.
//!
//! # Safety
//!
//! This module uses `unsafe` for Win32 FFI calls to `EnumDisplayMonitors`,
//! `GetPhysicalMonitorsFromHMONITOR`, and related functions. All unsafe
//! blocks are audited with safety comments explaining why the operation
//! is sound in context.

#![allow(unsafe_code)]

use crate::ddc_trait::DisplayDevice;
use log::{debug, warn};
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::*;

/// Enumerates all physical monitors connected to the Windows desktop
/// by calling `EnumDisplayMonitors` and retrieving physical monitor handles.
///
/// # Returns
///
/// A vector of [`DisplayDevice`] instances, one per detected DDC/CI-capable
/// physical monitor. Monitors that fail to open are silently skipped.
pub fn enumerate_windows_monitors() -> Vec<DisplayDevice> {
    let mut list = Vec::new();

    unsafe extern "system" fn callback(
        h: HMONITOR,
        _: HDC,
        _: *mut RECT,
        lp: LPARAM,
    ) -> BOOL {
        // SAFETY: The callback receives a valid LPARAM pointer to a Vec<DisplayDevice>.
        // The Vec is heap-allocated and outlives the EnumDisplayMonitors call.
        // HMONITOR is valid because it comes from EnumDisplayMonitors.
        // We must handle the case where the monitor might be disconnected.
        unsafe {
            let mut count: u32 = 0;

            // GetNumberOfPhysicalMonitorsFromHMONITOR can fail if the monitor
            // was disconnected between EnumDisplayMonitors and this callback
            if GetNumberOfPhysicalMonitorsFromHMONITOR(h, &mut count).is_err() {
                debug!("GetNumberOfPhysicalMonitorsFromHMONITOR failed for HMONITOR");
                return BOOL::from(true);
            }

            if count == 0 {
                debug!("No physical monitors found for HMONITOR, skipping");
                return BOOL::from(true);
            }

            let mut physical_monitors = vec![PHYSICAL_MONITOR::default(); count as usize];

            // GetPhysicalMonitorsFromHMONITOR can also fail for disconnected monitors
            if GetPhysicalMonitorsFromHMONITOR(h, &mut physical_monitors).is_err() {
                warn!("GetPhysicalMonitorsFromHMONITOR failed, monitor may be disconnected");
                return BOOL::from(true);
            }

            let l = &mut *(lp.0 as *mut Vec<DisplayDevice>);

            for monitor in physical_monitors {
                let desc = monitor.szPhysicalMonitorDescription;
                let len = desc.iter().position(|&c| c == 0).unwrap_or(desc.len());

                // Handle may be null if monitor was disconnected during enumeration
                // This is a critical edge case that needs explicit handling
                match std::ptr::NonNull::new(monitor.hPhysicalMonitor.0) {
                    Some(handle) => {
                        let info = String::from_utf16_lossy(&desc[..len]);
                        debug!("Found DDC/CI monitor: {}", info);
                        l.push(DisplayDevice::new(
                            info,
                            Box::new(crate::ddc_backends::ddc_win::WindowsBackend { handle }),
                        ));
                    }
                    None => {
                        warn!(
                            "Monitor '{}' had null physical handle (disconnected during enumeration), skipping",
                            String::from_utf16_lossy(&desc[..len])
                        );
                    }
                }
            }
        }
        BOOL::from(true)
    }

    // SAFETY: LPARAM constructed from raw pointer to a live Vec.
    // The callback only reads/writes through this pointer during EnumDisplayMonitors.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(callback),
            LPARAM(&mut list as *mut _ as isize),
        );
    }

    list
}