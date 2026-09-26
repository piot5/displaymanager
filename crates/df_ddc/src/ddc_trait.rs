use crate::ddc_types::{InputSource, MonitorCapabilities, PowerState};
use crate::error::DisplayManagerError;
// (remove duplicate re‑exports)

/// Represents a connected DDC/CI-capable display device.
///
/// Each instance wraps a platform-specific backend that implements [`DdcControl`]
/// and a human-readable description string.
pub struct DisplayDevice {
    /// Human-readable description of the monitor (e.g. "I2C Display Bus (i2c-3)").
    pub info: String,
    /// Platform-specific DDC/CI backend (private to enforce encapsulation).
    inner: Box<dyn DdcControl>,
}

impl DisplayDevice {
    /// Creates a new DisplayDevice instance.
    pub fn new(info: String, inner: Box<dyn DdcControl>) -> Self {
        Self { info, inner }
    }

    /// Reads a VCP feature value from the monitor.
    pub fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DisplayManagerError> {
        self.inner.get_vcp_feature(code)
    }

    /// Sets a value for a specific VCP code.
    pub fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DisplayManagerError> {
        self.inner.set_vcp_feature(code, value)
    }

    /// Fetches brightness and contrast capabilities.
    pub fn get_capabilities(&self) -> Result<MonitorCapabilities, DisplayManagerError> {
        self.inner.get_capabilities()
    }

    /// Reads the raw DDC/CI capabilities string (VCP code 0xF3).
    pub fn get_capabilities_string(&self) -> Result<String, DisplayManagerError> {
        self.inner.get_capabilities_string()
    }

    /// Sets the monitor power state.
    pub fn set_power(&self, state: PowerState) -> Result<(), DisplayManagerError> {
        self.inner.set_power(state)
    }

    /// Changes the monitor input source.
    pub fn set_input(&self, source: InputSource) -> Result<(), DisplayManagerError> {
        self.inner.set_input(source)
    }

    /// Sets the monitor brightness.
    pub fn set_brightness(&self, value: u32) -> Result<(), DisplayManagerError> {
        self.inner.set_brightness(value)
    }
}

/// Platform-agnostic trait for DDC/CI monitor control.
///
/// Implementations of this trait provide low-level access to VCP features
/// (brightness, contrast, input source, power state) over DDC/CI.
pub trait DdcControl: Send + Sync {
    /// Returns the current and maximum value for a given VCP code.
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DisplayManagerError>;

    /// Sets a value for a specific VCP code.
    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DisplayManagerError>;

    /// Fetches brightness and contrast capabilities.
    fn get_capabilities(&self) -> Result<MonitorCapabilities, DisplayManagerError> {
        let (b_cur, b_max) = self.get_vcp_feature(0x10)?;
        let (c_cur, c_max) = self.get_vcp_feature(0x12)?;
        Ok(MonitorCapabilities {
            brightness: b_cur,
            brightness_min: 0,
            brightness_max: b_max,
            contrast: c_cur,
            contrast_min: 0,
            contrast_max: c_max,
        })
    }

    /// Reads the raw DDC/CI capabilities string (VCP code 0xF3) and parses it.
    ///
    /// Returns a structured [`CapabilitiesString`] with protocol version,
    /// model name, and supported VCP codes. Falls back to raw string if parsing fails.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayManagerError::CommunicationFailed`] if the monitor does not respond
    /// to VCP code 0xF3, or if the response cannot be converted to a string.
    fn get_capabilities_string(&self) -> Result<String, DisplayManagerError> {
        self.get_vcp_feature(0xF3)
            .map(|(_, raw)| format!("0xF3 response: current={}", raw))
    }

    /// Sets the monitor power state.
    fn set_power(&self, state: PowerState) -> Result<(), DisplayManagerError> {
        self.set_vcp_feature(0xD6, state as u32)
    }

    /// Changes the monitor input source.
    fn set_input(&self, source: InputSource) -> Result<(), DisplayManagerError> {
        self.set_vcp_feature(0x60, source as u32)
    }

    /// Sets the monitor brightness.
    fn set_brightness(&self, value: u32) -> Result<(), DisplayManagerError> {
        self.set_vcp_feature(0x10, value)
    }
}
