//! Integration tests for DWARF-119: Draupnir property-based testing framework.
//!
//! These tests define the expected contract for the Draupnir runtime, following
//! the same structure as `dunit_tests.rs` (DWARF-118):
//!
//! - Part 1 (`dwarf-lib/runtime/draupnir/draupnir.dwarf`): the entry-point
//!   library source exposing `for_all` (arity 1-6) to `@test` bodies.
//! - Part 2 (`dwarf-lib/runtime/draupnir/combinators.dwarf`): the generator /
//!   combinator source exposing `int`, `nat`, `float`, `string`, `bool`,
//!   `list`, `option`, `result`, `one_of` (generators) and `refine`, `map`
//!   (combinators).
//! - Part 3 (`dwarf-lib/runtime/draupnir/shrink.dwarf`): the shrinking source
//!   exposing `shrink`.
//! - Part 4 (`dwarf-lib/src/draupnir.rs`): the Rust module declared in `lib.rs`
//!   exposing `pub fn compile_draupnir() -> String` (mirrors
//!   `dwarf-lib/src/dunit.rs::compile_dunit`). The call-site contract lives in
//!   `draupnir_module_tests.rs` so its compile-blocking failure is isolated.
//!
//! Now GREEN: the artefacts exist — `runtime/draupnir/` ships the library
//! sources and the `draupnir` Rust module is declared in `lib.rs`. Every test
//! passes and pins the surface the acceptance criteria require: a for_all entry
//! point with arity 1-6, the full generator/combinator set, a shrinking entry
//! point, and a forge-loadable compile entry point.

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::Parser;
use dwarf_syntax::hir::Decl;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Absolute path to `dwarf-lib/runtime/draupnir/<name>`.
fn runtime_source_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("runtime")
        .join("draupnir")
        .join(name)
}

