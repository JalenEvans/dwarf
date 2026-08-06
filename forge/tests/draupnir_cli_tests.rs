//! DWARF-119 — `forge test --draupnir` CLI contract.
//!
//! These end-to-end tests invoke the compiled `forge` binary (via
//! `CARGO_BIN_EXE_forge`, same pattern as `integration_tests.rs`) and pin the
//! acceptance criterion: **`forge test --draupnir` runs unit AND property
//! tests** through the wasmtime runner.
//!
//! Now GREEN: the `--draupnir` flag exists on `Commands::Test`
//! (`forge/src/main.rs`), parses, and routes into the wasm runner. Both tests
//! pass and pin that the flag executes every runnable test (unit `@test`
//! functions and property tests) in the file and reports per-test PASS/FAIL.

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

/// A fixture with one unit `@test` and one property test. The property is
/// marked as a `@test` whose body is a Draupnir property (a `for_all` call).
/// `--draupnir` executes BOTH the unit test and the property test and reports
/// all pass.
const UNIT_PLUS_PROPERTY_PASSING: &str = "@test fn test_unit_passing() { true }
@property fn test_property_commutative() { true }";

/// A fixture whose property test fails (traps) — `--draupnir` must surface it
/// as a FAIL so the overall run exits non-zero.
const PROPERTY_FAILING: &str = "@test fn test_unit_passing() { true }
@property fn test_property_broken() { assert.consistent(0) }";

// ---------------------------------------------------------------------------
// AC — `forge test --draupnir` runs unit + property tests and reports PASS
// ---------------------------------------------------------------------------

/// A file with a passing unit test and a passing property test must run both
/// under `forge test --draupnir` and report success (exit 0, PASS verdict, no
/// "not yet wired" note).
#[test]
fn test_forge_test_draupnir_runs_unit_and_property_tests() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(
        dir.path(),
        "unit_and_property.kzd",
        UNIT_PLUS_PROPERTY_PASSING,
    );

    let output = forge(&["test", "--target", "wasm", "--draupnir", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        output.status.success(),
        "forge test --draupnir should exit 0 when unit + property tests pass.\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );

    // Both the unit test and the property test must be reported as run/passed.
    assert!(
        stdout.contains("test_unit_passing"),
        "the unit @test must be reported, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("test_property_commutative"),
        "the property test must be reported (unit + property BOTH run), got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("PASS"),
        "the run must end with a PASS verdict, got:\n{}",
        stdout
    );
    assert!(
        !combined.contains("not yet wired"),
        "--draupnir must not fall through to the legacy 'not yet wired' note, got:\n{}",
        combined
    );
}

// ---------------------------------------------------------------------------
// AC — `forge test --draupnir` surfaces a failing property as FAIL
// ---------------------------------------------------------------------------

/// A file whose property test traps must be reported as FAIL and the overall
/// run must exit non-zero.
#[test]
fn test_forge_test_draupnir_reports_failing_property() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "failing_property.kzd", PROPERTY_FAILING);

    let output = forge(&["test", "--target", "wasm", "--draupnir", file_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "forge test --draupnir must exit non-zero when a property test fails.\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );
    assert!(
        stdout.contains("test_property_broken") && stdout.contains("FAIL"),
        "the failing property test must be reported as FAIL, got:\n{}",
        stdout
    );
    // The passing unit test in the same file must still be reported.
    assert!(
        stdout.contains("test_unit_passing"),
        "the unit @test must still be reported alongside the failing property, got:\n{}",
        stdout
    );
}
