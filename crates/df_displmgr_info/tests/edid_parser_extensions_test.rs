use df_displmgr_info::edid_parser::EdidParser;
use df_displmgr_info::edid_types::{DigitalInterfaceType, VideoInterfaceInfo};

fn make_base_edid(
    model_name: &str,
    mfg_bytes: [u8; 2],
    video_input_byte: u8,
    extension_count: u8,
) -> Vec<u8> {
    let mut raw = vec![0u8; 128];
    raw[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    raw[8] = mfg_bytes[0];
    raw[9] = mfg_bytes[1];
    raw[10] = 0x01;
    raw[11] = 0x00;
    raw[12] = 0x01;
    raw[13] = 0x00;
    raw[14] = 0x00;
    raw[15] = 0x00;
    raw[16] = 10;
    raw[17] = 36;
    raw[20] = video_input_byte;
    let name_bytes = model_name.as_bytes();
    raw[54] = 0;
    raw[55] = 0;
    raw[56] = 0;
    raw[57] = 0xFC;
    raw[58] = 0;
    let end = 59 + name_bytes.len().min(13);
    raw[59..end].copy_from_slice(&name_bytes[..end - 59]);
    raw[126] = extension_count;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    raw
}

fn make_cea_extension(audio_descriptors: &[(u8, u8)], hdr_eotf: u8, hdr_max_lum: u8) -> Vec<u8> {
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[1] = 0x03;
    ext[2] = 4;
    let mut curr = 4;
    if !audio_descriptors.is_empty() {
        let audio_len = (audio_descriptors.len() * 3) as u8;
        ext[curr] = (1 << 5) | (audio_len & 0x1F);
        curr += 1;
        for &(format_code, channels) in audio_descriptors {
            let byte0 = ((format_code & 0x0F) << 3) | (channels & 0x07);
            ext[curr] = byte0;
            ext[curr + 1] = 0;
            ext[curr + 2] = 0;
            curr += 3;
        }
    }
    if hdr_eotf > 0 || hdr_max_lum > 0 {
        ext[curr] = (7 << 5) | 5;
        curr += 1;
        ext[curr] = 0x06;
        curr += 1;
        ext[curr] = hdr_eotf;
        curr += 1;
        ext[curr] = hdr_max_lum;
        curr += 1;
        ext[curr] = 0;
        curr += 1;
        ext[curr] = 0;
        curr += 1;
    }
    ext[2] = curr as u8;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    ext
}

fn make_full_edid(
    model_name: &str,
    mfg_bytes: [u8; 2],
    video_input: u8,
    audio_descriptors: &[(u8, u8)],
    hdr_eotf: u8,
    hdr_max_lum: u8,
) -> Vec<u8> {
    let mut edid = make_base_edid(model_name, mfg_bytes, video_input, 1);
    let ext = make_cea_extension(audio_descriptors, hdr_eotf, hdr_max_lum);
    edid.extend_from_slice(&ext);
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    edid
}

#[test]
fn test_parse_empty_model_name_fallback() {
    let raw = make_base_edid("", [0x00, 0x00], 0x82, 0);
    let data = EdidParser::parse(&raw).unwrap();
    assert_eq!(data.model_name, "Generic Monitor");
}

#[test]
fn test_parse_extension_with_audio() {
    let edid = make_full_edid("AudioTest", [0x00, 0x00], 0x82, &[(1, 1)], 0x06, 50);
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 1);
    assert!(data.audio_caps.short_audio_descriptors[0].contains("Linear PCM"));
    assert!(data.audio_caps.short_audio_descriptors[0].contains("channels: 2"));
}

#[test]
fn test_parse_extension_with_multiple_audio() {
    let edid = make_full_edid(
        "MultiAudio",
        [0x00, 0x00],
        0x82,
        &[(2, 1), (7, 5)],
        0x06,
        50,
    );
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 2);
    assert!(data.audio_caps.short_audio_descriptors[0].contains("AC-3"));
    assert!(data.audio_caps.short_audio_descriptors[1].contains("DTS"));
}

#[test]
fn test_parse_extension_hdr_basic() {
    let edid = make_full_edid("HDRTest", [0x00, 0x00], 0x82, &[], 0x07, 50);
    let data = EdidParser::parse(&edid).unwrap();
    assert!(data.hdr_caps.supports_sdr_eotf);
    assert!(data.hdr_caps.supports_smpte_st2084);
    assert!(data.hdr_caps.max_luminance_cd_m2.is_some());
}

#[test]
fn test_parse_extension_hdr_all_eotfs() {
    let edid = make_full_edid("HDRAll", [0x00, 0x00], 0x82, &[], 0x0F, 100);
    let data = EdidParser::parse(&edid).unwrap();
    assert!(data.hdr_caps.supports_sdr_eotf);
    assert!(data.hdr_caps.supports_hdr_traditional);
    assert!(data.hdr_caps.supports_smpte_st2084);
    assert!(data.hdr_caps.supports_hlg);
}

