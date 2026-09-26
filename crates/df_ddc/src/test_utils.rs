//! Shared test utilities for the `df_ddc` crate.
//!
//! This module provides reusable mock backends and test helpers to avoid
//! code duplication across test files.

use crate::ddc_trait::DdcControl;
use crate::ddc_types::MonitorCapabilities;
use crate::error::DisplayManagerError;

/// A simple mock backend that returns default values for all operations.
///
/// This is useful for tests that don't need to verify specific method calls.
/// For more advanced testing, use [`MockBackendWithCalls`].
#[derive(Default)]
pub struct MockBackend;

impl DdcControl for MockBackend {
    fn get_vcp_feature(&self, _code: u8) -> Result<(u32, u32), DisplayManagerError> {
        Ok((50, 100))
    }

    fn set_vcp_feature(&self, _code: u8, _value: u32) -> Result<(), DisplayManagerError> {
        Ok(())
    }

    fn get_capabilities(&self) -> Result<MonitorCapabilities, DisplayManagerError> {
        Ok(MonitorCapabilities {
            brightness: 50,
            brightness_min: 0,
            brightness_max: 100,
            contrast: 50,
            contrast_min: 0,
            contrast_max: 100,
        })
    }
}

/// A mock backend that tracks method calls for verification.
///
/// Each method call is recorded in a `Vec<String>` accessible via the
/// `calls` field. This allows tests to verify that the correct methods
/// were called with the expected arguments.
pub struct MockBackendWithCalls {
    /// Recorded method calls (e.g. "get_vcp_feature(0x10)").
    pub calls: std::sync::Mutex<Vec<String>>,
}

impl MockBackendWithCalls {
    /// Creates a new `MockBackendWithCalls` with an empty call log.
    pub fn new() -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for MockBackendWithCalls {
    fn default() -> Self {
        Self::new()
    }
}

impl DdcControl for MockBackendWithCalls {
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DisplayManagerError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("get_vcp_feature({:#04X})", code));
        match code {
            0x10 => Ok((75, 100)),    // Brightness
            0x12 => Ok((50, 100)),    // Contrast
            0x60 => Ok((0x0F, 0x12)), // Input source
            0xD6 => Ok((0x01, 0x04)), // Power state
            0x62 => Ok((25, 100)),    // Volume
            _ => Err(DisplayManagerError::UnsupportedFeature(format!(
                "VCP code {:#04X}",
                code
            ))),
        }
    }

    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DisplayManagerError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("set_vcp_feature({:#04X}, {})", code, value));
        Ok(())
    }

    fn get_capabilities(&self) -> Result<MonitorCapabilities, DisplayManagerError> {
        Ok(MonitorCapabilities {
            brightness: 75,
            brightness_min: 0,
            brightness_max: 100,
            contrast: 50,
            contrast_min: 0,
            contrast_max: 100,
        })
    }
}
