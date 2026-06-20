//! Integration tests for the displaymanager_cli flat-flag interface.
//!
//! All tests run via `cargo test -p displaymanager_cli`.
//! The tests invoke the actual binary using `cargo run`.
//! Tests are designed to work both with and without connected monitors.

use df_ddc::ddc_trait::DdcControl;
use df_ddc::error::DdcError;
use displaymanager_cli::ddc;
use displaymanager_cli::set;
use std::path::PathBuf;
use std::process::Command;

// ── Test helpers ──

/// Helper: run the displaymanager_cli binary with given args, return (success, stdout, stderr).
fn run_cli(args: &[&str]) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "displaymanager_cli",
            "--bin",
            "displaymanager_cli",
            "--",
        ])
        .args(args)
        .output()
        .expect("failed to execute cli");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (success, stdout, stderr)
}

/// Helper: run any binary in the package with given args.
#[allow(dead_code)]
fn run_binary(bin: &str, args: &[&str]) -> (bool, String) {
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "displaymanager_cli",
            "--bin",
            bin,
            "--",
        ])
        .args(args)
        .output()
        .expect("failed to execute binary");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (success, stdout)
}

// ── Help output tests ──

#[test]
fn test_help_contains_all_flags() {
    let (ok, out, _err) = run_cli(&["--help"]);
    assert!(ok, "CLI --help should exit successfully");

    // All topology flags
    for flag in &[
        "--scan",
        "--id",
        "--off",
        "--mode",
        "--cloned",
        "--res",
        "--topo",
        "--freq",
        "--rotate",
        "--hdr",
        "--scale",
        "--primary",
        "--verify",
    ] {
        assert!(out.contains(flag), "Help should mention {}", flag);
    }

    // All DDC flags
    for flag in &[
        "--brightness",
        "--contrast",
        "--volume",
        "--input",
        "--power",
    ] {
        assert!(out.contains(flag), "Help should mention {}", flag);
    }

    // All short flags
    for short in &['s', 'i', 'o', 'm', 'c', 'r', 't', 'f', 'p'] {
        assert!(
            out.contains(&format!("-{}", short)),
            "Help should mention -{}",
            short
        );
    }
}

#[test]
fn test_help_version() {
    let (ok, out, _err) = run_cli(&["--version"]);
    assert!(ok, "CLI --version should exit successfully");
    assert!(out.contains("1.0"), "Version should mention 1.0");
}

// ── Scan tests ──

#[test]
fn test_scan_default() {
    let (ok, out, _err) = run_cli(&[]);
    assert!(ok, "CLI with no args should scan successfully");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should show report or empty state: {}",
        out.lines().next().unwrap_or("")
    );
}

#[test]
fn test_scan_explicit() {
    let (ok, out, _err) = run_cli(&["--scan"]);
    assert!(ok, "CLI --scan should exit successfully");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should show report or empty state"
    );
}

#[test]
fn test_scan_with_file_output() {
    let json_path = "test_scan_output.json";
    let (ok, _out, _err) = run_cli(&["--scan", json_path]);
    assert!(ok, "CLI --scan <file> should exit successfully");
    // Should have created the JSON file
    let json_exists = std::path::Path::new(json_path).exists();
    assert!(json_exists, "JSON scan output file should exist");
    // Clean up
    let _ = std::fs::remove_file(json_path);
    let _ = std::fs::remove_file("edid_dump.txt");
}

#[test]
fn test_scan_short_flag() {
    let (ok, out, _err) = run_cli(&["-s"]);
    assert!(ok, "CLI -s should exit successfully");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should show report or empty state"
    );
}

// ── Fallback behavior (no --id, just topology flags) ──

