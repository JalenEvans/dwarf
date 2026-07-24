//! Integration tests for `DwarfCompiler::compile()`.
//!
//! These tests verify the full compilation pipeline through the public API.
//! All tests are expected to FAIL in the current Red phase because `compile()`
//! panics with `todo!()`. They serve as the specification for the expected
//! behavior once implementation is complete.

use dwarf_lib::{CompileOptions, DwarfCompiler, DwarfError, Severity};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_options() -> CompileOptions {
    CompileOptions::default()
}

/// Assert that a `Result` is `Ok` and return the inner `CompileResult`.
/// Panics with a helpful message if the result is `Err`.
fn expect_ok(
    result: Result<dwarf_lib::CompileResult, Vec<DwarfError>>,
) -> dwarf_lib::CompileResult {
    match result {
        Ok(r) => r,
        Err(errors) => {
            let msg: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            panic!(
                "Expected Ok(CompileResult), got Err:\n  {}",
                msg.join("\n  ")
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1: Basic compilation of valid Dwarf source
// ---------------------------------------------------------------------------

#[test]
fn basic_compilation_valid_source() {
    let compiler = DwarfCompiler::new();
    let options = default_options();

    let result = expect_ok(compiler.compile("fn main() = 42", "main.dwarf", options));

    // The emitted output should be non-empty and contain the number literal.
    assert!(!result.output.is_empty(), "output should not be empty");
    assert!(
        result.output.contains("42"),
        "output should contain the compiled value '42', got: {}",
        result.output,
    );

    // No errors or warnings for valid source.
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics for valid source, got: {:?}",
        result.diagnostics,
    );

    // Default target is TypeScript.
    assert_eq!(
        result.output_extension, "ts",
        "expected output extension 'ts', got: {}",
        result.output_extension,
    );
}

// ---------------------------------------------------------------------------
// Test 2: Compilation of a function with parameters
// ---------------------------------------------------------------------------

#[test]
fn compilation_with_params() {
    let compiler = DwarfCompiler::new();
    let options = CompileOptions {
        target: "ts".to_string(),
        pretty: true,
        passes: None,
        skip_passes: vec![],
        source_map: false,
    };

    let result = expect_ok(compiler.compile(
        "fn add(a: Int, b: Int) -> Int = a + b",
        "math.dwarf",
        options,
    ));

    // Output should be non-empty.
    assert!(!result.output.is_empty(), "output should not be empty");

    // The emitted function should reference the function name and parameters.
    assert!(
        result.output.contains("add"),
        "output should contain function name 'add', got: {}",
        result.output,
    );
    assert!(
        result.output.contains("a"),
        "output should contain parameter 'a', got: {}",
        result.output,
    );
    assert!(
        result.output.contains("b"),
        "output should contain parameter 'b', got: {}",
        result.output,
    );

    // No errors for valid source.
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics,
    );
}

// ---------------------------------------------------------------------------
// Test 3: Compilation with type error source
// ---------------------------------------------------------------------------

#[test]
fn compilation_with_type_errors() {
    let compiler = DwarfCompiler::new();
    let options = default_options();

    // This source is syntactically valid but contains a type mismatch:
    // adding a string literal and an integer literal. The compiler should
    // report type errors via diagnostics but still produce output (it should
    // NOT abort compilation).
    let result =
        expect_ok(compiler.compile("fn main() = \"hello\" + 1", "type_error.dwarf", options));

    // Output should still be produced even when type errors are present.
    assert!(
        !result.output.is_empty(),
        "output should not be empty even with type errors"
    );

    // Diagnostics must be non-empty — at least one type error reported.
    assert!(
        !result.diagnostics.is_empty(),
        "expected at least one diagnostic for type-error source",
    );

    // At least one diagnostic should be an error-level severity.
    let has_error = result
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error));
    assert!(
        has_error,
        "expected at least one error-severity diagnostic, got: {:?}",
        result.diagnostics,
    );

    // Each diagnostic should have a code and message.
    for (i, diag) in result.diagnostics.iter().enumerate() {
        assert!(
            !diag.code.is_empty(),
            "diagnostic[{}].code should not be empty",
            i,
        );
        assert!(
            !diag.message.is_empty(),
            "diagnostic[{}].message should not be empty",
            i,
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Multiple compilations with reuse of the same compiler instance
// ---------------------------------------------------------------------------

#[test]
fn multiple_compilations_reuse() {
    let compiler = DwarfCompiler::new();

    // First compilation.
    let result1 = expect_ok(compiler.compile("fn main() = 42", "a.dwarf", default_options()));
    assert!(
        !result1.output.is_empty(),
        "first compilation output should not be empty"
    );

    // Second compilation on the same compiler instance — different source.
    let result2 = expect_ok(compiler.compile(
        "fn foo() = 99",
        "b.dwarf",
        CompileOptions {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: vec![],
            source_map: false,
        },
    ));
    assert!(
        !result2.output.is_empty(),
        "second compilation output should not be empty"
    );

    // Both results should independently be correct.
    assert!(
        result1.output.contains("42"),
        "first output should contain '42', got: {}",
        result1.output,
    );
    assert!(
        result2.output.contains("99"),
        "second output should contain '99', got: {}",
        result2.output,
    );

    // Both should have clean diagnostics.
    assert!(
        result1.diagnostics.is_empty(),
        "first compilation: expected no diagnostics, got: {:?}",
        result1.diagnostics,
    );
    assert!(
        result2.diagnostics.is_empty(),
        "second compilation: expected no diagnostics, got: {:?}",
        result2.diagnostics,
    );
}
