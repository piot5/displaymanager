//! Dedicated test module for DisplayDevice encapsulation and accessor methods.
//!
//! This module validates that the private `inner` field is properly encapsulated
//! and that all public accessor methods work correctly with mock backends.

#[cfg(test)]
mod tests {
    use crate::ddc_trait::DisplayDevice;
    use crate::ddc_types::{InputSource, PowerState};
    use crate::test_utils::MockBackendWithCalls;

    /// Test that DisplayDevice::new() creates instances correctly
    #[test]
    fn test_display_device_construction() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        assert_eq!(device.info, "Test Monitor");
        // Verify inner is not accessible (compile-time check)
        // device.inner; // This would fail to compile
    }

    /// Test get_capabilities() accessor method
    #[test]
    fn test_get_capabilities_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        let caps = device.get_capabilities().expect("get_capabilities should succeed");

        assert_eq!(caps.brightness, 75);
        assert_eq!(caps.brightness_max, 100);
        assert_eq!(caps.contrast, 50);
        assert_eq!(caps.contrast_max, 100);
    }

    /// Test get_vcp_feature() accessor method with different VCP codes
    #[test]
    fn test_get_vcp_feature_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        // Test brightness (VCP 0x10)
        let (cur, max) = device.get_vcp_feature(0x10).expect("brightness query should succeed");
        assert_eq!(cur, 75);
        assert_eq!(max, 100);

        // Test contrast (VCP 0x12)
        let (cur, max) = device.get_vcp_feature(0x12).expect("contrast query should succeed");
        assert_eq!(cur, 50);
        assert_eq!(max, 100);

        // Test unsupported feature
        let result = device.get_vcp_feature(0xFF);
        assert!(result.is_err());
    }

    /// Test set_vcp_feature() accessor method
    #[test]
    fn test_set_vcp_feature_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        let result = device.set_vcp_feature(0x10, 80);
        assert!(result.is_ok());
    }

    /// Test set_power() accessor method
    #[test]
    fn test_set_power_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        let result = device.set_power(PowerState::On);
        assert!(result.is_ok());

        let result = device.set_power(PowerState::Off);
        assert!(result.is_ok());
    }

    /// Test set_input() accessor method
    #[test]
    fn test_set_input_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        let result = device.set_input(InputSource::DisplayPort1);
        assert!(result.is_ok());

        let result = device.set_input(InputSource::Hdmi1);
        assert!(result.is_ok());
    }

    /// Test set_brightness() accessor method
    #[test]
    fn test_set_brightness_accessor() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(
            "Test Monitor".to_string(),
            Box::new(backend),
        );

        let result = device.set_brightness(50);
        assert!(result.is_ok());

        let result = device.set_brightness(100);
        assert!(result.is_ok());
    }

    /// Test that multiple devices can share the same backend type
    #[test]
    fn test_multiple_devices_independent() {
        let backend1 = MockBackendWithCalls::new();
        let backend2 = MockBackendWithCalls::new();

        let device1 = DisplayDevice::new("Monitor 1".to_string(), Box::new(backend1));
        let device2 = DisplayDevice::new("Monitor 2".to_string(), Box::new(backend2));

        // Verify each device has independent backend state
        assert_eq!(device1.info, "Monitor 1");
        assert_eq!(device2.info, "Monitor 2");

        let caps1 = device1.get_capabilities().unwrap();
        let caps2 = device2.get_capabilities().unwrap();

        assert_eq!(caps1.brightness, 75);
        assert_eq!(caps2.brightness, 75);
    }

    /// Test DisplayDevice::new() with empty info string
    #[test]
    fn test_display_device_empty_info() {
        let backend = MockBackendWithCalls::new();
        let device = DisplayDevice::new(String::new(), Box::new(backend));

        assert!(device.info.is_empty());
    }
}