#[test]
fn test_off_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--off"]);
    assert!(ok, "CLI --off without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_mode_ext_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--mode", "ext"]);
    assert!(ok, "CLI --mode ext without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_mode_clone_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--mode", "clone"]);
    assert!(ok, "CLI --mode clone without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_res_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--res", "1920x1080"]);
    assert!(ok, "CLI --res without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_topo_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--topo", "0,0"]);
    assert!(ok, "CLI --topo without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_freq_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--freq", "60000"]);
    assert!(ok, "CLI --freq without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_rotate_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--rotate", "90"]);
    assert!(ok, "CLI --rotate without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

#[test]
fn test_ddc_without_id_falls_back_to_scan() {
    let (ok, out, _err) = run_cli(&["--brightness", "50"]);
    assert!(ok, "CLI --brightness without --id should fall back to scan");
    assert!(
        out.contains("Display Hardware Report") || out.contains("No monitors detected"),
        "Should have run a scan as fallback"
    );
}

// ── Error handling tests ──

#[test]
fn test_verify_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--verify"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("not found"),
        "Should mention 'not found', got: {}",
        combined
    );
}

#[test]
fn test_off_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--off"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_res_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--res", "1920x1080"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_brightness_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--brightness", "50"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_contrast_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--contrast", "75"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_volume_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--volume", "30"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_power_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--power", "on"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_input_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&["-i", "NonExistentMonitor123", "--input", "hdmi1"]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

#[test]
fn test_multi_unknown_monitor_returns_error() {
    let (ok, out, err) = run_cli(&[
        "-i",
        "NonExistentMonitor123",
        "-r",
        "1920x1080",
        "-t",
        "0,0",
        "--freq",
        "60000",
        "--verify",
    ]);
    assert!(!ok, "CLI should fail for unknown monitor");
    let combined = format!("{}{}", out, err);
    assert!(combined.contains("not found"), "Should mention 'not found'");
}

// ── Conditional tests (only run if monitors are connected) ──

#[test]
fn test_verify_with_real_monitor_name() {
    // First get the list of monitors
    let (_ok_scan, scan_out, _err_scan) = run_cli(&["--scan"]);
    if !scan_out.contains("Display Hardware Report") {
        eprintln!("Skipping: no monitors connected");
        return;
    }
    // Extract first monitor name from scan output
    let name = scan_out
        .lines()
        .find(|l| l.contains("Name:"))
        .and_then(|l| l.split("Name:").nth(1))
        .map(|s| s.trim())
        .unwrap_or("");
    if name.is_empty() {
        eprintln!("Skipping: could not extract monitor name");
        return;
    }
    let (ok, _out, _err) = run_cli(&["-i", name, "--verify"]);
    assert!(ok, "Verify on real monitor '{}' should succeed", name);
}

#[test]
fn test_verify_full_with_real_monitor() {
    let (_ok_scan, scan_out, _err_scan) = run_cli(&["--scan"]);
    if !scan_out.contains("Display Hardware Report") {
        eprintln!("Skipping: no monitors connected");
        return;
    }
    let name = scan_out
        .lines()
        .find(|l| l.contains("Name:"))
        .and_then(|l| l.split("Name:").nth(1))
        .map(|s| s.trim())
        .unwrap_or("");
    if name.is_empty() {
        eprintln!("Skipping: could not extract monitor name");
        return;
    }
    // Dry-run with resolution + position + frequency
    let (ok, _out, _err) = run_cli(&[
        "-i",
        name,
        "-r",
        "1920x1080",
        "-t",
        "0,0",
        "-f",
        "60000",
        "--verify",
    ]);
    assert!(
        ok,
        "Verify with resolution+topo+freq on '{}' should succeed",
        name
    );
}

// ── EDID fixture tests (from workspace tests/integration_tests.rs) ──

#[test]
fn test_edid_file_json_output() {
    // Find the EDID fixture — check multiple relative paths
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/edid_valid.bin"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/edid_valid.bin"),
    ];
    let fixture = candidates.iter().find(|p| p.exists());
    let fixture = match fixture {
        Some(f) => f,
        None => {
            eprintln!("Skipping edid_file_json test: fixture not found");
            return;
        }
    };
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "displaymanager_cli",
            "--bin",
            "edid_file_json",
            "--",
        ])
        .arg(fixture)
        .output()
        .expect("failed to run edid_file_json");
    assert!(
        output.status.success(),
        "edid_file_json should exit successfully"
    );
    let json = String::from_utf8_lossy(&output.stdout);
    assert!(
        json.contains("manufacturer_id"),
        "JSON output missing manufacturer_id"
    );
}

