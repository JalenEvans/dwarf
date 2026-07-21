//! Integration tests for the dwarf parser.
//! These tests tokenize source strings and parse them, verifying
//! the structure of the resulting HIR.
//!
//! NOTE: These are expected to FAIL until the Parser is implemented.

use dwarf_lexer::Lexer;
use dwarf_parser::Parser;
use dwarf_syntax::hir::*;
use dwarf_syntax::token::TokenKind;

/// Helper: tokenize an input string into a Vec<Token> ending with Eof.
fn tokenize(input: &str) -> Vec<dwarf_syntax::token::Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }
    tokens
}

#[test]
fn test_parse_empty_input() {
    let tokens = tokenize("");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert!(program.is_empty(), "Empty input should produce no declarations");
}

#[test]
fn test_parse_single_literal_expr() {
    let tokens = tokenize("42");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    // A bare expression at top level might be wrapped in an implicit block
    // or treated as an expression statement
    assert!(!program.is_empty(), "Should produce at least one item");
}

#[test]
fn test_parse_fn_declaration() {
    let tokens = tokenize("fn add(a: i32, b: i32) -> i32 { a + b }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1, "Should produce one declaration");

    match &program[0] {
        Decl::Function {
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert!(return_type.is_some());
        }
        _ => panic!("Expected Function declaration"),
    }
}

#[test]
fn test_parse_fn_no_params() {
    let tokens = tokenize("fn main() { 42 }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::Function { name, .. } if name == "main"));
}

#[test]
fn test_parse_import_decl() {
    let tokens = tokenize("import math from \"std\"");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::Import { module, .. } if module == "std"));
}

#[test]
fn test_parse_type_alias() {
    let tokens = tokenize("type Age = i32");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::TypeDef { name, .. } if name == "Age"));
}

#[test]
fn test_parse_record_def() {
    let tokens = tokenize("type Person = { name: string, age: i32 }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_union_def() {
    let tokens = tokenize("type Option = Some(value) | None");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_if_expr() {
    let tokens = tokenize("fn test() { if x > 0 { 1 } else { 0 } }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_match_expr() {
    let tokens = tokenize("fn test() { match x { 1 => \"one\", _ => \"other\" } }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_pipe_expr() {
    let tokens = tokenize("fn test() { x |> transform }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_lambda() {
    let tokens = tokenize("fn test() { |x: i32| x + 1 }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_for_loop() {
    let tokens = tokenize("fn test() { for x in items { process(x) } }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_pub_decl() {
    let tokens = tokenize("pub fn add(a: i32) -> i32 { a }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    // The `pub` keyword could be a modifier on the declaration
    // or could create a wrapper. Either way, a declaration is produced.
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_multiple_decls() {
    let tokens = tokenize("fn a() { 1 } fn b() { 2 }");
    let mut parser = Parser::new(tokens);
    let (program, _errors) = parser.parse();
    assert_eq!(program.len(), 2, "Should parse two functions");
}

// ============================================================================
// Panic-mode error recovery tests
//
// These tests will FAIL until the parser gains error-recovery logic and the
// public API changes from `Result<Vec<Decl>, ParseError>` to
// `(Vec<Decl>, Vec<ParseError>)`.
// ============================================================================

#[test]
fn test_error_recovery_missing_rparen() {
    // fn missing closing paren, but still parses the next function
    let tokens = tokenize("fn broken(a: i32 { 1 } fn ok() { 2 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    // Should recover past the broken function and parse the next one
    assert!(!decls.is_empty(), "Should parse at least one declaration despite error");
    // The second function 'ok' should still be parsed
    assert_eq!(decls.len(), 1, "Only the valid function parses; broken one is skipped");
    if let Decl::Function { name, .. } = &decls[0] {
        assert_eq!(name, "ok", "The recovered function should be 'ok'");
    } else {
        panic!("Expected recovered decl to be a Function");
    }
    // Should have recorded at least one error
    assert!(!errors.is_empty(), "Should have recorded parse errors");
}

#[test]
fn test_error_recovery_invalid_fn_body() {
    // `@` is not a valid expression start, so the first body fails.
    // The parser recovers and still parses the second function.
    let tokens = tokenize("fn bad() { @ } fn fine() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    // The second function should parse successfully
    assert!(!decls.is_empty(), "Should still produce declarations");
    assert_eq!(decls.len(), 1, "Only the valid function parses");
    if let Decl::Function { name, .. } = &decls[0] {
        assert_eq!(name, "fine", "The recovered function should be 'fine'");
    } else {
        panic!("Expected recovered decl to be a Function");
    }
    assert!(!errors.is_empty(), "Should record errors");
}

#[test]
fn test_error_recovery_multiple_errors() {
    // Multiple broken functions should all produce errors
    let tokens = tokenize("fn a( { 1 } fn b( { 2 } fn c() { 3 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    // At minimum, c() should parse successfully
    assert!(!decls.is_empty(), "Should produce at least one valid decl");
    assert!(!errors.is_empty(), "Should record multiple errors");
}

#[test]
fn test_error_recovery_all_broken() {
    let tokens = tokenize("fn ( { } fn ( { }");
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();

    // Should not crash, should report errors
    assert!(!errors.is_empty(), "Should report errors for broken input");
}

#[test]
fn test_error_recovery_after_garbage() {
    // `@` alone triggers decorator parsing but fails (no name follows).
    // The parser recovers and still parses the function.
    let tokens = tokenize("@ fn ok() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    // Should recover past garbage and parse the function
    assert!(!decls.is_empty(), "Should parse function after garbage");
    assert_eq!(decls.len(), 1, "Only the valid function parses");
    if let Decl::Function { name, .. } = &decls[0] {
        assert_eq!(name, "ok", "The recovered function should be 'ok'");
    } else {
        panic!("Expected recovered decl to be a Function");
    }
    assert!(!errors.is_empty(), "Should record error for garbage");
}

#[test]
fn test_error_recovery_preserves_valid_decls() {
    let tokens = tokenize("fn first() { 1 } fn broken( { 2 } fn third() { 3 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    // first() and third() are valid; broken( errors and is skipped
    assert_eq!(decls.len(), 2, "Should parse the two valid functions");
    assert!(!errors.is_empty(), "Should record the broken function error");

    // Verify first and third are valid
    if let Decl::Function { name, .. } = &decls[0] {
        assert_eq!(name, "first");
    } else {
        panic!("Expected first decl to be Function");
    }
    if let Decl::Function { name, .. } = &decls[1] {
        assert_eq!(name, "third");
    } else {
        panic!("Expected third decl to be Function");
    }
}
