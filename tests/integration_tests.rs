use std::process::Command;
use std::path::PathBuf;
use std::fs;

#[test]
fn test_edid_parser_valid_blob() {
    // Load a known good EDID binary (fixture)
    let edid_path = PathBuf::from("tests/fixtures/edid_valid.bin");
    let edid_data = fs::read(&edid_path).expect("fixture not found");
    let parsed = df_displmgr_info::edid_parser::parse_edid(&edid_data).expect("parse failed");
    // Basic sanity checks
    assert!(!parsed.manufacturer_id.is_empty());
    assert!(parsed.serial_number.is_some());
    assert!(!parsed.descriptors.is_empty());
}

#[test]
fn test_edid_parser_invalid_blob() {
    // Corrupt the EDID data to trigger an error
    let mut edid_data = vec![0u8; 128];
    // Invalid header (should be 0x00,0xff,0xff,0xff,0xff,0xff,0xff,0x00)
    edid_data[0..8].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    let result = df_displmgr_info::edid_parser::parse_edid(&edid_data);
    assert!(result.is_err(), "Parsing should fail for corrupted EDID");
}

#[test]
fn test_mock_ddc_backend() {
    // Create a mock DDC implementation that records calls
    struct MockDdc;
    impl df_ddc::Ddc for MockDdc {
        fn get_vcp_feature(&self, _code: u8) -> Result<df_ddc::VcpFeature, df_ddc::Error> {
            Ok(df_ddc::VcpFeature {
                code: 0x10,
                current: 50,
                maximum: 100,
                raw: vec![],
            })
        }
        fn set_vcp_feature(&self, _code: u8, _value: u16) -> Result<(), df_ddc::Error> {
            Ok(())
        }
    }

    let mock = MockDdc;
    // Use the mock to query a feature
    let feature = mock.get_vcp_feature(0x10).expect("mock get should succeed");
    assert_eq!(feature.current, 50);
    assert_eq!(feature.maximum, 100);
    // Set a feature and ensure no error
    mock.set_vcp_feature(0x10, 75).expect("mock set should succeed");
}

#[test]
fn test_cli_info_command() {
    // Build the CLI binary (ensure it is compiled)
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--bin", "displaymanager_cli", "--", "info"])
        .output()
        .expect("failed to execute CLI");
    assert!(output.status.success(), "CLI should exit with success");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The info command should contain a header line
    assert!(stdout.contains("Display Manager Information"), "output missing header");
}

#[test]
fn test_cli_set_brightness() {
    // Attempt to set brightness to a safe value (0) which should be accepted even without a real monitor
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--quiet",
            "--bin",
            "displaymanager_cli",
            "--",
            "set",
            "--monitor",
            "0",
            "--brightness",
            "0",
        ])
        .output()
        .expect("failed to execute CLI set");
    // The command should exit with success (error handling is internal)
    assert!(output.status.success(), "CLI set should succeed even on mock");
}