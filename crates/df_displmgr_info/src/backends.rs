//! Platform-specific backend implementations for display enumeration and monitor data collection.
//! 
//! This module abstracts away platform-specific complexity (unsafe Windows/Linux API calls)
//! and provides a unified, safe interface for collecting monitor topology and statistics.

/// Windows-specific display enumeration backend using CCD + GDI + Registry.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

use crate::error::EdidError;

/// High-level structure representing comprehensive monitor details.
/// Defined here to avoid circular imports with backends.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorDetails {
    /// Unique target ID for the display output.
    pub target_id: u32,
    /// Human-readable monitor name.
    pub friendly_name: String,
    /// Whether the display is currently active.
    pub is_active: bool,
    /// Output technology (e.g. "HDMI", "DisplayPort").
    pub output_tech: String,
    /// GDI device name (Windows-specific).
    pub gdi_name: String,
    /// Device path string for the display.
    pub device_path: String,
    /// Display topology (position, size, rotation).
    pub topology: Option<crate::edid_types::MonitorTopology>,
    /// Parsed EDID data.
    pub edid: Option<crate::edid_types::EdidData>,
    /// DDC/CI telemetry and capabilities.
    pub ddc_stats: Option<crate::edid_types::DeepDdcStats>,
}

/// Platform-agnostic interface for collecting monitor data.
pub trait MonitorEnumerator {
    /// Collect all available monitors with their topology, EDID, and DDC stats.
    fn collect_monitors(&self) -> Result<Vec<MonitorDetails>, EdidError>;
}

/// Get the platform-specific enumerator for the current OS.
pub fn get_platform_enumerator() -> Box<dyn MonitorEnumerator> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsMonitorEnumerator)
    }
    
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxMonitorEnumerator)
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        compile_error!("Unsupported platform. Only Windows and Linux are supported.");
    }
}
