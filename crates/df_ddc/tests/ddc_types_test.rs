use df_ddc::ddc_types::{InputSource, MonitorCapabilities, PowerState, VcpCode};

/// Test MonitorCapabilities debug formatting
#[test]
fn test_monitor_capabilities_debug() {
    let caps = MonitorCapabilities {
        brightness: 50,
        brightness_max: 100,
        contrast: 75,
        contrast_max: 100,
    };

    let debug_str = format!("{:?}", caps);
    assert!(debug_str.contains("brightness"));
    assert!(debug_str.contains("50"));
    assert!(debug_str.contains("100"));
}

/// Test PowerState enum variants
#[test]
fn test_power_state_variants() {
    assert_eq!(PowerState::On as u32, 0x01);
    assert_eq!(PowerState::Off as u32, 0x04);
}

/// Test InputSource enum variants
#[test]
fn test_input_source_variants() {
    assert_eq!(InputSource::DisplayPort1 as u32, 0x0F);
    assert_eq!(InputSource::DisplayPort2 as u32, 0x10);
    assert_eq!(InputSource::Hdmi1 as u32, 0x11);
    assert_eq!(InputSource::Hdmi2 as u32, 0x12);
}

/// Test VcpCode enum values
#[test]
fn test_vcp_code_values() {
    assert_eq!(VcpCode::Brightness as u8, 0x10);
    assert_eq!(VcpCode::Contrast as u8, 0x12);
    assert_eq!(VcpCode::InputSource as u8, 0x60);
    assert_eq!(VcpCode::Volume as u8, 0x62);
    assert_eq!(VcpCode::RedGain as u8, 0x16);
    assert_eq!(VcpCode::GreenGain as u8, 0x18);
    assert_eq!(VcpCode::BlueGain as u8, 0x1A);
    assert_eq!(VcpCode::PowerMode as u8, 0xD6);
}

/// Test PowerState equality and comparison
#[test]
fn test_power_state_equality() {
    assert_eq!(PowerState::On, PowerState::On);
    assert_eq!(PowerState::Off, PowerState::Off);
    assert_ne!(PowerState::On, PowerState::Off);
}

/// Test InputSource equality and comparison
#[test]
fn test_input_source_equality() {
    assert_eq!(InputSource::Hdmi1, InputSource::Hdmi1);
    assert_eq!(InputSource::DisplayPort1, InputSource::DisplayPort1);
    assert_ne!(InputSource::Hdmi1, InputSource::DisplayPort1);
}

/// Test VcpCode equality
#[test]
fn test_vcp_code_equality() {
    assert_eq!(VcpCode::Brightness, VcpCode::Brightness);
    assert_eq!(VcpCode::RedGain, VcpCode::RedGain);
    assert_ne!(VcpCode::Brightness, VcpCode::Contrast);
}

/// Test MonitorCapabilities partial equality
#[test]
fn test_monitor_capabilities_equality() {
    let caps1 = MonitorCapabilities {
        brightness: 50,
        brightness_max: 100,
        contrast: 75,
        contrast_max: 100,
    };

    let caps2 = MonitorCapabilities {
        brightness: 50,
        brightness_max: 100,
        contrast: 75,
        contrast_max: 100,
    };

    let caps3 = MonitorCapabilities {
        brightness: 60,
        brightness_max: 100,
        contrast: 75,
        contrast_max: 100,
    };

    assert_eq!(caps1, caps2);
    assert_ne!(caps1, caps3);
}
