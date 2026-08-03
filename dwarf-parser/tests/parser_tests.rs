//! Integration tests for the dwarf parser.
//! These tests tokenize source strings and parse them, verifying
//! the structure of the resulting HIR.
//!
//! NOTE: These are expected to FAIL until the Parser is implemented.

use dwarf_lexer::Lexer;
use dwarf_parser::{ParseError, Parser};
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
    assert!(
        program.is_empty(),
        "Empty input should produce no declarations"
    );
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
// Decorator parsing edge-case tests
//
// The parser supports decorators (`@name(args?) decl`), producing
// Decl::Decorator { name, args, target, is_pub, span }.
// These tests verify patterns needed for xUnit testing support.
// ============================================================================

#[test]
fn test_decorator_suite_on_function() {
    let tokens = tokenize("@Suite fn math_tests() { 42 }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "Suite");
            assert!(args.is_empty(), "Suite should have no args");
            match target.as_ref() {
                Decl::Function {
                    name: fn_name,
                    params,
                    ..
                } => {
                    assert_eq!(fn_name, "math_tests");
                    assert!(params.is_empty(), "Expected no params");
                }
                other => panic!("Expected Function target, got {:?}", other),
            }
        }
        other => panic!("Expected Decorator, got {:?}", other),
    }
}

#[test]
fn test_decorator_suite_with_params() {
    let tokens = tokenize("@Suite fn math_tests(a: i32, b: i32) { a + b }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "Suite");
            assert!(args.is_empty());
            match target.as_ref() {
                Decl::Function {
                    name: fn_name,
                    params,
                    ..
                } => {
                    assert_eq!(fn_name, "math_tests");
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0].name, "a");
                    assert_eq!(params[1].name, "b");
                }
                other => panic!("Expected Function target, got {:?}", other),
            }
        }
        other => panic!("Expected Decorator, got {:?}", other),
    }
}

#[test]
fn test_decorator_with_argument() {
    let tokens = tokenize("@benchmark(1000) fn test_perf() { 42 }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "benchmark");
            assert_eq!(args.len(), 1, "Should have one argument");
            match &args[0] {
                Expr::Literal { value, .. } => {
                    assert_eq!(*value, LiteralValue::Int(1000));
                }
                other => panic!("Expected Literal arg, got {:?}", other),
            }
            match target.as_ref() {
                Decl::Function { name: fn_name, .. } => {
                    assert_eq!(fn_name, "test_perf");
                }
                other => panic!("Expected Function target, got {:?}", other),
            }
        }
        other => panic!("Expected Decorator, got {:?}", other),
    }
}

#[test]
fn test_multiple_decorators_on_function() {
    let tokens = tokenize("@Suite @Test fn run_all() { 42 }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    // Outer decorator: @Suite wraps @Test fn run_all()
    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "Suite");
            assert!(args.is_empty());

            // Inner target should be another Decorator: @Test fn run_all()
            match target.as_ref() {
                Decl::Decorator {
                    name: inner_name,
                    args: inner_args,
                    target: inner_target,
                    ..
                } => {
                    assert_eq!(inner_name, "Test");
                    assert!(inner_args.is_empty());
                    match inner_target.as_ref() {
                        Decl::Function { name: fn_name, .. } => {
                            assert_eq!(fn_name, "run_all");
                        }
                        other => panic!("Expected Function, got {:?}", other),
                    }
                }
                other => panic!("Expected inner Decorator, got {:?}", other),
            }
        }
        other => panic!("Expected outer Decorator, got {:?}", other),
    }
}

#[test]
fn test_decorator_on_record_def() {
    let tokens = tokenize("@Serializable type TestResult = { passed: bool, duration: float }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "Serializable");
            assert!(args.is_empty());
            match target.as_ref() {
                Decl::TypeDef {
                    name: record_name,
                    type_,
                    ..
                } => {
                    assert_eq!(record_name, "TestResult");
                    match type_ {
                        dwarf_syntax::hir::Type::Record(fields) => {
                            assert_eq!(fields.len(), 2);
                            assert_eq!(fields[0].0, "passed");
                            assert_eq!(fields[1].0, "duration");
                        }
                        other => panic!("Expected Record type, got {:?}", other),
                    }
                }
                other => panic!("Expected TypeDef target, got {:?}", other),
            }
        }
        other => panic!("Expected Decorator, got {:?}", other),
    }
}