#[test]
fn test_parse_extension_hdr_no_metadata() {
    let edid = make_full_edid("NoHDR", [0x00, 0x00], 0x82, &[], 0, 0);
    let data = EdidParser::parse(&edid).unwrap();
    assert!(!data.hdr_caps.supports_sdr_eotf);
    assert!(data.hdr_caps.max_luminance_cd_m2.is_none());
}

#[test]
fn test_parse_extension_combined_audio_and_hdr() {
    let edid = make_full_edid("Combo", [0x00, 0x00], 0x82, &[(1, 2), (10, 2)], 0x07, 75);
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 2);
    assert!(data.hdr_caps.supports_smpte_st2084);
}

#[test]
fn test_parse_extension_invalid_checksum() {
    let mut edid = make_full_edid("BadExt", [0x00, 0x00], 0x82, &[], 0x06, 50);
    edid[128 + 127] = 0xFF;
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_wrong_tag() {
    let mut edid = make_base_edid("WrongTag", [0x00, 0x00], 0x82, 1);
    let mut ext = vec![0u8; 128];
    ext[0] = 0x10;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_d_offset_less_than_four() {
    let mut edid = make_base_edid("DOffset", [0x00, 0x00], 0x82, 1);
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[2] = 2;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_insufficient_data() {
    let mut edid = make_base_edid("Trunc", [0x00, 0x00], 0x82, 1);
    edid.truncate(200);
    if edid.len() >= 128 {
        let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        edid[127] = sum.wrapping_neg();
    }
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.extension_blocks, 1);
}

#[test]
fn test_parse_extension_non_cea_extension() {
    let mut edid = make_base_edid("NonCea", [0x00, 0x00], 0x82, 1);
    let mut ext = vec![0u8; 128];
    ext[0] = 0x10;
    ext[2] = 4;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_hdr_with_max_frame_avg() {
    let mut edid = make_base_edid("FrameAvg", [0x00, 0x00], 0x82, 1);
    let mut ext = make_cea_extension(&[], 0x06, 100);
    // Patch max frame-average luminance at offset 8 and update d_offset
    ext[8] = 80;
    ext[2] = 10; // Update d_offset to include the patched byte
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert!(data.hdr_caps.max_frame_average_luminance_cd_m2.is_some());
}

#[test]
fn test_parse_extension_hdr_with_min_luminance() {
    let mut edid = make_base_edid("MinLum", [0x00, 0x00], 0x82, 1);
    let mut ext = make_cea_extension(&[], 0x06, 100);
    // Patch min luminance at offset 9 and update d_offset
    ext[9] = 128;
    ext[2] = 10; // Update d_offset to include the patched byte
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert!(data.hdr_caps.min_luminance_cd_m2.is_some());
}

#[test]
fn test_parse_extension_hdr_zero_max_luminance() {
    let edid = make_full_edid("ZeroMax", [0x00, 0x00], 0x82, &[], 0x06, 0);
    let data = EdidParser::parse(&edid).unwrap();
    assert!(data.hdr_caps.max_luminance_cd_m2.is_none());
}

#[test]
fn test_parse_extension_audio_extra_count() {
    let edid = make_full_edid("Extra0", [0x00, 0x00], 0x82, &[(1, 2), (2, 2)], 0, 0);
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 2);
}

#[test]
fn test_parse_extension_unknown_extended_tag() {
    let mut edid = make_base_edid("UnkExt", [0x00, 0x00], 0x82, 1);
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[2] = 4;
    ext[4] = (7 << 5) | 3;
    ext[5] = 0x99;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert!(!data.hdr_caps.supports_smpte_st2084);
}

#[test]
fn test_parse_extension_data_block_overflow() {
    let mut edid = make_base_edid("Overflow", [0x00, 0x00], 0x82, 1);
    let mut ext = vec![0u8; 128];
    ext[0] = 0x02;
    ext[2] = 6;
    ext[4] = (1 << 5) | 5;
    ext[5] = 0x01;
    let sum: u8 = ext[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ext[127] = sum.wrapping_neg();
    edid.extend_from_slice(&ext);
    edid[126] = 1;
    let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    edid[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_no_extension_blocks() {
    let edid = make_base_edid("NoExt", [0x00, 0x00], 0x82, 0);
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.extension_blocks, 0);
    assert_eq!(data.audio_caps.short_audio_descriptors.len(), 0);
}

#[test]
fn test_parse_extension_short_buffer() {
    let mut edid = make_base_edid("Short", [0x00, 0x00], 0x82, 1);
    edid.truncate(150);
    if edid.len() >= 128 {
        let sum: u8 = edid[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        edid[127] = sum.wrapping_neg();
    }
    let data = EdidParser::parse(&edid).unwrap();
    assert_eq!(data.extension_blocks, 1);
}

#[test]
fn test_parse_digital_displayport() {
    let mut raw = make_base_edid("DP", [0x00, 0x00], 0x82, 0);
    raw[20] = 0x83;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    match data.video_interface {
        VideoInterfaceInfo::Digital {
            bit_depth,
            interface_type,
        } => {
            assert_eq!(bit_depth, 8);
            assert!(matches!(interface_type, DigitalInterfaceType::DisplayPort));
        }
        _ => panic!("Expected Digital DisplayPort"),
    }
}

#[test]
fn test_parse_digital_dvi() {
    let mut raw = make_base_edid("DVI", [0x00, 0x00], 0x82, 0);
    raw[20] = 0x81;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    match data.video_interface {
        VideoInterfaceInfo::Digital {
            bit_depth,
            interface_type,
        } => {
            assert_eq!(bit_depth, 8);
            assert!(matches!(interface_type, DigitalInterfaceType::Dvi));
        }
        _ => panic!("Expected Digital DVI"),
    }
}

#[test]
fn test_parse_digital_unknown_interface() {
    let mut raw = make_base_edid("Unk", [0x00, 0x00], 0x82, 0);
    raw[20] = 0x84;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    match data.video_interface {
        VideoInterfaceInfo::Digital {
            bit_depth,
            interface_type,
        } => {
            assert_eq!(bit_depth, 8);
            assert!(matches!(interface_type, DigitalInterfaceType::Unknown));
        }
        _ => panic!("Expected Digital Unknown"),
    }
}

#[test]
fn test_parse_analog_signal_levels() {
    for (byte, expected_level) in [(0x00, 0.700), (0x20, 0.714), (0x40, 1.000), (0x60, 0.700)] {
        let mut raw = make_base_edid("Analog", [0x00, 0x00], 0x82, 0);
        raw[20] = byte;
        let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
        raw[127] = sum.wrapping_neg();
        let data = EdidParser::parse(&raw).unwrap();
        match data.video_interface {
            VideoInterfaceInfo::Analog {
                signal_level_v,
                setup_expected,
            } => {
                assert!((signal_level_v - expected_level).abs() < 0.001);
                assert!(!setup_expected);
            }
            _ => panic!("Expected Analog for byte 0x{:02X}", byte),
        }
    }
}

#[test]
fn test_parse_analog_setup_expected() {
    let mut raw = make_base_edid("Setup", [0x00, 0x00], 0x82, 0);
    raw[20] = 0x38;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    match data.video_interface {
        VideoInterfaceInfo::Analog {
            signal_level_v,
            setup_expected,
        } => {
            assert!((signal_level_v - 0.714).abs() < 0.001);
            assert!(setup_expected);
        }
        _ => panic!("Expected Analog with setup"),
    }
}

#[test]
fn test_parse_serial_number_ascii() {
    let mut raw = make_base_edid("SN", [0x00, 0x00], 0x82, 0);
    raw[54] = 0;
    raw[55] = 0;
    raw[56] = 0;
    raw[57] = 0xFF;
    raw[58] = 0;
    let sn = b"SN12345678";
    raw[59..69].copy_from_slice(sn);
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    assert_eq!(data.serial_number_ascii, Some("SN12345678".to_string()));
}

#[test]
fn test_parse_serial_number_ascii_with_nulls() {
    let mut raw = make_base_edid("SNNull", [0x00, 0x00], 0x82, 0);
    // Clear the descriptor area to remove the model name
    raw[54..72].fill(0);
    raw[54] = 0;
    raw[55] = 0;
    raw[56] = 0;
    raw[57] = 0xFF;
    raw[58] = 0;
    let sn = b"ABC\0\0\0\0\0";
    raw[59..67].copy_from_slice(sn);
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    assert_eq!(data.serial_number_ascii, Some("ABC".to_string()));
}

#[test]
fn test_parse_product_and_serial() {
    let mut raw = make_base_edid("Prod", [0x00, 0x00], 0x82, 0);
    raw[10] = 0x34;
    raw[11] = 0x12;
    raw[12] = 0x01;
    raw[13] = 0xEF;
    raw[14] = 0xCD;
    raw[15] = 0xAB;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    assert_eq!(data.product_code, 0x1234);
    assert_eq!(data.serial_number_binary, 0xABCDEF01);
}

#[test]
fn test_parse_week_year() {
    let mut raw = make_base_edid("Date", [0x00, 0x00], 0x82, 0);
    raw[16] = 25;
    raw[17] = 30;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    assert_eq!(data.week_of_manufacture, 25);
    assert_eq!(data.year_of_manufacture, 2020);
}

#[test]
fn test_parse_chromaticity_nonzero() {
    let mut raw = make_base_edid("Chroma", [0x00, 0x00], 0x82, 0);
    raw[25] = 0x40;
    raw[26] = 0x00;
    raw[27] = 0x02;
    raw[28] = 0x00;
    let sum: u8 = raw[..127].iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    raw[127] = sum.wrapping_neg();
    let data = EdidParser::parse(&raw).unwrap();
    let chroma = data.chromaticity.unwrap();
    assert!((chroma.red_x - 9.0 / 1024.0).abs() < 0.001);
}
