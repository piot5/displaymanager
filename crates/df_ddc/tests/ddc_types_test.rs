use df_ddc::ddc_types::MonitorCapabilities;

/// Test MonitorCapabilities debug formatting
#[test]
fn test_monitor_capabilities_debug() {
    let caps = MonitorCapabilities {
        brightness: 50,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 75,
        contrast_min: 0,
        contrast_max: 100,
    };

    let debug_str = format!("{:?}", caps);
    assert!(debug_str.contains("brightness"));
    assert!(debug_str.contains("50"));
    assert!(debug_str.contains("100"));
}

/// Test MonitorCapabilities partial equality
#[test]
fn test_monitor_capabilities_equality() {
    let caps1 = MonitorCapabilities {
        brightness: 50,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 75,
        contrast_min: 0,
        contrast_max: 100,
    };

    let caps2 = MonitorCapabilities {
        brightness: 50,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 75,
        contrast_min: 0,
        contrast_max: 100,
    };

    let caps3 = MonitorCapabilities {
        brightness: 60,
        brightness_min: 0,
        brightness_max: 100,
        contrast: 75,
        contrast_min: 0,
        contrast_max: 100,
    };

    assert_eq!(caps1, caps2);
    assert_ne!(caps1, caps3);
}