#[test]
fn test_decorator_with_pub_function() {
    let tokens = tokenize("@Test pub fn visible_test() { 42 }");
    let mut parser = Parser::new(tokens);
    let (program, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(program.len(), 1);

    match &program[0] {
        Decl::Decorator {
            name, args, target, ..
        } => {
            assert_eq!(name, "Test");
            assert!(args.is_empty());
            match target.as_ref() {
                Decl::Function {
                    name: fn_name,
                    is_pub,
                    ..
                } => {
                    assert_eq!(fn_name, "visible_test");
                    assert!(is_pub, "Function should be pub");
                }
                other => panic!("Expected Function target, got {:?}", other),
            }
        }
        other => panic!("Expected Decorator, got {:?}", other),
    }
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
    assert!(
        !decls.is_empty(),
        "Should parse at least one declaration despite error"
    );
    // The second function 'ok' should still be parsed
    assert_eq!(
        decls.len(),
        1,
        "Only the valid function parses; broken one is skipped"
    );
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
    assert!(
        !errors.is_empty(),
        "Should record the broken function error"
    );

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
    let result = pass.parse("fn main() { 42 }");
    assert!(result.is_ok(), "ParsePass should succeed on valid input");
    let (decls, errors) = result.unwrap();
    assert_eq!(decls.len(), 1);
    assert!(errors.is_empty());
}

#[test]
fn test_parse_pass_with_errors() {
    use dwarf_parser::pass::ParsePass;

    let pass = ParsePass;
    let result = pass.parse("fn broken( { 1 } fn ok() { 2 }");
    assert!(result.is_ok(), "ParsePass should handle errors gracefully");
    let (decls, errors) = result.unwrap();
    assert!(!decls.is_empty(), "Should produce partial HIR");
    assert!(!errors.is_empty(), "Should report errors");
}

#[test]
fn test_parse_pass_empty() {
    use dwarf_parser::pass::ParsePass;

    let pass = ParsePass;
    let result = pass.parse("");
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
    assert!(
        errors.iter().any(|e| e.message.contains("recursion")
            || e.message.contains("depth")
            || e.message.contains("too deep")),
        "Error should mention recursion/depth"
    );
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

    assert!(
        !errors.is_empty(),
        "Should report recursion depth error for types"
    );
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

    assert!(
        errors.is_empty(),
        "No errors for braced union: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, variants, .. } => {
            assert_eq!(name, "Shape");
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "Circle");
            assert!(
                variants[0].arg.is_some(),
                "Circle should have record payload"
            );
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
                    assert!(
                        span.end > span.start,
                        "Literal span should have positive length"
                    );
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

// ============================================================================
// Visibility / is_pub tests
//
// These tests will FAIL with a compile error because the HIR Decl variants do
// not yet have an `is_pub` field.  After `is_pub` is added to the relevant
// Decl variants and threaded through the parser, these should pass.
// ============================================================================

#[test]
fn test_pub_function_parsed() {
    let tokens = tokenize("pub fn main() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors for pub fn: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Function { name, is_pub, .. } => {
            assert_eq!(name, "main");
            assert!(is_pub, "Public function should have is_pub = true");
        }
        other => panic!("Expected Function, got {:?}", other),
    }
}

#[test]
fn test_private_function_parsed() {
    let tokens = tokenize("fn main() { 42 }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Function { name, is_pub, .. } => {
            assert_eq!(name, "main");
            assert!(!is_pub, "Private function should have is_pub = false");
        }
        other => panic!("Expected Function, got {:?}", other),
    }
}

#[test]
fn test_pub_type_decl_parsed() {
    let tokens = tokenize("pub type Option = Some(i32) | None");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::UnionDef { name, is_pub, .. } => {
            assert_eq!(name, "Option");
            assert!(is_pub);
        }
        other => panic!("Expected UnionDef, got {:?}", other),
    }
}

