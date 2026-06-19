use df_displmgr_info::edid_types::{
    AudioCapabilities, AudioMuteState, ChromaticityCoordinates, DeepDdcStats, DigitalInterfaceType,
    EdidData, HdrMetadata, InputSource, MonitorCapabilities, MonitorMode, PowerState, VcpCode,
    VideoInterfaceInfo,
};

/// Test MonitorCapabilities serialization
#[test]
fn test_monitor_capabilities_serialization() {
    let caps = MonitorCapabilities {
        brightness: 75,
        brightness_max: 100,
        contrast: 50,
        contrast_max: 100,
    };

    let json = serde_json::to_string(&caps).unwrap();
    assert!(json.contains("brightness"));
    assert!(json.contains("contrast"));

    let parsed: MonitorCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.brightness, 75);
    assert_eq!(parsed.contrast, 50);
}

/// Test PowerState enum variants
#[test]
fn test_power_state_variants() {
    assert_eq!(PowerState::On as u32, 0x01);
    assert_eq!(PowerState::Standby as u32, 0x02);
    assert_eq!(PowerState::Suspend as u32, 0x03);
    assert_eq!(PowerState::Off as u32, 0x04);
    assert_eq!(PowerState::Unknown as u32, 0x00);
}

/// Test PowerState serialization
#[test]
fn test_power_state_serialization() {
    let json = serde_json::to_string(&PowerState::On).unwrap();
    assert_eq!(json, "\"On\"");

    let parsed: PowerState = serde_json::from_str("\"Off\"").unwrap();
    assert_eq!(parsed, PowerState::Off);
}

/// Test InputSource enum variants
#[test]
fn test_input_source_variants() {
    assert_eq!(InputSource::AnalogVga as u32, 0x01);
    assert_eq!(InputSource::Dvi as u32, 0x03);
    assert_eq!(InputSource::Hdmi1 as u32, 0x11);
    assert_eq!(InputSource::DisplayPort1 as u32, 0x0F);
    assert_eq!(InputSource::UsbC as u32, 0x13);
    assert_eq!(InputSource::Unknown as u32, 0x00);
}

/// Test InputSource serialization
#[test]
fn test_input_source_serialization_roundtrip() {
    let variants = vec![
        InputSource::AnalogVga,
        InputSource::Hdmi1,
        InputSource::DisplayPort1,
        InputSource::UsbC,
    ];

    for v in variants {
        let json = serde_json::to_string(&v).unwrap();
        let parsed: InputSource = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, v);
    }
}

/// Test AudioMuteState variants
#[test]
fn test_audio_mute_state_variants() {
    assert_eq!(AudioMuteState::Muted as u32, 0x01);
    assert_eq!(AudioMuteState::Unmuted as u32, 0x02);
    assert_eq!(AudioMuteState::Unknown as u32, 0x00);
}

/// Test DigitalInterfaceType variants
#[test]
fn test_digital_interface_type_variants() {
    assert_eq!(DigitalInterfaceType::Hdmi, DigitalInterfaceType::Hdmi);
    assert_eq!(
        DigitalInterfaceType::DisplayPort,
        DigitalInterfaceType::DisplayPort
    );
    assert_eq!(DigitalInterfaceType::Dvi, DigitalInterfaceType::Dvi);
    assert_eq!(DigitalInterfaceType::Unknown, DigitalInterfaceType::Unknown);
}

/// Test VideoInterfaceInfo::Digital
#[test]
fn test_video_interface_digital() {
    let info = VideoInterfaceInfo::Digital {
        bit_depth: 8,
        interface_type: DigitalInterfaceType::Hdmi,
    };

    match info {
        VideoInterfaceInfo::Digital {
            bit_depth,
            interface_type,
        } => {
            assert_eq!(bit_depth, 8);
            assert!(matches!(interface_type, DigitalInterfaceType::Hdmi));
        }
        _ => panic!("Expected Digital variant"),
    }
}

/// Test VideoInterfaceInfo::Analog
#[test]
fn test_video_interface_analog() {
    let info = VideoInterfaceInfo::Analog {
        signal_level_v: 0.700,
        setup_expected: true,
    };

    match info {
        VideoInterfaceInfo::Analog {
            signal_level_v,
            setup_expected,
        } => {
            assert!((signal_level_v - 0.700).abs() < 0.001);
            assert!(setup_expected);
        }
        _ => panic!("Expected Analog variant"),
    }
}

/// Test VideoInterfaceInfo::Unknown
#[test]
fn test_video_interface_unknown() {
    let info = VideoInterfaceInfo::Unknown;
    assert!(matches!(info, VideoInterfaceInfo::Unknown));
}

/// Test MonitorMode
#[test]
fn test_monitor_mode_fields() {
    let mode = MonitorMode {
        width: 1920,
        height: 1080,
        refresh_rate: 60,
        interlaced: false,
    };

    assert_eq!(mode.width, 1920);
    assert_eq!(mode.height, 1080);
    assert_eq!(mode.refresh_rate, 60);
    assert!(!mode.interlaced);
}

