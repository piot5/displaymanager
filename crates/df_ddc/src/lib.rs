//! # df_ddc
//!
//! DDC/CI (VESA MCCS) monitor control backend.
//!
//! This crate provides low-level I/O for VCP features: brightness, contrast,
//! volume, input source, and power state across Windows and Linux.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use df_ddc::list_monitors;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     for dev in list_monitors() {
//!         let caps = dev.inner.get_capabilities()?;
//!         println!("{}: brightness={}/{}", dev.info, caps.brightness, caps.brightness_max);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Platform Support
//!
//! | OS | Enumeration | I/O |
//! |---|---|---|
//! | Windows | `EnumDisplayMonitors` + `GetPhysicalMonitorsFromHMONITOR` | `HighLevelMonitorConfigurationAPI` |
//! | Linux | scan of `/dev/i2c-*` with VCP `0x10` probe | raw I2C, DDC packets over `i2c-dev` |
//!
//! ## License
//!
//! Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
//! at your option.

#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod ddc_types;
/// DDC/CI trait definitions and device abstraction.
pub mod ddc_trait;
/// Platform-specific DDC/CI backend implementations.
pub mod ddc_backends;
/// Error types for DDC/CI operations.
pub mod error;

use crate::ddc_trait::DisplayDevice;

/// Enumerates all connected monitors supporting DDC/CI across different platforms.
///
/// This function filters out non-display I2C devices on Linux and
/// manages physical monitor handles on Windows.
pub fn list_monitors() -> Vec<DisplayDevice> {
let mut list: Vec<DisplayDevice> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // SAFETY: The Windows backend uses `unsafe` for Win32 GDI FFI calls.
        // The callback receives a valid LPARAM pointer to a Vec<DisplayDevice>
        // that outlives the EnumDisplayMonitors call. All unsafe blocks below
        // are audited and the pointer arithmetic is sound because the LPARAM
        // is constructed from a reference to a live Vec.
        #[allow(unsafe_code)]
        mod windows_ffi {
use crate::ddc_trait::DisplayDevice;
            use windows::Win32::Devices::Display::*;
            use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
            use windows::Win32::Graphics::Gdi::*;

            pub fn enumerate_windows_monitors() -> Vec<DisplayDevice> {
                let mut list = Vec::new();

                // SAFETY: The callback receives a valid LPARAM pointer to a Vec<DisplayDevice>.
                // The Vec is heap-allocated and outlives the EnumDisplayMonitors call.
                unsafe extern "system" fn callback(
                    h: HMONITOR,
                    _: HDC,
                    _: *mut RECT,
                    lp: LPARAM,
                ) -> BOOL {
                    let l = &mut *(lp.0 as *mut Vec<DisplayDevice>);
                    let mut count = 0;

                    if GetNumberOfPhysicalMonitorsFromHMONITOR(h, &mut count).is_ok() {
                        let mut physical_monitors =
                            vec![PHYSICAL_MONITOR::default(); count as usize];

                        if GetPhysicalMonitorsFromHMONITOR(h, &mut physical_monitors).is_ok() {
                            for monitor in physical_monitors {
                                let desc = monitor.szPhysicalMonitorDescription;
                                let len =
                                    desc.iter().position(|&c| c == 0).unwrap_or(desc.len());

                                l.push(DisplayDevice {
                                    info: String::from_utf16_lossy(&desc[..len]),
                                    inner: Box::new(
                                        crate::ddc_backends::ddc_win::WindowsBackend {
                                            handle: monitor.hPhysicalMonitor.0,
                                        },
                                    ),
                                });
                            }
                        }
                    }
                    BOOL::from(true)
                }

                // SAFETY: The LPARAM is constructed from a raw pointer to a live Vec.
                // The callback only reads/writes through this pointer during the
                // EnumDisplayMonitors call, and the Vec is not moved or dropped until
                // after the call completes.
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
        }

list.extend(windows_ffi::enumerate_windows_monitors());
    }

    #[cfg(target_os = "linux")]
    {
        use crate::ddc_backends::ddc_linux::LinuxBackend;
        use crate::ddc_trait::DdcControl;
        use std::fs;

        if let Ok(entries) = fs::read_dir("/dev") {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().into_owned();

                // Only attempt to probe I2C buses
                if file_name.starts_with("i2c-") {
                    let path = format!("/dev/{}", file_name);

                    // Skip common non-monitor internal buses to speed up enumeration
                    if file_name.contains("smbus") {
                        continue;
                    }

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