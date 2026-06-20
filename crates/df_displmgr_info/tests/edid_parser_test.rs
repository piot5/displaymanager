use df_displmgr_info::edid_parser::EdidParser;
use df_displmgr_info::edid_types::{DigitalInterfaceType, VideoInterfaceInfo};

/// Helper to build a minimal valid 128-byte EDID base block
fn make_base_edid(model_name: &str) -> Vec<u8> {
    let mut raw = vec![0u8; 128];
    raw[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    raw[8] = 0x00;
    raw[9] = 0x00;
    raw[10] = 0x01;
    raw[11] = 0x00;
    raw[12] = 0x01;
    raw[13] = 0x00;
    raw[14] = 0x00;
    raw[15] = 0x00;
    raw[16] = 10;
    raw[17] = 36;
    raw[20] = 0x82;
    let name_bytes = model_name.as_bytes();
    raw[54] = 0;
    raw[55] = 0;
    raw[56] = 0;
    raw[57] = 0xFC;
    raw[58] = 0;
    let end = 59 + name_bytes.len().min(13);
    raw[59..end].copy_from_slice(&name_bytes[..end - 59]);
    raw[126] = 0;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    raw
}

#[test]
fn test_parse_valid_edid() {
    let raw = make_base_edid("TestMonitor");
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    let data = result.unwrap();
    assert_eq!(data.model_name, "TestMonitor");
}

#[test]
fn test_parse_too_short() {
    let raw = vec![0u8; 64];
    let result = EdidParser::parse(&raw);
    assert!(result.is_err());
}

#[test]
fn test_parse_bad_header() {
    let mut raw = make_base_edid("X");
    raw[0] = 0x01;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_err());
}

#[test]
fn test_parse_bad_checksum() {
    let mut raw = make_base_edid("X");
    raw[127] = 0xFF;
    let result = EdidParser::parse(&raw);
    assert!(result.is_err());
}

#[test]
fn test_parse_edid_with_serial_number() {
    let mut raw = make_base_edid("Test");
    let serial = b"SN123456";
    raw[72] = 0;
    raw[73] = 0;
    raw[74] = 0;
    raw[75] = 0xFF;
    raw[76] = 0;
    raw[77..77 + serial.len()].copy_from_slice(serial);
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().serial_number_ascii,
        Some("SN123456".to_string())
    );
}

#[test]
fn test_parse_edid_product_and_serial() {
    let mut raw = make_base_edid("Test");
    raw[10] = 0x34;
    raw[11] = 0x12;
    raw[12] = 0x78;
    raw[13] = 0x56;
    raw[14] = 0x34;
    raw[15] = 0x12;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    let data = result.unwrap();
    assert_eq!(data.product_code, 0x1234);
    assert_eq!(data.serial_number_binary, 0x12345678);
}

#[test]
fn test_parse_edid_week_year() {
    let mut raw = make_base_edid("Test");
    raw[16] = 25;
    raw[17] = 30;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    let data = result.unwrap();
    assert_eq!(data.week_of_manufacture, 25);
    assert_eq!(data.year_of_manufacture, 2020);
}

#[test]
fn test_parse_edid_no_extensions() {
    let raw = make_base_edid("Basic");
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    let data = result.unwrap();
    assert_eq!(data.extension_blocks, 0);
    assert!(data.hdr_caps.max_luminance_cd_m2.is_none());
}

#[test]
fn test_parse_edid_insufficient_extension_data() {
    let mut raw = make_base_edid("Test");
    raw[126] = 1;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().extension_blocks, 1);
}

#[test]
fn test_parse_edid_bad_extension_checksum() {
    let mut raw = make_base_edid("Test");
    raw[126] = 1;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[127] = 0xFF;
    raw.extend_from_slice(&ext);
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
}

#[test]
fn test_parse_edid_non_cea_extension() {
    let mut raw = make_base_edid("Test");
    raw[126] = 1;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let mut ext = vec![0u8; 128];
    ext[0] = 0x01;
    let ext_sum: u8 = ext.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = ext_sum.wrapping_neg();
    raw.extend_from_slice(&ext);
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert!(result.unwrap().hdr_caps.max_luminance_cd_m2.is_none());
}

#[test]
fn test_parse_edid_d_offset_less_than_four() {
    let mut raw = make_base_edid("Test");
    raw[126] = 1;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[2] = 0x03;
    let ext_sum: u8 = ext.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = ext_sum.wrapping_neg();
    raw.extend_from_slice(&ext);
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
}

#[test]
fn test_parse_edid_zero_pixel_clock_dtd() {
    let mut raw = make_base_edid("Test");
    raw[54..72].copy_from_slice(&[0u8; 18]);
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert!(result.unwrap().modes.is_empty());
}

#[test]
fn test_parse_edid_zero_resolution_dtd() {
    let mut raw = make_base_edid("Test");
    raw[54] = 0x01;
    raw[55] = 0x00;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert!(result.unwrap().modes.is_empty());
}

#[test]
fn test_parse_edid_blank_model_name() {
    let mut raw = make_base_edid("");
    raw[54..72].fill(0);
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().model_name, "Generic Monitor");
}

#[test]
fn test_parse_edid_long_model_name() {
    let name = "1234567890123";
    let raw = make_base_edid(name);
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().model_name, name);
}

#[test]
fn test_parse_edid_exact_13_char_name() {
    let name = "ABCDEFGHIJKLM";
    let raw = make_base_edid(name);
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().model_name, name);
}

#[test]
fn test_parse_digital_interface_types() {
    let interface_types = vec![
        (0x82, DigitalInterfaceType::Hdmi),
        (0x83, DigitalInterfaceType::DisplayPort),
    ];
    for (video_byte, expected_type) in interface_types {
        let mut raw = make_base_edid("Test");
        raw[20] = video_byte;
        let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        raw[127] = sum.wrapping_neg();
        let result = EdidParser::parse(&raw);
        assert!(result.is_ok());
        match result.unwrap().video_interface {
            VideoInterfaceInfo::Digital { interface_type, .. } => {
                assert_eq!(interface_type, expected_type);
            }
            _ => panic!("Expected Digital interface"),
        }
    }
}

#[test]
fn test_parse_analog_signal_levels() {
    let signal_levels = vec![(0x00, 0.700), (0x20, 0.714), (0x40, 1.000)];
    for (video_byte, expected_level) in signal_levels {
        let mut raw = make_base_edid("Test");
        raw[20] = video_byte;
        let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        raw[127] = sum.wrapping_neg();
        let result = EdidParser::parse(&raw);
        assert!(result.is_ok());
        match result.unwrap().video_interface {
            VideoInterfaceInfo::Analog { signal_level_v, .. } => {
                assert!((signal_level_v - expected_level).abs() < 0.001);
            }
            _ => panic!("Expected Analog interface"),
        }
    }
}

#[test]
fn test_parse_analog_setup_expected() {
    let mut raw = make_base_edid("Test");
    raw[20] = 0x30;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let result = EdidParser::parse(&raw);
    assert!(result.is_ok());
    match result.unwrap().video_interface {
        VideoInterfaceInfo::Analog { setup_expected, .. } => {
            assert!(setup_expected);
        }
        _ => panic!("Expected Analog interface"),
    }
}
