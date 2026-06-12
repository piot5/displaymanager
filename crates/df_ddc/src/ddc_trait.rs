use crate::error::DdcError;
use crate::ddc_types::{MonitorCapabilities, PowerState, InputSource};

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
}

pub struct DisplayDevice {
    pub info: String,
    pub inner: Box<dyn DdcControl>,
}

impl DisplayDevice {
    pub fn get_capabilities(&self) -> Result<MonitorCapabilities, DdcError> {
        self.inner.get_capabilities()
    }

    pub fn set_brightness(&self, value: u32) -> Result<(), DdcError> {
        self.inner.set_vcp_feature(0x10, value)
    }
}

