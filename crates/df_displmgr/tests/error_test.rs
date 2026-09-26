use df_displmgr::error::DisplayManagerError;
use df_displmgr::types::DisplayId;

/// Test DisplayManagerError::ConnectionFailed
#[test]
fn test_connection_failed_error() {
    let err = DisplayManagerError::ConnectionFailed;
    assert!(err.to_string().contains("failed to connect"));
}

/// Test DisplayManagerError::MonitorNotFound
#[test]
fn test_not_found_error() {
    let id = DisplayId("HDMI-1".to_string());
    let err = DisplayManagerError::MonitorNotFound(id.to_string());
    assert!(err.to_string().contains("not found"));
    assert!(err.to_string().contains("HDMI-1"));
}

/// Test DisplayManagerError::ConfigurationRejected
#[test]
fn test_configuration_rejected_error() {
    let err = DisplayManagerError::ConfigurationRejected;
    assert!(err.to_string().contains("rejected"));
}

/// Test DisplayManagerError::HdrError
#[test]
fn test_hdr_error() {
    let err = DisplayManagerError::HdrError("invalid HDR metadata".to_string());
    assert!(err.to_string().contains("HDR error"));
    assert!(err.to_string().contains("invalid HDR metadata"));
}

/// Test DisplayManagerError::UnsupportedFeature
#[test]
fn test_unsupported_feature_error() {
    let err = DisplayManagerError::UnsupportedFeature("color management".to_string());
    assert!(err.to_string().contains("feature not supported"));
    assert!(err.to_string().contains("color management"));
}

/// Test DisplayManagerError::UnsupportedHardware
#[test]
fn test_unsupported_hardware_error() {
    let err = DisplayManagerError::UnsupportedHardware("HDR not available".to_string());
    assert!(err.to_string().contains("hardware does not support"));
    assert!(err.to_string().contains("HDR not available"));
}

/// Test DisplayManagerError::UnsupportedPlatform
#[test]
fn test_unsupported_platform_error() {
    let err = DisplayManagerError::UnsupportedPlatform("force_all on Linux".to_string());
    assert!(err.to_string().contains("operation not supported"));
    assert!(err.to_string().contains("force_all on Linux"));
}

/// Test DisplayManagerError::BackendError
#[test]
fn test_backend_error() {
    let err = DisplayManagerError::BackendError("Win32 error 0x80070005".to_string());
    assert!(err.to_string().contains("platform-specific backend error"));
    assert!(err.to_string().contains("Win32 error"));
}

/// Test DisplayManagerError::OutputDisabled
#[test]
fn test_output_disabled_error() {
    let id = DisplayId("DP-1".to_string());
    let err = DisplayManagerError::OutputDisabled(id.to_string());
    assert!(err.to_string().contains("currently disabled"));
    assert!(err.to_string().contains("DP-1"));
}

/// Test DisplayManagerError::StaleTopology
#[test]
fn test_stale_topology_error() {
    let err = DisplayManagerError::StaleTopology;
    assert!(err.to_string().contains("stale"));
    assert!(err.to_string().contains("re-acquire"));
}

/// Test DisplayManagerError::PermissionDenied
#[test]
fn test_permission_denied_error() {
    let err = DisplayManagerError::PermissionDenied;
    assert!(err.to_string().contains("insufficient privileges"));
}

/// Test DisplayManagerError::Timeout
#[test]
fn test_timeout_error() {
    let err = DisplayManagerError::Timeout { timeout_ms: 5000 };
    assert!(err.to_string().contains("timeout"));
    assert!(err.to_string().contains("5000"));
}

/// Test DisplayManagerError::Serialization
#[test]
fn test_serialization_error() {
    let err = DisplayManagerError::Serialization("JSON parse failed".to_string());
    assert!(err.to_string().contains("serialization error"));
    assert!(err.to_string().contains("JSON parse failed"));
}

/// Test DisplayManagerError::Io (from std::io::Error)
#[test]
fn test_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = DisplayManagerError::Io(io_err);
    assert!(err.to_string().contains("I/O error"));
    assert!(err.to_string().contains("file not found"));
}

