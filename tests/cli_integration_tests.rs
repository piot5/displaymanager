use std::process::Command;
use std::path::PathBuf;
use std::fs;

/// Helper to run a binary from the workspace root.
fn run_binary(args: &[&str]) -> (bool, String) {
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--bin", "displaymanager_cli", "--"])
        .args(args)
        .output()
        .expect("failed to execute cli");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (success, stdout)
}

#[test]
fn test_cli_info_outputs_header() {
    let (ok, out) = run_binary(&["info"]);
    assert!(ok, "CLI should exit successfully");
    assert!(out.contains("Display Manager Information"), "Missing header in info output");
}

#[test]
fn test_cli_set_brightness_success() {
    // Use monitor 0 and a safe brightness value.
    let (ok, _out) = run_binary(&["set", "--monitor", "0", "--brightness", "0"]);
    assert!(ok, "Setting brightness should succeed even without a real monitor");
}

/// Test the edid_file_json binary with a fixture.
#[test]
fn test_edid_file_json_output() {
    let fixture = PathBuf::from("tests/fixtures/edid_valid.bin");
    assert!(fixture.exists(), "EDID fixture missing");
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--bin", "edid_file_json", "--"])
        .arg(fixture)
        .output()
        .expect("failed to run edid_file_json");
    assert!(output.status.success(), "edid_file_json should exit successfully");
    let json = String::from_utf8_lossy(&output.stdout);
    // Basic sanity check: JSON should contain a manufacturer_id field.
    assert!(json.contains("manufacturer_id"), "JSON output missing manufacturer_id");
}

/// Test the edid_file_txt binary with a fixture.
#[test]
fn test_edid_file_txt_output() {
    let fixture = PathBuf::from("tests/fixtures/edid_valid.bin");
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--bin", "edid_file_txt", "--"])
        .arg(fixture)
        .output()
        .expect("failed to run edid_file_txt");
    assert!(output.status.success(), "edid_file_txt should exit successfully");
    let txt = String::from_utf8_lossy(&output.stdout);
    // The text output should contain a human‑readable manufacturer string.
    assert!(txt.contains("Manufacturer"), "Text output missing Manufacturer label");
}