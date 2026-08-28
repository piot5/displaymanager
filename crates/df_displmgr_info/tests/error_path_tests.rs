//! Error path tests for EdidError variants
//!
//! These tests validate error handling and propagation for all EdidError variants.

use df_displmgr_info::error::EdidError;
use df_core::DisplayManagerError;

// ============================================================================
// EdidError::AccessDenied Tests
// ============================================================================

/// Test EdidError::AccessDenied error message
#[test]
fn test_edid_error_access_denied() {
    let err = EdidError::AccessDenied;
    let err_str = err.to_string();
    assert!(
        err_str.contains("access denied") || err_str.contains("denied"),
        "Error should mention access denied, got: {}",
        err_str
    );
}

/// Test EdidError::AccessDenied error context
#[test]
fn test_edid_error_access_denied_context() {
    // On Linux this typically means missing i2c group membership
    // On Windows this indicates insufficient session rights
    let err = EdidError::AccessDenied;
    assert!(!err.to_string().is_empty(), "Error message should not be empty");
}

// ============================================================================
// EdidError::BackendNotAvailable Tests
// ============================================================================

#[test]
fn test_edid_error_backend_not_available() {
    let err = EdidError::BackendNotAvailable;
    let err_str = err.to_string();
    assert!(
        err_str.contains("backend not available") || err_str.contains("not available"),
        "Error should mention backend unavailable, got: {}",
        err_str
    );
}

// ============================================================================
// EdidError::CommunicationFailed Tests
// ============================================================================

#[test]
fn test_edid_error_communication_failed() {
    let err = EdidError::CommunicationFailed;
    let err_str = err.to_string();
    assert!(
        err_str.contains("communication") || err_str.contains("failed"),
        "Error should mention communication failure, got: {}",
        err_str
    );
}

// ============================================================================
// EdidError::ParseError Tests
// ============================================================================

#[test]
fn test_edid_error_parse_error() {
    let err = EdidError::ParseError;
    let err_str = err.to_string();
    assert!(
        err_str.contains("parse") || err_str.contains("EDID"),
        "Error should mention parse error, got: {}",
        err_str
    );
}

// ============================================================================
// EdidError::NotFound Tests
// ============================================================================

#[test]
fn test_edid_error_not_found() {
    let err = EdidError::NotFound;
    let err_str = err.to_string();
    assert!(
        err_str.contains("not found"),
        "Error should mention not found, got: {}",
        err_str
    );
}

// ============================================================================
// EdidError::DdcError Tests
// ============================================================================

#[test]
fn test_edid_error_ddc_error() {
    let err = EdidError::DdcError("I2C bus error".to_string());
    let err_str = err.to_string();
    assert!(
        err_str.contains("DDC error") && err_str.contains("I2C bus error"),
        "Error should mention DDC error and contain the message, got: {}",
        err_str
    );
}

// ============================================================================
// EdidError to DisplayManagerError Conversion Tests
// ============================================================================

#[test]
fn test_edid_error_to_display_manager_error_access_denied() {
    let edid_err = EdidError::AccessDenied;
    let dm_err: DisplayManagerError = edid_err.into();
    match &dm_err {
        DisplayManagerError::AccessDenied => {
            assert!(
                dm_err.to_string().contains("access denied"),
                "AccessDenied error should mention access denial, got: {}",
                dm_err
            );
        }
        other => panic!("Expected AccessDenied error, got: {:?}", other),
    }
}

#[test]
fn test_edid_error_to_display_manager_error_backend_not_available() {
    let edid_err = EdidError::BackendNotAvailable;
    let dm_err: DisplayManagerError = edid_err.into();
    match dm_err {
        DisplayManagerError::UnsupportedPlatform(msg) => {
            assert!(msg.contains("backend not available"));
        }
        _ => panic!("Expected UnsupportedPlatform error, got: {:?}", dm_err),
    }
}

#[test]
fn test_edid_error_to_display_manager_error_communication_failed() {
    let edid_err = EdidError::CommunicationFailed;
    let dm_err: DisplayManagerError = edid_err.into();
    match dm_err {
        DisplayManagerError::CommunicationFailed { reason } => {
            assert!(
                reason.contains("communication failed") || reason.contains("EDID communication"),
                "CommunicationFailed error should contain appropriate message, got: {}",
                reason
            );
        }
        other => panic!("Expected CommunicationFailed error, got: {:?}", other),
    }
}

#[test]
fn test_edid_error_to_display_manager_error_ddc_error() {
    let edid_err = EdidError::DdcError("I/O error".to_string());
    let dm_err: DisplayManagerError = edid_err.into();
    match dm_err {
        DisplayManagerError::CommunicationFailed { reason } => {
            assert!(reason.contains("I/O error"));
        }
        other => panic!("Expected CommunicationFailed error, got: {:?}", other),
    }
}

#[test]
fn test_edid_error_to_display_manager_error_parse_error() {
    let edid_err = EdidError::ParseError;
    let dm_err: DisplayManagerError = edid_err.into();
    match dm_err {
        DisplayManagerError::EdidParse(msg) => {
            assert!(msg.contains("parse error"));
        }
        _ => panic!("Expected EdidParse error, got: {:?}", dm_err),
    }
}

#[test]
fn test_edid_error_to_display_manager_error_not_found() {
    let edid_err = EdidError::NotFound;
    let dm_err: DisplayManagerError = edid_err.into();
    match dm_err {
        DisplayManagerError::MonitorNotFound(msg) => {
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected MonitorNotFound error, got: {:?}", dm_err),
    }
}