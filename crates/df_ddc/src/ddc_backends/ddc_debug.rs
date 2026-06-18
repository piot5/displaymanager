use crate::ddc_trait::DdcControl;
use crate::error::DdcError;
use crate::ddc_types::VcpCode;
use std::collections::HashMap;
use std::sync::Mutex;

/// Debug/mock DDC/CI backend for testing without hardware.
///
/// Simulates VCP register storage in a `HashMap`, allowing unit tests
/// to verify DDC/CI logic without real monitor hardware.
pub struct DebugBackend {
    /// Simulated storage for VCP registers.
    vcp_values: Mutex<HashMap<u8, u32>>,
}

impl Default for DebugBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugBackend {
    /// Creates a new debug backend with default VCP values (brightness=50, contrast=80, power=on).
    pub fn new() -> Self {
        let mut initial_values = HashMap::new();
        
        // Initialize default values (analogous to ddc_types.rs)
        initial_values.insert(VcpCode::Brightness as u8, 50);
        initial_values.insert(VcpCode::Contrast as u8, 80);
        initial_values.insert(VcpCode::PowerMode as u8, 0x01); // On
        
        Self {
            vcp_values: Mutex::new(initial_values),
        }
    }
}

impl DdcControl for DebugBackend {
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
        let values = self.vcp_values.lock().unwrap();
        
        // We simulate a max value of 100 for all controls
        let current = values.get(&code).cloned().unwrap_or(0);
        
        println!("[DEBUG BUS] Read Code {:#04x}: Value {}", code, current);
        Ok((current, 100))
    }

    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
        let mut values = self.vcp_values.lock().unwrap();
        
        println!("[DEBUG BUS] Write Code {:#04x}: Set to {}", code, value);
        values.insert(code, value);
        
        Ok(())
    }
}