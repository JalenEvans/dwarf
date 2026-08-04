//! Integration tests for DWARF-118: dUnit assertion library.
//!
//! These tests define the expected contract for the dUnit runtime:
//!
//! - Part 1 (`dwarf-lib/runtime/dunit/dunit.dwarf`): the assertion
//!   library source file must exist, parse cleanly, and expose the
//!   `assert`, `assert_eq`, and `assert_err` functions to `@test`
//!   bodies.
//! - Part 2 (`dwarf-lib/src/dunit.rs`): the Rust module must be
//!   declared in `lib.rs` and expose `pub fn compile_dunit() -> String`
//!   that reads the source file and returns compiled output.
//!
//! RED PHASE: Tests 1-3 may pass once the stub source/module exist;
//! Test 4 fails because the `compile_dunit()` stub returns an empty
//! string instead of compiled output. Test 4 is the implementation
//! contract.

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::Parser;
use dwarf_syntax::hir::Decl;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Absolute path to the dUnit source file under test.
fn dunit_source_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("runtime")
        .join("dunit")
        .join("dunit.dwarf")
}

/// Read the dUnit source file, panicking with a helpful message if absent.
fn read_dunit_source() -> String {
    let path = dunit_source_path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("Failed to read dunit.dwarf at {:?}: {}", path, e)
    })
}

/// Tokenize and parse Dwarf source, returning (decls, parse_errors).
fn parse_source(source: &str) -> (Vec<Decl>, Vec<dwarf_parser::ParseError>) {
    let tokens = TokenizePass
        .tokenize(source)
        .expect("dunit.dwarf should tokenize without lexer errors");
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Names of top-level function declarations in the parsed source.
fn parsed_function_names(source: &str) -> Vec<String> {
    let (decls, _errors) = parse_source(source);
    decls
        .iter()
        .filter_map(|decl| match decl {
            Decl::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

// ===========================================================================
// Part 1: dunit.dwarf source file
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 1: dunit.dwarf exists and parses cleanly
// ---------------------------------------------------------------------------

#[test]
fn test_dunit_source_file_exists_and_parses() {
    let path = dunit_source_path();

    assert!(
        path.exists(),
        "dunit.dwarf should exist at {:?}, but file not found",
        path
    );

    let source = read_dunit_source();

    // The file must contain at least one function declaration.
    let (decls, errors) = parse_source(&source);
    assert!(
        errors.is_empty(),
        "dunit.dwarf should parse without errors, got:\n  {}",
        errors
            .iter()
            .map(|e| format!("{} (code {})", e.message, e.code))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    assert!(
        !decls.is_empty(),
        "dunit.dwarf should parse to at least one declaration"
    );
}

// ---------------------------------------------------------------------------
// Test 2: dunit module exposes an `assert` function
// ---------------------------------------------------------------------------

#[test]
fn test_dunit_source_exposes_assert_function() {
    let source = read_dunit_source();

    // Raw signature check (robust to whitespace/comment differences).
    assert!(
        source.contains("fn assert("),
        "dunit.dwarf should contain 'fn assert(' function, got:\n{}",
        source
    );
    assert!(
        source.contains("condition: Bool") || source.contains("condition: bool"),
        "assert should take a Bool condition parameter"
    );
    assert!(
        source.contains("message: Str") || source.contains("message: String"),
        "assert should take a Str message parameter"
    );

    // Parsed declaration check: assert must be a top-level function.
    let names = parsed_function_names(&source);
    assert!(
        names.contains(&"assert".to_string()),
        "dunit.dwarf should declare a top-level 'assert' function, got functions: {:?}",
        names
    );
}

// ===========================================================================
// Part 2: dunit Rust module
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 3: dunit module is declared in lib.rs and exists on disk
// ---------------------------------------------------------------------------

#[test]
fn test_dunit_module_declared_in_lib() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let lib_rs_path = PathBuf::from(manifest_dir).join("src").join("lib.rs");

    assert!(
        lib_rs_path.exists(),
        "dwarf-lib/src/lib.rs should exist at {:?}",
        lib_rs_path
    );

    let lib_content = std::fs::read_to_string(&lib_rs_path)
        .expect("should be able to read lib.rs");

    assert!(
        lib_content.contains("mod dunit") || lib_content.contains("pub mod dunit"),
        "lib.rs should declare 'mod dunit' or 'pub mod dunit', got:\n{}",
        lib_content
    );

    // The module source file must exist so the crate compiles the module.
    let dunit_rs_path = PathBuf::from(manifest_dir).join("src").join("dunit.rs");
    assert!(
        dunit_rs_path.exists(),
        "dwarf-lib/src/dunit.rs should exist at {:?}, but file not found",
        dunit_rs_path
    );
}

// ---------------------------------------------------------------------------
// Test 4: compile_dunit() returns compiled output from the source file
// ---------------------------------------------------------------------------

#[test]
fn test_compile_dunit_returns_compiled_output() {
    // The public API surface: pub fn compile_dunit() -> String.
    // This reads dunit.dwarf and returns the compiled target output.
    let output = dwarf_lib::dunit::compile_dunit();

    assert!(
        !output.is_empty(),
        "compile_dunit() should return non-empty compiled output (currently the stub \
         returns an empty string — this is the RED-phase contract)"
    );

    // Compiled output should contain the assertion functions.
    assert!(
        output.contains("assert"),
        "compiled output should contain 'assert', got:\n{}",
        output
    );
    assert!(
        output.contains("assert_eq") || output.contains("assertEqual"),
        "compiled output should contain 'assert_eq', got:\n{}",
        output
    );
    assert!(
        output.contains("assert_err") || output.contains("assertError"),
        "compiled output should contain 'assert_err', got:\n{}",
        output
    );
}
