//! Unit tests for `df_displmgr_info` types and error handling.
//! These tests run without hardware and validate core data structures.

use df_displmgr_info::edid_types::*;
use df_displmgr_info::error::EdidError;

fn make_test_edid_data() -> EdidData {
    EdidData {
        model_name: "TestMonitor".into(),
        manufacturer_id: "ABC".into(),
        product_code: 1234,
        serial_number_binary: 5678,
        serial_number_ascii: Some("SN123".into()),
        week_of_manufacture: 10,
        year_of_manufacture: 2024,
        video_interface: VideoInterfaceInfo::Digital {
            bit_depth: 8,
            interface_type: DigitalInterfaceType::Hdmi,
        },
        chromaticity: Some(ChromaticityCoordinates {
            red_x: 0.64, red_y: 0.33,
            green_x: 0.30, green_y: 0.60,
            blue_x: 0.15, blue_y: 0.06,
            white_x: 0.312, white_y: 0.329,
        }),
        extension_blocks: 1,
        modes: vec![
            MonitorMode { width: 1920, height: 1080, refresh_rate: 60, interlaced: false },
            MonitorMode { width: 2560, height: 1440, refresh_rate: 144, interlaced: false },
        ],
        hdr_caps: HdrMetadata::default(),
        audio_caps: AudioCapabilities::default(),
    }
}

#[test]
fn test_edid_data_fields() {
    let d = make_test_edid_data();
    assert_eq!(d.model_name, "TestMonitor");
    assert_eq!(d.manufacturer_id, "ABC");
    assert_eq!(d.product_code, 1234);
    assert_eq!(d.serial_number_binary, 5678);
    assert_eq!(d.serial_number_ascii, Some("SN123".into()));
    assert_eq!(d.week_of_manufacture, 10);
    assert_eq!(d.year_of_manufacture, 2024);
    assert_eq!(d.modes.len(), 2);
    assert_eq!(d.extension_blocks, 1);
}

#[test]
fn test_monitor_mode_fields() {
    let m = MonitorMode { width: 1920, height: 1080, refresh_rate: 60, interlaced: false };
    assert_eq!(m.width, 1920);
    assert_eq!(m.height, 1080);
    assert_eq!(m.refresh_rate, 60);
    assert!(!m.interlaced);
}

#[test]
fn test_monitor_topology_fields() {
    let t = MonitorTopology {
        x: 100,
        y: 200,
        width: 3840,
        height: 2160,
        rotation: "Rotate90".into(),
    };
    assert_eq!(t.x, 100);
    assert_eq!(t.y, 200);
    assert_eq!(t.width, 3840);
    assert_eq!(t.height, 2160);
    assert_eq!(t.rotation, "Rotate90");
}

#[test]
fn test_monitor_topology_serialization() {
    let t = MonitorTopology {
        x: 100, y: 200, width: 3840, height: 2160,
        rotation: "Rotate90".into(),
    };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("3840"));

    let deserialized: MonitorTopology = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.x, 100);
    assert_eq!(deserialized.rotation, "Rotate90");
}

#[test]
fn test_monitor_capabilities_serialization() {
    let c = MonitorCapabilities {
        brightness: 50,
        brightness_max: 100,
        contrast: 70,
        contrast_max: 100,
    };
    let json = serde_json::to_string(&c).unwrap();
    assert!(json.contains("50"));

    let deserialized: MonitorCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.brightness, 50);
    assert_eq!(deserialized.contrast, 70);
}

