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

// ============================================================================
// ParsePass tests
//
// These tests will FAIL with a compile error because
// `dwarf_parser::pass::ParsePass` does not exist yet.
// ============================================================================

#[test]
fn test_parse_pass_simple() {
    use dwarf_parser::pass::ParsePass;

    let pass = ParsePass;
    let result = pass.parse("fn main() { 42 }".to_string());
    assert!(result.is_ok(), "ParsePass should succeed on valid input");
    let (decls, errors) = result.unwrap();
    assert_eq!(decls.len(), 1);
    assert!(errors.is_empty());
}

#[test]
fn test_parse_pass_with_errors() {
    use dwarf_parser::pass::ParsePass;

    let pass = ParsePass;
    let result = pass.parse("fn broken( { 1 } fn ok() { 2 }".to_string());
    assert!(result.is_ok(), "ParsePass should handle errors gracefully");
    let (decls, errors) = result.unwrap();
    assert!(!decls.is_empty(), "Should produce partial HIR");
    assert!(!errors.is_empty(), "Should report errors");
}

#[test]
fn test_parse_pass_empty() {
    use dwarf_parser::pass::ParsePass;

    let pass = ParsePass;
    let result = pass.parse("".to_string());
    assert!(result.is_ok());
    let (decls, errors) = result.unwrap();
    assert!(decls.is_empty());
    assert!(errors.is_empty());
}

// ============================================================================
// Doc comment handling tests
//
// The lexer emits `TokenKind::DocComment` for `///` comments. The parser
// should skip them anywhere they appear (before declarations, inside
// function bodies). These tests will FAIL until the parser learns to skip
// DocComment tokens instead of erroring on them.
// ============================================================================

