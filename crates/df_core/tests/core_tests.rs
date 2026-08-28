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
    let err: DisplayManagerError = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied").into();
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
fn test_vcp_code_brightness() {
    assert_eq!(
        VcpCode::Brightness.to_u8(),
        0x10,
        "Brightness should map to VCP code 0x10"
    );
}

#[test]
fn test_vcp_code_contrast() {
    assert_eq!(
        VcpCode::Contrast.to_u8(),
        0x12,
        "Contrast should map to VCP code 0x12"
    );
}

#[test]
fn test_vcp_code_red_gain() {
    assert_eq!(
        VcpCode::RedGain.to_u8(),
        0x16,
        "RedGain should map to VCP code 0x16"
    );
}

#[test]
fn test_vcp_code_blue_gain() {
    assert_eq!(
        VcpCode::BlueGain.to_u8(),
        0x1A,
        "BlueGain should map to VCP code 0x1A"
    );
}

#[test]
fn test_vcp_code_active0_focus() {
    assert_eq!(
        VcpCode::Active0Focus.to_u8(),
        0x0F,
        "Active0Focus should map to VCP code 0x0F"
    );
}

#[test]
fn test_vcp_code_power_mode() {
    assert_eq!(
        VcpCode::PowerMode.to_u8(),
        0xD6,
        "PowerMode should map to VCP code 0xD6"
    );
}

#[test]
fn test_vcp_code_known_variants_coverage() {
    // All VcpCode values are guaranteed to be u8 since the enum is #[repr(u8)]
    // This test verifies all defined variants are covered
    let _codes: &[VcpCode] = &[
        VcpCode::Brightness,
        VcpCode::Contrast,
        VcpCode::RedGain,
        VcpCode::GreenGain,
        VcpCode::BlueGain,
        VcpCode::Active0Focus,
        VcpCode::PowerMode,
        VcpCode::InputSource,
        VcpCode::AudioMute,
        VcpCode::Clock,
    ];
    // Verify all expected values map correctly
    assert_eq!(VcpCode::Brightness.to_u8(), 0x10);
    assert_eq!(VcpCode::Contrast.to_u8(), 0x12);
}

#[test]
fn test_vcp_code_from_u8_known() {
    assert!(
        VcpCode::from_u8(0x10).is_some(),
        "VCP code 0x10 (Brightness) should parse"
    );
    assert!(
        VcpCode::from_u8(0x12).is_some(),
        "VCP code 0x12 (Contrast) should parse"
    );
    assert!(
        VcpCode::from_u8(0x16).is_some(),
        "VCP code 0x16 (RedGain) should parse"
    );
    assert!(
        VcpCode::from_u8(0x1A).is_some(),
        "VCP code 0x1A (BlueGain) should parse"
    );
    assert!(
        VcpCode::from_u8(0xD6).is_some(),
        "VCP code 0xD6 (PowerMode) should parse"
    );
    assert!(
        VcpCode::from_u8(0x60).is_some(),
        "VCP code 0x60 (InputSource) should parse"
    );
}

#[test]
fn test_vcp_code_from_u8_unknown() {
    assert!(
        VcpCode::from_u8(0x00).is_none(),
        "VCP code 0x00 should return None (unknown)"
    );
    assert!(
        VcpCode::from_u8(0xFF).is_none(),
        "VCP code 0xFF should return None (unknown)"
    );
}

#[test]
fn test_vcp_code_roundtrip() {
    let known = vec![0x10, 0x12, 0x16, 0x1A, 0xD6, 0x60, 0x0F];
    for byte in known {
        let code = VcpCode::from_u8(byte).expect("Known VCP code should parse");
        assert_eq!(
            code.to_u8(),
            byte,
            "VCP roundtrip failed for 0x{:02X}",
            byte
        );
    }
}

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
fn test_input_source_display_port1() {
    let source = InputSource::from_vcp_value(0x0F);
    assert_eq!(
        source,
        InputSource::DisplayPort1,
        "VCP value 0x0F should map to DisplayPort1"
    );
}

#[test]
fn test_input_source_hdmi1() {
    let source = InputSource::from_vcp_value(0x11);
    assert_eq!(source, InputSource::Hdmi1, "VCP value 0x11 should map to Hdmi1");
}

