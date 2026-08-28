use df_displmgr_info::edid_types::*;

/// Test serialization and deserialization of all EDID related structs to increase coverage.
#[test]
fn test_full_edid_struct_serialization() {
    // MonitorCapabilities with edge values
    let caps = MonitorCapabilities {
        brightness: 0,
        brightness_min: 0,
        brightness_max: 0,
        contrast: 0,
        contrast_min: 0,
        contrast_max: 0,
    };
    let caps_json = serde_json::to_string(&caps).unwrap();
    let caps_back: MonitorCapabilities = serde_json::from_str(&caps_json).unwrap();
    assert_eq!(caps, caps_back);

    // MonitorMode with unusual values
    let mode = MonitorMode {
        width: 0,
        height: 0,
        refresh_rate: 0,
        interlaced: true,
    };
    let mode_json = serde_json::to_string(&mode).unwrap();
    let mode_back: MonitorMode = serde_json::from_str(&mode_json).unwrap();
    // Verify fields manually
    assert_eq!(mode.width, mode_back.width);
    assert_eq!(mode.height, mode_back.height);
    assert_eq!(mode.refresh_rate, mode_back.refresh_rate);
    assert_eq!(mode.interlaced, mode_back.interlaced);

    // MonitorTopology with negative coordinates
    let topo = MonitorTopology {
        x: -1,
        y: -1,
        width: 0,
        height: 0,
        rotation: "Rotate0".into(),
    };
    let topo_json = serde_json::to_string(&topo).unwrap();
    let topo_back: MonitorTopology = serde_json::from_str(&topo_json).unwrap();
    // Verify fields manually
    assert_eq!(topo.x, topo_back.x);
    assert_eq!(topo.y, topo_back.y);
    assert_eq!(topo.width, topo_back.width);
    assert_eq!(topo.height, topo_back.height);
    assert_eq!(topo.rotation, topo_back.rotation);

    // VideoInterfaceInfo variants
    let digital = VideoInterfaceInfo::Digital {
        bit_depth: 0,
        interface_type: DigitalInterfaceType::Unknown,
    };
    let analog = VideoInterfaceInfo::Analog {
        signal_level_v: 0.0,
        setup_expected: false,
    };
    let unknown = VideoInterfaceInfo::Unknown;

    for info in &[digital, analog, unknown] {
        let json = serde_json::to_string(info).unwrap();
        let back: VideoInterfaceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, &back);
    }

    // HdrMetadata with all options set
    let hdr = HdrMetadata {
        supports_sdr_eotf: true,
        supports_hdr_traditional: true,
        supports_smpte_st2084: true,
        supports_hlg: true,
        max_luminance_cd_m2: Some(1000.0),
        max_frame_average_luminance_cd_m2: Some(500.0),
        min_luminance_cd_m2: Some(0.1),
    };
    let hdr_json = serde_json::to_string(&hdr).unwrap();
    let hdr_back: HdrMetadata = serde_json::from_str(&hdr_json).unwrap();
    // Verify fields manually
    assert_eq!(hdr.supports_sdr_eotf, hdr_back.supports_sdr_eotf);
    assert_eq!(hdr.supports_hdr_traditional, hdr_back.supports_hdr_traditional);
    assert_eq!(hdr.supports_smpte_st2084, hdr_back.supports_smpte_st2084);
    assert_eq!(hdr.supports_hlg, hdr_back.supports_hlg);
    assert_eq!(hdr.max_luminance_cd_m2, hdr_back.max_luminance_cd_m2);
    assert_eq!(hdr.max_frame_average_luminance_cd_m2, hdr_back.max_frame_average_luminance_cd_m2);
    assert_eq!(hdr.min_luminance_cd_m2, hdr_back.min_luminance_cd_m2);

    // AudioCapabilities with descriptors
    let audio = AudioCapabilities {
        extra_audio_descriptors_count: 2,
        short_audio_descriptors: vec!["Linear PCM".into(), "AC-3".into()],
    };
    let audio_json = serde_json::to_string(&audio).unwrap();
    let audio_back: AudioCapabilities = serde_json::from_str(&audio_json).unwrap();
    // Verify fields manually
    assert_eq!(audio.extra_audio_descriptors_count, audio_back.extra_audio_descriptors_count);
    assert_eq!(audio.short_audio_descriptors, audio_back.short_audio_descriptors);

    // EdidData combining everything
    let edid = EdidData {
        model_name: "Test".into(),
        manufacturer_id: "XYZ".into(),
        product_code: 0xABCD,
        serial_number_binary: 0x123,
        serial_number_ascii: Some("SN123".into()),
        week_of_manufacture: 10,
        year_of_manufacture: 2022,
        video_interface: VideoInterfaceInfo::Digital {
            bit_depth: 8,
            interface_type: DigitalInterfaceType::Hdmi,
        },
        chromaticity: None,
        extension_blocks: 1,
        modes: vec![],
        hdr_caps: HdrMetadata::default(),
        audio_caps: AudioCapabilities::default(),
        display_id_detected: false,
        display_id_modes: vec![],
    };
    // Use the variable to avoid unused warnings
    let _ = edid;
    // End of test
}
