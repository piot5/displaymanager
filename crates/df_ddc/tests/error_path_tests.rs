//! Error path tests for DdcError variants
//!
//! These tests validate error handling and propagation for all DdcError variants,
//! ensuring proper error messages and conversion to DisplayManagerError.

use df_ddc::error::DdcError;
use df_core::DisplayManagerError;

// ============================================================================
// DdcError::InvalidDevice Tests
// ============================================================================

/// Test DdcError::InvalidDevice error message formatting
#[test]
fn test_ddc_error_invalid_device() {
    let err = DdcError::InvalidDevice {
        path: "/dev/i2c-5".to_string(),
    };
    let err_str = err.to_string();
    assert!(
        err_str.contains("invalid DDC/CI device"),
        "Error message should contain 'invalid DDC/CI device', got: {}",
        err_str
    );
    assert!(
        err_str.contains("/dev/i2c-5"),
        "Error message should contain the path, got: {}",
        err_str
    );
}

/// Test DdcError::InvalidDevice with different paths
#[test]
fn test_ddc_error_invalid_device_various_paths() {
    let paths = vec![
        "/dev/i2c-0",
        "/dev/i2c-3",
        "\\\\.\\DISPLAY1",
        "\\\\.\\DISPLAY2",
    ];

    for path in paths {
        let err = DdcError::InvalidDevice {
            path: path.to_string(),
        };
        assert!(
            err.to_string().contains(path),
            "Error should contain path: {}",
            path
        );
    }
}

// ============================================================================
// DdcError::BackendNotAvailable Tests
// ============================================================================

/// Test DdcError::BackendNotAvailable error message
#[test]
fn test_ddc_error_backend_not_available() {
    let err = DdcError::BackendNotAvailable {
        details: "No I2C devices found".to_string(),
    };
    let err_str = err.to_string();
    assert!(
        err_str.contains("backend not available"),
        "Error should mention backend unavailability, got: {}",
        err_str
    );
    assert!(
        err_str.contains("No I2C devices found"),
        "Error should contain details, got: {}",
        err_str
    );
}

/// Test DdcError::BackendNotAvailable with various detail messages
#[test]
fn test_ddc_error_backend_not_available_various_details() {
    let details = vec![
        "No DDC/CI-capable monitors connected",
        "Platform not supported",
        "Missing required permissions",
    ];

    for detail in details {
        let err = DdcError::BackendNotAvailable {
            details: detail.to_string(),
        };
        assert!(
            err.to_string().contains(detail),
            "Error should contain detail: {}",
            detail
        );
    }
}

// ============================================================================
// DdcError::CommunicationFailed Tests
// ============================================================================

/// Test DdcError::CommunicationFailed error message
#[test]
fn test_ddc_error_communication_failed() {
    let err = DdcError::CommunicationFailed {
        reason: "I2C bus timeout".to_string(),
    };
    let err_str = err.to_string();
    assert!(
        err_str.contains("communication failed"),
        "Error should mention communication failure, got: {}",
        err_str
    );
    assert!(
        err_str.contains("I2C bus timeout"),
        "Error should contain reason, got: {}",
        err_str
    );
}

/// Test DdcError::CommunicationFailed with various reasons
#[test]
fn test_ddc_error_communication_failed_various_reasons() {
    let reasons = vec![
        "Device disconnected",
        "Checksum mismatch",
        "Invalid response from monitor",
    ];

    for reason in reasons {
        let err = DdcError::CommunicationFailed {
            reason: reason.to_string(),
        };
        assert!(
            err.to_string().contains(reason),
            "Error should contain reason: {}",
            reason
        );
    }
}

// ============================================================================
// DdcError::AccessDenied Tests
// ============================================================================

/// Test DdcError::AccessDenied error message
#[test]
fn test_ddc_error_access_denied() {
    let err = DdcError::AccessDenied;
    let err_str = err.to_string();
    assert!(
        err_str.contains("access denied") || err_str.contains("permissions"),
        "Error should mention access/permissions, got: {}",
        err_str
    );
}