#[test]
fn test_input_source_dvi() {
    let source = InputSource::from_vcp_value(0x03);
    assert_eq!(
        source,
        InputSource::Dvi,
        "VCP value 0x03 should map to Dvi"
    );
}

#[test]
fn test_input_source_analog_vga() {
    let source = InputSource::from_vcp_value(0x01);
    assert_eq!(
        source,
        InputSource::AnalogVga,
        "VCP value 0x01 should map to AnalogVga"
    );
}

#[test]
fn test_input_source_composite() {
    let source = InputSource::from_vcp_value(0x05);
    assert_eq!(
        source,
        InputSource::Composite,
        "VCP value 0x05 should map to Composite"
    );
}

#[test]
fn test_input_source_svideo() {
    let source = InputSource::from_vcp_value(0x06);
    assert_eq!(
        source,
        InputSource::SVideo,
        "VCP value 0x06 should map to SVideo"
    );
}

#[test]
fn test_input_source_unknown() {
    assert_eq!(
        InputSource::from_vcp_value(0xFF),
        InputSource::Unknown,
        "Unknown VCP value should map to Unknown"
    );
}

#[test]
fn test_input_source_to_vcp_value() {
    assert_eq!(
        InputSource::DisplayPort1.to_vcp_value(),
        0x0F,
        "DisplayPort1 should map to VCP value 0x0F"
    );
    assert_eq!(
        InputSource::Hdmi1.to_vcp_value(),
        0x11,
        "Hdmi1 should map to VCP value 0x11"
    );
    assert_eq!(
        InputSource::Dvi.to_vcp_value(),
        0x03,
        "Dvi should map to VCP value 0x03"
    );
    assert_eq!(
        InputSource::AnalogVga.to_vcp_value(),
        0x01,
        "AnalogVga should map to VCP value 0x01"
    );
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
    let recovered: InputSource = serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(source, recovered);
}

#[test]
fn test_input_source_clone() {
    let source = InputSource::DisplayPort1;
    let cloned = source;
    assert_eq!(
        source, cloned,
        "Cloned InputSource should equal original"
    );
}

// ============================================================================
// PowerState Tests
// ============================================================================

#[test]
fn test_power_state_on() {
    assert_eq!(
        PowerState::from_vcp_value(0x01),
        PowerState::On,
        "VCP value 0x01 should map to On"
    );
}

#[test]
fn test_power_state_standby() {
    assert_eq!(
        PowerState::from_vcp_value(0x02),
        PowerState::Standby,
        "VCP value 0x02 should map to Standby"
    );
}

#[test]
fn test_power_state_suspend() {
    assert_eq!(
        PowerState::from_vcp_value(0x03),
        PowerState::Suspend,
        "VCP value 0x03 should map to Suspend"
    );
}

#[test]
fn test_power_state_off() {
    assert_eq!(
        PowerState::from_vcp_value(0x04),
        PowerState::Off,
        "VCP value 0x04 should map to Off"
    );
}

#[test]
fn test_power_state_unknown() {
    assert_eq!(
        PowerState::from_vcp_value(0x00),
        PowerState::Unknown,
        "Unknown VCP value should map to Unknown"
    );
}

#[test]
fn test_power_state_unknown_values() {
    // Values outside the defined range should return Unknown
    assert_eq!(
        PowerState::from_vcp_value(0xFF),
        PowerState::Unknown,
        "VCP value 0xFF should map to Unknown"
    );
    assert_eq!(
        PowerState::from_vcp_value(0x10),
        PowerState::Unknown,
        "VCP value 0x10 should map to Unknown"
    );
}

#[test]
fn test_power_state_to_vcp_value() {
    assert_eq!(
        PowerState::On.to_vcp_value(),
        0x01,
        "On should map to VCP value 0x01"
    );
    assert_eq!(
        PowerState::Standby.to_vcp_value(),
        0x02,
        "Standby should map to VCP value 0x02"
    );
    assert_eq!(
        PowerState::Off.to_vcp_value(),
        0x04,
        "Off should map to VCP value 0x04"
    );
}

#[test]
fn test_power_state_roundtrip() {
    let states = vec![PowerState::On, PowerState::Standby, PowerState::Suspend, PowerState::Off];
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
    let recovered: PowerState = serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(state, recovered);
}

