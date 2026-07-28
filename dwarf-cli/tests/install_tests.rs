//! RED-phase tests for the `dwarf install` CLI subcommand (Chunk 5).
//!
//! These tests verify the MVP behavior of `dwarf install`:
//! - The subcommand exists and shows help
//! - It generates extern declaration stubs for supported package sources
//! - It rejects unknown source prefixes
//!
//! EXPECTED: All tests FAIL because the `install` subcommand does not yet exist.
//! Once Chunk 5 is implemented, these tests should pass (GREEN phase).

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
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().join("target/debug/dwarf-cli")
}

// ---------------------------------------------------------------------------
// Test 1: install command exists and shows help
// ---------------------------------------------------------------------------
// Verifies that `dwarf install --help` succeeds and prints help text.
// This test will FAIL with a clap "unrecognized subcommand" error until
// the Install variant is added to the Commands enum in main.rs.
// ---------------------------------------------------------------------------
#[test]
fn test_install_help() {
    let output = dwarf(&["install", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "dwarf install --help should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    assert!(
        stdout.contains("install"),
        "Help output should mention 'install'"
    );
}

// ---------------------------------------------------------------------------
// Test 2: install npm package generates extern declaration
// ---------------------------------------------------------------------------
// Verifies that `dwarf install npm:express` produces an extern declaration
// in the format the parser expects:
//   extern "npm:express" fn express() -> ()
//
// The MVP may output to stdout OR write to a file. This test checks stdout.
// Will FAIL because the install subcommand doesn't exist yet.
// ---------------------------------------------------------------------------
#[test]
fn test_install_npm_package() {
    let output = dwarf(&["install", "npm:express"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "dwarf install npm:express should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Output should contain an extern declaration matching the parser format
    assert!(
        stdout.contains(r#"extern "npm:express""#),
        "Output should contain extern declaration with npm:express source.\nGot: {}",
        stdout
    );
    assert!(
        stdout.contains("fn"),
        "Output should contain 'fn' keyword for function declaration.\nGot: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// Test 3: install python package generates extern declaration
// ---------------------------------------------------------------------------
// Verifies that `dwarf install py:json` produces:
//   extern "py:json" fn ...
//
// Will FAIL because the install subcommand doesn't exist yet.
// ---------------------------------------------------------------------------
#[test]
fn test_install_python_package() {
    let output = dwarf(&["install", "py:json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "dwarf install py:json should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    assert!(
        stdout.contains(r#"extern "py:json""#),
        "Output should contain extern declaration with py:json source.\nGot: {}",
        stdout
    );
    assert!(
        stdout.contains("fn"),
        "Output should contain 'fn' keyword.\nGot: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// Test 4: install java package generates extern declaration
// ---------------------------------------------------------------------------
// Verifies that `dwarf install java:java.util.ArrayList` produces:
//   extern "java:java.util" fn ArrayList() -> ()
//
// The package path (java.util) becomes the source string, and the class
// name (ArrayList) becomes the function name.
//
// Will FAIL because the install subcommand doesn't exist yet.
// ---------------------------------------------------------------------------
#[test]
fn test_install_java_package() {
    let output = dwarf(&["install", "java:java.util.ArrayList"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "dwarf install java:java.util.ArrayList should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Java: package path becomes source, class name becomes fn name
    assert!(
        stdout.contains(r#"extern "java:java.util""#),
        "Output should contain extern declaration with java:java.util source.\nGot: {}",
        stdout
    );
    assert!(
        stdout.contains("ArrayList"),
        "Output should contain 'ArrayList' as the function name.\nGot: {}",
        stdout
    );
    assert!(
        stdout.contains("fn"),
        "Output should contain 'fn' keyword.\nGot: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// Test 5: install unknown source prefix produces error
// ---------------------------------------------------------------------------
// Verifies that `dwarf install foo:bar` exits non-zero with an error
// message about unknown source prefix. Only npm:, py:, java: are valid.
//
// Will FAIL because the install subcommand doesn't exist yet (clap will
// reject it as unrecognized subcommand, not as "unknown source prefix").
// Once implemented, this should fail with a domain-specific error.
// ---------------------------------------------------------------------------
#[test]
fn test_install_unknown_source_prefix() {
    let output = dwarf(&["install", "foo:bar"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should exit non-zero
    assert!(
        !output.status.success(),
        "dwarf install foo:bar should fail (unknown source prefix).\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // Error message should mention the unknown/invalid source
    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("unknown")
            || combined.contains("invalid")
            || combined.contains("unsupported")
            || combined.contains("error"),
        "Error output should mention unknown/invalid source.\nGot stdout: {}\nGot stderr: {}",
        stdout,
        stderr
    );
}
