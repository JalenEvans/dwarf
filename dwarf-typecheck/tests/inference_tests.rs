//! Integration tests for the expression type inference module.
//!
//! These tests validate the public API of `infer::infer_expr()` and the
//! `infer::TypeEnv` type.
//!
//! All tests involving `infer_expr` are expected to fail (Red phase) because
//! the inference logic is not yet implemented — only a stub exists that
//! always returns `Err("not implemented")`.
//!
//! Tests for `TypeEnv` (sections 12–13) DO pass because the `TypeEnv` struct
//! is fully implemented.

use dwarf_syntax::hir::*;
use dwarf_syntax::span::Span;
use dwarf_typecheck::infer::*;
use dwarf_typecheck::registry::TypeRegistry;
use dwarf_typecheck::types::*;

// ---------------------------------------------------------------------------
// Helper: create a dummy Span for synthetic HIR nodes
// ---------------------------------------------------------------------------

fn dummy_span() -> Span {
    Span::new(0, 0, 0)
}

// ===========================================================================
// 1. Literal expressions
//    Each literal maps to a fixed primitive TypeId:
//      Int   → 0
//      Float → 1
//      Str   → 2
//      Bool  → 3
//      Null  → 4
// ===========================================================================

#[test]
fn test_literal_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Literal {
        value: LiteralValue::Int(42),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_literal_float() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Literal {
        value: LiteralValue::Float(3.5),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(1));
}

#[test]
fn test_literal_str() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Literal {
        value: LiteralValue::Str("hello".to_string()),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(2));
}

#[test]
fn test_literal_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Literal {
        value: LiteralValue::Bool(true),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(3));
}

#[test]
fn test_literal_null() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Literal {
        value: LiteralValue::Null,
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(4));
}

// ===========================================================================
// 2. Variable references
//    A variable bound in the TypeEnv returns its bound type.
//    An unbound variable returns Err.
// ===========================================================================

#[test]
fn test_variable_bound_int() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // Int
    let expr = Expr::Variable {
        name: "x".to_string(),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_variable_bound_str() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();
    env.bind("s".to_string(), 2); // Str
    let expr = Expr::Variable {
        name: "s".to_string(),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(2));
}

#[test]
fn test_variable_unknown() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Variable {
        name: "z".to_string(),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(result.is_err(), "Unknown variable should return an error");
}

// ===========================================================================
// 3. Binary operations
//    Type-checking rules for binary operators:
//      Arithmetic (Add, Sub, Mul, Div): both operands must be Int → Int,
//          or both Str → Str (Add only, for concatenation).
//      Comparison (Eq, Ne, Lt, Gt, Le, Ge): both operands must be same type
//          → Bool.
//      Logical (And, Or): both operands must be Bool → Bool.
//    Mixed types (e.g. Int + Float, Bool + Int) produce Err.
// ===========================================================================

#[test]
fn test_binary_int_add_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(2),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "Int + Int should yield Int");
}

#[test]
fn test_binary_int_add_float() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // Dwarf may be strict about type mixing — Int + Float is an error
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Float(3.5),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "Int + Float should be an error in strict mode"
    );
}

#[test]
fn test_binary_int_eq_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(2),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(3), "Int == Int should yield Bool");
}

#[test]
fn test_binary_bool_and_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Binary {
        op: BinaryOp::And,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Bool(false),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(3), "Bool && Bool should yield Bool");
}

#[test]
fn test_binary_str_add_str() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Str("hello ".to_string()),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Str("world".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(2), "Str + Str should yield Str (concatenation)");
}

#[test]
fn test_binary_bool_add_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // Bool + Int — type mismatch
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "Bool + Int should be a type mismatch error"
    );
}

#[test]
fn test_binary_int_add_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // 1 + true — type mismatch
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "Int + Bool should be a type mismatch error"
    );
}

// ===========================================================================
// 4. Unary operations
//    Neg (−) applies to Int → Int.
//    Not (!) applies to Bool → Bool.
//    Applying the wrong unary op produces an error.
// ===========================================================================

#[test]
fn test_unary_neg_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "-Int should yield Int");
}

#[test]
fn test_unary_not_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(3), "!Bool should yield Bool");
}

#[test]
fn test_unary_neg_str() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Str("hello".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "-Str should be an error (cannot negate a string)"
    );
}

#[test]
fn test_unary_not_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "!Int should be an error (cannot negate an int)"
    );
}

// ===========================================================================
// 5. Block expressions
//    A block evaluates statements in order; the last expression's type is
//    the block's type. `let` bindings extend the environment for subsequent
//    statements in the same block.
// ===========================================================================

#[test]
fn test_block_single_expr() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::Block {
        stmts: vec![Stmt::Expr(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        })],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "Block with single Int expr should yield Int");
}

#[test]
fn test_block_let_bind() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // { let x = 42; x }
    let expr = Expr::Block {
        stmts: vec![
            Stmt::Let(
                Pat::Variable("x".to_string()),
                Expr::Literal {
                    value: LiteralValue::Int(42),
                    span: dummy_span(),
                },
            ),
            Stmt::Expr(Expr::Variable {
                name: "x".to_string(),
                span: dummy_span(),
            }),
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "Block with `let x = 42; x` should yield Int");
}

