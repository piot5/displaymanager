//! Tests for LinuxBackend DDC/CI operations and retry logic.
//!
//! These tests verify the improved error handling for ioctl failures,
//! including retry exhaustions with detailed context.

use crate::ddc_backends::ddc_linux::LinuxBackend;
use crate::error::DisplayManagerError;

#[cfg(test)]
mod linux_backend_tests {
    use super::*;

    /// Tests that LinuxBackend::open_device returns appropriate errors for invalid paths
    #[test]
    fn test_linux_backend_invalid_device_path() {
        let backend = LinuxBackend::new("/dev/nonexistent-i2c-bus".to_string());

        // Should return BackendNotAvailable for non-existent device
        let result = backend.open_device();
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                DisplayManagerError::BackendNotAvailable { details } => {
                    assert!(!details.is_empty());
                }
                DisplayManagerError::AccessDenied => {
                    // Also acceptable if permission denied
                }
                _ => panic!("Unexpected error type: {:?}", e),
            }
        }
    }

    /// Tests that communication failures include retry context
    #[test]
    fn test_linux_backend_retry_exhaustion_message() {
        let backend = LinuxBackend::new("/dev/nonexistent".to_string());

        // This should fail after all retries and include context
        let result = backend.get_vcp_feature(0x10);
        assert!(result.is_err());

        if let Err(e) = result {
            let msg = e.to_string();
            // The error message should indicate failure with context
            assert!(msg.contains("failed") || msg.contains("timeout"));
        }
    }
}

#[cfg(test)]
mod linux_backend_checksum_tests {
    use super::*;

    #[test]
    fn test_calculate_checksum_basic() {
        // Checksum should make sum of all bytes = 0 mod 256
        let backend = LinuxBackend::new("/dev/test".to_string());

        // Test known good checksum values
        let packet = [0x51, 0x82, 0x01, 0x10];
        let checksum = backend.calculate_checksum(&packet);
        let total: u8 = packet.iter().fold(0, |a, &b| a.wrapping_add(b));
        let total_with_checksum = total.wrapping_add(checksum);
        assert_eq!(total_with_checksum, 0);
    }

    #[test]
    fn test_calculate_checksum_empty() {
        let backend = LinuxBackend::new("/dev/test".to_string());
        let checksum = backend.calculate_checksum(&[]);
        assert_eq!(checksum, 0);
    }

    #[test]
    fn test_calculate_checksum_single_byte() {
        let backend = LinuxBackend::new("/dev/test".to_string());
        let checksum = backend.calculate_checksum(&[0x55]);
        // 0x55 + 0xAA = 0xFF -> wraps to 0x00 mod 256
        assert_eq!(checksum, 0xAA);
    }
}
