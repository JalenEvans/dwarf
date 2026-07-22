//! Integration tests for the PassManager infrastructure.
//!
//! These tests validate the pass pipeline orchestration layer:
//! - Pass registration and lifecycle
//! - Running all passes in order
//! - Skipping specific passes
//! - Error collection and partial results
//! - CompileOptions configuration
//!
//! NOTE: These tests are written as if the `dwarf_cli::pass_manager` module
//! already exists. They will fail to compile until that module is implemented
//! in `dwarf-cli/src/lib.rs`.

use dwarf_cli::pass_manager::*;
use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::pass::ParsePass;

#[test]
fn test_pass_manager_register() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    // Should not panic or error — type system ensures this works
}

#[test]
fn test_pass_manager_list_passes() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));

    let passes = pm.list_passes();
    assert_eq!(passes.len(), 2);
    assert!(passes.iter().any(|(n, _)| *n == "tokenize"));
    assert!(passes.iter().any(|(n, _)| *n == "parse"));
}

#[test]
fn test_pass_manager_run_all() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));

    let mut unit = CompilationUnit::new("fn main() { 42 }".to_string());
    let mut ctx = PassContext::new(CompileOptions::default());

    pm.run_all(&mut unit, &mut ctx);

    assert!(unit.tokens.is_some(), "Tokens should be populated");
    assert!(unit.decls.is_some(), "Decls should be populated");
}

#[test]
fn test_pass_manager_skip_passes() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));

    let mut unit = CompilationUnit::new("fn main() { 42 }".to_string());
    let options = CompileOptions {
        skip_passes: vec!["parse".to_string()],
        ..Default::default()
    };
    let mut ctx = PassContext::new(options);

    pm.run_all(&mut unit, &mut ctx);

    assert!(unit.tokens.is_some(), "Tokens should be populated");
    assert!(unit.decls.is_none(), "Decls should be None (parse skipped)");
}

#[test]
fn test_pass_manager_collects_errors() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));

    let mut unit = CompilationUnit::new("fn broken( { 1 } fn ok() { 2 }".to_string());
    let mut ctx = PassContext::new(CompileOptions::default());

    pm.run_all(&mut unit, &mut ctx);

    // Should have parse errors but still produce partial HIR
    assert!(
        !ctx.diagnostics().is_empty(),
        "Should have diagnostic errors for broken input"
    );
    assert!(unit.tokens.is_some(), "Tokens should be present");
    assert!(unit.decls.is_some(), "Should produce partial HIR");
}

#[test]
fn test_compile_options_default() {
    let options = CompileOptions::default();
    assert!(options.passes.is_none());
    assert!(options.skip_passes.is_empty());
}

#[test]
fn test_compile_options_selected_passes() {
    let options = CompileOptions {
        passes: Some(vec!["tokenize".to_string()]),
        skip_passes: vec![],
    };
    assert_eq!(options.passes.unwrap(), vec!["tokenize"]);
}
