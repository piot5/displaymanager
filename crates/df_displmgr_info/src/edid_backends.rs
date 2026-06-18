//! Platform-specific EDID backend implementations.

/// Windows EDID backend using Registry queries.
// SAFETY: This module uses `unsafe` for Win32 Registry FFI calls.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod edid_win_reg;

/// Windows EDID backend using DDC/CI.
// SAFETY: This module uses `unsafe` for Win32 GDI FFI calls.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod edid_win_ddc;

/// Windows EDID backend using CCD (Connecting and Configuring Displays).
// SAFETY: This module uses `unsafe` for Win32 Display FFI calls.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod edid_win_ccd;

/// Linux EDID backend using sysfs.
#[cfg(target_os = "linux")]
pub mod edid_linux_sys;

/// Linux EDID backend using DDC/CI.
#[cfg(target_os = "linux")]
pub mod edid_linux_ddc;