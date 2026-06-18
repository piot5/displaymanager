//! Trait definition for EDID data retrieval from display devices.

use crate::error::EdidError;

/// Platform-specific trait for reading raw EDID data from a display device.
pub trait EdidControl {
    /// Reads the raw EDID block from the display device.
    fn get_edid_raw(&self) -> Result<Vec<u8>, EdidError>;
}

/// Represents a connected display device with EDID retrieval capability.
pub struct DisplayDevice {
    /// Human-readable description of the display.
    pub info: String,
    /// Platform-specific EDID backend.
    pub inner: Box<dyn EdidControl>,
}