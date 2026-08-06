//! DWARF-129 — CLI-dispatch spec for `forge test --target wasm`.
//!
//! These tests pin the dispatch slice: a `.kzd` file run with target `"wasm"`
//! routes through the wasmtime test runner
//! ([`super::runner::WasmTestRunner`]) instead of the legacy Jest passthrough
//! in `dwarf_cli::test::run_test`, and it does not print the "not yet wired"
//! note.
//!
//! The contract is exercised through [`super::dispatch::run_wasm_tests`] — the
//! function `main.rs` calls when `is_wasm_target(&target)` is true.

#![cfg(test)]

use dwarf_lib::{CompileOptions, DwarfCompiler};

use super::dispatch::{is_wasm_target, run_wasm_tests};
use super::runner::WasmTestRunner;

// ---------------------------------------------------------------------------
// Fixtures — real `.kzd` sources written to temp files
// ---------------------------------------------------------------------------

const PASSING_SOURCE: &str = "@test fn test_passing() { true }";

const FAILING_SOURCE: &str = "@test fn test_fail() { assert.consistent(0) }";

const MULTI_SOURCE: &str = "@test fn test_a() { true }
@test fn test_b() { true }";

/// Write `source` to a fresh `.kzd` file and return its path together with the
/// owning `TempDir` guard.
///
/// The guard MUST be held by the caller for as long as the fixture needs to
/// exist: dropping it recursively deletes the temp dir (and with it the
/// `.kzd` file), so returning only the `PathBuf` would remove the fixture the
/// moment `write_kzd` returns — before `run_wasm_tests` reads it back from
/// disk.
fn write_kzd(source: &str) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir for .kzd fixture");
    let path = dir.path().join("tests.kzd");
    std::fs::write(&path, source).expect("write .kzd fixture");
    (path, dir)
}

/// Compile `source` to the wasm target via the public `DwarfCompiler` API.
fn compile_wasm(source: &str) -> dwarf_lib::CompileResult {
    let compiler = DwarfCompiler::new();
    let options = CompileOptions {
        target: "wasm".to_string(),
        ..Default::default()
    };
    compiler
        .compile(source, "tests.kzd", options)
        .expect("DwarfCompiler::compile must return Ok")
}

// ---------------------------------------------------------------------------
// Dispatch decision — `is_wasm_target`
// ---------------------------------------------------------------------------

/// The CLI must route `--target wasm` into the wasmtime runner.
#[test]
fn test_is_wasm_target_accepts_wasm() {
    assert!(
        is_wasm_target("wasm"),
        "the literal target \"wasm\" must route to the wasmtime runner"
    );
}

/// AC 3 — `forge test` with any legacy target keeps the existing behavior:
/// none of ts/py/java may be mistaken for the wasm path.
#[test]
fn test_is_wasm_target_rejects_legacy_targets() {
    for legacy in ["ts", "py", "java"] {
        assert!(
            !is_wasm_target(legacy),
            "target {legacy:?} must keep routing through the legacy runner"
        );
    }
}

// ---------------------------------------------------------------------------
// AC 1 — `forge test --target wasm <passing_test>.kzd` reports pass
// ---------------------------------------------------------------------------

/// A passing @test must produce exactly one result item with `passed: true`
/// and no failure message.
#[test]
fn test_run_wasm_tests_passing_test_reports_passed() {
    let (file, _dir) = write_kzd(PASSING_SOURCE);
    let results = run_wasm_tests(&[file], None);

    assert_eq!(
        results.len(),
        1,
        "a file with one @test must produce exactly one result item, got {:?}",
        results
    );
    assert!(
        results[0].passed,
        "the passing @test must report passed: true, got {:?}",
        results[0]
    );
    assert!(
        !results[0].message.to_lowercase().contains("fail"),
        "a passing test must not carry a failure message, got {:?}",
        results[0]
    );
}

// ---------------------------------------------------------------------------
// AC 2 — `forge test --target wasm <failing_assert>.kzd` reports fail
// ---------------------------------------------------------------------------

/// A trapping @test (`assert.consistent`) must produce one result item with
/// `passed: false` and an expected-vs-actual style message.
#[test]
fn test_run_wasm_tests_failing_assert_reports_failed_with_message() {
    let (file, _dir) = write_kzd(FAILING_SOURCE);
    let results = run_wasm_tests(&[file], None);

    assert_eq!(
        results.len(),
        1,
        "a file with one @test must produce exactly one result item, got {:?}",
        results
    );
    assert!(
        !results[0].passed,
        "the trapping @test must be reported as failed, got {:?}",
        results[0]
    );
    assert!(
        !results[0].message.is_empty(),
        "a failed test must carry an expected-vs-actual message, got {:?}",
        results[0]
    );
}