#[test]
fn test_block_multiple_lets() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // { let x = 42; let y = x + 1; y }
    let expr = Expr::Block {
        stmts: vec![
            Stmt::Let(
                Pat::Variable("x".to_string()),
                Expr::Literal {
                    value: LiteralValue::Int(42),
                    span: dummy_span(),
                },
            ),
            Stmt::Let(
                Pat::Variable("y".to_string()),
                Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Variable {
                        name: "x".to_string(),
                        span: dummy_span(),
                    }),
                    rhs: Box::new(Expr::Literal {
                        value: LiteralValue::Int(1),
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ),
            Stmt::Expr(Expr::Variable {
                name: "y".to_string(),
                span: dummy_span(),
            }),
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "Block with two lets should yield Int");
}

// ===========================================================================
// 6. If expressions
//    The condition must be Bool.
//    Both arms must have the same type (or the else branch must be absent).
//    When arms match, the result is that type.
// ===========================================================================

#[test]
fn test_if_both_arms_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    let expr = Expr::If {
        cond: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        then: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        else_: Some(Box::new(Expr::Literal {
            value: LiteralValue::Int(2),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(0),
        "if true {{ 1 }} else {{ 2 }} should yield Int"
    );
}

#[test]
fn test_if_arms_mismatch() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // if true { 1 } else { "no" } — arms differ (Int vs Str)
    let expr = Expr::If {
        cond: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        then: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        else_: Some(Box::new(Expr::Literal {
            value: LiteralValue::Str("no".to_string()),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "If arms with mismatched types should return an error"
    );
}

#[test]
fn test_if_condition_not_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // if 42 { 1 } else { 2 } — condition is Int, not Bool
    let expr = Expr::If {
        cond: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        then: Box::new(Expr::Literal {
            value: LiteralValue::Int(1),
            span: dummy_span(),
        }),
        else_: Some(Box::new(Expr::Literal {
            value: LiteralValue::Int(2),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(result.is_err(), "If condition must be Bool, not Int");
}

// ===========================================================================
// 7. Lambda expressions
//    A lambda with annotated parameter types can infer a function type.
//    |x: Int| x + 1 should register Func([Int], Int) as an anonymous type.
// ===========================================================================

#[test]
fn test_lambda_annotated_param() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // |x: Int| x + 1
    let expr = Expr::Lambda {
        params: vec![Param {
            name: "x".to_string(),
            type_: Some(Type::Named("int".to_string())),
        }],
        body: Box::new(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Variable {
                name: "x".to_string(),
                span: dummy_span(),
            }),
            rhs: Box::new(Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Expected: the lambda registers Func([Int], Int) as the first
    // anonymous type at ID 5 (after the 5 primitives).
    assert!(result.is_ok(), "Lambda should infer a function type");
    let type_id = result.unwrap();
    assert_eq!(
        registry.get(type_id),
        Some(&TypeDef::Func(vec![0], 0)),
        "Lambda should yield Func([Int], Int)"
    );
}

// ===========================================================================
// 8. Call expressions
//    A call requires the callee to be a function type, the argument types
//    to match the parameter types, and the result is the return type.
// ===========================================================================

#[test]
fn test_call_correct_types() {
    let mut registry = TypeRegistry::new();
    // Register a function type: Func([Int], Bool) at ID 9
    registry.register(TypeDef::Func(vec![0], 3));
    let mut env = TypeEnv::new();
    env.bind("f".to_string(), 9);

    // f(42) — correct arg type (Int for Int param)
    let expr = Expr::Call {
        func: Box::new(Expr::Variable {
            name: "f".to_string(),
            span: dummy_span(),
        }),
        args: vec![Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(3),
        "Calling Func([Int], Bool) with Int arg should yield Bool"
    );
}

#[test]
fn test_call_wrong_arg_types() {
    let mut registry = TypeRegistry::new();
    // Register a function type: Func([Int], Bool) at ID 9
    registry.register(TypeDef::Func(vec![0], 3));
    let mut env = TypeEnv::new();
    env.bind("f".to_string(), 9);

    // f("hi") — wrong arg type (Str instead of Int)
    let expr = Expr::Call {
        func: Box::new(Expr::Variable {
            name: "f".to_string(),
            span: dummy_span(),
        }),
        args: vec![Expr::Literal {
            value: LiteralValue::Str("hi".to_string()),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "Calling Func([Int], Bool) with Str arg should return an error"
    );
}

// ===========================================================================
// 9. Record expressions
//    A record literal { x: 1, y: 2 } registers an anonymous record type
//    in the registry and returns its TypeId.
// ===========================================================================

#[test]
fn test_record_expression() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // { x: 1, y: 2 }
    let expr = Expr::Record {
        fields: vec![
            (
                "x".to_string(),
                Expr::Literal {
                    value: LiteralValue::Int(1),
                    span: dummy_span(),
                },
            ),
            (
                "y".to_string(),
                Expr::Literal {
                    value: LiteralValue::Int(2),
                    span: dummy_span(),
                },
            ),
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Should register anonymous record { x: Int, y: Int } at ID 5
    assert!(
        result.is_ok(),
        "Record expression should infer a record type"
    );
    let type_id = result.unwrap();
    assert_eq!(
        registry.get(type_id),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "x".to_string(),
                type_id: 0, // Int
            },
            FieldDef {
                name: "y".to_string(),
                type_id: 0, // Int
            },
        ])),
        "Record {{ x: 1, y: 2 }} should yield Record {{ x: Int, y: Int }}"
    );
}

// ===========================================================================
// 10. Member access
//     point.x where point is a record with field x: Int should return Int.
// ===========================================================================

#[test]
fn test_member_access() {
    let mut registry = TypeRegistry::new();
    // Register a record type { x: Int, y: Int } at ID 5
    registry.register(TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
    ]));
    let mut env = TypeEnv::new();
    env.bind("point".to_string(), 9);

    // point.x
    let expr = Expr::Member {
        obj: Box::new(Expr::Variable {
            name: "point".to_string(),
            span: dummy_span(),
        }),
        field: "x".to_string(),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(result, Ok(0), "point.x where point.x: Int should yield Int");
}

// ===========================================================================
// 11. Match expressions
//     All arms must have the same type. The scrutinee type determines what
//     patterns are valid.
// ===========================================================================

#[test]
fn test_match_arms_same_type() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // Int

    // match x { 1 => "one", _ => "other" }
    let expr = Expr::Match {
        expr: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        arms: vec![
            MatchArm {
                pattern: Pat::Literal(LiteralValue::Int(1)),
                guard: None,
                body: Expr::Literal {
                    value: LiteralValue::Str("one".to_string()),
                    span: dummy_span(),
                },
            },
            MatchArm {
                pattern: Pat::Wildcard,
                guard: None,
                body: Expr::Literal {
                    value: LiteralValue::Str("other".to_string()),
                    span: dummy_span(),
                },
            },
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "Match with both arms returning Str should yield Str"
    );
}

// ===========================================================================
// 12. Empty TypeEnv
//     A newly-created TypeEnv has no bindings.
// ===========================================================================

#[test]
fn test_type_env_new_empty() {
    let env = TypeEnv::new();
    assert_eq!(env.len(), 0, "New TypeEnv should have length 0");
    assert!(env.is_empty(), "New TypeEnv should be empty");
}

#[test]
fn test_type_env_lookup_empty() {
    let env = TypeEnv::new();
    assert_eq!(
        env.lookup("anything"),
        None,
        "Lookup in empty TypeEnv should return None"
    );
    assert_eq!(
        env.lookup("x"),
        None,
        "Lookup in empty TypeEnv should return None for any name"
    );
    assert_eq!(
        env.lookup(""),
        None,
        "Lookup in empty TypeEnv should return None even for empty string"
    );
}

// ===========================================================================
// 13. TypeEnv operations
//     Bind and lookup round-trip correctly. Multiple bindings coexist.
// ===========================================================================

#[test]
fn test_type_env_bind_and_lookup() {
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // Int
    assert_eq!(
        env.lookup("x"),
        Some(0),
        "Should retrieve Int (0) for bound variable 'x'"
    );
    assert!(!env.is_empty(), "TypeEnv should not be empty after binding");
    assert_eq!(env.len(), 1, "TypeEnv should have length 1");
}

#[test]
fn test_type_env_bind_multiple() {
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // Int
    env.bind("y".to_string(), 1); // Float
    env.bind("s".to_string(), 2); // Str

    assert_eq!(env.len(), 3, "TypeEnv should have 3 bindings");
    assert!(!env.is_empty(), "TypeEnv should not be empty");

    assert_eq!(
        env.lookup("x"),
        Some(0),
        "Variable 'x' should resolve to Int"
    );
    assert_eq!(
        env.lookup("y"),
        Some(1),
        "Variable 'y' should resolve to Float"
    );
    assert_eq!(
        env.lookup("s"),
        Some(2),
        "Variable 's' should resolve to Str"
    );
    assert_eq!(
        env.lookup("z"),
        None,
        "Unbound variable 'z' should return None"
    );
}

// ===========================================================================
// 14. Array expressions (DWARF-55 Phase 1)
//     Arrays infer to List<T> where T is the common type of all elements.
//     Empty arrays infer to List<Null> (or a fresh type variable).
//     Heterogeneous arrays produce a type error.
//     Currently stubbed to return Ok(0) — these tests will pass once
//     the real inference is implemented.
// ===========================================================================

#[test]
fn test_array_empty() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // []
    let expr = Expr::Array {
        items: vec![],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Red-phase: stub returns Ok(0), but should return a List type
    assert!(result.is_ok(), "Empty array should infer successfully");
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Empty array should NOT infer to Int (stub fails here)"
    );

    // After implementation: the result should be a GenericInstance (List)
    // with one type argument (the element type, likely Null or a fresh variable)
    match registry.get(type_id) {
        Some(TypeDef::GenericInstance { base: _, args }) => {
            assert_eq!(args.len(), 1, "List type should have one type argument");
            assert!(
                registry.get(args[0]).is_some(),
                "Element type should be registered"
            );
        }
        other => {
            panic!(
                "Empty array should infer to a GenericInstance (List) type, got {:?}",
                other
            );
        }
    }
}

#[test]
fn test_array_homogeneous_int() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // [1, 2, 3]
    let expr = Expr::Array {
        items: vec![
            Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            },
            Expr::Literal {
                value: LiteralValue::Int(2),
                span: dummy_span(),
            },
            Expr::Literal {
                value: LiteralValue::Int(3),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    assert!(
        result.is_ok(),
        "Homogeneous array should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Homogeneous array should NOT infer to Int (stub fails here)"
    );

    // After implementation: should be List<Int>
    match registry.get(type_id) {
        Some(TypeDef::GenericInstance { base: _, args }) => {
            assert_eq!(args.len(), 1, "List type should have one type argument");
            assert_eq!(args[0], 0, "Element type should be Int");
        }
        other => {
            panic!(
                "Array should infer to a GenericInstance (List) type, got {:?}",
                other
            );
        }
    }
}

#[test]
fn test_array_nested() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // [[1, 2], [3, 4]]
    let inner1 = Expr::Array {
        items: vec![
            Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            },
            Expr::Literal {
                value: LiteralValue::Int(2),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };
    let inner2 = Expr::Array {
        items: vec![
            Expr::Literal {
                value: LiteralValue::Int(3),
                span: dummy_span(),
            },
            Expr::Literal {
                value: LiteralValue::Int(4),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };
    let expr = Expr::Array {
        items: vec![inner1, inner2],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    assert!(result.is_ok(), "Nested array should infer successfully");
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Nested array should NOT infer to Int (stub fails here)"
    );

    // After implementation: should be List<List<Int>>
    match registry.get(type_id) {
        Some(TypeDef::GenericInstance { base: _, args }) => {
            assert_eq!(args.len(), 1, "Outer List should have one type argument");
            let inner_type_id = args[0];
            // Inner should be List<Int>
            match registry.get(inner_type_id) {
                Some(TypeDef::GenericInstance {
                    base: _,
                    args: inner_args,
                }) => {
                    assert_eq!(
                        inner_args.len(),
                        1,
                        "Inner List should have one type argument"
                    );
                    assert_eq!(inner_args[0], 0, "Innermost element type should be Int");
                }
                other => {
                    panic!("Inner array should be GenericInstance, got {:?}", other);
                }
            }
        }
        other => {
            panic!("Outer array should be GenericInstance, got {:?}", other);
        }
    }
}

#[test]
fn test_array_heterogeneous_error() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // [1, "hello"] — type mismatch between Int and Str
    let expr = Expr::Array {
        items: vec![
            Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            },
            Expr::Literal {
                value: LiteralValue::Str("hello".to_string()),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    assert!(
        result.is_err(),
        "Heterogeneous array [Int, Str] should produce a type error (stub fails here)"
    );
}

// ===========================================================================
// 15. Wildcard expressions (DWARF-55 Phase 1)
//     A wildcard `_` is a placeholder expression that infers to a bottom
//     type (Null) or a fresh type variable. It should never crash.
//     Currently stubbed to return Ok(0).
// ===========================================================================

#[test]
fn test_wildcard_infers() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();
    // _
    let expr = Expr::Wildcard { span: dummy_span() };
    let result = infer_expr(&expr, &env, &mut registry);

    assert!(
        result.is_ok(),
        "Wildcard expression should infer successfully"
    );
    let type_id = result.unwrap();

    // Red-phase: stub returns Int (0), but wildcard should NOT be Int
    assert_ne!(
        type_id, 0,
        "Wildcard should NOT infer to Int (stub fails here)"
    );

    // After implementation: wildcard should be Null (4) or a fresh type variable.
    // Assert it's a valid registered type.
    assert!(
        registry.get(type_id).is_some(),
        "Wildcard type should be registered in the registry"
    );
}

// ===========================================================================
// 16. Variant expression inference (DWARF-55 Phase 2)
//     Variant expressions like `None`, `Some(42)` look up variant definitions
//     in registered union types. Unit variants have no payload; payload
//     variants carry a single expression that must match the declared type.
//     Unknown variant names produce an error.
// ===========================================================================

/// Helper: register an `Option<Int>` union type in the registry.
///
/// This creates a union with two variants:
///   - `Some(Int)` — payload variant
///   - `None` — unit variant
///
/// Returns the assigned TypeId (first user type after 5 primitives + 4 built-in generics).
fn register_option_type(registry: &mut TypeRegistry) -> TypeId {
    let some_variant = VariantDef {
        name: "Some".to_string(),
        type_id: Some(0), // Int payload
    };
    let none_variant = VariantDef {
        name: "None".to_string(),
        type_id: None, // Unit variant
    };
    registry.register(TypeDef::Union(vec![some_variant, none_variant]))
}

#[test]
fn test_variant_unit_no_arg() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    register_option_type(&mut registry);

    // None — unit variant without a payload
    let expr = Expr::Variant {
        name: "None".to_string(),
        arg: None,
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Red-phase: stub returns Ok(0), but it should be the union type
    assert!(
        result.is_ok(),
        "Unit variant 'None' should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Unit variant should NOT infer to Int (stub fails here)"
    );

    // After implementation: should be a Union containing the None variant
    match registry.get(type_id) {
        Some(TypeDef::Union(variants)) => {
            assert!(
                variants.iter().any(|v| v.name == "None"),
                "Union should contain a 'None' variant"
            );
        }
        other => {
            panic!(
                "Variant expression should infer to a Union type, got {:?}",
                other
            );
        }
    }
}

#[test]
fn test_variant_with_payload() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // Register Option<Int> at ID 5
    register_option_type(&mut registry);

    // Some(42) — variant with Int payload matching the union definition
    let expr = Expr::Variant {
        name: "Some".to_string(),
        arg: Some(Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Red-phase: stub returns Ok(0), but it should be the union type
    assert!(
        result.is_ok(),
        "Payload variant 'Some(42)' should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Payload variant should NOT infer to Int (stub fails here)"
    );

    // After implementation: should be a Union containing the Some variant
    match registry.get(type_id) {
        Some(TypeDef::Union(variants)) => {
            assert!(
                variants
                    .iter()
                    .any(|v| v.name == "Some" && v.type_id == Some(0)),
                "Union should contain 'Some' variant with Int payload"
            );
        }
        other => {
            panic!(
                "Variant expression should infer to a Union type, got {:?}",
                other
            );
        }
    }
}

#[test]
fn test_variant_payload_mismatch() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // where Some expects Int payload
    register_option_type(&mut registry);

    // Some("hello") — arg is Str but Some expects Int
    let expr = Expr::Variant {
        name: "Some".to_string(),
        arg: Some(Box::new(Expr::Literal {
            value: LiteralValue::Str("hello".to_string()),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but this should be a type error
    assert!(
        result.is_err(),
        "Payload type mismatch (Str vs expected Int) should produce an error (stub fails here)"
    );
}

#[test]
fn test_variant_unknown_name() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // No union registered — Foo doesn't exist in any variant set

    // Foo(42) — unknown variant name
    let expr = Expr::Variant {
        name: "Foo".to_string(),
        arg: Some(Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but unknown variant should be an error
    assert!(
        result.is_err(),
        "Unknown variant name 'Foo' should produce an error (stub fails here)"
    );
}

// ===========================================================================
// 17. Pipe expressions (DWARF-55 Phase 3)
//     lhs |> rhs is equivalent to rhs(lhs). RHS must evaluate to a function
//     type, and LHS type must match the function's first parameter type.
//     The result is the function's return type.
//     Currently stubbed to return Ok(0) — these tests fail under the stub.
// ===========================================================================

#[test]
fn test_pipe_simple() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register a function type: Func([Int], Str) at ID 9
    registry.register(TypeDef::Func(vec![0], 2)); // f: Int -> Str
    env.bind("f".to_string(), 9);

    // 5 |> f — equivalent to f(5), f returns Str
    let expr = Expr::Pipe {
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(5),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Variable {
            name: "f".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Expected: Ok(2) (Str), stub returns Ok(0) (Int)
    assert_eq!(
        result,
        Ok(2),
        "5 |> f where f: Int -> Str should yield Str (stub fails here)"
    );
}

#[test]
fn test_pipe_chain() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register f: Int -> Float at ID 9
    registry.register(TypeDef::Func(vec![0], 1));
    // Register g: Float -> Bool at ID 10
    registry.register(TypeDef::Func(vec![1], 3));
    env.bind("f".to_string(), 9);
    env.bind("g".to_string(), 10);

    // 5 |> f |> g — equivalent to g(f(5))
    // inner pipe: 5 |> f → Float (1)
    // outer pipe: Float |> g → Bool (3)
    let inner_pipe = Expr::Pipe {
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Int(5),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Variable {
            name: "f".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let outer_pipe = Expr::Pipe {
        lhs: Box::new(inner_pipe),
        rhs: Box::new(Expr::Variable {
            name: "g".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&outer_pipe, &env, &mut registry);
    // Expected: Ok(3) (Bool), stub returns Ok(0) (Int)
    assert_eq!(
        result,
        Ok(3),
        "5 |> f |> g where f: Int->Float, g: Float->Bool should yield Bool (stub fails here)"
    );
}

#[test]
fn test_pipe_type_mismatch() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register a function type: Func([Int], Int) at ID 9
    registry.register(TypeDef::Func(vec![0], 0)); // f: Int -> Int
    env.bind("f".to_string(), 9);

    // "hello" |> f — Str (2) doesn't match Int (0) param
    let expr = Expr::Pipe {
        lhs: Box::new(Expr::Literal {
            value: LiteralValue::Str("hello".to_string()),
            span: dummy_span(),
        }),
        rhs: Box::new(Expr::Variable {
            name: "f".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Expected: Err (type mismatch), stub returns Ok(0)
    assert!(
        result.is_err(),
        "Pipe with type mismatch (Str into Int param) should return an error (stub fails here)"
    );
}

// ===========================================================================
// 18. Propagate expressions (DWARF-55 Phase 3)
//     ?expr unwraps Ok(T) / Some(T) from a union type. If the inner expression
//     is a union with an Ok/Some variant carrying a payload, the result is the
//     payload type. If the inner expression is not an appropriate union type,
//     an error is produced.
//     Currently stubbed to return Ok(0) — these tests fail under the stub.
// ===========================================================================

/// Helper: register an `Option<Bool>` union type in the registry.
///
/// Creates a union with variants:
///   - `Some(Bool)` — payload variant
///   - `None` — unit variant
///
/// Returns the assigned TypeId (first user type after 5 primitives + 4 built-in generics).
fn register_option_bool_type(registry: &mut TypeRegistry) -> TypeId {
    let some_variant = VariantDef {
        name: "Some".to_string(),
        type_id: Some(3), // Bool payload
    };
    let none_variant = VariantDef {
        name: "None".to_string(),
        type_id: None, // Unit variant
    };
    registry.register(TypeDef::Union(vec![some_variant, none_variant]))
}

#[test]
fn test_propagate_result() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // Register Option<Bool> at ID 5
    register_option_bool_type(&mut registry);

    // Some(true) — variant expression returning Option<Bool>
    let inner_expr = Expr::Variant {
        name: "Some".to_string(),
        arg: Some(Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        })),
        span: dummy_span(),
    };

    // ?Some(true) — propagate unwraps the inner Bool payload
    let expr = Expr::Propagate {
        expr: Box::new(inner_expr),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Expected: Ok(3) (Bool), stub returns Ok(0) (Int)
    assert_eq!(
        result,
        Ok(3),
        "?Some(true) where Some has Bool payload should yield Bool (stub fails here)"
    );
}

#[test]
fn test_propagate_on_non_option_result() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // ?42 — propagate on a literal Int, which is not Result/Option-like
    let expr = Expr::Propagate {
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Expected: Err, stub returns Ok(0)
    assert!(
        result.is_err(),
        "Propagate on non-union type (Int) should produce an error (stub fails here)"
    );
}

// ===========================================================================
// 19. For loop inference (DWARF-55 Phase 4)
//     `for x in iterable { body }` binds the loop variable to the element
//     type of a list, infers the body in a scoped environment, and returns
//     Null (unit). Iterating over a non-list type produces a type error.
//     Currently stubbed to return Ok(0) — these tests fail under the stub.
// ===========================================================================

#[test]
fn test_for_loop_list() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // for x in [1, 2, 3] { x }
    let expr = Expr::For {
        binding: Pat::Variable("x".to_string()),
        iterable: Box::new(Expr::Array {
            items: vec![
                Expr::Literal {
                    value: LiteralValue::Int(1),
                    span: dummy_span(),
                },
                Expr::Literal {
                    value: LiteralValue::Int(2),
                    span: dummy_span(),
                },
                Expr::Literal {
                    value: LiteralValue::Int(3),
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        }),
        body: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but for loop should NOT infer to Int
    assert!(
        result.is_ok(),
        "For loop over a list should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "For loop should NOT infer to Int (stub fails here)"
    );

    // After implementation: the loop itself should be Null (4)
}

#[test]
fn test_for_loop_empty_list() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // for x in [] { null }
    let expr = Expr::For {
        binding: Pat::Variable("x".to_string()),
        iterable: Box::new(Expr::Array {
            items: vec![],
            span: dummy_span(),
        }),
        body: Box::new(Expr::Literal {
            value: LiteralValue::Null,
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but for loop should NOT infer to Int
    assert!(
        result.is_ok(),
        "For loop over an empty list should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "For loop over empty list should NOT infer to Int (stub fails here)"
    );

    // After implementation: the loop itself should be Null (4)
}

#[test]
fn test_for_loop_non_list() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // for x in 42 { x } — iterating over Int (not a List)
    let expr = Expr::For {
        binding: Pat::Variable("x".to_string()),
        iterable: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        body: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but iterating over a non-list should error
    assert!(
        result.is_err(),
        "For loop over non-list type (Int) should produce an error (stub fails here)"
    );
}

// ===========================================================================
// 20. Assign expression inference (DWARF-55 Phase 4)
//     `target = value` checks that the target and value types are compatible
//     and returns Null (unit). A type mismatch produces a type error.
//     Currently stubbed to return Ok(0) — these tests fail under the stub.
// ===========================================================================

#[test]
fn test_assign_simple() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // x: Int

    // x = 42
    let expr = Expr::Assign {
        target: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        value: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but assign should NOT infer to Int
    assert!(
        result.is_ok(),
        "Assign with matching types should infer successfully"
    );
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "Assign should NOT infer to Int (stub fails here)"
    );

    // After implementation: assignment should be Null (4)
}

#[test]
fn test_assign_type_mismatch() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();
    env.bind("x".to_string(), 0); // x: Int

    // x = "hello" — Int vs Str mismatch
    let expr = Expr::Assign {
        target: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        value: Box::new(Expr::Literal {
            value: LiteralValue::Str("hello".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);

    // Stub returns Ok(0), but type mismatch should error
    assert!(
        result.is_err(),
        "Assign with type mismatch (Int vs Str) should produce an error (stub fails here)"
    );
}

// ===========================================================================
// 21. ForAll and AssertConsistent inference (DWARF-55 Phase 5)
//     ForAll: forAll(x: Int) { property } binds a variable with an explicit
//     type annotation and requires the property expression to be Bool.
//     AssertConsistent: assertConsistent(expr) is a pass-through that returns
//     the inner expression's type.
//     Both are currently stubbed to return Ok(0) — these tests fail under
//     the stub.
// ===========================================================================

#[test]
fn test_forall_valid_property() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // forAll(x: Int) { x == x }
    // The property `x == x` compares two Ints, producing Bool.
    // After implementation: Ok(3) (Bool).
    let expr = Expr::ForAll {
        type_: Type::Named("Int".to_string()),
        binding: Pat::Variable("x".to_string()),
        property: Box::new(Expr::Binary {
            op: BinaryOp::Eq,
            lhs: Box::new(Expr::Variable {
                name: "x".to_string(),
                span: dummy_span(),
            }),
            rhs: Box::new(Expr::Variable {
                name: "x".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Stub returns Ok(0), but ForAll with a Bool property should yield Bool
    assert_ne!(
        result.unwrap_or(0),
        0,
        "ForAll with valid Bool property should NOT infer to Int (stub fails here)"
    );
}

#[test]
fn test_forall_invalid_property() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // forAll(x: Int) { 42 }
    // The property is an Int literal, which is not Bool — should error.
    let expr = Expr::ForAll {
        type_: Type::Named("Int".to_string()),
        binding: Pat::Variable("x".to_string()),
        property: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Stub returns Ok(0), but a non-Bool property should produce an error
    assert!(
        result.is_err(),
        "ForAll with non-Bool property should produce an error (stub fails here)"
    );
}

#[test]
fn test_assert_consistent_custom_type() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // assertConsistent({}) — pass-through on an empty record expression.
    // The record expression produces a unique TypeId > 0 (not the stub Int),
    // so assert_ne!(..., 0) properly verifies the pass-through delegates to
    // infer_expr rather than returning the blanket stub.
    let expr = Expr::AssertConsistent {
        expr: Box::new(Expr::Record {
            fields: vec![],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert!(result.is_ok(), "assertConsistent({{}}) should not fail");
    let type_id = result.unwrap();
    assert_ne!(
        type_id, 0,
        "assertConsistent({{}}) should pass through the record type (which is != Int), stub fails here"
    );

    // Verify it's actually a Record type (pass-through property)
    match registry.get(type_id) {
        Some(TypeDef::Record(fields)) => {
            assert!(fields.is_empty(), "Empty record should have no fields");
        }
        other => {
            panic!(
                "assertConsistent should pass through the inner Record type, got {:?}",
                other
            );
        }
    }
}

#[test]
fn test_assert_consistent_bool() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // assertConsistent(true) — pass-through, should return Bool (3)
    let expr = Expr::AssertConsistent {
        expr: Box::new(Expr::Literal {
            value: LiteralValue::Bool(true),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    // Stub returns Ok(0), but assertConsistent(true) should yield Bool
    assert_ne!(
        result.unwrap_or(0),
        0,
        "assertConsistent(true) should NOT infer to Int (stub fails here)"
    );
}

// ===========================================================================
// 22. Try/Catch/Throw inference (DWARF-57 Phase 2)
//     try { body } catch e { handler } infers to the common type of body and
//     handler. throw expr is well-typed if expr is well-typed.
// ===========================================================================

/// Helper: register a `Result<{ value: Str }, Int>` union type in the registry.
///
/// Creates a union with variants:
///   - `Ok({ value: Str })` — payload variant carrying a record
///   - `Err(Int)` — payload variant used as the error type
///
/// Returns the assigned TypeId of the union.
fn register_result_value_type(registry: &mut TypeRegistry) -> TypeId {
    let record_id = registry.register(TypeDef::Record(vec![FieldDef {
        name: "value".to_string(),
        type_id: 2, // Str
    }]));
    let ok_variant = VariantDef {
        name: "Ok".to_string(),
        type_id: Some(record_id),
    };
    let err_variant = VariantDef {
        name: "Err".to_string(),
        type_id: Some(0), // Int as a stand-in error type
    };
    registry.register(TypeDef::Union(vec![ok_variant, err_variant]))
}

#[test]
fn test_try_catch_same_type() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // try { "ok" } catch e { "fallback" }
    let expr = Expr::Try {
        body: Box::new(Expr::Literal {
            value: LiteralValue::Str("ok".to_string()),
            span: dummy_span(),
        }),
        binding: Pat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(Expr::Literal {
            value: LiteralValue::Str("fallback".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "try/catch with both arms Str should yield Str (stub fails here)"
    );
}

#[test]
fn test_try_catch_type_mismatch() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // try { 42 } catch e { "oops" }
    let expr = Expr::Try {
        body: Box::new(Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        }),
        binding: Pat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(Expr::Literal {
            value: LiteralValue::Str("oops".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "try/catch arms with mismatched types should error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("mismatch") || err.contains("mismatched"),
        "Expected a type mismatch error, got: {}",
        err
    );
}

#[test]
fn test_try_catch_nested() {
    let mut registry = TypeRegistry::new();
    let env = TypeEnv::new();

    // try { try { "a" } catch e { "b" } } catch e { "c" }
    let inner = Expr::Try {
        body: Box::new(Expr::Literal {
            value: LiteralValue::Str("a".to_string()),
            span: dummy_span(),
        }),
        binding: Pat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(Expr::Literal {
            value: LiteralValue::Str("b".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let expr = Expr::Try {
        body: Box::new(inner),
        binding: Pat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(Expr::Literal {
            value: LiteralValue::Str("c".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "nested try/catch expressions returning Str should yield Str (stub fails here)"
    );
}

#[test]
fn test_throw_typechecks() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register Error: Str -> Int (constructor returning an error value)
    let error_func_id = registry.register(TypeDef::Func(vec![2], 0));
    env.bind("Error".to_string(), error_func_id);

    // throw Error("msg")
    let expr = Expr::Throw {
        expr: Box::new(Expr::Call {
            func: Box::new(Expr::Variable {
                name: "Error".to_string(),
                span: dummy_span(),
            }),
            args: vec![Expr::Literal {
                value: LiteralValue::Str("msg".to_string()),
                span: dummy_span(),
            }],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_ok(),
        "throw Error(\"msg\") should type-check (stub fails here)"
    );
}

#[test]
fn test_throw_in_try_body() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register Error: Str -> Int
    let error_func_id = registry.register(TypeDef::Func(vec![2], 0));
    env.bind("Error".to_string(), error_func_id);

    // try { throw Error("fail") } catch e { "recovered" }
    let expr = Expr::Try {
        body: Box::new(Expr::Throw {
            expr: Box::new(Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "Error".to_string(),
                    span: dummy_span(),
                }),
                args: vec![Expr::Literal {
                    value: LiteralValue::Str("fail".to_string()),
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        binding: Pat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(Expr::Literal {
            value: LiteralValue::Str("recovered".to_string()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "try with throw in body and Str handler should yield Str (stub fails here)"
    );
}

#[test]
fn test_propagate_on_result() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Register Result<{ value: Str }, Int>
    let result_type = register_result_value_type(&mut registry);
    env.bind("result".to_string(), result_type);

    // result?.value  ===  (?result).value
    let expr = Expr::Member {
        obj: Box::new(Expr::Propagate {
            expr: Box::new(Expr::Variable {
                name: "result".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        field: "value".to_string(),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "result?.value where result: Result({{value: Str}}, _) should yield Str (stub fails here)"
    );
}

#[test]
fn test_propagate_on_non_result() {
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // x: Int (not a Result)
    env.bind("x".to_string(), 0);

    // x?.value  ===  (?x).value
    let expr = Expr::Member {
        obj: Box::new(Expr::Propagate {
            expr: Box::new(Expr::Variable {
                name: "x".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        field: "value".to_string(),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert!(
        result.is_err(),
        "x?.value where x is not Result/Option should produce an error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("Propagate") || err.contains("Result") || err.contains("Option"),
        "Expected a propagate-specific error, got: {}",
        err
    );
}

// ===========================================================================
// 23. Optional access `?.` inference (DWARF-72 Chunk B)
//     `obj?.field` where `obj: Option<{ field: T }>` infers to `T` — the
//     OptionalAccess unwraps the Option and accesses the field.
//     `obj?.field` where `obj: { field: T }` (non-optional) infers to `T`
//     without crashing — it treats non-optional as a pass-through.
//     Chained `obj?.field?.value` unwraps nested Options.
//
//     These tests will FAIL to compile because Expr::OptionalAccess does
//     not exist in the HIR yet.
// ===========================================================================

/// Helper: register `Option<{ field: Int }>` in the registry.
///
/// Creates:
///   - record { field: Int } at some TypeId
///   - GenericInstance { base: OPTION_TYPE_ID (5), args: [record_id] }
///
/// Returns the GenericInstance TypeId.
fn register_option_record_field_int(registry: &mut TypeRegistry) -> TypeId {
    let record_id = registry.register(TypeDef::Record(vec![FieldDef {
        name: "field".to_string(),
        type_id: 0, // Int
    }]));
    registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![record_id],
    })
}

/// Helper: register a plain record `{ field: Int }` (non-optional).
fn register_record_field_int(registry: &mut TypeRegistry) -> TypeId {
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "field".to_string(),
        type_id: 0, // Int
    }]))
}

#[test]
fn test_optional_access_on_option_record() {
    // obj: Option<{ field: Int }>
    // obj?.field  →  Int (0)
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    let option_record_id = register_option_record_field_int(&mut registry);
    env.bind("obj".to_string(), option_record_id);

    // obj?.field
    let expr = Expr::OptionalAccess {
        obj: Box::new(Expr::Variable {
            name: "obj".to_string(),
            span: dummy_span(),
        }),
        field: "field".to_string(),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(0),
        "obj?.field where obj: Option<{{ field: Int }}> should yield Int (0)"
    );
}

#[test]
fn test_optional_access_on_non_optional_record() {
    // obj: { field: Int }  (not wrapped in Option)
    // obj?.field  →  Int (0)  — should not crash, treats as pass-through
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    let record_id = register_record_field_int(&mut registry);
    env.bind("obj".to_string(), record_id);

    // obj?.field
    let expr = Expr::OptionalAccess {
        obj: Box::new(Expr::Variable {
            name: "obj".to_string(),
            span: dummy_span(),
        }),
        field: "field".to_string(),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(0),
        "obj?.field where obj: {{ field: Int }} (non-optional) should yield Int (0) without crashing"
    );
}

#[test]
fn test_optional_access_chained_nested_options() {
    // obj: Option<{ field: Option<{ value: Str }> }>
    // obj?.field?.value  →  Str (2)
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    // Inner record: { value: Str }
    let inner_record_id = registry.register(TypeDef::Record(vec![FieldDef {
        name: "value".to_string(),
        type_id: 2, // Str
    }]));
    // Option<{ value: Str }>
    let option_inner_id = registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![inner_record_id],
    });
    // Outer record: { field: Option<{ value: Str }> }
    let outer_record_id = registry.register(TypeDef::Record(vec![FieldDef {
        name: "field".to_string(),
        type_id: option_inner_id,
    }]));
    // Option<{ field: Option<{ value: Str }> }>
    let option_outer_id = registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![outer_record_id],
    });

    env.bind("obj".to_string(), option_outer_id);

    // obj?.field?.value
    let expr = Expr::OptionalAccess {
        obj: Box::new(Expr::OptionalAccess {
            obj: Box::new(Expr::Variable {
                name: "obj".to_string(),
                span: dummy_span(),
            }),
            field: "field".to_string(),
            span: dummy_span(),
        }),
        field: "value".to_string(),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "obj?.field?.value where obj: Option<{{ field: Option<{{ value: Str }}> }}> should yield Str (2)"
    );
}

// ===========================================================================
// 24. Non-null assertion `!` inference (DWARF-72 Chunk C)
//     `x!` where `x: Option<Int>` infers to `Int` — the NonNullAssert
//     unwraps the Option and returns the inner type.
//     `x!` where `x: String | Null` infers to `String` — strips the Null
//     variant from a union type.
//     `result!!` where `result: Option<Option<Int>>` infers to `Int` —
//     double assertion unwraps nested Options.
//
//     These tests will FAIL to compile because Expr::NonNullAssert does
//     not exist in the HIR yet.
// ===========================================================================

/// Helper: register `Option<Int>` in the registry.
///
/// Creates a GenericInstance with base OPTION_TYPE_ID and args [Int].
/// Returns the GenericInstance TypeId.
fn register_option_int(registry: &mut TypeRegistry) -> TypeId {
    registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![0], // Int
    })
}

/// Helper: register `String | Null` union type in the registry.
///
/// Creates a Union with two variants:
///   - String (type_id: 2)
///   - Null (type_id: 4)
///
/// Returns the Union TypeId.
fn register_string_or_null(registry: &mut TypeRegistry) -> TypeId {
    registry.register(TypeDef::Union(vec![
        VariantDef {
            name: "String".to_string(),
            type_id: Some(2), // Str
        },
        VariantDef {
            name: "Null".to_string(),
            type_id: Some(4), // Null
        },
    ]))
}

/// Helper: register `Option<Option<Int>>` in the registry.
///
/// Creates nested GenericInstances.
/// Returns the outer Option TypeId.
fn register_option_option_int(registry: &mut TypeRegistry) -> TypeId {
    let inner_option = registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![0], // Int
    });
    registry.register(TypeDef::GenericInstance {
        base: OPTION_TYPE_ID,
        args: vec![inner_option],
    })
}

#[test]
fn test_non_null_assert_on_option_int() {
    // x: Option<Int>
    // x!  →  Int (0)
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    let option_int_id = register_option_int(&mut registry);
    env.bind("x".to_string(), option_int_id);

    // x!
    let expr = Expr::NonNullAssert {
        expr: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(0),
        "x! where x: Option<Int> should yield Int (0)"
    );
}

#[test]
fn test_non_null_assert_on_string_or_null() {
    // x: String | Null
    // x!  →  String (2)
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    let string_or_null_id = register_string_or_null(&mut registry);
    env.bind("x".to_string(), string_or_null_id);

    // x!
    let expr = Expr::NonNullAssert {
        expr: Box::new(Expr::Variable {
            name: "x".to_string(),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(2),
        "x! where x: String | Null should yield String (2)"
    );
}

#[test]
fn test_non_null_assert_double_nested_option() {
    // result: Option<Option<Int>>
    // result!!  →  Int (0)
    let mut registry = TypeRegistry::new();
    let mut env = TypeEnv::new();

    let option_option_int_id = register_option_option_int(&mut registry);
    env.bind("result".to_string(), option_option_int_id);

    // result!!
    let expr = Expr::NonNullAssert {
        expr: Box::new(Expr::NonNullAssert {
            expr: Box::new(Expr::Variable {
                name: "result".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let result = infer_expr(&expr, &env, &mut registry);
    assert_eq!(
        result,
        Ok(0),
        "result!! where result: Option<Option<Int>> should yield Int (0)"
    );
}
