//! End-to-end integration tests for the `forge` CLI binary.
//!
//! These tests invoke the compiled forge binary via `std::process::Command`,
//! exercising the full passthrough path from forge into dwarf_cli. Each test
//! uses isolated temp directories so they can run in parallel without
//! interfering with each other.

use std::fs;
use std::process::Command;

/// Helper: run the forge binary with the given arguments and return the Output.
fn forge(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .output()
        .expect("Failed to execute forge binary")
}

/// Helper: write a .kzd file into `dir` with the given name and content,
/// returning the full path to the file.
fn write_kzd(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).expect("Failed to write .kzd test file");
    file_path
}

// ---------------------------------------------------------------------------
// 1. forge check — valid file exits 0
// ---------------------------------------------------------------------------

#[test]
fn test_forge_check_valid_file() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "valid.kzd", "fn main() { 42 }");

    let output = forge(&["check", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "forge check on a valid .kzd file should exit 0.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );
}

// ---------------------------------------------------------------------------
// 2. forge check — invalid file exits non-zero
// ---------------------------------------------------------------------------

#[test]
fn test_forge_check_invalid_file() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "broken.kzd", "fn broken( { 1 }");

    let output = forge(&["check", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "forge check on a broken .kzd file should exit non-zero.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    // The compiler should report some kind of error diagnostic.
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.to_lowercase().contains("error"),
        "Expected an error message in output, got:\n{}",
        combined,
    );
}

// ---------------------------------------------------------------------------
// 3. forge build --target ts — produces a .ts output file
// ---------------------------------------------------------------------------

#[test]
fn test_forge_build_typescript() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "hello.kzd", "fn main() { 42 }");
    let out_dir = dir.path().join("out");

    let output = forge(&[
        "build",
        file_path.to_str().unwrap(),
        "--target",
        "ts",
        "--out-dir",
        out_dir.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "forge build --target ts should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    // The build command writes to <out_dir>/ts/<stem>.ts
    let expected_ts = out_dir.join("ts").join("hello.ts");
    assert!(
        expected_ts.exists(),
        "Expected TypeScript output file at {:?}, but it does not exist.\n\
         Contents of out_dir:\n{:?}",
        expected_ts,
        fs::read_dir(&out_dir)
            .map(|entries| entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .collect::<Vec<_>>())
            .unwrap_or_default(),
    );

    // The output file should not be empty.
    let ts_content = fs::read_to_string(&expected_ts).expect("Failed to read .ts output");
    assert!(
        !ts_content.is_empty(),
        "TypeScript output file should not be empty",
    );
}

// ---------------------------------------------------------------------------
// 4. forge emit --target py — produces Python output to stdout
// ---------------------------------------------------------------------------

#[test]
fn test_forge_emit_python() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "greet.kzd", "fn main() { 42 }");

    let output = forge(&["emit", file_path.to_str().unwrap(), "--target", "py"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "forge emit --target py should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    // emit prints "// <file> [<target>]:\n<output>" to stdout.
    // Verify we got something that looks like Python output.
    assert!(
        !stdout.is_empty(),
        "forge emit should produce output on stdout",
    );

    // The output should contain Python-like content (function definition,
    // return statement, or at minimum some generated code).
    let lower = stdout.to_lowercase();
    let has_python_signals = lower.contains("def ")
        || lower.contains("return")
        || lower.contains("42")
        || lower.contains("main");
    assert!(
        has_python_signals,
        "Expected Python-like output from forge emit --target py, got:\n{}",
        stdout,
    );
}

// ---------------------------------------------------------------------------
// 5. forge --help — exits 0 and prints usage
// ---------------------------------------------------------------------------

#[test]
fn test_forge_help() {
    let output = forge(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "forge --help should exit 0.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    assert!(
        stdout.contains("forge"),
        "Help output should mention 'forge'.\nGot:\n{}",
        stdout,
    );

    // Verify that key subcommands are listed in the help output.
    for subcmd in &["check", "build", "emit", "run", "fmt", "test"] {
        assert!(
            stdout.contains(subcmd),
            "Help output should list the '{}' subcommand.\nGot:\n{}",
            subcmd,
            stdout,
        );
    }
}

// ---------------------------------------------------------------------------
// 6. forge build --help — exits 0
// ---------------------------------------------------------------------------

#[test]
fn test_forge_build_help() {
    let output = forge(&["build", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "forge build --help should exit 0.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    // The build subcommand help should mention its key flags.
    assert!(
        stdout.contains("--target"),
        "build --help should mention --target.\nGot:\n{}",
        stdout,
    );
    assert!(
        stdout.contains("--out-dir"),
        "build --help should mention --out-dir.\nGot:\n{}",
        stdout,
    );
}