#[test]
fn test_doc_comment_before_function() {
    let tokens = tokenize("/// This is a doc comment\nfn main() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert_eq!(decls.len(), 1, "Doc comment should be skipped");
    assert!(errors.is_empty(), "No errors expected");

    if let Decl::Function { name, .. } = &decls[0] {
        assert_eq!(name, "main");
    } else {
        panic!("Expected Function decl");
    }
}

#[test]
fn test_doc_comment_inside_function_body() {
    let tokens = tokenize("fn main() { /// inner doc\n 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert_eq!(decls.len(), 1, "Doc comment in body should be skipped");
    assert!(errors.is_empty(), "No errors expected");
}

#[test]
fn test_multiple_doc_comments() {
    let tokens = tokenize("/// First\n/// Second\nfn main() { /// Third\n 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert_eq!(decls.len(), 1);
    assert!(errors.is_empty());
}

// ============================================================================
// Recursion depth limiting tests
//
// The parser is recursive descent with no depth guard. Deeply nested input
// (e.g. thousands of '(') overflows the stack and aborts the process.
// The parser should enforce a recursion depth limit and report a graceful
// ParseError instead of crashing.
//
// The deep-nesting tests will FAIL until the depth limit is implemented:
// either the process aborts with a stack overflow, or the input parses
// successfully and the `!errors.is_empty()` assertion fails.
// ============================================================================

#[test]
fn test_deeply_nested_expression_graceful_error() {
    // 150 nested parens should trigger depth guard (MAX_DEPTH=64)
    let mut input = String::new();
    for _ in 0..150 {
        input.push('(');
    }
    input.push('1');
    for _ in 0..150 {
        input.push(')');
    }

    let tokens = tokenize(&input);
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();

    // Should have a recursion depth error, not crash
    assert!(!errors.is_empty(), "Should report recursion depth error");
    assert!(errors.iter().any(|e| e.message.contains("recursion") || e.message.contains("depth") || e.message.contains("too deep")),
        "Error should mention recursion/depth");
}

#[test]
fn test_moderate_nesting_works() {
    // 50 nested parens should still work
    let mut input = String::new();
    for _ in 0..50 {
        input.push('(');
    }
    input.push('1');
    for _ in 0..50 {
        input.push(')');
    }

    let tokens = tokenize(&input);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "Moderate nesting should parse fine");
    assert_eq!(decls.len(), 1);
}

#[test]
fn test_deeply_nested_type_graceful_error() {
    // Deeply nested type: ((((((...i32...))))))
    let mut input = String::from("type X = ");
    for _ in 0..150 {
        input.push('(');
    }
    input.push_str("i32");
    for _ in 0..150 {
        input.push(')');
    }

    let tokens = tokenize(&input);
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();

    assert!(!errors.is_empty(), "Should report recursion depth error for types");
}

// ============================================================================
// Union definition parsing tests
//
// These tests verify that union definitions produce Decl::UnionDef with the
// correct variants. Several will FAIL under the current parser because:
//   1. `is_at_union_start()` only checks for `(` after the first variant,
//      missing unit-only and braced-variant unions.
//   2. Variants without a parenthesized payload cause the parser to fall
//      through to parse_type(), producing Decl::TypeDef(Union(...)) instead
//      of Decl::UnionDef.
//   3. Braced variant payloads (`Circle { radius: f64 }`) are not supported.
//   4. `None | Some(i32)` silently loses the `(i32)` payload and creates a
//      phantom bare-expression declaration for the leftover tokens.
// ============================================================================

#[test]
fn test_union_def_mixed_payloads() {
    // Unit-only + paren-payload variants
    let tokens = tokenize("type Option = None | Some(i32)");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors for valid union: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, variants, .. } => {
            assert_eq!(name, "Option");
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "None");
            assert!(variants[0].arg.is_none());
            assert_eq!(variants[1].name, "Some");
            assert!(variants[1].arg.is_some(), "Some should have payload");
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

#[test]
fn test_union_def_unit_only() {
    // All unit variants
    let tokens = tokenize("type Color = Red | Green | Blue");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors for unit union: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, variants, .. } => {
            assert_eq!(name, "Color");
            assert_eq!(variants.len(), 3);
            assert_eq!(variants[0].name, "Red");
            assert_eq!(variants[1].name, "Green");
            assert_eq!(variants[2].name, "Blue");
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

#[test]
fn test_union_def_single_variant() {
    let tokens = tokenize("type Wrapper = Wrapped(i32)");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, variants, .. } => {
            assert_eq!(name, "Wrapper");
            assert_eq!(variants.len(), 1);
            assert_eq!(variants[0].name, "Wrapped");
            assert!(variants[0].arg.is_some());
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

#[test]
fn test_union_def_braced_variant() {
    // Braced variant payload: Circle { radius: f64 }
    let tokens = tokenize("type Shape = Circle { radius: f64 } | Nothing");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors for braced union: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, variants, .. } => {
            assert_eq!(name, "Shape");
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "Circle");
            assert!(variants[0].arg.is_some(), "Circle should have record payload");
            assert_eq!(variants[1].name, "Nothing");
            assert!(variants[1].arg.is_none());
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

#[test]
fn test_union_def_all_braced_variants() {
    let tokens = tokenize("type Shape = Circle { radius: f64 } | Rect { width: f64, height: f64 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { variants, .. } => {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "Circle");
            assert!(variants[0].arg.is_some());
            assert_eq!(variants[1].name, "Rect");
            assert!(variants[1].arg.is_some());
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

#[test]
fn test_union_def_type_alias_not_confused() {
    // type alias with union type should still be TypeDef, not UnionDef
    let tokens = tokenize("type IntOrString = i32 | string");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::TypeDef { name, type_, .. } => {
            assert_eq!(name, "IntOrString");
            assert!(matches!(type_, Type::Union(_)));
        }
        other => panic!("Expected TypeDef, got {:?}", other),
    }
}

// ============================================================================
// Span correctness tests
//
// These tests will FAIL because:
//   - Expr::Literal and Expr::Variable are tuple variants (no span field)
//   - Binary op spans in parse_term use self.previous().span (just the last token)
//   - Comparison spans in parse_comparison hardcode 0 for the start
// ============================================================================

/// After span fix: Literal should carry a span
#[test]
fn test_literal_has_span() {
    let tokens = tokenize("fn f() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, _) = parser.parse();

    assert_eq!(decls.len(), 1);
    if let Decl::Function { body, .. } = &decls[0] {
        // Function body is a Block; extract the inner expression
        if let Expr::Block { stmts, .. } = body {
            if let Stmt::Expr(expr) = &stmts[0] {
                if let Expr::Literal { value, span } = expr {
                    assert_eq!(*value, LiteralValue::Int(42));
                    assert!(span.start > 0, "Literal span should not be zero");
                    assert!(span.end > span.start, "Literal span should have positive length");
                } else {
                    panic!("Expected Expr::Literal");
                }
            } else {
                panic!("Expected Stmt::Expr");
            }
        } else {
            panic!("Expected Expr::Block as function body, got something else");
        }
    } else {
        panic!("Expected Function decl");
    }
}

/// After span fix: Variable should carry a span
#[test]
fn test_variable_has_span() {
    let tokens = tokenize("fn f() { x }");
    let mut parser = Parser::new(tokens);
    let (decls, _) = parser.parse();

    assert_eq!(decls.len(), 1);
    if let Decl::Function { body, .. } = &decls[0] {
        // Function body is a Block; extract the inner expression
        if let Expr::Block { stmts, .. } = body {
            if let Stmt::Expr(expr) = &stmts[0] {
                if let Expr::Variable { name, span } = expr {
                    assert_eq!(name, "x");
                    assert!(span.start > 0, "Variable span should not be zero");
                } else {
                    panic!("Expected Expr::Variable");
                }
            } else {
                panic!("Expected Stmt::Expr");
            }
        } else {
            panic!("Expected Expr::Block as function body, got something else");
        }
    } else {
        panic!("Expected Function decl");
    }
}

/// After span fix: binary op span should cover entire expression
#[test]
fn test_binary_op_span_covers_full_expression() {
    let tokens = tokenize("fn f() { 1 + 2 }");
    let mut parser = Parser::new(tokens);
    let (decls, _) = parser.parse();

    assert_eq!(decls.len(), 1);
    if let Decl::Function { body, .. } = &decls[0] {
        // Function body is a Block; extract the inner expression
        if let Expr::Block { stmts, .. } = body {
            if let Stmt::Expr(expr) = &stmts[0] {
                if let Expr::Binary { span, .. } = expr {
                    // Span should cover from start of LHS (1 at byte ~9) to end of RHS (2 at byte ~14)
                    assert!(
                        span.start < 10,
                        "Binary span start should be near beginning of '1', got start={}",
                        span.start
                    );
                    assert!(
                        span.end >= 14,
                        "Binary span end should cover end of '2', got end={}",
                        span.end
                    );
                } else {
                    panic!("Expected Expr::Binary");
                }
            } else {
                panic!("Expected Stmt::Expr");
            }
        } else {
            panic!("Expected Expr::Block as function body, got something else");
        }
    }
}

/// After span fix: comparison span covers full expression
#[test]
fn test_comparison_span_covers_full_expression() {
    let tokens = tokenize("fn f() { a < b }");
    let mut parser = Parser::new(tokens);
    let (decls, _) = parser.parse();

    assert_eq!(decls.len(), 1);
    if let Decl::Function { body, .. } = &decls[0] {
        // Function body is a Block; extract the inner expression
        if let Expr::Block { stmts, .. } = body {
            if let Stmt::Expr(expr) = &stmts[0] {
                if let Expr::Binary { span, .. } = expr {
                    assert!(
                        span.start > 0,
                        "Comparison span start should not be zero, got start={}",
                        span.start
                    );
                    assert!(
                        span.end > span.start,
                        "Comparison span should have positive length"
                    );
                } else {
                    panic!("Expected Expr::Binary");
                }
            } else {
                panic!("Expected Stmt::Expr");
            }
        } else {
            panic!("Expected Expr::Block as function body, got something else");
        }
    }
}