/// Test MonitorMode serialization
#[test]
fn test_monitor_mode_serialization() {
    let mode = MonitorMode {
        width: 2560,
        height: 1440,
        refresh_rate: 144,
        interlaced: false,
    };

    let json = serde_json::to_string(&mode).unwrap();
    let parsed: MonitorMode = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.width, 2560);
    assert_eq!(parsed.refresh_rate, 144);
}

/// Test ChromaticityCoordinates
#[test]
fn test_chromaticity_coordinates() {
    let chroma = ChromaticityCoordinates {
        red_x: 0.640,
        red_y: 0.330,
        green_x: 0.300,
        green_y: 0.600,
        blue_x: 0.150,
        blue_y: 0.060,
        white_x: 0.313,
        white_y: 0.329,
    };

    assert!((chroma.red_x - 0.640).abs() < 0.001);
    assert!((chroma.green_y - 0.600).abs() < 0.001);
    assert!((chroma.blue_x - 0.150).abs() < 0.001);
}

/// Test HdrMetadata default
#[test]
fn test_hdr_metadata_default() {
    let hdr = HdrMetadata::default();
    assert!(!hdr.supports_sdr_eotf);
    assert!(!hdr.supports_hdr_traditional);
    assert!(!hdr.supports_smpte_st2084);
    assert!(!hdr.supports_hlg);
    assert!(hdr.max_luminance_cd_m2.is_none());
}

/// Test HdrMetadata with values
#[test]
fn test_hdr_metadata_with_values() {
    let hdr = HdrMetadata {
        supports_sdr_eotf: true,
        supports_hdr_traditional: true,
        supports_smpte_st2084: true,
        supports_hlg: false,
        max_luminance_cd_m2: Some(1000.0),
        max_frame_average_luminance_cd_m2: Some(400.0),
        min_luminance_cd_m2: Some(0.1),
    };

    assert!(hdr.supports_smpte_st2084);
    assert!(!hdr.supports_hlg);
    assert_eq!(hdr.max_luminance_cd_m2, Some(1000.0));
}

/// Test AudioCapabilities default
#[test]
fn test_audio_capabilities_default() {
    let audio = AudioCapabilities::default();
    assert_eq!(audio.extra_audio_descriptors_count, 0);
    assert!(audio.short_audio_descriptors.is_empty());
}

/// Test AudioCapabilities with values
#[test]
fn test_audio_capabilities_with_values() {
    let mut audio = AudioCapabilities::default();
    audio.extra_audio_descriptors_count = 2;
    audio.short_audio_descriptors = vec![
        "Linear PCM (channels: 2)".to_string(),
        "AC-3 / Dolby Digital (channels: 5)".to_string(),
    ];

    assert_eq!(audio.extra_audio_descriptors_count, 2);
    assert_eq!(audio.short_audio_descriptors.len(), 2);
}

/// Test EdidData creation
#[test]
fn test_edid_data_fields() {
    let edid = EdidData {
        model_name: "TestMonitor".to_string(),
        manufacturer_id: "DEL".to_string(),
        product_code: 0x1234,
        serial_number_binary: 0x5678,
        serial_number_ascii: Some("SN123".to_string()),
        week_of_manufacture: 10,
        year_of_manufacture: 2020,
        video_interface: VideoInterfaceInfo::Digital {
            bit_depth: 8,
            interface_type: DigitalInterfaceType::Hdmi,
        },
        chromaticity: None,
        extension_blocks: 1,
        modes: vec![],
        hdr_caps: HdrMetadata::default(),
        audio_caps: AudioCapabilities::default(),
    };

    assert_eq!(edid.model_name, "TestMonitor");
    assert_eq!(edid.manufacturer_id, "DEL");
    assert_eq!(edid.year_of_manufacture, 2020);
    assert_eq!(edid.extension_blocks, 1);
}

/// Test EdidData serialization
#[test]
fn test_edid_data_serialization_roundtrip() {
    let edid = EdidData {
        model_name: "TestMonitor".to_string(),
        manufacturer_id: "ABC".to_string(),
        product_code: 0xABCD,
        serial_number_binary: 0x12345678,
        serial_number_ascii: Some("Serial".to_string()),
        week_of_manufacture: 5,
        year_of_manufacture: 2023,
        video_interface: VideoInterfaceInfo::Analog {
            signal_level_v: 0.700,
            setup_expected: false,
        },
        chromaticity: None,
        extension_blocks: 0,
        modes: vec![],
        hdr_caps: HdrMetadata::default(),
        audio_caps: AudioCapabilities::default(),
    };

    let json = serde_json::to_string(&edid).unwrap();
    let parsed: EdidData = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.model_name, edid.model_name);
    assert_eq!(parsed.manufacturer_id, edid.manufacturer_id);
    assert_eq!(parsed.product_code, edid.product_code);
}

