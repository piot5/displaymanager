use df_displmgr::error::DisplayError;
use df_displmgr::types::DisplayId;

/// Test DisplayError::ConnectionFailed
#[test]
fn test_connection_failed_error() {
    let err = DisplayError::ConnectionFailed;
    assert!(err.to_string().contains("failed to connect"));
}

/// Test DisplayError::NotFound
#[test]
fn test_not_found_error() {
    let id = DisplayId("HDMI-1".to_string());
    let err = DisplayError::NotFound(id.clone());
    assert!(err.to_string().contains("not found"));
    assert!(err.to_string().contains("HDMI-1"));
}

/// Test DisplayError::ConfigurationRejected
#[test]
fn test_configuration_rejected_error() {
    let err = DisplayError::ConfigurationRejected;
    assert!(err.to_string().contains("rejected"));
}

/// Test DisplayError::HdrError
#[test]
fn test_hdr_error() {
    let err = DisplayError::HdrError("invalid HDR metadata".to_string());
    assert!(err.to_string().contains("HDR error"));
    assert!(err.to_string().contains("invalid HDR metadata"));
}

/// Test DisplayError::UnsupportedFeature
#[test]
fn test_unsupported_feature_error() {
    let err = DisplayError::UnsupportedFeature("color management".to_string());
    assert!(err.to_string().contains("feature not supported"));
    assert!(err.to_string().contains("color management"));
}

/// Test DisplayError::UnsupportedHardware
#[test]
fn test_unsupported_hardware_error() {
    let err = DisplayError::UnsupportedHardware("HDR not available".to_string());
    assert!(err.to_string().contains("hardware does not support"));
    assert!(err.to_string().contains("HDR not available"));
}

/// Test DisplayError::UnsupportedPlatform
#[test]
fn test_unsupported_platform_error() {
    let err = DisplayError::UnsupportedPlatform("force_all on Linux".to_string());
    assert!(err.to_string().contains("operation not supported"));
    assert!(err.to_string().contains("force_all on Linux"));
}

/// Test DisplayError::BackendError
#[test]
fn test_backend_error() {
    let err = DisplayError::BackendError("Win32 error 0x80070005".to_string());
    assert!(err.to_string().contains("platform-specific backend error"));
    assert!(err.to_string().contains("Win32 error"));
}

/// Test DisplayError::OutputDisabled
#[test]
fn test_output_disabled_error() {
    let id = DisplayId("DP-1".to_string());
    let err = DisplayError::OutputDisabled(id.clone());
    assert!(err.to_string().contains("currently disabled"));
    assert!(err.to_string().contains("DP-1"));
}

/// Test DisplayError::StaleTopology
#[test]
fn test_stale_topology_error() {
    let err = DisplayError::StaleTopology;
    assert!(err.to_string().contains("stale"));
    assert!(err.to_string().contains("re-acquire"));
}

/// Test DisplayError::PermissionDenied
#[test]
fn test_permission_denied_error() {
    let err = DisplayError::PermissionDenied;
    assert!(err.to_string().contains("insufficient privileges"));
}

/// Test DisplayError::Timeout
#[test]
fn test_timeout_error() {
    let err = DisplayError::Timeout { timeout_ms: 5000 };
    assert!(err.to_string().contains("timed out"));
    assert!(err.to_string().contains("5000"));
}

/// Test DisplayError::Serialization
#[test]
fn test_serialization_error() {
    let err = DisplayError::Serialization("JSON parse failed".to_string());
    assert!(err.to_string().contains("serialization error"));
    assert!(err.to_string().contains("JSON parse failed"));
}

/// Test DisplayError::Io (from std::io::Error)
#[test]
fn test_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = DisplayError::Io(io_err);
    assert!(err.to_string().contains("I/O error"));
    assert!(err.to_string().contains("file not found"));
}

/// Test error Display trait implementation
#[test]
fn test_error_display_trait() {
    let errors: Vec<DisplayError> = vec![
        DisplayError::ConnectionFailed,
        DisplayError::NotFound(DisplayId("test".into())),
        DisplayError::ConfigurationRejected,
        DisplayError::HdrError("test".into()),
        DisplayError::UnsupportedFeature("test".into()),
        DisplayError::UnsupportedHardware("test".into()),
        DisplayError::UnsupportedPlatform("test".into()),
        DisplayError::BackendError("test".into()),
        DisplayError::OutputDisabled(DisplayId("test".into())),
        DisplayError::StaleTopology,
        DisplayError::PermissionDenied,
        DisplayError::Timeout { timeout_ms: 100 },
        DisplayError::Serialization("test".into()),
    ];

    // All errors should be displayable
    for err in errors {
        let _ = format!("{}", err);
    }
}

/// Test error Debug trait implementation
#[test]
fn test_error_debug_trait() {
    let err = DisplayError::NotFound(DisplayId("HDMI-1".into()));
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("NotFound"));
}

/// Test DisplayResult type alias
#[test]
fn test_display_result_type() {
    fn example_ok() -> df_displmgr::DisplayResult<String> {
        Ok("success".to_string())
    }

    fn example_err() -> df_displmgr::DisplayResult<String> {
        Err(DisplayError::ConnectionFailed)
    }

    assert!(example_ok().is_ok());
    assert!(example_err().is_err());
}

/// Test error propagation with ?
#[test]
fn test_error_propagation() {
    fn might_fail(fail: bool) -> df_displmgr::DisplayResult<()> {
        if fail {
            Err(DisplayError::Timeout { timeout_ms: 1000 })
        } else {
            Ok(())
        }
    }

    fn propagate(fail: bool) -> df_displmgr::DisplayResult<()> {
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
        DisplayError::ConnectionFailed,
        DisplayError::StaleTopology,
        DisplayError::PermissionDenied,
    ];

    for err in errors {
        match err {
            DisplayError::ConnectionFailed => assert!(true),
            DisplayError::StaleTopology => assert!(true),
            DisplayError::PermissionDenied => assert!(true),
            _ => panic!("Unexpected error variant"),
        }
    }
}

/// Test error with different DisplayId values
#[test]
fn test_error_with_display_ids() {
    let ids = vec!["HDMI-1", "DP-1", "eDP-1", "VGA-1"];

    for id_str in ids {
        let id = DisplayId(id_str.to_string());
        let err = DisplayError::NotFound(id.clone());
        assert!(err.to_string().contains(id_str));

        let err2 = DisplayError::OutputDisabled(id);
        assert!(err2.to_string().contains(id_str));
    }
}

/// Test timeout with various values
#[test]
fn test_timeout_various_values() {
    let timeouts = vec![0, 1, 100, 1000, 60000];

    for timeout in timeouts {
        let err = DisplayError::Timeout {
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
    let display_err: df_displmgr::DisplayError = io_err.into();

    match display_err {
        df_displmgr::DisplayError::Io(_) => assert!(true),
        _ => panic!("Expected Io error variant"),
    }
}

/// Test error messages are descriptive
#[test]
fn test_error_messages_descriptive() {
    let test_cases = vec![
        (DisplayError::ConnectionFailed, "connect"),
        (DisplayError::ConfigurationRejected, "rejected"),
        (DisplayError::StaleTopology, "re-acquire"),
        (DisplayError::PermissionDenied, "privileges"),
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
