use crate::error::DdcError;
use crate::ddc_types::{MonitorCapabilities, PowerState, InputSource};
// (remove duplicate re‑exports)

/// Represents a connected DDC/CI-capable display device.
///
/// Each instance wraps a platform-specific backend that implements [`DdcControl`]
/// and a human-readable description string.
pub struct DisplayDevice {
    /// Human-readable description of the monitor (e.g. "I2C Display Bus (i2c-3)").
    pub info: String,
    /// Platform-specific DDC/CI backend.
    pub inner: Box<dyn DdcControl>,
}

/// Platform-agnostic trait for DDC/CI monitor control.
///
/// Implementations of this trait provide low-level access to VCP features
/// (brightness, contrast, input source, power state) over DDC/CI.
pub trait DdcControl: Send + Sync {
    /// Returns the current and maximum value for a given VCP code.
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError>;
    
    /// Sets a value for a specific VCP code.
    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError>;

    /// Fetches brightness and contrast capabilities.
    fn get_capabilities(&self) -> Result<MonitorCapabilities, DdcError> {
        let (b_cur, b_max) = self.get_vcp_feature(0x10)?;
        let (c_cur, c_max) = self.get_vcp_feature(0x12)?;
        Ok(MonitorCapabilities { 
            brightness: b_cur, brightness_max: b_max, 
            contrast: c_cur, contrast_max: c_max 
        })
    }

    /// Sets the monitor power state.
    fn set_power(&self, state: PowerState) -> Result<(), DdcError> {
        self.set_vcp_feature(0xD6, state as u32)
    }

    /// Changes the monitor input source.
    fn set_input(&self, source: InputSource) -> Result<(), DdcError> {
        self.set_vcp_feature(0x60, source as u32)
    }

    /// Sets the monitor brightness.
    fn set_brightness(&self, value: u32) -> Result<(), DdcError> {
        self.set_vcp_feature(0x10, value)
    }
}