// ============================================================================
// MonitorCapabilities Tests
// ============================================================================

#[test]
fn test_monitor_capabilities_defaults() {
    let caps = MonitorCapabilities::default();
    assert_eq!(
        caps.brightness, 50,
        "Default brightness should be 50"
    );
    assert_eq!(
        caps.brightness_min, 0,
        "Default brightness_min should be 0"
    );
    assert_eq!(
        caps.brightness_max, 100,
        "Default brightness_max should be 100"
    );
    assert_eq!(
        caps.contrast, 50,
        "Default contrast should be 50"
    );
    assert_eq!(
        caps.contrast_min, 0,
        "Default contrast_min should be 0"
    );
    assert_eq!(
        caps.contrast_max, 100,
        "Default contrast_max should be 100"
    );
}

#[test]
fn test_monitor_capabilities_brightness_boundary() {
    let caps = MonitorCapabilities {
        brightness: 100,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 50,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(
        caps.brightness, 100,
        "Brightness should be 100"
    );
}

#[test]
fn test_monitor_capabilities_contrast_boundary() {
    let caps = MonitorCapabilities {
        brightness: 0,
        brightness_min: 0,
        brightness_max: 50,
        contrast: 100,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(
        caps.contrast, 100,
        "Contrast should be 100"
    );
    assert_eq!(
        caps.brightness, 0,
        "Brightness should be 0"
    );
}

#[test]
fn test_monitor_capabilities_no_brightness_support() {
    let caps = MonitorCapabilities {
        brightness: 0,
        brightness_min: 0,
        brightness_max: 0,
        contrast: 50,
        contrast_min: 0,
        contrast_max: 100,
    };
    assert_eq!(
        caps.brightness_max, 0,
        "brightness_max should be 0 when unsupported"
    );
}

#[test]
fn test_monitor_capabilities_no_contrast_support() {
    let caps = MonitorCapabilities {
        brightness: 50,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 0,
        contrast_min: 0,
        contrast_max: 0,
    };
    assert_eq!(
        caps.contrast_max, 0,
        "contrast_max should be 0 when unsupported"
    );
}

#[test]
fn test_monitor_capabilities_serialization() {
    let caps = MonitorCapabilities::default();
    let json = serde_json::to_string(&caps).expect("Serialization should succeed");
    let recovered: MonitorCapabilities = serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(
        caps.brightness, recovered.brightness,
        "Serialized brightness should match"
    );
    assert_eq!(
        caps.contrast, recovered.contrast,
        "Serialized contrast should match"
    );
}

#[test]
fn test_monitor_capabilities_clone() {
    let caps = MonitorCapabilities::default();
    let cloned = caps;
    assert_eq!(
        caps.brightness, cloned.brightness,
        "Cloned brightness should match original"
    );
    assert_eq!(
        caps.contrast, cloned.contrast,
        "Cloned contrast should match original"
    );
}

// ============================================================================
// Additional VcpCode Tests
// ============================================================================

#[test]
fn test_vcp_code_active_control() {
    assert_eq!(
        VcpCode::ActiveControl.to_u8(),
        0x0E,
        "ActiveControl should map to VCP code 0x0E"
    );
}

#[test]
fn test_vcp_code_active0_hue_saturation() {
    assert_eq!(
        VcpCode::Active0Hue.to_u8(),
        0x14,
        "Active0Hue should map to VCP code 0x14"
    );
    assert_eq!(
        VcpCode::Active0Saturation.to_u8(),
        0x15,
        "Active0Saturation should map to VCP code 0x15"
    );
}

#[test]
fn test_vcp_code_audio_mute() {
    assert_eq!(
        VcpCode::AudioMute.to_u8(),
        0x8D,
        "AudioMute should map to VCP code 0x8D"
    );
}

#[test]
fn test_vcp_code_sharpness() {
    assert_eq!(
        VcpCode::Sharpness.to_u8(),
        0xD9,
        "Sharpness should map to VCP code 0xD9"
    );
}

#[test]
fn test_vcp_code_clock() {
    assert_eq!(
        VcpCode::Clock.to_u8(),
        0x3E,
        "Clock should map to VCP code 0x3E"
    );
}

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
            PowerState::On | PowerState::Off | PowerState::Standby | PowerState::Suspend | PowerState::Unknown => {}
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