/// Test DeepDdcStats creation
#[test]
fn test_deep_ddc_stats_fields() {
    let stats = DeepDdcStats {
        core_caps: MonitorCapabilities {
            brightness: 80,
            brightness_max: 100,
            contrast: 60,
            contrast_max: 100,
        },
        input_source: InputSource::Hdmi1,
        power_state: PowerState::On,
        volume: Some((50, 100)),
        audio_mute: AudioMuteState::Unmuted,
        color_gains: Some((100, 100, 100)),
        horizontal_freq_hz: Some(30000),
        vertical_freq_centihz: Some(6000),
        operating_hours: Some(1234),
        osd_language_code: Some(0x656E), // English
        panel_type_code: Some(1),
    };

    assert_eq!(stats.core_caps.brightness, 80);
    assert_eq!(stats.input_source, InputSource::Hdmi1);
    assert_eq!(stats.power_state, PowerState::On);
    assert_eq!(stats.volume, Some((50, 100)));
    assert_eq!(stats.audio_mute, AudioMuteState::Unmuted);
}

/// Test DeepDdcStats serialization
#[test]
fn test_deep_ddc_stats_serialization() {
    let stats = DeepDdcStats {
        core_caps: MonitorCapabilities {
            brightness: 50,
            brightness_max: 100,
            contrast: 50,
            contrast_max: 100,
        },
        input_source: InputSource::DisplayPort1,
        power_state: PowerState::On,
        volume: None,
        audio_mute: AudioMuteState::Unknown,
        color_gains: None,
        horizontal_freq_hz: None,
        vertical_freq_centihz: None,
        operating_hours: None,
        osd_language_code: None,
        panel_type_code: None,
    };

    let json = serde_json::to_string(&stats).unwrap();
    let parsed: DeepDdcStats = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.input_source, InputSource::DisplayPort1);
    assert_eq!(parsed.power_state, PowerState::On);
}

/// Test VcpCode enum values
#[test]
fn test_vcp_code_values() {
    assert_eq!(VcpCode::Brightness as u8, 0x10);
    assert_eq!(VcpCode::Contrast as u8, 0x12);
    assert_eq!(VcpCode::RedGain as u8, 0x16);
    assert_eq!(VcpCode::GreenGain as u8, 0x18);
    assert_eq!(VcpCode::BlueGain as u8, 0x1A);
    assert_eq!(VcpCode::Volume as u8, 0x62);
    assert_eq!(VcpCode::AudioMute as u8, 0x8D);
    assert_eq!(VcpCode::InputSource as u8, 0x60);
    assert_eq!(VcpCode::PowerMode as u8, 0xD6);
    assert_eq!(VcpCode::HorizontalFrequency as u8, 0xAC);
    assert_eq!(VcpCode::VerticalFrequency as u8, 0xAE);
    assert_eq!(VcpCode::OperatingHours as u8, 0xC0);
    assert_eq!(VcpCode::OsdLanguage as u8, 0xCC);
    assert_eq!(VcpCode::FlatPanelType as u8, 0xB6);
}

/// Test VcpCode equality
#[test]
fn test_vcp_code_equality() {
    assert_eq!(VcpCode::Brightness, VcpCode::Brightness);
    assert_eq!(VcpCode::RedGain, VcpCode::RedGain);
    assert_ne!(VcpCode::Brightness, VcpCode::Contrast);
    assert_ne!(VcpCode::Brightness, VcpCode::Volume); // Different VCP codes
}

/// Test MonitorCapabilities equality
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

/// Test PowerState equality
#[test]
fn test_power_state_equality() {
    assert_eq!(PowerState::On, PowerState::On);
    assert_eq!(PowerState::Off, PowerState::Off);
    assert_ne!(PowerState::On, PowerState::Off);
    assert_ne!(PowerState::Standby, PowerState::On);
}

/// Test InputSource equality
#[test]
fn test_input_source_equality() {
    assert_eq!(InputSource::Hdmi1, InputSource::Hdmi1);
    assert_eq!(InputSource::DisplayPort1, InputSource::DisplayPort1);
    assert_ne!(InputSource::Hdmi1, InputSource::DisplayPort1);
    assert_ne!(InputSource::Hdmi1, InputSource::Hdmi2);
}

/// Test AudioMuteState equality
#[test]
fn test_audio_mute_state_equality() {
    assert_eq!(AudioMuteState::Muted, AudioMuteState::Muted);
    assert_eq!(AudioMuteState::Unmuted, AudioMuteState::Unmuted);
    assert_ne!(AudioMuteState::Muted, AudioMuteState::Unmuted);
}

/// Test DigitalInterfaceType equality
#[test]
fn test_digital_interface_type_equality() {
    assert_eq!(DigitalInterfaceType::Hdmi, DigitalInterfaceType::Hdmi);
    assert_eq!(
        DigitalInterfaceType::DisplayPort,
        DigitalInterfaceType::DisplayPort
    );
    assert_ne!(
        DigitalInterfaceType::Hdmi,
        DigitalInterfaceType::DisplayPort
    );
}
