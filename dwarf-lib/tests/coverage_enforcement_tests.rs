//! Integration tests for test-coverage enforcement in the compiler pipeline.
//!
//! DWARF-117: after typechecking, the pipeline runs the test coverage pass
//! (and edge analysis). Public functions without test coverage produce
//! `DWARF-E-COVER` diagnostics — errors in `Required` mode, warnings in
//! `Warning` mode — while `--quick` bypasses the checks and `--test-coverage=off`
//! disables them entirely.

use dwarf_lib::{CompileOptions, CoverageMode, DwarfCompiler, Severity};

/// A public function with no test coverage — must be flagged by the coverage pass.
const UNCOVERED_PUB_FN: &str = "pub fn add(a: Int, b: Int) -> Int = a + b";

fn compile(source: &str, options: CompileOptions) -> dwarf_lib::CompileResult {
    let compiler = DwarfCompiler::new();
    compiler
        .compile(source, "coverage.kzd", options)
        .expect("pipeline should return Ok with diagnostics, not Err")
}

fn coverage_mode(mode: CoverageMode) -> CompileOptions {
    CompileOptions {
        test_coverage: mode,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Required mode → hard errors
// ---------------------------------------------------------------------------

#[test]
fn required_mode_flags_uncovered_public_function() {
    let result = compile(UNCOVERED_PUB_FN, coverage_mode(CoverageMode::Required));

    let cover_errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.contains("COVER"))
        .collect();

    assert!(
        !cover_errors.is_empty(),
        "Required mode should flag the uncovered public function, got: {:?}",
        result.diagnostics
    );
    assert!(
        cover_errors
            .iter()
            .all(|d| matches!(d.severity, Severity::Error)),
        "Required mode coverage diagnostics must be errors, got: {:?}",
        cover_errors
    );
}

// ---------------------------------------------------------------------------
// Warning mode → warnings, build continues
// ---------------------------------------------------------------------------

#[test]
fn warning_mode_emits_coverage_warning_but_not_error() {
    let result = compile(UNCOVERED_PUB_FN, coverage_mode(CoverageMode::Warning));

    let cover_diags: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code.contains("COVER"))
        .collect();

    assert!(
        !cover_diags.is_empty(),
        "Warning mode should still report uncovered functions, got: {:?}",
        result.diagnostics
    );
    assert!(
        cover_diags
            .iter()
            .all(|d| matches!(d.severity, Severity::Warning)),
        "Warning mode coverage diagnostics must be warnings, got: {:?}",
        cover_diags
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error)),
        "Warning mode must not produce errors, got: {:?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// --quick bypasses coverage checks
// ---------------------------------------------------------------------------

#[test]
fn quick_mode_bypasses_coverage_checks() {
    let result = compile(
        UNCOVERED_PUB_FN,
        CompileOptions {
            quick: true,
            test_coverage: CoverageMode::Required,
            ..Default::default()
        },
    );

    assert!(
        !result.diagnostics.iter().any(|d| d.code.contains("COVER")),
        "--quick should bypass coverage checks, got: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "--quick source should compile clean, got: {:?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// --test-coverage=off disables coverage checks
// ---------------------------------------------------------------------------

#[test]
fn off_mode_disables_coverage_checks() {
    let result = compile(UNCOVERED_PUB_FN, coverage_mode(CoverageMode::Off));

    assert!(
        !result.diagnostics.iter().any(|d| d.code.contains("COVER")),
        "--test-coverage=off should disable coverage checks, got: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "off mode source should compile clean, got: {:?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// Covered public function passes in Required mode
// ---------------------------------------------------------------------------

#[test]
fn required_mode_passes_when_annotation_grants_coverage() {
    let source = "pub fn add(a: Int, b: Int) -> Int = a + b\n@tested(add) fn test_add() { 42 }";
    let result = compile(source, coverage_mode(CoverageMode::Required));

    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error)),
        "A @tested-covered public function must pass in Required mode, got: {:?}",
        result.diagnostics
    );
}