// ============================================================================
// Variant pattern tests
//
// `Pat::Variant` and `Pat::Record` exist in `hir.rs` but `parse_pattern` only
// handles wildcard, literal, and variable patterns.  These tests will FAIL
// with parse errors until `parse_pattern` is extended to handle variant
// destructuring.
// ============================================================================

#[test]
fn test_match_variant_pattern_some() {
    let tokens = tokenize("fn f(x: Option) { match x { Some(val) => val, None => 0 } }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "Variant patterns should parse: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1);

    if let Decl::Function { body, .. } = &decls[0] {
        if let Expr::Block { stmts, .. } = body {
            if let Some(Stmt::Expr(Expr::Match { arms, .. })) = stmts.first() {
                assert_eq!(arms.len(), 2);
                // First arm: Some(val) => val
                assert!(
                    matches!(&arms[0].pattern, Pat::Variant { name, .. } if name == "Some"),
                    "Expected Pat::Variant for Some"
                );
                // Second arm: None => 0
                assert!(
                    matches!(&arms[1].pattern, Pat::Variant { name, .. } if name == "None"),
                    "Expected Pat::Variant for None"
                );
            } else {
                panic!("Expected Match expr");
            }
        } else {
            panic!("Expected Block");
        }
    }
}

#[test]
fn test_match_literal_pattern() {
    let tokens = tokenize("fn f() { match x { 0 => \"zero\", _ => \"other\" } }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);
}

#[test]
fn test_match_nested_variant_pattern() {
    let tokens = tokenize("fn f(x: Option) { match x { Some(Some(val)) => val, _ => 0 } }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "Nested variant patterns should parse: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1);
}

#[test]
fn test_match_variant_pattern_single_arm() {
    let tokens = tokenize("fn f() { match x { Some(val) => val } }");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty());
    assert_eq!(decls.len(), 1);
}

// ============================================================================
// ParseError code field tests
//
// These tests will FAIL with a compile error because `ParseError` does not
// yet have a `code` field. The `code` field should be of type `&'static str`
// and hold an error code like "DWARF-E-PARSE-0001".
// ============================================================================

#[test]
fn test_parse_error_has_code() {
    use dwarf_syntax::span::Span;

    // ParseError should have a code field
    let err = ParseError {
        message: "test error".to_string(),
        span: Span::default(),
        code: "DWARF-E-PARSE-0001",
    };
    assert_eq!(err.code, "DWARF-E-PARSE-0001");
}

// ============================================================================
// Extern declaration parsing tests (RED Phase — expected to fail)
//
// These tests will FAIL to compile because TokenKind::Extern and
// Decl::Extern do not exist yet. They define the expected behavior for
// Phase 1 of extern/FFI support: parsing extern declarations with source,
// name, params, and return type.
//
// Expected Decl::Extern shape:
//   Decl::Extern {
//       source: String,      // e.g. "npm:express"
//       name: String,        // e.g. "express"
//       params: Vec<Param>,
//       return_type: Option<Type>,
//       is_pub: bool,
//       span: Span,
//   }
// ============================================================================

