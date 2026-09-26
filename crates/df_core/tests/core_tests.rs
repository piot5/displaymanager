use df_core::{DisplayManagerError, InputSource, MonitorCapabilities, PowerState, VcpCode};

// ============================================================================
// DisplayManagerError Tests
// ============================================================================

#[test]
fn test_display_manager_error_ddc_communication() {
    let err = DisplayManagerError::CommunicationFailed {
        reason: "timeout".into(),
    };
    assert!(
        format!("{}", err).contains("communication failed"),
        "Error message should contain 'communication failed'"
    );
    assert!(
        format!("{}", err).contains("timeout"),
        "Error message should contain 'timeout'"
    );
}

#[test]
fn test_display_manager_error_unsupported_platform() {
    let err = DisplayManagerError::UnsupportedPlatform("macOS".into());
    assert!(
        format!("{}", err).contains("not supported"),
        "Error should mention not supported"
    );
}

#[test]
fn test_display_manager_error_unsupported_feature() {
    let err = DisplayManagerError::UnsupportedFeature("VCP 0xFF".into());
    assert!(
        format!("{}", err).contains("not supported"),
        "Error should mention not supported"
    );
}

#[test]
fn test_display_manager_error_stale_topology() {
    let err = DisplayManagerError::StaleTopology;
    assert!(
        format!("{}", err).contains("stale"),
        "Error should mention stale"
    );
}

#[test]
fn test_display_manager_error_timeout() {
    let err = DisplayManagerError::Timeout { timeout_ms: 5000 };
    assert!(
        format!("{}", err).contains("timeout after 5000 ms"),
        "Error should contain timeout duration"
    );
}

#[test]
fn test_display_manager_error_io() {
    let err: DisplayManagerError =
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied").into();
    assert!(
        format!("{}", err).contains("I/O error"),
        "Error should mention I/O error"
    );
    assert!(
        format!("{}", err).contains("access denied"),
        "Error should contain the reason"
    );
}

// ============================================================================
// VcpCode Tests
// ============================================================================

#[test]
fn test_vcp_code_equality() {
    assert_eq!(
        VcpCode::Brightness,
        VcpCode::Brightness,
        "Same VCP codes should be equal"
    );
    assert_ne!(
        VcpCode::Brightness,
        VcpCode::Contrast,
        "Different VCP codes should not be equal"
    );
}

// ============================================================================
// InputSource Tests
// ============================================================================

#[test]
fn test_input_source_from_vcp_values() {
    assert_eq!(InputSource::from_vcp_value(0x01), InputSource::AnalogVga);
    assert_eq!(InputSource::from_vcp_value(0x03), InputSource::Dvi);
    assert_eq!(InputSource::from_vcp_value(0x05), InputSource::Composite);
    assert_eq!(InputSource::from_vcp_value(0x06), InputSource::SVideo);
    assert_eq!(InputSource::from_vcp_value(0x0F), InputSource::DisplayPort1);
    assert_eq!(InputSource::from_vcp_value(0x11), InputSource::Hdmi1);
    assert_eq!(InputSource::from_vcp_value(0x12), InputSource::Hdmi2);
    assert_eq!(InputSource::from_vcp_value(0xFF), InputSource::Unknown);
}

#[test]
fn test_input_source_to_vcp_value() {
    assert_eq!(InputSource::DisplayPort1.to_vcp_value(), 0x0F);
    assert_eq!(InputSource::Hdmi1.to_vcp_value(), 0x11);
    assert_eq!(InputSource::Dvi.to_vcp_value(), 0x03);
    assert_eq!(InputSource::AnalogVga.to_vcp_value(), 0x01);
}

#[test]
fn test_input_source_roundtrip() {
    let sources = vec![
        InputSource::DisplayPort1,
        InputSource::Hdmi1,
        InputSource::Dvi,
        InputSource::AnalogVga,
        InputSource::Composite,
        InputSource::SVideo,
    ];
    for source in sources {
        let value = source.to_vcp_value();
        let recovered = InputSource::from_vcp_value(value);
        assert_eq!(
            source, recovered,
            "InputSource roundtrip failed for {:?}",
            source
        );
    }
}

#[test]
fn test_input_source_serialization() {
    let source = InputSource::Hdmi1;
    let json = serde_json::to_string(&source).expect("Serialization should succeed");
    let recovered: InputSource =
        serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(source, recovered);
}