// ============================================================================
// DdcError::UnsupportedFeature Tests
// ============================================================================

/// Test DdcError::UnsupportedFeature error message
#[test]
fn test_ddc_error_unsupported_feature() {
    let err = DdcError::UnsupportedFeature("HDR".into());
    let err_str = err.to_string();
    assert!(
        err_str.contains("not supported"),
        "Error should mention feature not supported, got: {}",
        err_str
    );
}

// ============================================================================
// DdcError to DisplayManagerError Conversion Tests
// ============================================================================

/// Test DdcError to DisplayManagerError conversion for InvalidDevice.
///
/// Note: `DdcError` is a type alias for `DisplayManagerError`, so `.into()`
/// preserves the variant unchanged.
#[test]
fn test_ddc_error_to_display_manager_error_invalid_device() {
    let ddc_err = DdcError::InvalidDevice {
        path: "test-device".to_string(),
    };
    let dm_err: DisplayManagerError = ddc_err.into();
    match &dm_err {
        DisplayManagerError::InvalidDevice { path } => {
            assert!(path.contains("test-device"));
        }
        other => panic!("Expected InvalidDevice, got: {:?}", other),
    }
}

/// Test DdcError to DisplayManagerError conversion for BackendNotAvailable.
///
/// Note: `DdcError` is a type alias for `DisplayManagerError`, so `.into()`
/// preserves the variant unchanged.
#[test]
fn test_ddc_error_to_display_manager_error_backend_not_available() {
    let ddc_err = DdcError::BackendNotAvailable {
        details: "Linux backend unavailable".to_string(),
    };
    let dm_err: DisplayManagerError = ddc_err.into();
    match &dm_err {
        DisplayManagerError::BackendNotAvailable { details } => {
            assert!(details.contains("Linux backend unavailable"));
        }
        other => panic!("Expected BackendNotAvailable, got: {:?}", other),
    }
}

/// Test DdcError to DisplayManagerError conversion for CommunicationFailed.
///
/// Note: `DdcError` is a type alias for `DisplayManagerError`, so `.into()`
/// preserves the variant unchanged.
#[test]
fn test_ddc_error_to_display_manager_error_communication_failed() {
    let ddc_err = DdcError::CommunicationFailed {
        reason: "timeout".to_string(),
    };
    let dm_err: DisplayManagerError = ddc_err.into();
    match &dm_err {
        DisplayManagerError::CommunicationFailed { reason } => {
            assert!(reason.contains("timeout"));
        }
        other => panic!("Expected CommunicationFailed, got: {:?}", other),
    }
}

/// Test DdcError to DisplayManagerError conversion for AccessDenied.
///
/// Note: `DdcError` is a type alias for `DisplayManagerError`, so `.into()`
/// preserves the variant unchanged.
#[test]
fn test_ddc_error_to_display_manager_error_access_denied() {
    let ddc_err = DdcError::AccessDenied;
    let dm_err: DisplayManagerError = ddc_err.into();
    assert!(
        dm_err.to_string().contains("access denied"),
        "AccessDenied message should mention access denied"
    );
    match &dm_err {
        DisplayManagerError::AccessDenied => {}
        other => panic!("Expected AccessDenied, got: {:?}", other),
    }
}

/// Test DdcError to DisplayManagerError conversion for UnsupportedFeature.
///
/// Note: `DdcError` is a type alias for `DisplayManagerError`, so `.into()`
/// preserves the variant unchanged.
#[test]
fn test_ddc_error_to_display_manager_error_unsupported_feature() {
    let ddc_err = DdcError::UnsupportedFeature("VCP 0xFF".into());
    let dm_err: DisplayManagerError = ddc_err.into();
    match &dm_err {
        DisplayManagerError::UnsupportedFeature(msg) => {
            assert!(msg.contains("VCP"));
        }
        other => panic!("Expected UnsupportedFeature, got: {:?}", other),
    }
}