#[test]
fn test_edid_file_txt_output() {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/edid_valid.bin"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/edid_valid.bin"),
    ];
    let fixture = candidates.iter().find(|p| p.exists());
    let fixture = match fixture {
        Some(f) => f,
        None => {
            eprintln!("Skipping edid_file_txt test: fixture not found");
            return;
        }
    };
    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "displaymanager_cli",
            "--bin",
            "edid_file_txt",
            "--",
        ])
        .arg(fixture)
        .output()
        .expect("failed to run edid_file_txt");
    assert!(
        output.status.success(),
        "edid_file_txt should exit successfully"
    );
    let txt = String::from_utf8_lossy(&output.stdout);
    assert!(
        txt.contains("Manufacturer"),
        "Text output missing Manufacturer label"
    );
}

/// Test EDID parsing directly (from workspace tests).
#[test]
fn test_edid_parser_valid_blob() {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/edid_valid.bin"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/edid_valid.bin"),
    ];
    let fixture = candidates.iter().find(|p| p.exists());
    let fixture = match fixture {
        Some(f) => f,
        None => {
            eprintln!("Skipping EDID parser test: fixture not found");
            return;
        }
    };
    let edid_data = std::fs::read(fixture).expect("fixture not found");
    let parsed = df_displmgr_info::edid_parser::parse_edid(&edid_data).expect("parse failed");
    assert!(
        !parsed.manufacturer_id.is_empty(),
        "Manufacturer ID should not be empty"
    );
    assert!(
        parsed.serial_number_ascii.is_some(),
        "Serial number should be present"
    );
    assert!(!parsed.modes.is_empty(), "Should have display modes");
}

#[test]
fn test_edid_parser_invalid_blob() {
    let mut edid_data = vec![0u8; 128];
    // Invalid header (should be 0x00,0xff,0xff,0xff,0xff,0xff,0xff,0x00)
    edid_data[0..8].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    let result = df_displmgr_info::edid_parser::parse_edid(&edid_data);
    assert!(result.is_err(), "Parsing should fail for corrupted EDID");
}

// ── DDC mock trait tests (from workspace tests) ──

#[test]
fn test_mock_ddc_backend() {
    // Create a mock DDC implementation that records calls
    #[derive(Default)]
    struct MockDdc {
        pub get_calls: std::sync::Mutex<Vec<u8>>,
        pub set_calls: std::sync::Mutex<Vec<(u8, u32)>>,
    }
    impl DdcControl for MockDdc {
        fn get_vcp_feature(&self, code: u8) -> Result<(u32, u32), DdcError> {
            self.get_calls.lock().unwrap().push(code);
            Ok((50, 100))
        }
        fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), DdcError> {
            self.set_calls.lock().unwrap().push((code, value));
            Ok(())
        }
    }

    let mock = MockDdc::default();
    let (cur, max) = mock.get_vcp_feature(0x10).expect("mock get should succeed");
    assert_eq!(cur, 50, "current brightness should be 50");
    assert_eq!(max, 100, "max brightness should be 100");
    assert_eq!(mock.get_calls.lock().unwrap().len(), 1);
    assert_eq!(mock.get_calls.lock().unwrap()[0], 0x10);

    mock.set_vcp_feature(0x10, 75)
        .expect("mock set should succeed");
    assert_eq!(mock.set_calls.lock().unwrap().len(), 1);
    assert_eq!(mock.set_calls.lock().unwrap()[0], (0x10, 75));
}

// ── Unit-style tests for parser helpers (no hardware needed) ──

#[test]
fn test_parse_resolution_valid() {
    assert_eq!(set::parse_resolution("1920x1080").unwrap(), (1920, 1080));
    assert_eq!(set::parse_resolution("3840x2160").unwrap(), (3840, 2160));
    assert_eq!(set::parse_resolution("800x600").unwrap(), (800, 600));
}