// ============================================================================
// PowerState Tests
// ============================================================================

#[test]
fn test_power_state_from_vcp_values() {
    assert_eq!(PowerState::from_vcp_value(0x00), PowerState::Unknown);
    assert_eq!(PowerState::from_vcp_value(0x01), PowerState::On);
    assert_eq!(PowerState::from_vcp_value(0x02), PowerState::Standby);
    assert_eq!(PowerState::from_vcp_value(0x03), PowerState::Suspend);
    assert_eq!(PowerState::from_vcp_value(0x04), PowerState::Off);
    // Out-of-range values should return Unknown
    assert_eq!(PowerState::from_vcp_value(0xFF), PowerState::Unknown);
    assert_eq!(PowerState::from_vcp_value(0x10), PowerState::Unknown);
}

#[test]
fn test_power_state_to_vcp_value() {
    assert_eq!(PowerState::On.to_vcp_value(), 0x01);
    assert_eq!(PowerState::Standby.to_vcp_value(), 0x02);
    assert_eq!(PowerState::Off.to_vcp_value(), 0x04);
}

#[test]
fn test_power_state_roundtrip() {
    let states = vec![
        PowerState::On,
        PowerState::Standby,
        PowerState::Suspend,
        PowerState::Off,
    ];
    for state in states {
        let value = state.to_vcp_value();
        let recovered = PowerState::from_vcp_value(value);
        assert_eq!(
            state, recovered,
            "PowerState roundtrip failed for {:?}",
            state
        );
    }
}

#[test]
fn test_power_state_serialization() {
    let state = PowerState::On;
    let json = serde_json::to_string(&state).expect("Serialization should succeed");
    let recovered: PowerState =
        serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(state, recovered);
}

// ============================================================================
// MonitorCapabilities Tests
// ============================================================================

#[test]
fn test_monitor_capabilities_defaults() {
    let caps = MonitorCapabilities::default();
    assert_eq!(caps.brightness, 50);
    assert_eq!(caps.brightness_min, 0);
    assert_eq!(caps.brightness_max, 100);
    assert_eq!(caps.contrast, 50);
    assert_eq!(caps.contrast_min, 0);
    assert_eq!(caps.contrast_max, 100);
}