/// Read a Draupnir source file, panicking with a helpful message if absent.
fn read_source(name: &str) -> String {
    let path = runtime_source_path(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read draupnir runtime source {:?}: {}", path, e))
}

/// Tokenize and parse Dwarf source, returning (decls, parse_errors).
fn parse_source(source: &str) -> (Vec<Decl>, Vec<dwarf_parser::ParseError>) {
    let tokens = TokenizePass
        .tokenize(source)
        .expect("draupnir source should tokenize without lexer errors");
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

/// Assert every required function name is present among the parsed names.
fn assert_functions_present(source: &str, required: &[&str]) {
    let names = parsed_function_names(source);
    let missing: Vec<_> = required
        .iter()
        .filter(|name| !names.contains(&name.to_string()))
        .collect();
    assert!(
        missing.is_empty(),
        "expected functions not found: {:?}; declared functions: {:?}",
        missing,
        names
    );
}

// ===========================================================================
// Part 1: runtime/draupnir/draupnir.dwarf — for_all entry point
// ===========================================================================

/// AC: "Draupnir.for_all with Int generators passes a commutative property".
/// The runtime directory must exist and the entry-point source must parse
/// cleanly (mirror of the dunit Part 1 test).
#[test]
fn test_draupnir_entrypoint_library_exists_and_parses() {
    let path = runtime_source_path("draupnir.dwarf");

    assert!(
        path.exists(),
        "draupnir.dwarf should exist at {:?}, but file not found",
        path
    );

    let source = read_source("draupnir.dwarf");
    let (decls, errors) = parse_source(&source);
    assert!(
        errors.is_empty(),
        "draupnir.dwarf should parse without errors, got:\n  {}",
        errors
            .iter()
            .map(|e| format!("{} (code {})", e.message, e.code))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    assert!(
        !decls.is_empty(),
        "draupnir.dwarf should parse to at least one declaration"
    );
}

/// The entry-point library must declare the canonical `for_all` function.
#[test]
fn test_draupnir_for_all_function_exists() {
    let source = read_source("draupnir.dwarf");

    // Raw signature check (robust to whitespace/comment differences).
    assert!(
        source.contains("fn for_all"),
        "draupnir.dwarf should contain 'fn for_all' function, got:\n{}",
        source
    );

    // Parsed declaration check: for_all must be a top-level function.
    assert_functions_present(&source, &["for_all"]);
}

/// AC: "for_all arity 1-6". The entry-point library must declare one fixed-arity
/// function per generator count: `for_all_1` .. `for_all_6`. Each takes its
/// generators followed by the property callback.
#[test]
fn test_draupnir_for_all_arity_variants_1_to_6_declared() {
    let source = read_source("draupnir.dwarf");
    let required: Vec<String> = (1..=6).map(|n| format!("for_all_{}", n)).collect();
    let names = parsed_function_names(&source);
    let missing: Vec<_> = required
        .iter()
        .filter(|name| !names.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "draupnir.dwarf should declare for_all_1..for_all_6 (arity 1-6), \
         missing: {:?}; declared functions: {:?}",
        missing,
        names
    );
}

// ===========================================================================
// Part 2: runtime/draupnir/combinators.dwarf — generators + combinators
// ===========================================================================

/// The generators the acceptance criteria require: int, nat, float, string,
/// bool, list, option, result, one_of. AC: "list, option, result generators
/// produce valid values" — the producing functions must exist.
#[test]
fn test_draupnir_generators_declared() {
    let source = read_source("combinators.dwarf");
    assert_functions_present(
        &source,
        &[
            "int", "nat", "float", "string", "bool", "list", "option", "result", "one_of",
        ],
    );
}

/// AC: "refine filters correctly" plus the `map` combinator. Both must be
/// declared in the combinator source.
#[test]
fn test_draupnir_combinators_declared() {
    let source = read_source("combinators.dwarf");
    assert!(
        source.contains("fn refine"),
        "combinators.dwarf should contain 'fn refine', got:\n{}",
        source
    );
    assert!(
        source.contains("fn map"),
        "combinators.dwarf should contain 'fn map', got:\n{}",
        source
    );
    assert_functions_present(&source, &["refine", "map"]);
}

// ===========================================================================
// Part 3: runtime/draupnir/shrink.dwarf — shrinking engine
// ===========================================================================

/// AC: "Failing property shrinks to minimal counterexample". The shrinking
/// source must exist and expose a shrinking entry point. Edge-case ordering
/// (0/min/max first) is a behavioral guarantee of the generator source, not
/// of this file's surface — the deep-behavioral implementation must honor it.
#[test]
fn test_draupnir_shrink_source_exists_and_exposes_shrink() {
    let path = runtime_source_path("shrink.dwarf");
    assert!(
        path.exists(),
        "shrink.dwarf should exist at {:?}, but file not found",
        path
    );
    let source = read_source("shrink.dwarf");
    assert!(
        source.contains("fn shrink"),
        "shrink.dwarf should contain 'fn shrink', got:\n{}",
        source
    );
    assert_functions_present(&source, &["shrink"]);
}

// ===========================================================================
// Part 4: dwarf-lib/src/draupnir.rs — Rust compile entry point
// ===========================================================================

/// dwarf-lib must declare a `draupnir` module in lib.rs and the module file
/// must exist on disk (mirror of the dunit module wiring).
#[test]
fn test_draupnir_module_declared_in_lib() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let lib_rs_path = PathBuf::from(manifest_dir).join("src").join("lib.rs");

    assert!(
        lib_rs_path.exists(),
        "dwarf-lib/src/lib.rs should exist at {:?}",
        lib_rs_path
    );
    let lib_content = std::fs::read_to_string(&lib_rs_path).expect("should be able to read lib.rs");
    assert!(
        lib_content.contains("mod draupnir") || lib_content.contains("pub mod draupnir"),
        "lib.rs should declare a 'mod draupnir' module, got:\n{}",
        lib_content
    );

    // The module source file must exist so the crate compiles the module.
    let module_path = PathBuf::from(manifest_dir).join("src").join("draupnir.rs");
    assert!(
        module_path.exists(),
        "dwarf-lib/src/draupnir.rs should exist at {:?}, but file not found",
        module_path
    );
}

// ===========================================================================
// Part 5: dwarf-lib/src/draupnir.rs — call-site contract
// ===========================================================================
// Pinned in `draupnir_module_tests.rs` (compile-blocking failure). Kept separate so
// the runtime-source tests above run and fail individually on file absence.
