//! RED-phase integration tests for extern/FFI support through the full
//! compilation pipeline (lexer → parser → typecheck → MIR → LIR → emitter).
//!
//! These tests verify that extern declarations and calls to extern functions
//! flow through the entire pipeline without crashing and produce sensible
//! output. They are expected to FAIL until extern codegen is implemented.
//!
//! See: feat/ffi-host-interop branch, Phase 1-3 integration.

use dwarf_lib::{CompileOptions, DwarfCompiler, Severity};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ts_options() -> CompileOptions {
    CompileOptions {
        target: "ts".to_string(),
        pretty: true,
        passes: None,
        skip_passes: vec![],
        source_map: false,
        stdlib_path: None,
    }
}

/// Assert that compilation succeeded (Ok) and return the CompileResult.
/// Panics with diagnostic details on Err.
fn expect_ok(
    result: Result<dwarf_lib::CompileResult, Vec<dwarf_lib::DwarfError>>,
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
// Test 1: Extern declaration + call in function body (full pipeline)
//
// This is the key integration test: a Dwarf source file declares an extern
// function from an npm package, then calls it from within a regular function.
// The full pipeline must:
//   1. Parse the extern declaration into Decl::Extern
//   2. Parse the function with a call to the extern name
//   3. Typecheck without crashing (extern names resolve as known bindings)
//   4. Lower through MIR → LIR
//   5. Emit TypeScript with an import for the extern and a call expression
//
// Expected to FAIL until extern codegen is wired through all passes.
// ---------------------------------------------------------------------------

#[test]
fn test_extern_call_in_function_body_full_pipeline() {
    let source = r#"
        extern "npm:express" fn express() -> Any

        fn create_app() -> Any {
            express()
        }
    "#;

    let compiler = DwarfCompiler::new();
    let options = ts_options();
    let result = expect_ok(compiler.compile(source, "app.dwarf", options));

    // Pipeline should produce non-empty output
    assert!(
        !result.output.is_empty(),
        "output should not be empty for extern + call source"
    );

    // The emitted TypeScript should reference the extern function name
    assert!(
        result.output.contains("express"),
        "output should reference the extern function 'express', got:\n{}",
        result.output
    );

    // The emitted TypeScript should contain the calling function
    assert!(
        result.output.contains("create_app"),
        "output should contain the calling function 'create_app', got:\n{}",
        result.output
    );

    // For TypeScript target, extern with "npm:" source should produce an
    // import statement (or at minimum, not a stub comment).
    // This assertion specifies the expected codegen behavior:
    assert!(
        result.output.contains("import") || result.output.contains("express"),
        "TS output should contain an import or reference for npm extern, got:\n{}",
        result.output
    );

    // No error-severity diagnostics for valid extern usage
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error));
    assert!(
        !has_errors,
        "expected no error diagnostics for valid extern usage, got: {:?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// Test 2: Multiple externs with calls — pipeline does not crash
//
// Verifies that the pipeline handles multiple extern declarations from
// different sources (npm, py) and function bodies that call them.
// Backend filtering should apply: TS target emits only npm externs.
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_externs_with_calls_pipeline() {
    let source = r#"
        extern "npm:express" fn express() -> Any
        extern "npm:fs" fn readFileSync(path: String) -> String
        extern "py:json" fn dumps(obj: Any) -> String

        fn setup() -> Any {
            express()
        }

        fn load(path: String) -> String {
            readFileSync(path)
        }
    "#;

    let compiler = DwarfCompiler::new();
    let options = ts_options();
    let result = expect_ok(compiler.compile(source, "multi.dwarf", options));

    // Output should not be empty
    assert!(
        !result.output.is_empty(),
        "output should not be empty for multi-extern source"
    );

    // TS backend should reference the npm externs
    assert!(
        result.output.contains("express"),
        "TS output should reference npm extern 'express'"
    );
    assert!(
        result.output.contains("readFileSync"),
        "TS output should reference npm extern 'readFileSync'"
    );

    // TS backend should NOT emit Python extern
    // (py:json is filtered out for TypeScript target)
    // Note: this specifies expected backend-filtering behavior
    let output_lower = result.output.to_lowercase();
    assert!(
        !output_lower.contains("py:json") && !output_lower.contains("dumps"),
        "TS output should NOT reference py: extern 'dumps', got:\n{}",
        result.output
    );
}
