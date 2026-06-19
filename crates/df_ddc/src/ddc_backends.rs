//! Platform-specific DDC/CI backend implementations.

/// Windows DDC/CI backend using `HighLevelMonitorConfigurationAPI`.
// SAFETY: This module uses `unsafe` for Win32 GDI FFI calls.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod ddc_win;

/// Linux DDC/CI backend using raw I2C via `i2c-dev`.
#[cfg(target_os = "linux")]
pub mod ddc_linux;

/// Debug / logging utilities for DDC/CI operations.
pub mod ddc_debug;