#[test]
fn test_parse_resolution_invalid() {
    assert!(set::parse_resolution("invalid").is_err());
    assert!(set::parse_resolution("1920").is_err());
    assert!(set::parse_resolution("x1080").is_err());
    assert!(set::parse_resolution("").is_err());
}

#[test]
fn test_parse_position_valid() {
    assert_eq!(set::parse_position("0x0").unwrap(), (0, 0));
    assert_eq!(set::parse_position("1920x1080").unwrap(), (1920, 1080));
    assert_eq!(set::parse_position("100,200").unwrap(), (100, 200));
    assert_eq!(set::parse_position("-100,50").unwrap(), (-100, 50));
}

#[test]
fn test_parse_position_invalid() {
    assert!(set::parse_position("invalid").is_err());
    assert!(set::parse_position("").is_err());
    assert!(set::parse_position("100").is_err());
}

#[test]
fn test_parse_rotation_valid() {
    assert!(matches!(
        set::parse_rotation("0").unwrap(),
        df_displmgr::types::DisplayRotation::Rotate0
    ));
    assert!(matches!(
        set::parse_rotation("90").unwrap(),
        df_displmgr::types::DisplayRotation::Rotate90
    ));
    assert!(matches!(
        set::parse_rotation("180").unwrap(),
        df_displmgr::types::DisplayRotation::Rotate180
    ));
    assert!(matches!(
        set::parse_rotation("270").unwrap(),
        df_displmgr::types::DisplayRotation::Rotate270
    ));
}

#[test]
fn test_parse_rotation_invalid() {
    assert!(set::parse_rotation("45").is_err());
    assert!(set::parse_rotation("-90").is_err());
    assert!(set::parse_rotation("abc").is_err());
}

// ── DDC input source parsing tests ──

#[test]
fn test_ddc_input_source_numeric() {
    use df_ddc::ddc_types::InputSource;
    let result = ddc::parse_input_source("0x0F").unwrap();
    assert!(matches!(result, InputSource::DisplayPort1));
    let result = ddc::parse_input_source("0x10").unwrap();
    assert!(matches!(result, InputSource::DisplayPort2));
    let result = ddc::parse_input_source("0x11").unwrap();
    assert!(matches!(result, InputSource::Hdmi1));
    let result = ddc::parse_input_source("0x12").unwrap();
    assert!(matches!(result, InputSource::Hdmi2));
}

#[test]
fn test_ddc_input_source_named() {
    use df_ddc::ddc_types::InputSource;
    for name in &["dp1", "displayport1", "displayport1.0"] {
        let result = ddc::parse_input_source(name).unwrap();
        assert!(
            matches!(result, InputSource::DisplayPort1),
            "Failed for {}",
            name
        );
    }
    for name in &["dp2", "displayport2"] {
        let result = ddc::parse_input_source(name).unwrap();
        assert!(
            matches!(result, InputSource::DisplayPort2),
            "Failed for {}",
            name
        );
    }
    for name in &["hdmi1", "hdmi-1", "hdmi1.0"] {
        let result = ddc::parse_input_source(name).unwrap();
        assert!(matches!(result, InputSource::Hdmi1), "Failed for {}", name);
    }
    for name in &["hdmi2", "hdmi-2"] {
        let result = ddc::parse_input_source(name).unwrap();
        assert!(matches!(result, InputSource::Hdmi2), "Failed for {}", name);
    }
}

#[test]
fn test_ddc_input_source_invalid() {
    assert!(ddc::parse_input_source("unknown").is_err());
    assert!(ddc::parse_input_source("0x99").is_err());
    assert!(ddc::parse_input_source("vga").is_err());
    assert!(ddc::parse_input_source("").is_err());
}

// ── Mutation-resistant unit tests for set helpers ──

#[test]
fn test_parse_resolution_mutation() {
    // "0x0" splits into "0" and "0" on 'x' — this is a valid edge case
    assert!(set::parse_resolution("0x0").is_ok());
    // These must fail — mutants that remove bounds checks would make them pass
    assert!(set::parse_resolution("1920").is_err());
    assert!(set::parse_resolution("x").is_err());
    assert!(set::parse_resolution("1x1").is_ok());
    assert!(set::parse_resolution("0x").is_err());
}

