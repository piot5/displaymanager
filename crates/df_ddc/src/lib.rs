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
//!         let caps = dev.get_capabilities()?;
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
//! | Linux | scan of `/dev/i2c-*` with VCP `0x10` probe (parallel with rayon) | raw I2C, DDC packets over `i2c-dev` |
//!
//! ## License
//!
//! Licensed under either of [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE)
//! at your option.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

/// Platform-specific DDC/CI backend implementations.
pub mod ddc_backends;
/// DDC/CI capabilities string parsing.
pub mod ddc_capabilities;
/// DDC/CI trait definitions and device abstraction.
pub mod ddc_trait;
pub mod ddc_types;
/// Error types for DDC/CI operations.
pub mod error;
/// Windows-specific DDC/CI enumeration via Win32 GDI FFI.
#[cfg(target_os = "windows")]
pub mod windows_ffi;

use crate::ddc_trait::DisplayDevice;
use log::{debug, warn};

#[cfg(target_os = "linux")]
use rayon::prelude::*;

/// Enumerates all connected monitors supporting DDC/CI across different platforms.
///
/// This function filters out non-display I2C devices on Linux and
/// manages physical monitor handles on Windows.
///
/// # Returns
///
/// A vector of [`DisplayDevice`] instances, one per detected DDC/CI-capable monitor.
/// Devices that fail to open or respond are silently skipped with a warning log.
///
/// # Platform-specific behavior
///
/// - **Windows**: Uses `EnumDisplayMonitors` and `GetPhysicalMonitorsFromHMONITOR` to
///   enumerate displays and obtain DDC handles.
/// - **Linux**: Scans `/dev/i2c-*` devices, skipping internal SMBus interfaces,
///   and probes each with a VCP `0x10` query to verify DDC support.
///   Uses parallel probing via `rayon` for improved performance.
///
/// # Progress Bar
///
/// When compiled with the `progress` feature on Linux, use [`list_monitors_with_progress`]
/// to show a progress bar during enumeration.
pub fn list_monitors() -> Vec<DisplayDevice> {
    let mut list: Vec<DisplayDevice> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        debug!("Enumerating DDC/CI monitors on Windows via EnumDisplayMonitors");
        list.extend(crate::windows_ffi::enumerate_windows_monitors());
    }

    #[cfg(target_os = "linux")]
    {
        debug!("Enumerating DDC/CI monitors on Linux via /dev/i2c-* scan");
        let i2c_paths = collect_i2c_paths();
        let probed_devices = probe_i2c_buses(&i2c_paths, None);
        list.extend(probed_devices);
    }

    list
}

/// Collects all I2C bus paths from /dev, filtering out smbus interfaces.
#[cfg(target_os = "linux")]
fn collect_i2c_paths() -> Vec<(String, String)> {
    use std::fs;

    if let Ok(entries) = fs::read_dir("/dev") {
        entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().to_string_lossy().into_owned();

                // Only process I2C buses, skip smbus internal interfaces
                if file_name.starts_with("i2c-") && !file_name.contains("smbus") {
                    let path = format!("/dev/{}", file_name);
                    Some((file_name, path))
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Probes I2C buses in parallel, returning DDC/CI-capable monitors.
/// An optional progress bar can be passed for visual feedback.
#[cfg(target_os = "linux")]
fn probe_i2c_buses(
    i2c_paths: &[(String, String)],
    progress: Option<&indicatif::ProgressBar>,
) -> Vec<DisplayDevice> {
    use crate::ddc_backends::ddc_linux::LinuxBackend;
    use crate::ddc_trait::DdcControl;

    i2c_paths
        .iter()
        .filter_map(|(file_name, path)| {
            if let Some(pb) = progress {
                pb.inc(1);
            }

            let backend = LinuxBackend::new(path.clone());

            // Verification: Only add devices that respond to a basic DDC request.
            if backend.get_vcp_feature(0x10).is_ok() {
                debug!("Found DDC/CI monitor on bus: {}", file_name);
                Some(DisplayDevice::new(
                    format!("I2C Display Bus ({})", file_name),
                    Box::new(backend),
                ))
            } else {
                None
            }
        })
        .collect()
}

/// Enumerates all DDC/CI monitors with a progress bar (Linux only).
///
/// Similar to [`list_monitors`] but displays a progress bar during enumeration.
/// Requires the `progress` feature to be enabled.
///
/// # Returns
///
/// A vector of [`DisplayDevice`] instances, one per detected DDC/CI-capable monitor.
#[cfg(all(feature = "progress", target_os = "linux"))]
pub fn list_monitors_with_progress() -> Vec<DisplayDevice> {
    use indicatif::{ProgressBar, ProgressStyle};

    debug!("Enumerating DDC/CI monitors on Linux with progress indication");

    let i2c_paths = collect_i2c_paths();
    let total = i2c_paths.len();
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );

    let probed_devices = probe_i2c_buses(&i2c_paths, Some(&pb));
    pb.finish_with_message("DDC/CI enumeration complete");
    probed_devices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddc_types::{MonitorCapabilities, PowerState};

    #[test]
    fn test_list_monitors_returns_vec() {
        let monitors: Vec<DisplayDevice> = list_monitors();
        println!("Detected {} DDC monitors", monitors.len());
    }

    #[test]
    fn test_display_device_construction() {
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(test_utils::MockBackend),
        );
        assert_eq!(device.info, "Test Monitor");
        let caps = device.get_capabilities().expect("get_capabilities");
        assert_eq!(caps.brightness, 50);
    }

    #[test]
    fn test_display_device_set_power() {
        use crate::ddc_trait::DdcControl;
        use crate::ddc_types::PowerState;

        let backend = test_utils::MockBackend;
        assert!(backend.set_power(PowerState::On).is_ok());
        assert!(backend.set_power(PowerState::Off).is_ok());
    }

    #[test]
    fn test_monitor_capabilities_fields() {
        let caps = MonitorCapabilities {
            brightness: 50,
            brightness_min: 0,
            brightness_max: 100,
            contrast: 75,
            contrast_min: 0,
            contrast_max: 100,
        };
        assert_eq!(caps.brightness, 50);
        assert_eq!(caps.brightness_min, 0);
        assert_eq!(caps.brightness_max, 100);
    }

    #[test]
    fn test_power_state_values() {
        assert_eq!(PowerState::On as u32, 0x01);
        assert_eq!(PowerState::Off as u32, 0x04);
    }

    #[test]
    fn test_ddc_error_display() {
        let err = crate::error::DdcError::UnsupportedFeature("HDR".into());
        assert!(err.to_string().contains("not supported"));
    }

    // Note: context(), with_vcp_code(), with_bus_path() were removed during the
    // error-type consolidation (see error.rs). Use structured variants or logging
    // for error context instead.
}

/// Shared test utilities (mock backends, test helpers).
#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod ddc_trait_encapsulation_tests;

#[cfg(target_os = "windows")]
#[cfg(test)]
mod windows_ffi_tests;
