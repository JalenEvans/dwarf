//! DWARF-130 — deep acceptance-criteria CLI tests for the wasm Draupnir path.
//!
//! These end-to-end tests invoke the compiled `forge` binary via
//! `CARGO_BIN_EXE_forge` (same helper pattern as `draupnir_cli_tests.rs`) and
//! pin the two FORGE-level acceptance criteria that are still missing:
//!
//! 1. **Runtime injection (AC)** — a property body that calls `for_all` must
//!    NOT produce `unknown property: for_all`. The wasm dispatch currently
//!    compiles each `.kzd` via `DwarfCompiler` to target `"wasm"` but never
//!    injects the Draupnir runtime (`dwarf_lib::draupnir::compile_draupnir`),
//!    so `for_all` is an undeclared variable during typechecking and the run
//!    surfaces `unknown variable: for_all`.
//!
//! 2. **Explicit diagnostic (AC)** — when a property body genuinely cannot
//!    compile to wasm (the wasm backend raises `EmitterError::UnsupportedFeature`
//!    for anything outside the i32 subset, e.g. a string literal), the run must
//!    surface a clean human-readable diagnostic containing `unsupported feature`
//!    — never a silent PASS for a test that never ran, never a crash. Today the
//!    emitter error is swallowed into an empty WAT module and the runner reports
//!    a bare `WAT parse error` instead.
//!
//! RED-PHASE: both sections fail right now because the injection and diagnostic
//! surfacing are missing. The implementation must drive them green without
//! touching the DWARF-119 surface (`draupnir_cli_tests.rs`).

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

/// Fixture A — a property whose body calls `for_all`. With the Draupnir runtime
/// absent from the wasm compile path, `for_all` is an undeclared variable and
/// typechecking fails with `unknown variable: for_all`. Runtime injection must
/// make that error disappear and see the run proceed past it.
const PROPERTY_CALLS_FOR_ALL: &str = "@property fn test_property_commutative() { for_all(0, 0) }";

/// Fixture B — a property whose body holds a string literal. The wasm backend
/// supports only the i32 subset (`Int`/`Bool` literals, no strings), so it
/// raises `EmitterError::UnsupportedFeature`. The run must surface that as a
/// clean `unsupported feature` diagnostic instead of an empty-module
/// `WAT parse error` or a silent PASS.
const PROPERTY_UNSUPPORTED: &str = "@property fn test_property_with_str() { \"x\" }";

// ---------------------------------------------------------------------------
// AC 1 — runtime injection: `for_all` in a property body must not be an
// `unknown variable: for_all` error.
// ---------------------------------------------------------------------------

/// A property body that calls `for_all` must NOT produce the `unknown variable:
/// for_all` diagnostic. This pins the runtime-injection bug: the wasm dispatch
/// must pull the Draupnir runtime into the compile path so `for_all` resolves.
#[test]
fn test_wasm_property_for_all_not_unknown_variable() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "injects_runtime.kzd", PROPERTY_CALLS_FOR_ALL);

    let output = forge(&[
        "test",
        "--target",
        "wasm",
        "--draupnir",
        file_path.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // The injection bug is present when the runner reports `unknown variable:
    // for_all` — the runtime was never made visible to the compile path.
    assert!(
        !combined.contains("unknown variable: for_all"),
        "a property calling `for_all` must compile once the Draupnir runtime is \
         injected — the `unknown variable: for_all` error indicates the Dispatch \
         did NOT inject the runtime.\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );
}

// ---------------------------------------------------------------------------
// AC 2 — explicit diagnostic: a non-compilable property must surface a clean
// `unsupported feature` diagnostic, never a silent PASS.
// ---------------------------------------------------------------------------

/// A property that cannot compile to wasm (string literal → wasm
/// `UnsupportedFeature`) must surface a human-readable `unsupported feature`
/// diagnostic, the run must exit non-zero, and it must never be reported as a
/// bare PASS (i.e., a test that ran nothing but passed).
#[test]
fn test_wasm_property_unsupported_feature_surfaces_diagnostic() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = write_kzd(dir.path(), "unsupported_property.kzd", PROPERTY_UNSUPPORTED);

    let output = forge(&[
        "test",
        "--target",
        "wasm",
        "--draupnir",
        file_path.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // The run must not exit cleanly: a test that could never run must not be a
    // silent PASS.
    assert!(
        !output.status.success(),
        "a property that cannot compile to wasm must fail the run, not pass \
         silently.\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );

    // The diagnostic must name the unsupported-feature cause (the wasm backend
    // supports only i32) rather than an opaque `WAT parse error` on an empty
    // module or a blanket pass.
    assert!(
        combined.contains("unsupported feature"),
        "the run must surface a clean diagnostic mentioning `unsupported feature` \
         for a property the wasm backend cannot emit (string literal outside the \
         i32 subset).\n\
         exit: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr,
    );

    // Never a silent PASS: the per-file result must be FAIL, not `All tests passed`.
    assert!(
        !combined.contains("All tests passed"),
        "a property that never ran must not be reported as all-passing.\n\
         stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr,
    );
}
