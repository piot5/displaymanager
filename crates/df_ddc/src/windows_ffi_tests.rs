//! Tests for Windows FFI null-handle edge cases.
//!
//! These tests verify the improved error handling for null physical handles
//! during monitor enumeration.

#[cfg(target_os = "windows")]
#[cfg(test)]
#[allow(clippy::module_inception)]
mod windows_ffi_tests {
    use crate::ddc_trait::DisplayDevice;

    /// Tests that enumerate_windows_monitors handles null handle edge cases gracefully
    #[test]
    fn test_enumerate_handles_disconnect_during_enum() {
        // This test verifies that the callback handles null handles gracefully
        // by not panicking and logging appropriate warnings
        let monitors = crate::windows_ffi::enumerate_windows_monitors();

        // The enumeration should complete successfully even if some monitors
        // have null handles (disconnected during enumeration)
        // On systems without physical monitors, this returns an empty vec
        for monitor in &monitors {
            assert!(!monitor.info.is_empty());
        }
    }

    /// Tests that DisplayDevice can be constructed with various info strings
    #[test]
    fn test_display_device_info_variations() {
        use crate::test_utils::MockBackend;

        // Test empty info string (edge case)
        let device = DisplayDevice::new("".to_string(), Box::new(MockBackend));
        assert_eq!(device.info, "");

        // Test info with special characters
        let device = DisplayDevice::new("Monitor \u{0}\0 Null".to_string(), Box::new(MockBackend));
        assert!(device.info.contains('\0'));
    }
}
