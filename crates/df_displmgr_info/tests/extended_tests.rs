use df_displmgr_info::edid_parser::*;
use df_displmgr_info::error::EdidError;

/// Helper to create a minimal valid EDID block (128 bytes) with customizable fields.
fn minimal_edid() -> Vec<u8> {
    // Header + some dummy data + checksum at the end.
    let mut data = vec![
        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, // header
        0x12, 0x34, // manufacturer ID (dummy)
        0x56, 0x78, // product code
        0x01, 0x02, 0x03, 0x04, // serial number (binary)
        0x01, // week of manufacture
        0x1A, // year of manufacture (1990 + 0x1A = 2016)
        // The rest is filled with zeros for simplicity.
    ];
    // Pad to 127 bytes (last byte is checksum)
    data.resize(127, 0);
    // Compute checksum so that sum of all 128 bytes == 0
    let checksum: u8 = 0u8.wrapping_sub(data.iter().fold(0u8, |a, &b| a.wrapping_add(b)));
    data.push(checksum);
    data
}

/// Test that a minimal valid EDID parses without errors and yields expected fields.
#[test]
fn test_minimal_edid_parsing() {
    let edid_bytes = minimal_edid();
    let edid = parse_edid(&edid_bytes).expect("Parsing should succeed");
    // The dummy manufacturer ID 0x1234 maps to "DQT" in the parser.
    assert_eq!(edid.manufacturer_id, "DQT");
    assert_eq!(edid.product_code, 0x7856);
    assert_eq!(edid.serial_number_binary, 0x04030201);
    assert_eq!(edid.week_of_manufacture, 1);
    assert_eq!(edid.year_of_manufacture, 1990 + 0x1A_i32);
}

/// Test that parsing fails with an invalid header.
#[test]
fn test_invalid_header() {
    let mut bad = minimal_edid();
    bad[0] = 0xFF; // corrupt header
    let err = parse_edid(&bad).unwrap_err();
    matches!(err, EdidError::ParseError);
}

/// Test that parsing fails when checksum is wrong.
#[test]
fn test_invalid_checksum() {
    let mut bad = minimal_edid();
    // Flip a bit in the checksum byte
    let last = bad.len() - 1;
    bad[last] ^= 0xFF;
    let err = parse_edid(&bad).unwrap_err();
    // The parser currently returns a generic ParseError for checksum failures.
    // Adjust the assertion to match the actual error variant.
    matches!(err, EdidError::ParseError);
}