// ---------------------------------------------------------------------------
// Per-test granularity + filter
// ---------------------------------------------------------------------------

/// One file with two @test functions must yield one result item per test —
/// never a single per-file bucket (which is how the legacy Jest passthrough
/// behaves).
#[test]
fn test_run_wasm_tests_multiple_test_functions_one_item_each() {
    let (file, _dir) = write_kzd(MULTI_SOURCE);
    let results = run_wasm_tests(&[file], None);

    assert_eq!(
        results.len(),
        2,
        "two @test functions must produce two result items, got {:?}",
        results
    );
    assert!(
        results.iter().all(|r| r.passed),
        "both @test functions pass, got {:?}",
        results
    );
}

/// `filter` must restrict execution to matching tests. `TestResultItem` is
/// per-test, so a filtered run yields fewer items.
#[test]
fn test_run_wasm_tests_respects_filter() {
    let (file, _dir) = write_kzd(MULTI_SOURCE);
    let results = run_wasm_tests(&[file], Some("test_a"));

    assert_eq!(
        results.len(),
        1,
        "filtering to one test must yield one result item, got {:?}",
        results
    );
    assert!(
        results[0].passed,
        "the filtered test passes, got {:?}",
        results[0]
    );
}

// ---------------------------------------------------------------------------
// Fixture pipeline — the compiled fixtures run end-to-end
// ---------------------------------------------------------------------------

/// A no-return-type @test compiles to WAT that `wat::parse_str` accepts and
/// the wasmtime runner reports as passing. This proves the passing fixture is
/// sound and pins the compiler+parser+runner pipeline the dispatch depends on.
#[test]
fn test_compile_no_return_type_test_is_parseable_and_runs() {
    let result = compile_wasm(PASSING_SOURCE);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| !matches!(d.severity, dwarf_lib::Severity::Error)),
        "compiling the passing fixture must not produce errors, got {:?}",
        result.diagnostics
    );

    let wasm = wat::parse_str(&result.output)
        .expect("compiled WAT must be a valid module for the runner to execute");

    let test_result = WasmTestRunner::new()
        .run_test(&wasm, "test_passing")
        .expect("a valid, exported @test must execute and return Ok");
    assert!(
        test_result.passed,
        "the emitted passing @test must report passed: true, got {:?}",
        test_result
    );
}

/// A no-return-type `assert.consistent` fixture compiles to WAT that the
/// runner executes as a trap → `passed: false` with a message.
#[test]
fn test_compile_assert_consistent_is_runnable_and_fails() {
    let result = compile_wasm(FAILING_SOURCE);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| !matches!(d.severity, dwarf_lib::Severity::Error)),
        "compiling the failing fixture must not produce errors, got {:?}",
        result.diagnostics
    );

    let wasm = wat::parse_str(&result.output)
        .expect("compiled WAT must be a valid module for the runner to execute");

    let test_result = WasmTestRunner::new()
        .run_test(&wasm, "test_fail")
        .expect("a trapping @test must still return Ok(TestResult)");
    assert!(
        !test_result.passed,
        "the assert.consistent @test must be reported as failed, got {:?}",
        test_result
    );
    assert!(
        test_result.message.is_some(),
        "a failed test must carry a message, got {:?}",
        test_result
    );
}

// ---------------------------------------------------------------------------
// Return-typed @test — field ordering in the emitted WAT
// ---------------------------------------------------------------------------

/// The dispatch must be able to compile the `.kzd` it receives. A return-typed
/// `@test fn test_passing() -> Bool { true }` (the shape `forge scaffold-tests`
/// generates) must emit WAT that parses — the emitter places the `(export ...)`
/// field ahead of `(result i32)` so `wat::parse_str` accepts it.
#[test]
fn test_return_typed_test_compiles_to_parseable_wasm() {
    let source = "@test fn test_passing() -> Bool { true }";
    let result = compile_wasm(source);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| !matches!(d.severity, dwarf_lib::Severity::Error)),
        "compiling a return-typed @test must not produce errors, got {:?}",
        result.diagnostics
    );

    match wat::parse_str(&result.output) {
        Ok(wasm) => {
            let test_result = WasmTestRunner::new()
                .run_test(&wasm, "test_passing")
                .expect("a valid, exported @test must execute and return Ok");
            assert!(test_result.passed, "got {:?}", test_result);
        }
        Err(err) => {
            panic!(
                "compiled WAT for a return-typed @test must be parseable by wat. \
                 Got parse error: {err}\n\nEmitted WAT:\n{}",
                result.output
            );
        }
    }
}