#[test]
fn test_input_source_variants() {
    let sources = [
        InputSource::AnalogVga,
        InputSource::Dvi,
        InputSource::Hdmi1,
        InputSource::Hdmi2,
        InputSource::DisplayPort1,
        InputSource::DisplayPort2,
        InputSource::UsbC,
        InputSource::Unknown,
    ];
    for (i, a) in sources.iter().enumerate() {
        for (j, b) in sources.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn test_power_state_variants() {
    assert_ne!(PowerState::On, PowerState::Off);
    assert_ne!(PowerState::Standby, PowerState::Suspend);
    assert_eq!(PowerState::Unknown as u32, 0x00);
}

#[test]
fn test_audio_mute_state() {
    assert_ne!(AudioMuteState::Muted, AudioMuteState::Unmuted);
    assert_eq!(AudioMuteState::Muted as u32, 0x01);
    assert_eq!(AudioMuteState::Unmuted as u32, 0x02);
}

#[test]
fn test_video_interface_info_debug() {
    let digital = VideoInterfaceInfo::Digital {
        bit_depth: 10,
        interface_type: DigitalInterfaceType::Hdmi,
    };
    let debug_str = format!("{:?}", digital);
    assert!(debug_str.contains("Digital"));
    assert!(debug_str.contains("10"));

    let analog = VideoInterfaceInfo::Analog {
        signal_level_v: 0.714,
        setup_expected: true,
    };
    let debug_str = format!("{:?}", analog);
    assert!(debug_str.contains("Analog"));

    let unknown = VideoInterfaceInfo::Unknown;
    let debug_str = format!("{:?}", unknown);
    assert!(debug_str.contains("Unknown"));
}

#[test]
fn test_chromaticity_coordinates() {
    let c = ChromaticityCoordinates {
        red_x: 0.640, red_y: 0.330,
        green_x: 0.300, green_y: 0.600,
        blue_x: 0.150, blue_y: 0.060,
        white_x: 0.312, white_y: 0.329,
    };
    assert!((c.red_x - 0.640).abs() < f32::EPSILON);
    assert!((c.white_y - 0.329).abs() < f32::EPSILON);
}

#[test]
fn test_hdr_metadata() {
    let mut h = HdrMetadata::default();
    assert!(!h.supports_sdr_eotf);
    assert!(!h.supports_smpte_st2084);
    assert!(h.max_luminance_cd_m2.is_none());

    h.supports_smpte_st2084 = true;
    h.max_luminance_cd_m2 = Some(1000.0);
    assert!(h.supports_smpte_st2084);
    assert_eq!(h.max_luminance_cd_m2, Some(1000.0));
}

#[test]
fn test_audio_capabilities() {
    let mut a = AudioCapabilities::default();
    assert!(a.short_audio_descriptors.is_empty());
    assert_eq!(a.extra_audio_descriptors_count, 0);

    a.short_audio_descriptors.push("Linear PCM".into());
    a.extra_audio_descriptors_count = 2;
    assert_eq!(a.short_audio_descriptors.len(), 1);
    assert_eq!(a.extra_audio_descriptors_count, 2);
}

#[test]
fn test_edid_error_display() {
    let err = EdidError::ParseError;
    let msg = format!("{}", err);
    assert!(!msg.is_empty());

    let err = EdidError::NotFound;
    let msg = format!("{}", err);
    assert!(!msg.is_empty());

    let err = EdidError::BackendNotAvailable;
    let msg = format!("{}", err);
    assert!(!msg.is_empty());

    let err = EdidError::DdcError("DDC test error".into());
    let msg = format!("{}", err);
    assert!(msg.contains("DDC test error"));
}

#[test]
fn test_edid_data_serialization_roundtrip() {
    let data = make_test_edid_data();
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("TestMonitor"));

    let deserialized: EdidData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.model_name, "TestMonitor");
    assert_eq!(deserialized.product_code, 1234);
    assert_eq!(deserialized.modes.len(), 2);
    assert_eq!(deserialized.modes[1].refresh_rate, 144);
}

#[test]
fn test_deep_ddc_stats_serialization() {
    let s = DeepDdcStats {
        core_caps: MonitorCapabilities {
            brightness: 50, brightness_max: 100,
            contrast: 70, contrast_max: 100,
        },
        input_source: InputSource::Hdmi1,
        power_state: PowerState::On,
        volume: Some((30, 100)),
        audio_mute: AudioMuteState::Unmuted,
        color_gains: Some((50, 50, 50)),
        horizontal_freq_hz: Some(67500),
        vertical_freq_centihz: Some(6000),
        operating_hours: Some(12345),
        osd_language_code: Some(0x01),
        panel_type_code: Some(0x02),
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("50"));
    assert!(json.contains("12345"));

    let deserialized: DeepDdcStats = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.core_caps.brightness, 50);
    assert_eq!(deserialized.input_source, InputSource::Hdmi1);
    assert_eq!(deserialized.volume, Some((30, 100)));
    assert_eq!(deserialized.operating_hours, Some(12345));
}

#[test]
fn test_digital_interface_type_variants() {
    assert_ne!(DigitalInterfaceType::Hdmi, DigitalInterfaceType::DisplayPort);
    assert_ne!(DigitalInterfaceType::Dvi, DigitalInterfaceType::Unknown);
}

#[test]
fn test_vcp_code_values() {
    assert_eq!(VcpCode::Brightness as u8, 0x10);
    assert_eq!(VcpCode::Contrast as u8, 0x12);
    assert_eq!(VcpCode::InputSource as u8, 0x60);
    assert_eq!(VcpCode::Volume as u8, 0x62);
    assert_eq!(VcpCode::PowerMode as u8, 0xD6);
}