#[test]
fn test_monitor_capabilities_boundary() {
    // Brightness at max
    let caps = MonitorCapabilities {
        brightness: 100,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 50,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(caps.brightness, 100);

    // Contrast at max, brightness at min
    let caps = MonitorCapabilities {
        brightness: 0,
        brightness_min: 0,
        brightness_max: 50,
        contrast: 100,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(caps.contrast, 100);
    assert_eq!(caps.brightness, 0);

    // No brightness support
    let caps = MonitorCapabilities {
        brightness: 0,
        brightness_min: 0,
        brightness_max: 0,
        contrast: 50,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(caps.brightness_max, 0);

    // No contrast support
    let caps = MonitorCapabilities {
        brightness: 50,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 0,
        contrast_min: 0,
        contrast_max: 0,
    };
    assert_eq!(caps.contrast_max, 0);
}

#[test]
fn test_monitor_capabilities_serialization() {
    let caps = MonitorCapabilities::default();
    let json = serde_json::to_string(&caps).expect("Serialization should succeed");
    let recovered: MonitorCapabilities =
        serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(caps.brightness, recovered.brightness);
    assert_eq!(caps.contrast, recovered.contrast);
}

// ============================================================================
// Additional VcpCode Tests
// ============================================================================

// ============================================================================
// Missing InputSource Variants Tests
// ============================================================================

#[test]
fn test_input_source_hdmi2() {
    let source = InputSource::from_vcp_value(0x12);
    assert_eq!(
        source,
        InputSource::Hdmi2,
        "VCP value 0x12 should map to Hdmi2"
    );
}

#[test]
fn test_input_source_display_port2() {
    let source = InputSource::from_vcp_value(0x10);
    assert_eq!(
        source,
        InputSource::DisplayPort2,
        "VCP value 0x10 should map to DisplayPort2"
    );
}

#[test]
fn test_input_source_usbc() {
    let source = InputSource::from_vcp_value(0x13);
    assert_eq!(
        source,
        InputSource::UsbC,
        "VCP value 0x13 should map to UsbC"
    );
}

// ============================================================================
// Exhaustive InputSource Tests (all 256 values)
// ============================================================================

#[test]
fn test_input_source_from_vcp_value_exhaustive() {
    // Test all 256 possible input values
    for value in 0u8..=0xFF {
        let source = InputSource::from_vcp_value(value);
        // All values should return a valid InputSource variant (no panic)
        match source {
            InputSource::Unknown
            | InputSource::AnalogVga
            | InputSource::Dvi
            | InputSource::Composite
            | InputSource::SVideo
            | InputSource::DisplayPort1
            | InputSource::DisplayPort2
            | InputSource::Hdmi1
            | InputSource::Hdmi2
            | InputSource::UsbC => {}
        }
    }
}

#[test]
fn test_input_source_known_values_coverage() {
    // Verify all defined variants map correctly
    assert_eq!(InputSource::Unknown.to_vcp_value(), 0x00);
    assert_eq!(InputSource::AnalogVga.to_vcp_value(), 0x01);
    assert_eq!(InputSource::Dvi.to_vcp_value(), 0x03);
    assert_eq!(InputSource::Composite.to_vcp_value(), 0x05);
    assert_eq!(InputSource::SVideo.to_vcp_value(), 0x06);
    assert_eq!(InputSource::DisplayPort1.to_vcp_value(), 0x0F);
    assert_eq!(InputSource::DisplayPort2.to_vcp_value(), 0x10);
    assert_eq!(InputSource::Hdmi1.to_vcp_value(), 0x11);
    assert_eq!(InputSource::Hdmi2.to_vcp_value(), 0x12);
    assert_eq!(InputSource::UsbC.to_vcp_value(), 0x13);
}

// ============================================================================
// Exhaustive PowerState Tests (all 256 values)
// ============================================================================

#[test]
fn test_power_state_from_vcp_value_exhaustive() {
    // Test all 256 possible power values
    for value in 0u8..=0xFF {
        let state = PowerState::from_vcp_value(value);
        // All values should return a valid PowerState variant (no panic)
        match state {
            PowerState::On
            | PowerState::Off
            | PowerState::Standby
            | PowerState::Suspend
            | PowerState::Unknown => {}
        }
    }
}

// ============================================================================
// Exhaustive VcpCode Tests (all 256 values)
// ============================================================================

#[test]
fn test_vcp_code_from_u8_exhaustive() {
    // Test all 256 possible VCP codes
    for code in 0u8..=0xFF {
        // Should never panic - either Some or None
        let _ = VcpCode::from_u8(code);
    }
}

#[test]
fn test_vcp_code_known_codes_coverage() {
    // Verify all defined VcpCode variants map correctly
    assert_eq!(VcpCode::ActiveControl.to_u8(), 0x0E);
    assert_eq!(VcpCode::Active0Focus.to_u8(), 0x0F);
    assert_eq!(VcpCode::Brightness.to_u8(), 0x10);
    assert_eq!(VcpCode::Contrast.to_u8(), 0x12);
    assert_eq!(VcpCode::Active0Hue.to_u8(), 0x14);
    assert_eq!(VcpCode::Active0Saturation.to_u8(), 0x15);
    assert_eq!(VcpCode::RedGain.to_u8(), 0x16);
    assert_eq!(VcpCode::GreenGain.to_u8(), 0x18);
    assert_eq!(VcpCode::BlueGain.to_u8(), 0x1A);
    assert_eq!(VcpCode::Clock.to_u8(), 0x3E);
    assert_eq!(VcpCode::InputSource.to_u8(), 0x60);
    assert_eq!(VcpCode::Volume.to_u8(), 0x62);
    assert_eq!(VcpCode::AudioMute.to_u8(), 0x8D);
    assert_eq!(VcpCode::HorizontalFrequency.to_u8(), 0xAC);
    assert_eq!(VcpCode::VerticalFrequency.to_u8(), 0xAE);
    assert_eq!(VcpCode::FlatPanelType.to_u8(), 0xB6);
    assert_eq!(VcpCode::OperatingHours.to_u8(), 0xC0);
    assert_eq!(VcpCode::OsdLanguage.to_u8(), 0xCC);
    assert_eq!(VcpCode::PowerMode.to_u8(), 0xD6);
    assert_eq!(VcpCode::Sharpness.to_u8(), 0xD9);
}
