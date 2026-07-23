//! The [`EmitterBackend`] trait and related types.
//!
//! Every codegen backend must implement this trait to translate LIR
//! declarations, expressions, patterns, and types into a target-specific
//! output (e.g., JavaScript source, WebAssembly bytes, etc.).

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::error::EmitterError;

/// A backend that can emit code from LIR declarations.
///
/// Each method accepts a reference to a LIR construct and produces
/// a string representation suitable for the target language. The
/// `Output` associated type allows backends to produce richer output
/// (e.g., a list of source files) rather than just a single string.
pub trait EmitterBackend {
    /// The final result of emitting a complete module.
    type Output;

    /// Emit an entire module from its top-level declarations.
    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<Self::Output, EmitterError>;

    /// Emit a single top-level declaration.
    fn emit_decl(&mut self, decl: &LirDecl) -> Result<String, EmitterError>;

    /// Emit an expression.
    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError>;

    /// Emit a pattern.
    fn emit_pat(&mut self, pat: &LirPat) -> Result<String, EmitterError>;

    /// Emit a type annotation.
    fn emit_type(&mut self, ty: &Type) -> Result<String, EmitterError>;

    /// Emit a literal value.
    fn emit_literal(&mut self, lit: &LirLiteral) -> Result<String, EmitterError>;

    /// Emit a binary operator.
    fn emit_binary_op(&mut self, op: &LirBinaryOp) -> Result<String, EmitterError>;

    /// Emit a unary operator.
    fn emit_unary_op(&mut self, op: &LirUnaryOp) -> Result<String, EmitterError>;

    /// Emit a target hint.
    fn emit_target_hint(&mut self, hint: &TargetHint) -> Result<String, EmitterError>;

    /// Emit an effect annotation.
    fn emit_effect(&mut self, effect: &Effect) -> Result<String, EmitterError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::{Effect, LirArm, LirBinaryOp, LirExpr, LirField, LirLiteral, LirParam, LirPat, LirStmt, LirUnaryOp, LirVariant, LirDecl, TargetHint};
    use dwarf_syntax::hir::Type;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Mock backend — delegates to DebugBackend
    // ------------------------------------------------------------------
    //
    // The unit-test MockBackend wraps the production DebugBackend so that
    // all formatting logic lives in one place.  Test code creates a
    // MockBackend via `MockBackend::new()` and uses it exactly as before.

    use crate::debug_backend::DebugBackend;

    struct MockBackend(DebugBackend);

    impl MockBackend {
        fn new() -> Self {
            Self(DebugBackend::new())
        }
    }

    impl EmitterBackend for MockBackend {
        type Output = String;

        fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
            self.0.emit_module(decls)
        }

        fn emit_decl(&mut self, decl: &LirDecl) -> Result<String, EmitterError> {
            self.0.emit_decl(decl)
        }

        fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
            self.0.emit_expr(expr)
        }

        fn emit_pat(&mut self, pat: &LirPat) -> Result<String, EmitterError> {
            self.0.emit_pat(pat)
        }

        fn emit_type(&mut self, ty: &Type) -> Result<String, EmitterError> {
            self.0.emit_type(ty)
        }

        fn emit_literal(&mut self, lit: &LirLiteral) -> Result<String, EmitterError> {
            self.0.emit_literal(lit)
        }

        fn emit_binary_op(&mut self, op: &LirBinaryOp) -> Result<String, EmitterError> {
            self.0.emit_binary_op(op)
        }

        fn emit_unary_op(&mut self, op: &LirUnaryOp) -> Result<String, EmitterError> {
            self.0.emit_unary_op(op)
        }

        fn emit_target_hint(&mut self, hint: &TargetHint) -> Result<String, EmitterError> {
            self.0.emit_target_hint(hint)
        }

        fn emit_effect(&mut self, effect: &Effect) -> Result<String, EmitterError> {
            self.0.emit_effect(effect)
        }
    }

    // ------------------------------------------------------------------
    // Helper: span factory
    // ------------------------------------------------------------------

    fn s() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint_none() -> TargetHint {
        TargetHint::None
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_module
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_module_empty() {
        let mut backend = MockBackend::new();
        let result = backend.emit_module(&[]).unwrap();
        assert_eq!(result, "", "empty module should produce empty string");
    }

    #[test]
    fn test_emit_module_single_function() {
        let mut backend = MockBackend::new();
        let decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_module(&[decl]).unwrap();
        assert!(!result.is_empty(), "module with a function should not be empty");
        assert!(result.contains("fn main"), "should contain 'fn main'");
        assert!(result.contains("pure"), "should contain effect 'pure'");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_decl
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_decl_function_private() {
        let mut backend = MockBackend::new();
        let decl = LirDecl::Function {
            name: "helper".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Impure,
            hint: TargetHint::None,
            is_pub: false,
            span: s(),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(!result.starts_with("pub"), "private fn should not start with 'pub'");
        assert!(result.contains("fn helper"), "should contain 'fn helper'");
        assert!(result.contains("impure"), "should contain 'impure'");
    }

    #[test]
    fn test_emit_decl_record() {
        let mut backend = MockBackend::new();
        let decl = LirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                LirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                LirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(result.contains("record Point"));
        assert!(result.contains("x: Int"));
        assert!(result.contains("y: Int"));
    }

    #[test]
    fn test_emit_decl_union() {
        let mut backend = MockBackend::new();
        let decl = LirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![
                LirVariant {
                    name: "Some".into(),
                    arg: Some(Type::Named("Int".into())),
                },
                LirVariant {
                    name: "None".into(),
                    arg: None,
                },
            ],
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(result.contains("union Option"));
        assert!(result.contains("Some(Int)"));
        assert!(result.contains("None"));
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_expr — every LirExpr variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_expr_literal() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "42");
    }

    #[test]
    fn test_emit_expr_variable() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Variable {
            name: "x".into(),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "var(x)");
    }

    #[test]
    fn test_emit_expr_call() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: hint_none(),
                span: s(),
            }),
            args: vec![LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: hint_none(),
                span: s(),
            }],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "call(var(f), [1])");
    }

    #[test]
    fn test_emit_expr_member() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Member {
            obj: Box::new(LirExpr::Variable {
                name: "obj".into(),
                hint: hint_none(),
                span: s(),
            }),
            field: "attr".into(),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "member(var(obj), attr)");
    }

    #[test]
    fn test_emit_expr_if_with_else() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::If {
            cond: Box::new(LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: hint_none(),
                span: s(),
            }),
            then: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: hint_none(),
                span: s(),
            }),
            else_: Some(Box::new(LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: hint_none(),
                span: s(),
            })),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "if(true, 1, 2)"
        );
    }

    #[test]
    fn test_emit_expr_if_no_else() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::If {
            cond: Box::new(LirExpr::Literal {
                value: LirLiteral::Bool(false),
                hint: hint_none(),
                span: s(),
            }),
            then: Box::new(LirExpr::Literal {
                value: LirLiteral::Null,
                hint: hint_none(),
                span: s(),
            }),
            else_: None,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "if(false, null)");
    }

    #[test]
    fn test_emit_expr_match() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Match {
            expr: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            arms: vec![LirArm {
                pattern: LirPat::Wildcard,
                guard: None,
                body: LirExpr::Literal {
                    value: LirLiteral::Int(0),
                    hint: hint_none(),
                    span: s(),
                },
            }],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "match(var(x), [_ => 0])"
        );
    }

    #[test]
    fn test_emit_expr_match_with_guard() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Match {
            expr: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            arms: vec![LirArm {
                pattern: LirPat::Variable("n".into()),
                guard: Some(LirExpr::Literal {
                    value: LirLiteral::Bool(true),
                    hint: hint_none(),
                    span: s(),
                }),
                body: LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: hint_none(),
                    span: s(),
                },
            }],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "match(var(x), [n if true => 1])"
        );
    }

    #[test]
    fn test_emit_expr_block() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Block {
            stmts: vec![
                LirStmt::Let {
                    pat: LirPat::Variable("y".into()),
                    value: LirExpr::Literal {
                        value: LirLiteral::Int(2),
                        hint: hint_none(),
                        span: s(),
                    },
                },
                LirStmt::Expr(LirExpr::Literal {
                    value: LirLiteral::Int(3),
                    hint: hint_none(),
                    span: s(),
                }),
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "block([let y = 2; 3])"
        );
    }

    #[test]
    fn test_emit_expr_assign() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Assign {
            target: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            value: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "assign(var(x), 42)");
    }

    #[test]
    fn test_emit_expr_lambda() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Lambda {
            params: vec![
                LirParam {
                    name: "a".into(),
                    type_: None,
                },
                LirParam {
                    name: "b".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            body: Box::new(LirExpr::Variable {
                name: "a".into(),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "lambda((a, b: Int), var(a))"
        );
    }

    #[test]
    fn test_emit_expr_record() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Record {
            fields: vec![
                (
                    "x".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Int(1),
                        hint: hint_none(),
                        span: s(),
                    },
                ),
                (
                    "y".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Int(2),
                        hint: hint_none(),
                        span: s(),
                    },
                ),
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "record({x: 1, y: 2})"
        );
    }

    #[test]
    fn test_emit_expr_variant_with_arg() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: hint_none(),
                span: s(),
            })),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "variant(Some, 1)");
    }

    #[test]
    fn test_emit_expr_variant_no_arg() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Variant {
            name: "None".into(),
            arg: None,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "variant(None)");
    }

    #[test]
    fn test_emit_expr_array() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Array {
            items: vec![
                LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: hint_none(),
                    span: s(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(2),
                    hint: hint_none(),
                    span: s(),
                },
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "array([1, 2])");
    }

    #[test]
    fn test_emit_expr_binary() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: hint_none(),
                span: s(),
            }),
            rhs: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "binary(1, +, 2)");
    }

    #[test]
    fn test_emit_expr_unary() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Unary {
            op: LirUnaryOp::Neg,
            expr: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(5),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "unary(-, 5)");
    }

    #[test]
    fn test_emit_expr_wildcard() {
        let mut backend = MockBackend::new();
        let expr = LirExpr::Wildcard {
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "wildcard");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_pat — every LirPat variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_pat_wildcard() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_pat(&LirPat::Wildcard).unwrap(), "_");
    }

    #[test]
    fn test_emit_pat_literal() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_pat(&LirPat::Literal(LirLiteral::Int(7))).unwrap(), "7");
    }

    #[test]
    fn test_emit_pat_variable() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_pat(&LirPat::Variable("binding".into())).unwrap(), "binding");
    }

    #[test]
    fn test_emit_pat_variant_with_arg() {
        let mut backend = MockBackend::new();
        let pat = LirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirPat::Variable("inner".into()))),
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "Some(inner)");
    }

    #[test]
    fn test_emit_pat_variant_no_arg() {
        let mut backend = MockBackend::new();
        let pat = LirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "None");
    }

    #[test]
    fn test_emit_pat_record_no_rest() {
        let mut backend = MockBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _ }");
    }

    #[test]
    fn test_emit_pat_record_with_rest() {
        let mut backend = MockBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: true,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _, .. }");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_type — every Type variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_type_named() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_type(&Type::Named("Int".into())).unwrap(), "Int");
    }

    #[test]
    fn test_emit_type_record() {
        let mut backend = MockBackend::new();
        let ty = Type::Record(vec![
            ("x".into(), Box::new(Type::Named("Int".into()))),
            ("y".into(), Box::new(Type::Named("Int".into()))),
        ]);
        assert_eq!(backend.emit_type(&ty).unwrap(), "record(x: Int, y: Int)");
    }

    #[test]
    fn test_emit_type_union() {
        let mut backend = MockBackend::new();
        let ty = Type::Union(vec![
            Type::Named("Ok".into()),
            Type::Named("Err".into()),
        ]);
        assert_eq!(backend.emit_type(&ty).unwrap(), "union(Ok | Err)");
    }

    #[test]
    fn test_emit_type_func() {
        let mut backend = MockBackend::new();
        let ty = Type::Func {
            params: vec![Type::Named("Int".into()), Type::Named("String".into())],
            return_: Box::new(Type::Named("Bool".into())),
        };
        assert_eq!(backend.emit_type(&ty).unwrap(), "(Int, String) -> Bool");
    }

    #[test]
    fn test_emit_type_generic() {
        let mut backend = MockBackend::new();
        let ty = Type::Generic {
            base: "Array".into(),
            args: vec![Type::Named("Int".into())],
        };
        assert_eq!(backend.emit_type(&ty).unwrap(), "Array<Int>");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_literal — every LirLiteral variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_literal_int() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Int(42)).unwrap(), "42");
    }

    #[test]
    fn test_emit_literal_float() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Float(3.5)).unwrap(), "3.5");
    }

    #[test]
    fn test_emit_literal_str() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Str("hello".into())).unwrap(), "\"hello\"");
    }

    #[test]
    fn test_emit_literal_bool() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Bool(true)).unwrap(), "true");
        assert_eq!(backend.emit_literal(&LirLiteral::Bool(false)).unwrap(), "false");
    }

    #[test]
    fn test_emit_literal_null() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Null).unwrap(), "null");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_binary_op — every operator
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_binary_op_arithmetic() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Add).unwrap(), "+");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Sub).unwrap(), "-");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Mul).unwrap(), "*");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Div).unwrap(), "/");
    }

    #[test]
    fn test_emit_binary_op_comparison() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Eq).unwrap(), "==");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ne).unwrap(), "!=");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Lt).unwrap(), "<");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Gt).unwrap(), ">");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Le).unwrap(), "<=");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ge).unwrap(), ">=");
    }

    #[test]
    fn test_emit_binary_op_logical() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::And).unwrap(), "&&");
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Or).unwrap(), "||");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_unary_op — every operator
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_unary_op_neg() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Neg).unwrap(), "-");
    }

    #[test]
    fn test_emit_unary_op_not() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Not).unwrap(), "!");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_target_hint — every variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_target_hint_none() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::None).unwrap(), "none");
    }

    #[test]
    fn test_emit_target_hint_async() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::Async).unwrap(), "async");
    }

    #[test]
    fn test_emit_target_hint_optional() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::Optional).unwrap(), "optional");
    }

    #[test]
    fn test_emit_target_hint_result() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::Result).unwrap(), "result");
    }

    #[test]
    fn test_emit_target_hint_react_component() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::ReactComponent).unwrap(), "react_component");
    }

    // ------------------------------------------------------------------
    // Unit tests: emit_effect — every variant
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_effect_pure() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Pure).unwrap(), "pure");
    }

    #[test]
    fn test_emit_effect_async() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Async).unwrap(), "async");
    }

    #[test]
    fn test_emit_effect_impure() {
        let mut backend = MockBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Impure).unwrap(), "impure");
    }

    // ------------------------------------------------------------------
    // Trait object safety — verify the trait can be used dynamically
    // ------------------------------------------------------------------

    #[test]
    fn test_trait_object_safe() {
        // This test verifies the trait can be used as a trait object.
        // It should compile because none of the methods take Self by value
        // or have generic parameters.
        fn use_dyn(_backend: &mut dyn EmitterBackend<Output = String>) {}
        let mut backend = MockBackend::new();
        use_dyn(&mut backend);
    }

    // ------------------------------------------------------------------
    // Multi-decl module emission
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_module_multiple_decls() {
        let mut backend = MockBackend::new();
        let decls = vec![
            LirDecl::RecordDef {
                name: "Point".into(),
                fields: vec![
                    LirField {
                        name: "x".into(),
                        type_: Type::Named("Int".into()),
                    },
                ],
                is_pub: true,
                span: s(),
            },
            LirDecl::Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                body: LirExpr::Literal {
                    value: LirLiteral::Int(0),
                    hint: hint_none(),
                    span: s(),
                },
                effect: Effect::Pure,
                hint: TargetHint::None,
                is_pub: true,
                span: s(),
            },
        ];
        let result = backend.emit_module(&decls).unwrap();
        assert!(result.contains("record Point"));
        assert!(result.contains("fn main"));
        // Check that both declarations appear and are separated by a newline
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2, "two decls should produce two lines");
    }
}