#[test]
fn test_parse_extern_npm() {
    let tokens = tokenize(r#"extern "npm:express" fn express() -> Any"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern {
            source,
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(source, "npm:express");
            assert_eq!(name, "express");
            assert!(params.is_empty(), "Expected no params");
            assert!(
                matches!(return_type, Some(Type::Named(ref n)) if n == "Any"),
                "Expected return type Named(\"Any\"), got {:?}",
                return_type
            );
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_extern_python() {
    let tokens = tokenize(r#"extern "py:json" fn dumps(obj: Any) -> String"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern {
            source,
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(source, "py:json");
            assert_eq!(name, "dumps");
            assert_eq!(params.len(), 1, "Expected one param");
            assert_eq!(params[0].name, "obj");
            assert!(
                matches!(&params[0].type_, Some(Type::Named(n)) if n == "Any"),
                "Expected param type Named(\"Any\")"
            );
            assert!(
                matches!(return_type, Some(Type::Named(ref n)) if n == "String"),
                "Expected return type Named(\"String\"), got {:?}",
                return_type
            );
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_extern_java_generic_return() {
    let tokens = tokenize(r#"extern "java:java.util" fn ArrayList() -> List<any>"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern {
            source,
            name,
            return_type,
            ..
        } => {
            assert_eq!(source, "java:java.util");
            assert_eq!(name, "ArrayList");
            match return_type {
                Some(Type::Generic { base, args }) => {
                    assert_eq!(base, "List");
                    assert_eq!(args.len(), 1);
                    assert!(
                        matches!(&args[0], Type::Named(n) if n == "any"),
                        "Expected generic arg Named(\"any\"), got {:?}",
                        &args[0]
                    );
                }
                other => panic!("Expected Generic return type List<any>, got {:?}", other),
            }
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_extern_multiple_params() {
    let tokens =
        tokenize(r#"extern "npm:fs" fn readFileSync(path: String, encoding: String) -> String"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern { name, params, .. } => {
            assert_eq!(name, "readFileSync");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "path");
            assert_eq!(params[1].name, "encoding");
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_extern_no_return_type() {
    let tokens = tokenize(r#"extern "npm:console" fn log(msg: String)"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern {
            name, return_type, ..
        } => {
            assert_eq!(name, "log");
            assert!(return_type.is_none(), "Expected no return type");
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_extern_invalid_no_source() {
    // `extern` without a source string should produce a parse error
    let tokens = tokenize("extern fn foo() -> Int");
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();
    assert!(
        !errors.is_empty(),
        "Should error when extern has no source string"
    );
}

#[test]
fn test_parse_extern_invalid_no_fn_keyword() {
    // `extern "source"` without the `fn` keyword should produce a parse error
    let tokens = tokenize(r#"extern "npm:express" express() -> Any"#);
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();
    assert!(
        !errors.is_empty(),
        "Should error when extern is missing 'fn' keyword"
    );
}

// ============================================================================
// Multiple extern declarations test
//
// Verifies that the parser can handle multiple extern declarations in a
// single source file, each with distinct sources, names, and signatures.
// This is critical for FFI interop where a module may import functions
// from several host packages.
// ============================================================================

#[test]
fn test_parse_multiple_extern_declarations() {
    let source = r#"
        extern "npm:express" fn express() -> Any
        extern "npm:fs" fn readFileSync(path: String, encoding: String) -> String
        extern "py:json" fn dumps(obj: Any) -> String
    "#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for multiple externs: {:?}",
        errors
    );
    assert_eq!(decls.len(), 3, "Should parse three extern declarations");

    // First extern: npm:express
    match &decls[0] {
        Decl::Extern {
            source,
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(source, "npm:express");
            assert_eq!(name, "express");
            assert!(params.is_empty(), "express should have no params");
            assert!(
                matches!(return_type, Some(Type::Named(ref n)) if n == "Any"),
                "Expected return type Named(\"Any\"), got {:?}",
                return_type
            );
        }
        other => panic!("Expected Extern declaration for decl[0], got {:?}", other),
    }

    // Second extern: npm:fs with two params
    match &decls[1] {
        Decl::Extern {
            source,
            name,
            params,
            ..
        } => {
            assert_eq!(source, "npm:fs");
            assert_eq!(name, "readFileSync");
            assert_eq!(params.len(), 2, "readFileSync should have two params");
            assert_eq!(params[0].name, "path");
            assert_eq!(params[1].name, "encoding");
        }
        other => panic!("Expected Extern declaration for decl[1], got {:?}", other),
    }

    // Third extern: py:json
    match &decls[2] {
        Decl::Extern {
            source,
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(source, "py:json");
            assert_eq!(name, "dumps");
            assert_eq!(params.len(), 1, "dumps should have one param");
            assert_eq!(params[0].name, "obj");
            assert!(
                matches!(return_type, Some(Type::Named(ref n)) if n == "String"),
                "Expected return type Named(\"String\"), got {:?}",
                return_type
            );
        }
        other => panic!("Expected Extern declaration for decl[2], got {:?}", other),
    }
}

// ============================================================================
// pub extern visibility test
//
// Verifies that `pub extern "source" fn name()` parses correctly with
// is_pub = true. This is needed so that extern declarations can be
// re-exported from modules, allowing downstream consumers to call
// host-provided functions.
// ============================================================================

#[test]
fn test_parse_pub_extern_visibility() {
    let tokens = tokenize(r#"pub extern "npm:express" fn express() -> Any"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for pub extern: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one pub extern declaration");

    match &decls[0] {
        Decl::Extern {
            source,
            name,
            is_pub,
            params,
            return_type,
            ..
        } => {
            assert_eq!(source, "npm:express");
            assert_eq!(name, "express");
            assert!(is_pub, "pub extern should have is_pub = true");
            assert!(params.is_empty(), "Expected no params");
            assert!(
                matches!(return_type, Some(Type::Named(ref n)) if n == "Any"),
                "Expected return type Named(\"Any\"), got {:?}",
                return_type
            );
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_private_extern_visibility() {
    // Sanity check: extern without `pub` should have is_pub = false
    let tokens = tokenize(r#"extern "npm:fs" fn readFileSync(path: String) -> String"#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Extern { name, is_pub, .. } => {
            assert_eq!(name, "readFileSync");
            assert!(!is_pub, "extern without pub should have is_pub = false");
        }
        other => panic!("Expected Extern declaration, got {:?}", other),
    }
}

// ============================================================================
// CONST DECLARATION PARSING (RED Phase — expected to fail)
//
// These tests will FAIL to compile because TokenKind::Const and
// Decl::Const do not exist yet. They define the expected behavior for
// const declarations: module-level immutable bindings with optional
// type annotations.
//
// Expected Decl::Const shape:
//   Decl::Const {
//       name: String,          // e.g. "MAX_SIZE"
//       value: Box<Expr>,      // the initializer expression
//       type_: Option<Type>,   // optional type annotation
//       is_pub: bool,          // visibility modifier
//       span: Span,
//   }
// ============================================================================

#[test]
fn test_parse_const_int_literal() {
    let tokens = tokenize("const x = 42");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const {
            name,
            value,
            type_,
            is_pub,
            ..
        } => {
            assert_eq!(name, "x");
            assert!(
                matches!(
                    value.as_ref(),
                    Expr::Literal {
                        value: LiteralValue::Int(42),
                        ..
                    }
                ),
                "Expected Int(42) literal, got {:?}",
                value
            );
            assert!(type_.is_none(), "No type annotation expected");
            assert!(!is_pub, "const without pub should have is_pub = false");
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_with_type_annotation() {
    let tokens = tokenize("const x: Int = 42");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const {
            name, value, type_, ..
        } => {
            assert_eq!(name, "x");
            assert!(
                matches!(
                    value.as_ref(),
                    Expr::Literal {
                        value: LiteralValue::Int(42),
                        ..
                    }
                ),
                "Expected Int(42) literal, got {:?}",
                value
            );
            assert!(
                matches!(type_, Some(Type::Named(ref n)) if n == "Int"),
                "Expected type annotation Named(\"Int\"), got {:?}",
                type_
            );
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_string_literal() {
    let tokens = tokenize(r#"const greeting = "hello""#);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const { name, value, .. } => {
            assert_eq!(name, "greeting");
            assert!(
                matches!(value.as_ref(), Expr::Literal { value: LiteralValue::Str(ref s), .. } if s == "hello"),
                "Expected Str(\"hello\") literal, got {:?}",
                value
            );
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_float_literal() {
    let tokens = tokenize("const pi = 3.14");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const { name, value, .. } => {
            assert_eq!(name, "pi");
            assert!(
                matches!(value.as_ref(), Expr::Literal { value: LiteralValue::Float(f), .. } if (f - 3.14).abs() < f64::EPSILON),
                "Expected Float(3.14) literal, got {:?}",
                value
            );
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_bool_literal() {
    let tokens = tokenize("const enabled = true");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const { name, value, .. } => {
            assert_eq!(name, "enabled");
            assert!(
                matches!(
                    value.as_ref(),
                    Expr::Literal {
                        value: LiteralValue::Bool(true),
                        ..
                    }
                ),
                "Expected Bool(true) literal, got {:?}",
                value
            );
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_pub_visibility() {
    let tokens = tokenize("pub const MAX_SIZE = 100");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const {
            name,
            value,
            is_pub,
            ..
        } => {
            assert_eq!(name, "MAX_SIZE");
            assert!(
                matches!(
                    value.as_ref(),
                    Expr::Literal {
                        value: LiteralValue::Int(100),
                        ..
                    }
                ),
                "Expected Int(100) literal, got {:?}",
                value
            );
            assert!(is_pub, "pub const should have is_pub = true");
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_with_expression_value() {
    // const can have an expression as its value, not just a literal
    let tokens = tokenize("const double = 21 * 2");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1);

    match &decls[0] {
        Decl::Const { name, value, .. } => {
            assert_eq!(name, "double");
            assert!(
                matches!(value.as_ref(), Expr::Binary { .. }),
                "Expected Binary expression, got {:?}",
                value
            );
        }
        other => panic!("Expected Const declaration, got {:?}", other),
    }
}

#[test]
fn test_parse_const_missing_value_errors() {
    // `const x` without `= value` should produce a parse error
    let tokens = tokenize("const x");
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();
    assert!(
        !errors.is_empty(),
        "Should error when const has no initializer value"
    );
}

#[test]
fn test_parse_const_missing_name_errors() {
    // `const = 42` without a name should produce a parse error
    let tokens = tokenize("const = 42");
    let mut parser = Parser::new(tokens);
    let (_decls, errors) = parser.parse();
    assert!(!errors.is_empty(), "Should error when const has no name");
}

#[test]
fn test_parse_multiple_const_declarations() {
    let source = r#"
        const MAX = 100
        const MIN = 0
        const NAME = "dwarf"
    "#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for multiple consts: {:?}",
        errors
    );
    assert_eq!(decls.len(), 3, "Should parse three const declarations");

    // Verify each is a Const
    for (i, decl) in decls.iter().enumerate() {
        assert!(
            matches!(decl, Decl::Const { .. }),
            "decl[{}] should be Const, got {:?}",
            i,
            decl
        );
    }
}

#[test]
fn test_parse_const_mixed_with_other_decls() {
    let source = r#"
        const VERSION = 1
        fn main() { 42 }
        const DEBUG = false
    "#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(errors.is_empty(), "No errors expected: {:?}", errors);
    assert_eq!(decls.len(), 3, "Should parse three declarations");

    assert!(matches!(&decls[0], Decl::Const { name, .. } if name == "VERSION"));
    assert!(matches!(&decls[1], Decl::Function { name, .. } if name == "main"));
    assert!(matches!(&decls[2], Decl::Const { name, .. } if name == "DEBUG"));
}

// ============================================================================
// OPTIONAL CHAINING `?.` PARSING (RED Phase — expected to fail)
//
// These tests will FAIL to compile because:
//   1. TokenKind::QuestionDot does not exist in the lexer
//   2. Expr::OptionalAccess does not exist in the HIR
//   3. The parser's postfix loop does not handle `?.`
//
// Expected Expr::OptionalAccess shape:
//   Expr::OptionalAccess {
//       obj: Box<Expr>,
//       field: String,
//       span: Span,
//   }
// ============================================================================

/// Helper: extract the body expression from a single-function parse.
/// Assumes `fn f() { <expr> }` and returns the inner expression.
fn parse_fn_body(source: &str) -> Expr {
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(errors.is_empty(), "No parse errors expected: {:?}", errors);
    assert_eq!(decls.len(), 1, "Should parse one declaration");

    if let Decl::Function { body, .. } = &decls[0] {
        if let Expr::Block { stmts, .. } = body {
            assert!(!stmts.is_empty(), "Function body should not be empty");
            if let Stmt::Expr(expr) = &stmts[0] {
                return expr.clone();
            }
            panic!("Expected Stmt::Expr as last statement");
        }
        panic!("Expected Block body");
    }
    panic!("Expected Function declaration");
}

#[test]
fn test_parse_optional_access_simple() {
    // obj?.field  →  OptionalAccess { obj: Var("obj"), field: "field" }
    let expr = parse_fn_body("fn f() { obj?.field }");
    match &expr {
        Expr::OptionalAccess { obj, field, .. } => {
            assert!(
                matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "obj"),
                "obj should be Variable(\"obj\"), got {:?}",
                obj
            );
            assert_eq!(field, "field", "field should be \"field\"");
        }
        other => panic!("Expected OptionalAccess, got {:?}", other),
    }
}

#[test]
fn test_parse_optional_access_value() {
    // obj?.value  →  OptionalAccess { obj: Var("obj"), field: "value" }
    let expr = parse_fn_body("fn f() { obj?.value }");
    match &expr {
        Expr::OptionalAccess { obj, field, .. } => {
            assert!(matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "obj"));
            assert_eq!(field, "value");
        }
        other => panic!("Expected OptionalAccess, got {:?}", other),
    }
}

#[test]
fn test_parse_optional_access_chained() {
    // obj?.name?.first
    // Should parse as:
    //   OptionalAccess {
    //     obj: OptionalAccess { obj: Var("obj"), field: "name" },
    //     field: "first"
    //   }
    let expr = parse_fn_body("fn f() { obj?.name?.first }");
    match &expr {
        Expr::OptionalAccess { obj, field, .. } => {
            assert_eq!(field, "first", "Outer field should be \"first\"");
            match obj.as_ref() {
                Expr::OptionalAccess {
                    obj: inner_obj,
                    field: inner_field,
                    ..
                } => {
                    assert_eq!(inner_field, "name", "Inner field should be \"name\"");
                    assert!(
                        matches!(inner_obj.as_ref(), Expr::Variable { name, .. } if name == "obj"),
                        "Innermost obj should be Variable(\"obj\")"
                    );
                }
                other => panic!("Expected inner OptionalAccess, got {:?}", other),
            }
        }
        other => panic!("Expected outer OptionalAccess, got {:?}", other),
    }
}

#[test]
fn test_parse_regular_member_access_still_works() {
    // obj.field (without ?) should still parse as Member, not OptionalAccess
    let expr = parse_fn_body("fn f() { obj.field }");
    match &expr {
        Expr::Member { obj, field, .. } => {
            assert!(matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "obj"));
            assert_eq!(field, "field");
        }
        other => panic!("Expected Member (not OptionalAccess), got {:?}", other),
    }
}

#[test]
fn test_parse_optional_access_followed_by_call() {
    // obj?.method()  — optional access followed by function call
    // Should parse as: Call { func: OptionalAccess { obj: Var("obj"), field: "method" }, args: [] }
    let expr = parse_fn_body("fn f() { obj?.method() }");
    match &expr {
        Expr::Call { func, args, .. } => {
            assert!(args.is_empty(), "method() should have no args");
            match func.as_ref() {
                Expr::OptionalAccess { obj, field, .. } => {
                    assert!(
                        matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "obj"),
                        "obj should be Variable(\"obj\")"
                    );
                    assert_eq!(field, "method", "field should be \"method\"");
                }
                other => panic!("Expected OptionalAccess as callee, got {:?}", other),
            }
        }
        other => panic!("Expected Call expr, got {:?}", other),
    }
}

#[test]
fn test_parse_optional_access_mixed_with_regular() {
    // obj?.a.b  — optional access then regular member
    // Should parse as: Member { obj: OptionalAccess { obj: Var("obj"), field: "a" }, field: "b" }
    let expr = parse_fn_body("fn f() { obj?.a.b }");
    match &expr {
        Expr::Member { obj, field, .. } => {
            assert_eq!(field, "b", "Outer field should be \"b\"");
            match obj.as_ref() {
                Expr::OptionalAccess {
                    obj: inner_obj,
                    field: inner_field,
                    ..
                } => {
                    assert_eq!(inner_field, "a", "Inner field should be \"a\"");
                    assert!(
                        matches!(inner_obj.as_ref(), Expr::Variable { name, .. } if name == "obj"),
                        "Innermost obj should be Variable(\"obj\")"
                    );
                }
                other => panic!("Expected OptionalAccess as inner, got {:?}", other),
            }
        }
        other => panic!("Expected Member expr, got {:?}", other),
    }
}