/// Test error Display trait implementation
#[test]
fn test_error_display_trait() {
    let errors: Vec<DisplayManagerError> = vec![
        DisplayManagerError::ConnectionFailed,
        DisplayManagerError::MonitorNotFound("test".into()),
        DisplayManagerError::ConfigurationRejected,
        DisplayManagerError::HdrError("test".into()),
        DisplayManagerError::UnsupportedFeature("test".into()),
        DisplayManagerError::UnsupportedHardware("test".into()),
        DisplayManagerError::UnsupportedPlatform("test".into()),
        DisplayManagerError::BackendError("test".into()),
        DisplayManagerError::OutputDisabled("test".into()),
        DisplayManagerError::StaleTopology,
        DisplayManagerError::PermissionDenied,
        DisplayManagerError::Timeout { timeout_ms: 100 },
        DisplayManagerError::Serialization("test".into()),
    ];

    // All errors should be displayable
    for err in errors {
        let _ = format!("{}", err);
    }
}

/// Test error Debug trait implementation
#[test]
fn test_error_debug_trait() {
    let err = DisplayManagerError::MonitorNotFound("HDMI-1".into());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("NotFound"));
}

/// Test the canonical `Result<T, DisplayManagerError>` signature used by the API.
#[test]
fn test_display_result_type() {
    fn example_ok() -> Result<String, DisplayManagerError> {
        Ok("success".to_string())
    }

    fn example_err() -> Result<String, DisplayManagerError> {
        Err(DisplayManagerError::ConnectionFailed)
    }

    assert!(example_ok().is_ok());
    assert!(example_err().is_err());
}

/// Test error propagation with ?
#[test]
fn test_error_propagation() {
    fn might_fail(fail: bool) -> Result<(), DisplayManagerError> {
        if fail {
            Err(DisplayManagerError::Timeout { timeout_ms: 1000 })
        } else {
            Ok(())
        }
    }

    fn propagate(fail: bool) -> Result<(), DisplayManagerError> {
        might_fail(fail)?;
        Ok(())
    }

    assert!(propagate(false).is_ok());
    assert!(propagate(true).is_err());
}

/// Test multiple error variants in a match
#[test]
fn test_error_matching() {
    let errors = vec![
        DisplayManagerError::ConnectionFailed,
        DisplayManagerError::StaleTopology,
        DisplayManagerError::PermissionDenied,
    ];

    for err in errors {
        match err {
            DisplayManagerError::ConnectionFailed
            | DisplayManagerError::StaleTopology
            | DisplayManagerError::PermissionDenied => {}
            _ => panic!("Unexpected error variant: {:?}", err),
        }
    }
}

/// Test error with different DisplayId values
#[test]
fn test_error_with_display_ids() {
    let ids = vec!["HDMI-1", "DP-1", "eDP-1", "VGA-1"];

    for id_str in ids {
        let id = DisplayId(id_str.to_string());
        let err = DisplayManagerError::MonitorNotFound(id.to_string());
        assert!(err.to_string().contains(id_str));

        let err2 = DisplayManagerError::OutputDisabled(id.to_string());
        assert!(err2.to_string().contains(id_str));
    }
}

/// Test timeout with various values
#[test]
fn test_timeout_various_values() {
    let timeouts = vec![0, 1, 100, 1000, 60000];

    for timeout in timeouts {
        let err = DisplayManagerError::Timeout {
            timeout_ms: timeout,
        };
        let err_str = err.to_string();
        assert!(err_str.contains(&timeout.to_string()));
    }
}

/// Test error conversion from io::Error
#[test]
fn test_io_error_conversion() {
    use std::io;

    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let display_err: df_displmgr::DisplayManagerError = io_err.into();

    match display_err {
        df_displmgr::DisplayManagerError::Io(_) => {}
        _ => panic!("Expected Io error variant, got: {:?}", display_err),
    }
}

/// Test error messages are descriptive
#[test]
fn test_error_messages_descriptive() {
    let test_cases = vec![
        (DisplayManagerError::ConnectionFailed, "connect"),
        (DisplayManagerError::ConfigurationRejected, "rejected"),
        (DisplayManagerError::StaleTopology, "re-acquire"),
        (DisplayManagerError::PermissionDenied, "privileges"),
    ];

    for (err, expected_text) in test_cases {
        let err_str = err.to_string();
        assert!(
            err_str.contains(expected_text),
            "Error message '{}' should contain '{}'",
            err_str,
            expected_text
        );
    }
}
