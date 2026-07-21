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
    let program = parser.parse().unwrap();
    assert!(program.is_empty(), "Empty input should produce no declarations");
}

#[test]
fn test_parse_single_literal_expr() {
    let tokens = tokenize("42");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    // A bare expression at top level might be wrapped in an implicit block
    // or treated as an expression statement
    assert!(!program.is_empty(), "Should produce at least one item");
}

#[test]
fn test_parse_fn_declaration() {
    let tokens = tokenize("fn add(a: i32, b: i32) -> i32 { a + b }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
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
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::Function { name, .. } if name == "main"));
}

#[test]
fn test_parse_import_decl() {
    let tokens = tokenize("import math from \"std\"");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::Import { module, .. } if module == "std"));
}

#[test]
fn test_parse_type_alias() {
    let tokens = tokenize("type Age = i32");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
    assert!(matches!(&program[0], Decl::TypeDef { name, .. } if name == "Age"));
}

#[test]
fn test_parse_record_def() {
    let tokens = tokenize("type Person = { name: string, age: i32 }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_union_def() {
    let tokens = tokenize("type Option = Some(value) | None");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_if_expr() {
    let tokens = tokenize("fn test() { if x > 0 { 1 } else { 0 } }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_match_expr() {
    let tokens = tokenize("fn test() { match x { 1 => \"one\", _ => \"other\" } }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_pipe_expr() {
    let tokens = tokenize("fn test() { x |> transform }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_lambda() {
    let tokens = tokenize("fn test() { |x: i32| x + 1 }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_for_loop() {
    let tokens = tokenize("fn test() { for x in items { process(x) } }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_pub_decl() {
    let tokens = tokenize("pub fn add(a: i32) -> i32 { a }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    // The `pub` keyword could be a modifier on the declaration
    // or could create a wrapper. Either way, a declaration is produced.
    assert_eq!(program.len(), 1);
}

#[test]
fn test_parse_multiple_decls() {
    let tokens = tokenize("fn a() { 1 } fn b() { 2 }");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    assert_eq!(program.len(), 2, "Should parse two functions");
}