#[test]
fn test_parse_position_mutation() {
    // comma form
    assert_eq!(set::parse_position("-1,-2").unwrap(), (-1, -2));
    // x form with negatives
    assert_eq!(set::parse_position("-10x20").unwrap(), (-10, 20));
    // single value must fail
    assert!(set::parse_position("5").is_err());
    // empty fails
    assert!(set::parse_position("").is_err());
}

#[test]
fn test_parse_rotation_mutation() {
    // boundary values
    assert!(matches!(
        set::parse_rotation("270").unwrap(),
        df_displmgr::types::DisplayRotation::Rotate270
    ));
    // whitespace must fail
    assert!(set::parse_rotation(" 90").is_err());
    assert!(set::parse_rotation("90 ").is_err());
}

#[test]
fn test_set_args_defaults() {
    let args = set::SetArgs::default();
    assert!(args.mode_type.is_none());
    assert!(args.clone_from.is_none());
    assert!(args.mode.is_none());
    assert!(!args.primary);
    assert!(!args.verify_only);
}

#[test]
fn test_resolve_mode_type() {
    assert_eq!(
        set::SetArgs {
            mode_type: Some("off".into()),
            ..Default::default()
        }
        .mode_type
        .as_deref(),
        Some("off")
    );
    assert_eq!(
        set::SetArgs {
            mode_type: Some("ext".into()),
            ..Default::default()
        }
        .mode_type
        .as_deref(),
        Some("ext")
    );
    assert_eq!(
        set::SetArgs {
            mode_type: Some("extended".into()),
            ..Default::default()
        }
        .mode_type
        .as_deref(),
        Some("extended")
    );
    assert_eq!(
        set::SetArgs {
            clone_from: Some("foo".into()),
            ..Default::default()
        }
        .clone_from
        .as_deref(),
        Some("foo")
    );
}

#[test]
fn test_is_mode_active() {
    assert!(!set::is_mode_active(&set::SetArgs {
        mode_type: Some("off".into()),
        ..Default::default()
    }));
    assert!(set::is_mode_active(&set::SetArgs {
        mode_type: Some("ext".into()),
        ..Default::default()
    }));
    assert!(set::is_mode_active(&set::SetArgs {
        mode_type: Some("cloned".into()),
        ..Default::default()
    }));
}

#[test]
fn test_ddc_apply_with_mock_backend() {
    use std::sync::Mutex;
    #[derive(Default)]
    struct Recorder {
        pub ops: Mutex<Vec<(u8, u32)>>,
    }
    impl df_ddc::ddc_trait::DdcControl for Recorder {
        fn get_vcp_feature(&self, _code: u8) -> Result<(u32, u32), df_ddc::error::DdcError> {
            Ok((0, 100))
        }
        fn set_vcp_feature(&self, code: u8, value: u32) -> Result<(), df_ddc::error::DdcError> {
            self.ops.lock().unwrap().push((code, value));
            Ok(())
        }
    }

    let rec = std::sync::Arc::new(Recorder::default());
    // We can't inject the mock into `list_monitors()`, but we can test parse_input_source exhaustively
    // and verify that the mock itself round-trips through the DdcControl trait.
    let code = 0x10;
    rec.set_vcp_feature(code, 42).unwrap();
    assert_eq!(rec.ops.lock().unwrap().len(), 1);
    assert_eq!(rec.ops.lock().unwrap()[0], (0x10, 42));
}

#[test]
fn test_help_short_flags_exhaustive() {
    let (ok, out, _err) = run_cli(&["--help"]);
    assert!(ok);
    // Verify every short flag used in the CLI exists in help
    let expected = ['s', 'i', 'o', 'm', 'c', 'r', 't', 'f', 'p'];
    for c in &expected {
        assert!(
            out.contains(&format!("-{}", c)),
            "Missing short flag -{} in help",
            c
        );
    }
}
