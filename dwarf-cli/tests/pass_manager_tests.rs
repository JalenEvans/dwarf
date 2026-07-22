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
use dwarf_typecheck::pass::TypeCheckPass;

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

#[test]
fn test_typecheck_pass_registration() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    pm.register(Box::new(TypeCheckPass::new()));

    let passes = pm.list_passes();
    assert_eq!(passes.len(), 3);
    assert!(passes.iter().any(|(n, _)| *n == "typecheck"));
}

#[test]
fn test_typecheck_pass_runs_after_parse() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    pm.register(Box::new(TypeCheckPass::new()));

    // A valid simple function
    let mut unit = CompilationUnit::new("fn answer() { 42 }".to_string());
    let mut ctx = PassContext::new(CompileOptions::default());

    pm.run_all(&mut unit, &mut ctx);

    assert!(unit.tokens.is_some(), "Tokens should be populated");
    assert!(unit.decls.is_some(), "Decls should be populated");
    // Type checking should complete without errors for valid code
    // (Type errors would show up in diagnostics, but this is valid)
}

#[test]
fn test_typecheck_pass_type_error() {
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    pm.register(Box::new(TypeCheckPass::new()));

    // Code with a type error: adding int and string
    let mut unit = CompilationUnit::new("fn broken() { 1 + \"hello\" }".to_string());
    let mut ctx = PassContext::new(CompileOptions::default());

    pm.run_all(&mut unit, &mut ctx);

    // Should have at least one diagnostic for the type error
    assert!(
        !ctx.diagnostics().is_empty(),
        "Should have diagnostic for type error"
    );

    // The diagnostic should be a TYPE error
    let has_type_error = ctx
        .diagnostics()
        .iter()
        .any(|d| d.code.starts_with("DWARF-E-TYPE-"));
    assert!(
        has_type_error,
        "Should have at least one DWARF-E-TYPE- diagnostic"
    );
}
