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
    // Register a function type: Func([Int], Bool) at ID 5
    registry.register(TypeDef::Func(vec![0], 3));
    let mut env = TypeEnv::new();
    env.bind("f".to_string(), 5);

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
    // Register a function type: Func([Int], Bool) at ID 5
    registry.register(TypeDef::Func(vec![0], 3));
    let mut env = TypeEnv::new();
    env.bind("f".to_string(), 5);

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
    env.bind("point".to_string(), 5);

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
                    assert_eq!(
                        inner_args[0], 0,
                        "Innermost element type should be Int"
                    );
                }
                other => {
                    panic!(
                        "Inner array should be GenericInstance, got {:?}",
                        other
                    );
                }
            }
        }
        other => {
            panic!(
                "Outer array should be GenericInstance, got {:?}",
                other
            );
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
    let expr = Expr::Wildcard {
        span: dummy_span(),
    };
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
/// Returns the assigned TypeId (typically 5, after the 5 primitives).
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

    // Register Option<Int> at ID 5
    register_option_type(&mut registry);

    // None — unit variant without a payload
    let expr = Expr::Variant {
        name: "None".to_string(),
        arg: None,
        span: dummy_span(),
    };
    let result = infer_expr(&expr, &env, &mut registry);

    // Red-phase: stub returns Ok(0), but it should be the union type (ID 5+)
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

    // Red-phase: stub returns Ok(0), but it should be the union type (ID 5+)
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
                variants.iter().any(|v| v.name == "Some"
                    && v.type_id == Some(0)),
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

    // Register Option<Int> at ID 5 where Some expects Int payload
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
