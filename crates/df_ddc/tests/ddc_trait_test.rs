use df_ddc::ddc_trait::{DdcControl, DisplayDevice};
use df_ddc::ddc_types::{MonitorCapabilities, PowerState, InputSource};
use std::sync::Mutex;

/// Mock DDC control implementation for testing trait methods
struct MockDdcControl {
    vcp_values: Mutex<std::collections::HashMap<u8, (u32, u32)>>,
}

impl MockDdcControl {
    fn new() -> Self {
        let mut values = std::collections::HashMap::new();
        values.insert(0x10, (128, 255)); // brightness: current=128, max=255
        values.insert(0x12, (64, 128));  // contrast: current=64, max=128
        values.insert(0xD6, (1, 1));     // power state
        values.insert(0x60, (1, 5));     // input source
        
        Self {
            vcp_values: Mutex::new(values),
        }
    }
}

impl DdcControl for MockDdcControl {
    fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), df_ddc::error::DdcError> {
        let values = self.vcp_values.lock().unwrap();
        values.get(&code).copied().ok_or(df_ddc::error::DdcError::UnsupportedFeature)
    }
    
    fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), df_ddc::error::DdcError> {
        let mut values = self.vcp_values.lock().unwrap();
        if let Some((current, max)) = values.get_mut(&code) {
            *current = value;
            Ok(())
        } else {
            Err(df_ddc::error::DdcError::UnsupportedFeature)
        }
    }
}

/// Test DisplayDevice structure creation
#[test]
fn test_display_device_creation() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert_eq!(device.info, "Test Monitor");
}

/// Test get_capabilities through trait
#[test]
fn test_get_capabilities() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    let caps = device.inner.get_capabilities().unwrap();
    assert_eq!(caps.brightness, 128);
    assert_eq!(caps.brightness_max, 255);
    assert_eq!(caps.contrast, 64);
    assert_eq!(caps.contrast_max, 128);
}

/// Test set_power through trait
#[test]
fn test_set_power() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert!(device.inner.set_power(PowerState::On).is_ok());
    assert!(device.inner.set_power(PowerState::Off).is_ok());
}

/// Test set_input through trait
#[test]
fn test_set_input() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert!(device.inner.set_input(InputSource::Hdmi1).is_ok());
    assert!(device.inner.set_input(InputSource::DisplayPort1).is_ok());
    assert!(device.inner.set_input(InputSource::DisplayPort2).is_ok());
}

/// Test set_brightness through trait
#[test]
fn test_set_brightness() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert!(device.inner.set_brightness(200).is_ok());
    assert!(device.inner.set_brightness(0).is_ok());
    assert!(device.inner.set_brightness(255).is_ok());
}

/// Test get_vcp_feature with unsupported code
#[test]
fn test_get_vcp_feature_unsupported() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    let result = device.inner.get_vcp_feature(0xFF);
    assert!(result.is_err());
}

/// Test set_vcp_feature with unsupported code
#[test]
fn test_set_vcp_feature_unsupported() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    let result = device.inner.set_vcp_feature(0xFF, 100);
    assert!(result.is_err());
}

/// Test get_vcp_feature returns correct values
#[test]
fn test_get_vcp_feature_success() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    let (brightness, max) = device.inner.get_vcp_feature(0x10).unwrap();
    assert_eq!(brightness, 128);
    assert_eq!(max, 255);
}

/// Test set_vcp_feature updates values
#[test]
fn test_set_vcp_feature_updates() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert!(device.inner.set_vcp_feature(0x10, 200).is_ok());
    let (new_value, max) = device.inner.get_vcp_feature(0x10).unwrap();
    assert_eq!(new_value, 200);
    assert_eq!(max, 255); // max should remain unchanged
}

/// Test multiple operations on same device
#[test]
fn test_multiple_operations() {
    let device = DisplayDevice {
        info: "Test Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    // Get initial brightness
    let (initial_brightness, _) = device.inner.get_vcp_feature(0x10).unwrap();
    assert_eq!(initial_brightness, 128);
    
    // Set new brightness
    assert!(device.inner.set_brightness(180).is_ok());
    
    // Verify change
    let (new_brightness, _) = device.inner.get_vcp_feature(0x10).unwrap();
    assert_eq!(new_brightness, 180);
    
    // Get capabilities still works
    let caps = device.inner.get_capabilities().unwrap();
    assert_eq!(caps.brightness, 180); // updated value
    assert_eq!(caps.brightness_max, 255);
}

/// Test DisplayDevice with different info strings
#[test]
fn test_display_device_info_strings() {
    let device1 = DisplayDevice {
        info: "I2C Display Bus (i2c-3)".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    let device2 = DisplayDevice {
        info: "HDMI Monitor".to_string(),
        inner: Box::new(MockDdcControl::new()),
    };
    
    assert!(device1.info.contains("i2c-3"));
    assert_eq!(device2.info, "HDMI Monitor");
}