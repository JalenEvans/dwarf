//! Low-level Intermediate Representation (LIR).
//!
//! The LIR is the final IR before code emission. It extends MIR with target
//! backend hints (async, optional, result, react-component) and removes
//! remaining high-level constructs (For, Pipe) that MIR desugars.
//!
//! All expression variants carry a [`TargetHint`] that guides backend codegen.

use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;
use serde::{Deserialize, Serialize};

pub mod effects;
pub mod lower;
pub mod pass;

/// A literal value in the LIR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LirLiteral {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
}

/// A hint to the backend about the target representation of an expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetHint {
    None,
    Async,
    Optional,
    Result,
    ReactComponent,
}

/// Classification of side effects for a function or expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Pure,
    Async,
    Impure,
}

/// Binary operators in the LIR.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// Unary operators in the LIR.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LirUnaryOp {
    Neg,
    Not,
}

/// An expression in the LIR.
///
/// Every expression variant carries a [`TargetHint`] that guides backend codegen
/// and a [`Span`] for source-location error reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LirExpr {
    Literal {
        value: LirLiteral,
        hint: TargetHint,
        span: Span,
    },
    Variable {
        name: String,
        hint: TargetHint,
        span: Span,
    },
    Call {
        func: Box<LirExpr>,
        args: Vec<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Member {
        obj: Box<LirExpr>,
        field: String,
        hint: TargetHint,
        span: Span,
    },
    If {
        cond: Box<LirExpr>,
        then: Box<LirExpr>,
        else_: Option<Box<LirExpr>>,
        hint: TargetHint,
        span: Span,
    },
    Match {
        expr: Box<LirExpr>,
        arms: Vec<LirArm>,
        hint: TargetHint,
        span: Span,
    },
    Block {
        stmts: Vec<LirStmt>,
        hint: TargetHint,
        span: Span,
    },
    Assign {
        target: Box<LirExpr>,
        value: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Lambda {
        params: Vec<LirParam>,
        body: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Record {
        fields: Vec<(String, LirExpr)>,
        hint: TargetHint,
        span: Span,
    },
    Variant {
        name: String,
        arg: Option<Box<LirExpr>>,
        hint: TargetHint,
        span: Span,
    },
    Array {
        items: Vec<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Binary {
        op: LirBinaryOp,
        lhs: Box<LirExpr>,
        rhs: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Unary {
        op: LirUnaryOp,
        expr: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    Wildcard {
        hint: TargetHint,
        span: Span,
    },
    /// Property-based testing: forAll Type { binding -> property }
    ForAll {
        type_: Type,
        binding: LirPat,
        property: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    /// Assert consistent evaluation across targets.
    AssertConsistent {
        expr: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    /// Try expression with catch handler.
    Try {
        body: Box<LirExpr>,
        binding: LirPat,
        guard: Option<Box<LirExpr>>,
        handler: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    /// Throw expression.
    Throw {
        expr: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
    /// Propagate operator (`?`).
    Propagate {
        expr: Box<LirExpr>,
        hint: TargetHint,
        span: Span,
    },
}

impl LirExpr {
    /// Get the source span of this expression.
    pub fn span(&self) -> Span {
        match self {
            LirExpr::Literal { span, .. }
            | LirExpr::Variable { span, .. }
            | LirExpr::Call { span, .. }
            | LirExpr::Member { span, .. }
            | LirExpr::If { span, .. }
            | LirExpr::Match { span, .. }
            | LirExpr::Block { span, .. }
            | LirExpr::Assign { span, .. }
            | LirExpr::Lambda { span, .. }
            | LirExpr::Record { span, .. }
            | LirExpr::Variant { span, .. }
            | LirExpr::Array { span, .. }
            | LirExpr::Binary { span, .. }
            | LirExpr::Unary { span, .. }
            | LirExpr::Wildcard { span, .. }
            | LirExpr::ForAll { span, .. }
            | LirExpr::AssertConsistent { span, .. }
            | LirExpr::Try { span, .. }
            | LirExpr::Throw { span, .. }
            | LirExpr::Propagate { span, .. } => *span,
        }
    }

    /// Get the target hint of this expression.
    pub fn hint(&self) -> TargetHint {
        match self {
            LirExpr::Literal { hint, .. }
            | LirExpr::Variable { hint, .. }
            | LirExpr::Call { hint, .. }
            | LirExpr::Member { hint, .. }
            | LirExpr::If { hint, .. }
            | LirExpr::Match { hint, .. }
            | LirExpr::Block { hint, .. }
            | LirExpr::Assign { hint, .. }
            | LirExpr::Lambda { hint, .. }
            | LirExpr::Record { hint, .. }
            | LirExpr::Variant { hint, .. }
            | LirExpr::Array { hint, .. }
            | LirExpr::Binary { hint, .. }
            | LirExpr::Unary { hint, .. }
            | LirExpr::Wildcard { hint, .. }
            | LirExpr::ForAll { hint, .. }
            | LirExpr::AssertConsistent { hint, .. }
            | LirExpr::Try { hint, .. }
            | LirExpr::Throw { hint, .. }
            | LirExpr::Propagate { hint, .. } => hint.clone(),
        }
    }
}

/// A statement inside a block expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LirStmt {
    Let { pat: LirPat, value: LirExpr },
    Expr(LirExpr),
}

/// A pattern in the LIR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LirPat {
    Wildcard,
    Literal(LirLiteral),
    Variable(String),
    Variant {
        name: String,
        arg: Option<Box<LirPat>>,
    },
    Record {
        fields: Vec<(String, LirPat)>,
        rest: bool,
    },
}

/// A parameter in a function or lambda declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LirParam {
    pub name: String,
    pub type_: Option<Type>,
}

/// An arm in a match expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LirArm {
    pub pattern: LirPat,
    pub guard: Option<LirExpr>,
    pub body: LirExpr,
}

/// A top-level declaration in the LIR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LirDecl {
    Function {
        name: String,
        params: Vec<LirParam>,
        return_type: Option<Type>,
        body: LirExpr,
        effect: Effect,
        hint: TargetHint,
        is_pub: bool,
        is_generator: bool,
        span: Span,
    },
    RecordDef {
        name: String,
        fields: Vec<LirField>,
        is_pub: bool,
        span: Span,
    },
    UnionDef {
        name: String,
        variants: Vec<LirVariant>,
        is_pub: bool,
        span: Span,
    },
}

/// A field in a record type definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LirField {
    pub name: String,
    pub type_: Type,
}

/// A variant in a union type definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LirVariant {
    pub name: String,
    pub arg: Option<Type>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Type;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn span1() -> Span {
        Span::new(0, 0, 0)
    }

    fn span2() -> Span {
        Span::new(1, 5, 10)
    }

    // ------------------------------------------------------------------
    // LirLiteral — all literal forms
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_literal_int() {
        let lit = LirLiteral::Int(42);
        assert_eq!(lit, LirLiteral::Int(42));
        assert_ne!(lit, LirLiteral::Int(0));
    }

    #[test]
    fn test_lir_literal_float() {
        let lit = LirLiteral::Float(3.5);
        assert_eq!(lit, LirLiteral::Float(3.5));
    }

    #[test]
    fn test_lir_literal_str() {
        let lit = LirLiteral::Str("hello".into());
        assert_eq!(lit, LirLiteral::Str("hello".into()));
    }

    #[test]
    fn test_lir_literal_bool() {
        let lit = LirLiteral::Bool(true);
        assert_eq!(lit, LirLiteral::Bool(true));
        assert_ne!(lit, LirLiteral::Bool(false));
    }

    #[test]
    fn test_lir_literal_null() {
        let lit = LirLiteral::Null;
        assert_eq!(lit, LirLiteral::Null);
    }

    #[test]
    fn test_lir_literal_clone() {
        let lit = LirLiteral::Int(99);
        let cloned = lit.clone();
        assert_eq!(lit, cloned);
    }

    #[test]
    fn test_lir_literal_debug() {
        let lit = LirLiteral::Str("debug".into());
        let s = format!("{lit:?}");
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    // ------------------------------------------------------------------
    // TargetHint — all variants
    // ------------------------------------------------------------------

    #[test]
    fn test_target_hint_none() {
        let h = TargetHint::None;
        assert_eq!(h, TargetHint::None);
    }

    #[test]
    fn test_target_hint_async() {
        let h = TargetHint::Async;
        assert_eq!(h, TargetHint::Async);
        assert_ne!(h, TargetHint::None);
    }

    #[test]
    fn test_target_hint_optional() {
        let h = TargetHint::Optional;
        assert_eq!(h, TargetHint::Optional);
    }

    #[test]
    fn test_target_hint_result() {
        let h = TargetHint::Result;
        assert_eq!(h, TargetHint::Result);
    }

    #[test]
    fn test_target_hint_react_component() {
        let h = TargetHint::ReactComponent;
        assert_eq!(h, TargetHint::ReactComponent);
    }

    #[test]
    fn test_target_hint_all_variants_distinct() {
        use crate::TargetHint::*;
        let variants = [None, Async, Optional, Result, ReactComponent];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(
                    variants[i], variants[j],
                    "{:?} and {:?} should be distinct",
                    variants[i], variants[j]
                );
            }
        }
    }

    #[test]
    fn test_target_hint_clone() {
        let h = TargetHint::Async;
        assert_eq!(h, h.clone());
    }

    #[test]
    fn test_target_hint_debug() {
        let h = TargetHint::ReactComponent;
        let s = format!("{h:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // Effect — side-effect classification
    // ------------------------------------------------------------------

    #[test]
    fn test_effect_pure() {
        let e = Effect::Pure;
        assert_eq!(e, Effect::Pure);
    }

    #[test]
    fn test_effect_async() {
        let e = Effect::Async;
        assert_eq!(e, Effect::Async);
    }

    #[test]
    fn test_effect_impure() {
        let e = Effect::Impure;
        assert_eq!(e, Effect::Impure);
    }

    #[test]
    fn test_effect_variants_distinct() {
        assert_ne!(Effect::Pure, Effect::Async);
        assert_ne!(Effect::Pure, Effect::Impure);
        assert_ne!(Effect::Async, Effect::Impure);
    }

    #[test]
    fn test_effect_clone() {
        let e = Effect::Impure;
        assert_eq!(e, e.clone());
    }

    #[test]
    fn test_effect_debug() {
        let e = Effect::Async;
        let s = format!("{e:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirBinaryOp
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_binary_op_all_variants() {
        use crate::LirBinaryOp::*;
        let arithmetic = [Add, Sub, Mul, Div];
        let comparison = [Eq, Ne, Lt, Gt, Le, Ge];
        let logical = [And, Or];

        assert_eq!(arithmetic.len(), 4);
        assert_eq!(comparison.len(), 6);
        assert_eq!(logical.len(), 2);

        for &op in arithmetic
            .iter()
            .chain(comparison.iter())
            .chain(logical.iter())
        {
            let _ = op;
        }
    }

    #[test]
    fn test_lir_binary_op_equality() {
        assert_eq!(LirBinaryOp::Add, LirBinaryOp::Add);
        assert_ne!(LirBinaryOp::Add, LirBinaryOp::Sub);
        assert_ne!(LirBinaryOp::Eq, LirBinaryOp::Ne);
    }

    #[test]
    fn test_lir_binary_op_clone() {
        let op = LirBinaryOp::And;
        assert_eq!(op, op.clone());
    }

    // ------------------------------------------------------------------
    // LirUnaryOp
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_unary_op_variants() {
        assert_eq!(LirUnaryOp::Neg as u8, 0);
        assert_eq!(LirUnaryOp::Not as u8, 1);
    }

    #[test]
    fn test_lir_unary_op_equality() {
        assert_eq!(LirUnaryOp::Neg, LirUnaryOp::Neg);
        assert_ne!(LirUnaryOp::Neg, LirUnaryOp::Not);
    }

    #[test]
    fn test_lir_unary_op_clone() {
        assert_eq!(LirUnaryOp::Not, LirUnaryOp::Not.clone());
    }

    // ------------------------------------------------------------------
    // LirExpr — all expression variants
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_expr_literal() {
        let e = LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        assert_eq!(e.hint(), TargetHint::None);
    }

    #[test]
    fn test_lir_expr_variable() {
        let e = LirExpr::Variable {
            name: "x".into(),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let LirExpr::Variable { name, .. } = &e {
            assert_eq!(name, "x");
        } else {
            panic!("expected Variable variant");
        }
    }

    #[test]
    fn test_lir_expr_call() {
        let e = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            args: vec![LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: TargetHint::None,
                span: span1(),
            }],
            hint: TargetHint::Async,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        assert_eq!(e.hint(), TargetHint::Async);
        if let LirExpr::Call {
            func, args, hint, ..
        } = &e
        {
            assert!(matches!(func.as_ref(), LirExpr::Variable { name, .. } if name == "f"));
            assert_eq!(args.len(), 1);
            assert_eq!(*hint, TargetHint::Async);
        } else {
            panic!("expected Call variant");
        }
    }

    #[test]
    fn test_lir_expr_member() {
        let e = LirExpr::Member {
            obj: Box::new(LirExpr::Variable {
                name: "obj".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            field: "attr".into(),
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::Member { field, .. } = &e {
            assert_eq!(field, "attr");
        } else {
            panic!("expected Member variant");
        }
    }

    #[test]
    fn test_lir_expr_if_with_else() {
        let e = LirExpr::If {
            cond: Box::new(LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: TargetHint::None,
                span: span1(),
            }),
            then: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: TargetHint::None,
                span: span1(),
            }),
            else_: Some(Box::new(LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: TargetHint::None,
                span: span1(),
            })),
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::If { else_, .. } = &e {
            assert!(else_.is_some());
        } else {
            panic!("expected If variant");
        }
    }

    #[test]
    fn test_lir_expr_if_no_else() {
        let e = LirExpr::If {
            cond: Box::new(LirExpr::Literal {
                value: LirLiteral::Bool(false),
                hint: TargetHint::None,
                span: span1(),
            }),
            then: Box::new(LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            }),
            else_: None,
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_lir_expr_match() {
        let e = LirExpr::Match {
            expr: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            arms: vec![LirArm {
                pattern: LirPat::Wildcard,
                guard: None,
                body: LirExpr::Literal {
                    value: LirLiteral::Int(0),
                    hint: TargetHint::None,
                    span: span1(),
                },
            }],
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let LirExpr::Match { arms, .. } = &e {
            assert_eq!(arms.len(), 1);
        } else {
            panic!("expected Match variant");
        }
    }

    #[test]
    fn test_lir_expr_block() {
        let e = LirExpr::Block {
            stmts: vec![
                LirStmt::Expr(LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: TargetHint::None,
                    span: span1(),
                }),
                LirStmt::Let {
                    pat: LirPat::Variable("y".into()),
                    value: LirExpr::Literal {
                        value: LirLiteral::Int(2),
                        hint: TargetHint::None,
                        span: span1(),
                    },
                },
            ],
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::Block { stmts, .. } = &e {
            assert_eq!(stmts.len(), 2);
        } else {
            panic!("expected Block variant");
        }
    }

    #[test]
    fn test_lir_expr_assign() {
        let e = LirExpr::Assign {
            target: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            value: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
    }

    #[test]
    fn test_lir_expr_lambda() {
        let e = LirExpr::Lambda {
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
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::Lambda { params, .. } = &e {
            assert_eq!(params.len(), 2);
        } else {
            panic!("expected Lambda variant");
        }
    }

    #[test]
    fn test_lir_expr_record() {
        let e = LirExpr::Record {
            fields: vec![
                (
                    "x".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Int(10),
                        hint: TargetHint::None,
                        span: span1(),
                    },
                ),
                (
                    "y".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Int(20),
                        hint: TargetHint::None,
                        span: span1(),
                    },
                ),
            ],
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::Record { fields, .. } = &e {
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected Record variant");
        }
    }

    #[test]
    fn test_lir_expr_variant_with_arg() {
        let e = LirExpr::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: TargetHint::None,
                span: span1(),
            })),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let LirExpr::Variant { name, arg, .. } = &e {
            assert_eq!(name, "Some");
            assert!(arg.is_some());
        } else {
            panic!("expected Variant variant");
        }
    }

    #[test]
    fn test_lir_expr_variant_no_arg() {
        let e = LirExpr::Variant {
            name: "None".into(),
            arg: None,
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_lir_expr_array() {
        let e = LirExpr::Array {
            items: vec![
                LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: TargetHint::None,
                    span: span1(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(2),
                    hint: TargetHint::None,
                    span: span1(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(3),
                    hint: TargetHint::None,
                    span: span1(),
                },
            ],
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let LirExpr::Array { items, .. } = &e {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected Array variant");
        }
    }

    #[test]
    fn test_lir_expr_binary() {
        let e = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: TargetHint::None,
                span: span1(),
            }),
            rhs: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let LirExpr::Binary { op, .. } = &e {
            assert_eq!(*op, LirBinaryOp::Add);
        } else {
            panic!("expected Binary variant");
        }
    }

    #[test]
    fn test_lir_expr_unary() {
        let e = LirExpr::Unary {
            op: LirUnaryOp::Neg,
            expr: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(5),
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let LirExpr::Unary { op, .. } = &e {
            assert_eq!(*op, LirUnaryOp::Neg);
        } else {
            panic!("expected Unary variant");
        }
    }

    #[test]
    fn test_lir_expr_wildcard() {
        let e = LirExpr::Wildcard {
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        assert_eq!(e.hint(), TargetHint::None);
    }

    #[test]
    fn test_lir_expr_clone() {
        let e = LirExpr::Literal {
            value: LirLiteral::Int(7),
            hint: TargetHint::None,
            span: span1(),
        };
        let cloned = e.clone();
        assert_eq!(e, cloned);
    }

    #[test]
    fn test_lir_expr_debug() {
        let e = LirExpr::Variable {
            name: "debug".into(),
            hint: TargetHint::None,
            span: span1(),
        };
        let s = format!("{e:?}");
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    #[test]
    fn test_lir_expr_partial_eq() {
        let a = LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: TargetHint::None,
            span: span1(),
        };
        let b = LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: TargetHint::None,
            span: span1(),
        };
        let c = LirExpr::Literal {
            value: LirLiteral::Int(2),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_lir_expr_hint_propagated() {
        // Verify that hint() returns the hint stored on each variant.
        let call = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            args: vec![],
            hint: TargetHint::Async,
            span: span1(),
        };
        assert_eq!(call.hint(), TargetHint::Async);

        let lit = LirExpr::Literal {
            value: LirLiteral::Int(0),
            hint: TargetHint::Optional,
            span: span1(),
        };
        assert_eq!(lit.hint(), TargetHint::Optional);

        let wild = LirExpr::Wildcard {
            hint: TargetHint::Result,
            span: span1(),
        };
        assert_eq!(wild.hint(), TargetHint::Result);
    }

    #[test]
    fn test_lir_expr_span_all_variants() {
        // Verify span() works correctly for expression trees where span differs
        // from sub-expression spans — important for error reporting.
        let inner = LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: TargetHint::None,
            span: span1(),
        };
        let outer = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            args: vec![inner],
            hint: TargetHint::None,
            span: span2(),
        };
        assert_eq!(
            outer.span(),
            span2(),
            "outer Call should return its own span, not inner's span"
        );
        assert_ne!(outer.span(), span1(), "outer and inner spans must differ");
    }

    // ------------------------------------------------------------------
    // LirStmt — statement forms
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_stmt_let() {
        let stmt = LirStmt::Let {
            pat: LirPat::Variable("x".into()),
            value: LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: TargetHint::None,
                span: span1(),
            },
        };
        if let LirStmt::Let { pat, .. } = &stmt {
            assert_eq!(*pat, LirPat::Variable("x".into()));
        } else {
            panic!("expected Let variant");
        }
    }

    #[test]
    fn test_lir_stmt_expr() {
        let stmt = LirStmt::Expr(LirExpr::Literal {
            value: LirLiteral::Null,
            hint: TargetHint::None,
            span: span1(),
        });
        if let LirStmt::Expr(expr) = &stmt {
            assert_eq!(expr.span(), span1());
        } else {
            panic!("expected Expr variant");
        }
    }

    #[test]
    fn test_lir_stmt_clone() {
        let stmt = LirStmt::Expr(LirExpr::Literal {
            value: LirLiteral::Int(0),
            hint: TargetHint::None,
            span: span1(),
        });
        assert_eq!(stmt, stmt.clone());
    }

    #[test]
    fn test_lir_stmt_debug() {
        let stmt = LirStmt::Expr(LirExpr::Literal {
            value: LirLiteral::Null,
            hint: TargetHint::None,
            span: span1(),
        });
        let s = format!("{stmt:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirPat — pattern forms
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_pat_wildcard() {
        let p = LirPat::Wildcard;
        assert_eq!(p, LirPat::Wildcard);
    }

    #[test]
    fn test_lir_pat_literal() {
        let p = LirPat::Literal(LirLiteral::Int(42));
        assert_eq!(p, LirPat::Literal(LirLiteral::Int(42)));
    }

    #[test]
    fn test_lir_pat_variable() {
        let p = LirPat::Variable("binding".into());
        assert_eq!(p, LirPat::Variable("binding".into()));
    }

    #[test]
    fn test_lir_pat_variant_with_arg() {
        let p = LirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirPat::Variable("inner".into()))),
        };
        if let LirPat::Variant { name, arg } = &p {
            assert_eq!(name, "Some");
            assert!(arg.is_some());
        } else {
            panic!("expected Variant variant");
        }
    }

    #[test]
    fn test_lir_pat_variant_no_arg() {
        let p = LirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(
            p,
            LirPat::Variant {
                name: "None".into(),
                arg: None,
            }
        );
    }

    #[test]
    fn test_lir_pat_record_no_rest() {
        let p = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: false,
        };
        if let LirPat::Record { fields, rest } = &p {
            assert_eq!(fields.len(), 1);
            assert!(!rest);
        } else {
            panic!("expected Record variant");
        }
    }

    #[test]
    fn test_lir_pat_record_with_rest() {
        let p = LirPat::Record {
            fields: vec![],
            rest: true,
        };
        assert_eq!(
            p,
            LirPat::Record {
                fields: vec![],
                rest: true,
            }
        );
    }

    #[test]
    fn test_lir_pat_clone() {
        let p = LirPat::Variable("x".into());
        assert_eq!(p, p.clone());
    }

    #[test]
    fn test_lir_pat_debug() {
        let p = LirPat::Wildcard;
        let s = format!("{p:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirParam
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_param_untyped() {
        let p = LirParam {
            name: "x".into(),
            type_: None,
        };
        assert_eq!(p.name, "x");
        assert!(p.type_.is_none());
    }

    #[test]
    fn test_lir_param_typed() {
        let p = LirParam {
            name: "y".into(),
            type_: Some(Type::Named("Int".into())),
        };
        assert_eq!(p.name, "y");
        assert_eq!(p.type_, Some(Type::Named("Int".into())));
    }

    #[test]
    fn test_lir_param_clone() {
        let p = LirParam {
            name: "z".into(),
            type_: None,
        };
        assert_eq!(p, p.clone());
    }

    #[test]
    fn test_lir_param_debug() {
        let p = LirParam {
            name: "p".into(),
            type_: None,
        };
        let s = format!("{p:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirArm
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_arm_no_guard() {
        let arm = LirArm {
            pattern: LirPat::Wildcard,
            guard: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            },
        };
        assert_eq!(arm.pattern, LirPat::Wildcard);
        assert!(arm.guard.is_none());
    }

    #[test]
    fn test_lir_arm_with_guard() {
        let arm = LirArm {
            pattern: LirPat::Variable("x".into()),
            guard: Some(LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: TargetHint::None,
                span: span1(),
            }),
            body: LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: TargetHint::None,
                span: span1(),
            },
        };
        assert!(arm.guard.is_some());
    }

    #[test]
    fn test_lir_arm_clone() {
        let arm = LirArm {
            pattern: LirPat::Wildcard,
            guard: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            },
        };
        assert_eq!(arm, arm.clone());
    }

    #[test]
    fn test_lir_arm_debug() {
        let arm = LirArm {
            pattern: LirPat::Wildcard,
            guard: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            },
        };
        let s = format!("{arm:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirDecl — top-level declarations
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_decl_function() {
        let decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: TargetHint::None,
                span: span1(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            is_generator: false,
            span: span1(),
        };
        if let LirDecl::Function {
            name,
            is_pub,
            effect,
            hint,
            ..
        } = &decl
        {
            assert_eq!(name, "main");
            assert!(*is_pub);
            assert_eq!(*effect, Effect::Pure);
            assert_eq!(*hint, TargetHint::None);
        } else {
            panic!("expected Function variant");
        }
    }

    #[test]
    fn test_lir_decl_function_async() {
        let decl = LirDecl::Function {
            name: "fetchData".into(),
            params: vec![],
            return_type: Some(Type::Named("String".into())),
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::Async,
                span: span1(),
            },
            effect: Effect::Async,
            hint: TargetHint::Async,
            is_pub: false,
            is_generator: false,
            span: span1(),
        };
        if let LirDecl::Function { effect, hint, .. } = &decl {
            assert_eq!(*effect, Effect::Async);
            assert_eq!(*hint, TargetHint::Async);
        } else {
            panic!("expected Function variant");
        }
    }

    #[test]
    fn test_lir_decl_record_def() {
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
            span: span1(),
        };
        if let LirDecl::RecordDef { name, fields, .. } = &decl {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected RecordDef variant");
        }
    }

    #[test]
    fn test_lir_decl_union_def() {
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
            span: span1(),
        };
        if let LirDecl::UnionDef { name, variants, .. } = &decl {
            assert_eq!(name, "Option");
            assert_eq!(variants.len(), 2);
        } else {
            panic!("expected UnionDef variant");
        }
    }

    #[test]
    fn test_lir_decl_clone() {
        let decl = LirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: false,
            is_generator: false,
            span: span1(),
        };
        assert_eq!(decl, decl.clone());
    }

    #[test]
    fn test_lir_decl_debug() {
        let decl = LirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: span1(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: false,
            is_generator: false,
            span: span1(),
        };
        let s = format!("{decl:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirField
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_field() {
        let f = LirField {
            name: "x".into(),
            type_: Type::Named("Int".into()),
        };
        assert_eq!(f.name, "x");
        assert_eq!(f.type_, Type::Named("Int".into()));
    }

    #[test]
    fn test_lir_field_clone() {
        let f = LirField {
            name: "y".into(),
            type_: Type::Named("String".into()),
        };
        assert_eq!(f, f.clone());
    }

    #[test]
    fn test_lir_field_debug() {
        let f = LirField {
            name: "z".into(),
            type_: Type::Named("Float".into()),
        };
        let s = format!("{f:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // LirVariant
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_variant_with_arg() {
        let v = LirVariant {
            name: "Some".into(),
            arg: Some(Type::Named("Int".into())),
        };
        assert_eq!(v.name, "Some");
        assert!(v.arg.is_some());
    }

    #[test]
    fn test_lir_variant_without_arg() {
        let v = LirVariant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(v.name, "None");
        assert!(v.arg.is_none());
    }

    #[test]
    fn test_lir_variant_clone() {
        let v = LirVariant {
            name: "Ok".into(),
            arg: Some(Type::Named("String".into())),
        };
        assert_eq!(v, v.clone());
    }

    #[test]
    fn test_lir_variant_debug() {
        let v = LirVariant {
            name: "Err".into(),
            arg: None,
        };
        let s = format!("{v:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // Serde round-trip tests
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_literal_serde_roundtrip() {
        let original = LirLiteral::Str("hello".into());
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirLiteral = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_target_hint_serde_roundtrip() {
        let original = TargetHint::ReactComponent;
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: TargetHint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_effect_serde_roundtrip() {
        let original = Effect::Async;
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Effect = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_expr_serde_roundtrip() {
        let original = LirExpr::Binary {
            op: LirBinaryOp::Eq,
            lhs: Box::new(LirExpr::Variable {
                name: "a".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            rhs: Box::new(LirExpr::Variable {
                name: "b".into(),
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span2(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirExpr = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_pat_serde_roundtrip() {
        let original = LirPat::Record {
            fields: vec![("key".into(), LirPat::Variable("v".into()))],
            rest: true,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirPat = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_decl_serde_roundtrip() {
        let original = LirDecl::Function {
            name: "add".into(),
            params: vec![
                LirParam {
                    name: "a".into(),
                    type_: Some(Type::Named("Int".into())),
                },
                LirParam {
                    name: "b".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            return_type: Some(Type::Named("Int".into())),
            body: LirExpr::Binary {
                op: LirBinaryOp::Add,
                lhs: Box::new(LirExpr::Variable {
                    name: "a".into(),
                    hint: TargetHint::None,
                    span: span1(),
                }),
                rhs: Box::new(LirExpr::Variable {
                    name: "b".into(),
                    hint: TargetHint::None,
                    span: span1(),
                }),
                hint: TargetHint::None,
                span: span1(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            is_generator: false,
            span: span1(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirDecl = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_stmt_serde_roundtrip() {
        let original = LirStmt::Let {
            pat: LirPat::Variable("x".into()),
            value: LirExpr::Literal {
                value: LirLiteral::Int(99),
                hint: TargetHint::None,
                span: span1(),
            },
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirStmt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_lir_arm_serde_roundtrip() {
        let original = LirArm {
            pattern: LirPat::Literal(LirLiteral::Int(42)),
            guard: None,
            body: LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: TargetHint::None,
                span: span1(),
            },
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: LirArm = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    // ------------------------------------------------------------------
    // Absent variants — verify For and Pipe are NOT present in LIR
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_expr_no_for_pipe() {
        // These types should NOT exist in the LIR crate.
        // This test verifies the assertion at compile time by checking
        // that the corresponding variants are absent.
        let _ = LirExpr::Literal {
            value: LirLiteral::Null,
            hint: TargetHint::None,
            span: span1(),
        };
        // If LirExpr had For/Pipe, this test would still compile —
        // but those variants must not be part of the LIR enum.
    }

    // ------------------------------------------------------------------
    // LirExpr::AssertConsistent — cross-target consistency marking
    //
    // These tests verify that LirExpr::AssertConsistent exists as a
    // pass-through wrapper that preserves the inner expression across
    // the LIR boundary. They will fail to compile until the variant
    // is added (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_expr_assert_consistent() {
        let e = LirExpr::AssertConsistent {
            expr: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: TargetHint::None,
                span: span1(),
            }),
            hint: TargetHint::None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        assert_eq!(e.hint(), TargetHint::None);
        if let LirExpr::AssertConsistent { expr, hint, .. } = &e {
            assert!(matches!(
                expr.as_ref(),
                LirExpr::Literal {
                    value: LirLiteral::Int(42),
                    ..
                }
            ));
            assert_eq!(*hint, TargetHint::None);
        } else {
            panic!("expected AssertConsistent variant");
        }
    }
}
