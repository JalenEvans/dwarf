use std::path::PathBuf;
use std::process::Command;

/// Helper to run the dwarf binary with given args.
fn dwarf(args: &[&str]) -> std::process::Output {
    let bin_path = get_binary_path();
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to run dwarf binary")
}

/// Find the dwarf binary in cargo's build output.
fn get_binary_path() -> PathBuf {
    // Check common build output directories
    let candidates = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/dwarf-cli"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/dwarf-cli"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/dwarf-cli"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!("Cannot find dwarf-cli binary. Tried: {:?}", candidates);
}

#[test]
fn test_cli_help() {
    let output = dwarf(&["check", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dwarf check --help should succeed.\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
    assert!(stdout.contains("check"), "Help should mention 'check'");
    assert!(stdout.contains("--json"), "Help should mention --json");
    assert!(stdout.contains("--passes"), "Help should mention --passes");
    assert!(stdout.contains("--skip-passes"), "Help should mention --skip-passes");
    assert!(stdout.contains("--list-passes"), "Help should mention --list-passes");
}

#[test]
fn test_cli_list_passes() {
    let output = dwarf(&["check", "--list-passes"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "--list-passes should succeed");
    assert!(stdout.contains("tokenize"), "Should list tokenize pass");
    assert!(stdout.contains("parse"), "Should list parse pass");
}

#[test]
fn test_cli_valid_file() {
    // Create a temp file with valid dwarf code
    let dir = std::env::temp_dir().join("dwarf_test");
    std::fs::create_dir_all(&dir).ok();
    let file_path = dir.join("test_valid.kzd");
    std::fs::write(&file_path, "fn main() { 42 }").unwrap();

    let output = dwarf(&["check", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Valid file should exit 0.\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_cli_invalid_file() {
    let dir = std::env::temp_dir().join("dwarf_test");
    std::fs::create_dir_all(&dir).ok();
    let file_path = dir.join("test_invalid.kzd");
    std::fs::write(&file_path, "fn broken( { 1 }").unwrap();

    let output = dwarf(&["check", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "Invalid file should exit non-zero.\nstdout: {}\nstderr: {}",
        stdout, stderr
    );
    assert!(
        stdout.contains("error") || stderr.contains("error"),
        "Should report errors"
    );

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_cli_nonexistent_file() {
    let output = dwarf(&["check", "/tmp/nonexistent_file.kzd"]);
    assert!(
        !output.status.success(),
        "Nonexistent file should exit non-zero"
    );
}

#[test]
fn test_cli_json_output() {
    let dir = std::env::temp_dir().join("dwarf_test");
    std::fs::create_dir_all(&dir).ok();
    let file_path = dir.join("test_json.kzd");
    std::fs::write(&file_path, "fn main() { 42 }").unwrap();

    let output = dwarf(&["check", "--json", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "JSON output should succeed");

    // Should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(json["ok"], true, "JSON should have ok: true");

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_cli_json_output_with_errors() {
    let dir = std::env::temp_dir().join("dwarf_test");
    std::fs::create_dir_all(&dir).ok();
    let file_path = dir.join("test_json_err.kzd");
    std::fs::write(&file_path, "fn broken( { 1 }").unwrap();

    let output = dwarf(&["check", "--json", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // JSON output even with errors should be valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON even with errors");

    assert_eq!(json["ok"], false, "JSON should have ok: false for errors");
    assert!(json["errors"].is_array(), "JSON should have errors array");
    assert!(
        json["errors"].as_array().map_or(false, |e| !e.is_empty()),
        "JSON errors array should not be empty"
    );

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}

#[test]
fn test_cli_passes_filter() {
    let dir = std::env::temp_dir().join("dwarf_test");
    std::fs::create_dir_all(&dir).ok();
    let file_path = dir.join("test_pass_filter.kzd");
    std::fs::write(&file_path, "fn main() { 42 }").unwrap();

    // Only run tokenize pass
    let output = dwarf(&[
        "check",
        "--passes",
        "tokenize",
        file_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "Filtered passes should work");

    // Cleanup
    std::fs::remove_file(&file_path).ok();
}
