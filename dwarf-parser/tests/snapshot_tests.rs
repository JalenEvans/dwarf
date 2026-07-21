//! Snapshot tests for the dwarf parser.
//!
//! Each test parses a small source snippet via `ParsePass` and snapshots the
//! resulting HIR declarations as JSON (HIR types derive `serde::Serialize`).
//! Follows the pattern established in `dwarf-lexer/tests/lexer_tests.rs`.

use dwarf_parser::pass::ParsePass;
use dwarf_syntax::hir::Decl;
use insta::assert_json_snapshot;

/// Parse `input` and return the declarations, asserting that tokenizing and
/// parsing both succeeded without errors.
fn parse_ok(input: &str) -> Vec<Decl> {
    let pass = ParsePass;
    let (decls, errors) = pass.parse(input.to_string()).unwrap();
    assert!(
        errors.is_empty(),
        "expected no parse errors for {input:?}, got: {errors:?}"
    );
    decls
}

#[test]
fn snapshot_simple_function() {
    let decls = parse_ok("fn main() { 42 }");
    assert_json_snapshot!(decls);
}

#[test]
fn snapshot_function_with_params() {
    let decls = parse_ok("fn add(a: i32, b: i32) -> i32 { a + b }");
    assert_json_snapshot!(decls);
}

#[test]
fn snapshot_record_definition() {
    let decls = parse_ok("type Point = { x: i32, y: i32 }");
    assert_json_snapshot!(decls);
}

#[test]
fn snapshot_union_definition() {
    // Note: union variant payloads use parens (`Circle(f64)`), not braces —
    // the parser only accepts the paren form (see `Parser::parse_union_def`).
    let decls = parse_ok("type Shape = Circle(f64) | Square(f64)");
    assert_json_snapshot!(decls);
}

#[test]
fn snapshot_match_expression() {
    // Note: match is a prefix expression (`match x { ... }`), not postfix.
    let decls = parse_ok(r#"fn test(x: i32) { match x { 0 => "zero", _ => "other" } }"#);
    assert_json_snapshot!(decls);
}

#[test]
fn snapshot_pipe_expression() {
    let decls = parse_ok("fn pipe(x: i32) { x |> add(1) |> mul(2) }");
    assert_json_snapshot!(decls);
}
