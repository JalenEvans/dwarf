//! Mid-level Intermediate Representation (MIR).
//!
//! The MIR is a desugared, simplified IR produced by lowering from the HIR.
//! All syntactic sugar (pipes, propagation operators, for-loops, decorators,
//! raw strings, imports) has been eliminated. The MIR mirrors the HIR in
//! capabilities but with a minimal, canonical set of expression forms.

pub mod desugar;
pub mod modules;
pub mod pass;

use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ---------------------------------------------------------------------------
// Literals
// ---------------------------------------------------------------------------

/// A literal value in the MIR.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirLiteral {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirBinaryOp {
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

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirUnaryOp {
    Neg,
    Not,
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// A MIR expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirExpr {
    Literal {
        value: MirLiteral,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    },
    Call {
        func: Box<MirExpr>,
        args: Vec<MirExpr>,
        span: Span,
    },
    Member {
        obj: Box<MirExpr>,
        field: String,
        span: Span,
    },
    If {
        cond: Box<MirExpr>,
        then: Box<MirExpr>,
        else_: Option<Box<MirExpr>>,
        span: Span,
    },
    Match {
        expr: Box<MirExpr>,
        arms: Vec<MirArm>,
        span: Span,
    },
    Loop {
        body: Box<MirExpr>,
        span: Span,
    },
    Block {
        stmts: Vec<MirStmt>,
        span: Span,
    },
    Assign {
        target: Box<MirExpr>,
        value: Box<MirExpr>,
        span: Span,
    },
    Lambda {
        params: Vec<MirParam>,
        body: Box<MirExpr>,
        span: Span,
    },
    Record {
        fields: Vec<(String, MirExpr)>,
        span: Span,
    },
    Variant {
        name: String,
        arg: Option<Box<MirExpr>>,
        span: Span,
    },
    Array {
        items: Vec<MirExpr>,
        span: Span,
    },
    Binary {
        op: MirBinaryOp,
        lhs: Box<MirExpr>,
        rhs: Box<MirExpr>,
        span: Span,
    },
    Unary {
        op: MirUnaryOp,
        expr: Box<MirExpr>,
        span: Span,
    },
    Wildcard {
        span: Span,
    },
    /// Property-based testing: forAll Type { binding -> property }
    ForAll {
        type_: Type,
        binding: MirPat,
        property: Box<MirExpr>,
        span: Span,
    },
}

impl MirExpr {
    /// Return the source span of this expression.
    pub fn span(&self) -> Span {
        match self {
            MirExpr::Literal { span, .. }
            | MirExpr::Variable { span, .. }
            | MirExpr::Call { span, .. }
            | MirExpr::Member { span, .. }
            | MirExpr::If { span, .. }
            | MirExpr::Match { span, .. }
            | MirExpr::Loop { span, .. }
            | MirExpr::Block { span, .. }
            | MirExpr::Assign { span, .. }
            | MirExpr::Lambda { span, .. }
            | MirExpr::Record { span, .. }
            | MirExpr::Variant { span, .. }
            | MirExpr::Array { span, .. }
            | MirExpr::Binary { span, .. }
            | MirExpr::Unary { span, .. }
            | MirExpr::Wildcard { span }
            | MirExpr::ForAll { span, .. } => *span,
        }
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// A MIR statement (inside a block expression).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirStmt {
    Let { pat: MirPat, value: MirExpr },
    Expr(MirExpr),
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

/// A pattern used in let bindings and match arms.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirPat {
    Wildcard,
    Literal(MirLiteral),
    Variable(String),
    Variant {
        name: String,
        arg: Option<Box<MirPat>>,
    },
    Record {
        fields: Vec<(String, MirPat)>,
        rest: bool,
    },
}

// ---------------------------------------------------------------------------
// Parameter & arm helpers
// ---------------------------------------------------------------------------

/// A parameter declaration (in functions and lambdas).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MirParam {
    pub name: String,
    pub type_: Option<Type>,
}

/// An arm in a match expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MirArm {
    pub pattern: MirPat,
    pub guard: Option<MirExpr>,
    pub body: MirExpr,
}

// ---------------------------------------------------------------------------
// Top-level declarations
// ---------------------------------------------------------------------------

/// A top-level declaration in the MIR.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MirDecl {
    Function {
        name: String,
        params: Vec<MirParam>,
        return_type: Option<Type>,
        body: MirExpr,
        is_pub: bool,
        is_generator: bool,
        span: Span,
    },
    TypeDef {
        name: String,
        type_: Type,
        is_pub: bool,
        span: Span,
    },
    RecordDef {
        name: String,
        fields: Vec<MirField>,
        is_pub: bool,
        span: Span,
    },
    UnionDef {
        name: String,
        variants: Vec<MirVariant>,
        is_pub: bool,
        span: Span,
    },
}

/// A field in a record type definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MirField {
    pub name: String,
    pub type_: Type,
}

/// A variant in a union type definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MirVariant {
    pub name: String,
    pub arg: Option<Type>,
}

#[cfg(test)]
mod tests {
    use crate::*;
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
    // MirLiteral — all literal forms
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_literal_int() {
        let lit = MirLiteral::Int(42);
        assert_eq!(lit, MirLiteral::Int(42));
        assert_ne!(lit, MirLiteral::Int(0));
    }

    #[test]
    fn test_mir_literal_float() {
        let lit = MirLiteral::Float(3.5);
        assert_eq!(lit, MirLiteral::Float(3.5));
    }

    #[test]
    fn test_mir_literal_str() {
        let lit = MirLiteral::Str("hello".into());
        assert_eq!(lit, MirLiteral::Str("hello".into()));
    }

    #[test]
    fn test_mir_literal_bool() {
        let lit = MirLiteral::Bool(true);
        assert_eq!(lit, MirLiteral::Bool(true));
        assert_ne!(lit, MirLiteral::Bool(false));
    }

    #[test]
    fn test_mir_literal_null() {
        let lit = MirLiteral::Null;
        assert_eq!(lit, MirLiteral::Null);
    }

    #[test]
    fn test_mir_literal_clone() {
        let lit = MirLiteral::Int(99);
        let cloned = lit.clone();
        assert_eq!(lit, cloned);
    }

    #[test]
    fn test_mir_literal_debug() {
        let lit = MirLiteral::Str("debug".into());
        let s = format!("{lit:?}");
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    // ------------------------------------------------------------------
    // MirExpr — all expression variants
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_expr_literal() {
        let e = MirExpr::Literal {
            value: MirLiteral::Int(1),
            span: span1(),
        };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_mir_expr_variable() {
        let e = MirExpr::Variable {
            name: "x".into(),
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let MirExpr::Variable { name, .. } = &e {
            assert_eq!(name, "x");
        } else {
            panic!("expected Variable variant");
        }
    }

    #[test]
    fn test_mir_expr_call() {
        let e = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "f".into(),
                span: span1(),
            }),
            args: vec![MirExpr::Literal {
                value: MirLiteral::Int(0),
                span: span1(),
            }],
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Call { func, args, .. } = &e {
            assert!(matches!(func.as_ref(), MirExpr::Variable { name, .. } if name == "f"));
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected Call variant");
        }
    }

    #[test]
    fn test_mir_expr_member() {
        let e = MirExpr::Member {
            obj: Box::new(MirExpr::Variable {
                name: "obj".into(),
                span: span1(),
            }),
            field: "attr".into(),
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Member { field, .. } = &e {
            assert_eq!(field, "attr");
        } else {
            panic!("expected Member variant");
        }
    }

    #[test]
    fn test_mir_expr_if_with_else() {
        let e = MirExpr::If {
            cond: Box::new(MirExpr::Literal {
                value: MirLiteral::Bool(true),
                span: span1(),
            }),
            then: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(1),
                span: span1(),
            }),
            else_: Some(Box::new(MirExpr::Literal {
                value: MirLiteral::Int(2),
                span: span1(),
            })),
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::If { else_, .. } = &e {
            assert!(else_.is_some());
        } else {
            panic!("expected If variant");
        }
    }

    #[test]
    fn test_mir_expr_if_no_else() {
        let e = MirExpr::If {
            cond: Box::new(MirExpr::Literal {
                value: MirLiteral::Bool(false),
                span: span1(),
            }),
            then: Box::new(MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            }),
            else_: None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_mir_expr_match() {
        let e = MirExpr::Match {
            expr: Box::new(MirExpr::Variable {
                name: "x".into(),
                span: span1(),
            }),
            arms: vec![MirArm {
                pattern: MirPat::Wildcard,
                guard: None,
                body: MirExpr::Literal {
                    value: MirLiteral::Int(0),
                    span: span1(),
                },
            }],
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let MirExpr::Match { arms, .. } = &e {
            assert_eq!(arms.len(), 1);
        } else {
            panic!("expected Match variant");
        }
    }

    #[test]
    fn test_mir_expr_block() {
        let e = MirExpr::Block {
            stmts: vec![
                MirStmt::Expr(MirExpr::Literal {
                    value: MirLiteral::Int(1),
                    span: span1(),
                }),
                MirStmt::Let {
                    pat: MirPat::Variable("y".into()),
                    value: MirExpr::Literal {
                        value: MirLiteral::Int(2),
                        span: span1(),
                    },
                },
            ],
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Block { stmts, .. } = &e {
            assert_eq!(stmts.len(), 2);
        } else {
            panic!("expected Block variant");
        }
    }

    #[test]
    fn test_mir_expr_assign() {
        let e = MirExpr::Assign {
            target: Box::new(MirExpr::Variable {
                name: "x".into(),
                span: span1(),
            }),
            value: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(42),
                span: span1(),
            }),
            span: span2(),
        };
        assert_eq!(e.span(), span2());
    }

    #[test]
    fn test_mir_expr_lambda() {
        let e = MirExpr::Lambda {
            params: vec![
                MirParam {
                    name: "a".into(),
                    type_: None,
                },
                MirParam {
                    name: "b".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            body: Box::new(MirExpr::Variable {
                name: "a".into(),
                span: span1(),
            }),
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Lambda { params, .. } = &e {
            assert_eq!(params.len(), 2);
        } else {
            panic!("expected Lambda variant");
        }
    }

    #[test]
    fn test_mir_expr_record() {
        let e = MirExpr::Record {
            fields: vec![
                (
                    "x".into(),
                    MirExpr::Literal {
                        value: MirLiteral::Int(10),
                        span: span1(),
                    },
                ),
                (
                    "y".into(),
                    MirExpr::Literal {
                        value: MirLiteral::Int(20),
                        span: span1(),
                    },
                ),
            ],
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Record { fields, .. } = &e {
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected Record variant");
        }
    }

    #[test]
    fn test_mir_expr_variant_with_arg() {
        let e = MirExpr::Variant {
            name: "Some".into(),
            arg: Some(Box::new(MirExpr::Literal {
                value: MirLiteral::Int(1),
                span: span1(),
            })),
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let MirExpr::Variant { name, arg, .. } = &e {
            assert_eq!(name, "Some");
            assert!(arg.is_some());
        } else {
            panic!("expected Variant variant");
        }
    }

    #[test]
    fn test_mir_expr_variant_no_arg() {
        let e = MirExpr::Variant {
            name: "None".into(),
            arg: None,
            span: span1(),
        };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_mir_expr_array() {
        let e = MirExpr::Array {
            items: vec![
                MirExpr::Literal {
                    value: MirLiteral::Int(1),
                    span: span1(),
                },
                MirExpr::Literal {
                    value: MirLiteral::Int(2),
                    span: span1(),
                },
                MirExpr::Literal {
                    value: MirLiteral::Int(3),
                    span: span1(),
                },
            ],
            span: span2(),
        };
        assert_eq!(e.span(), span2());
        if let MirExpr::Array { items, .. } = &e {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected Array variant");
        }
    }

    #[test]
    fn test_mir_expr_binary() {
        let e = MirExpr::Binary {
            op: MirBinaryOp::Add,
            lhs: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(1),
                span: span1(),
            }),
            rhs: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(2),
                span: span1(),
            }),
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let MirExpr::Binary { op, .. } = &e {
            assert_eq!(*op, MirBinaryOp::Add);
        } else {
            panic!("expected Binary variant");
        }
    }

    #[test]
    fn test_mir_expr_unary() {
        let e = MirExpr::Unary {
            op: MirUnaryOp::Neg,
            expr: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(5),
                span: span1(),
            }),
            span: span1(),
        };
        assert_eq!(e.span(), span1());
        if let MirExpr::Unary { op, .. } = &e {
            assert_eq!(*op, MirUnaryOp::Neg);
        } else {
            panic!("expected Unary variant");
        }
    }

    #[test]
    fn test_mir_expr_wildcard() {
        let e = MirExpr::Wildcard { span: span1() };
        assert_eq!(e.span(), span1());
    }

    #[test]
    fn test_mir_expr_clone() {
        let e = MirExpr::Literal {
            value: MirLiteral::Int(7),
            span: span1(),
        };
        let cloned = e.clone();
        assert_eq!(e, cloned);
    }

    #[test]
    fn test_mir_expr_debug() {
        let e = MirExpr::Variable {
            name: "debug".into(),
            span: span1(),
        };
        let s = format!("{e:?}");
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    #[test]
    fn test_mir_expr_partial_eq() {
        let a = MirExpr::Literal {
            value: MirLiteral::Int(1),
            span: span1(),
        };
        let b = MirExpr::Literal {
            value: MirLiteral::Int(1),
            span: span1(),
        };
        let c = MirExpr::Literal {
            value: MirLiteral::Int(2),
            span: span1(),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_mir_expr_span_all_variants() {
        // Verify span() works correctly for expression trees where span differs
        // from sub-expression spans — important for error reporting.
        let inner = MirExpr::Literal {
            value: MirLiteral::Int(1),
            span: span1(),
        };
        let outer = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "f".into(),
                span: span1(),
            }),
            args: vec![inner],
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
    // MirStmt — statement forms
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_stmt_let() {
        let stmt = MirStmt::Let {
            pat: MirPat::Variable("x".into()),
            value: MirExpr::Literal {
                value: MirLiteral::Int(42),
                span: span1(),
            },
        };
        if let MirStmt::Let { pat, .. } = &stmt {
            assert_eq!(*pat, MirPat::Variable("x".into()));
        } else {
            panic!("expected Let variant");
        }
    }

    #[test]
    fn test_mir_stmt_expr() {
        let stmt = MirStmt::Expr(MirExpr::Literal {
            value: MirLiteral::Null,
            span: span1(),
        });
        if let MirStmt::Expr(expr) = &stmt {
            assert_eq!(expr.span(), span1());
        } else {
            panic!("expected Expr variant");
        }
    }

    #[test]
    fn test_mir_stmt_clone() {
        let stmt = MirStmt::Expr(MirExpr::Literal {
            value: MirLiteral::Int(0),
            span: span1(),
        });
        assert_eq!(stmt, stmt.clone());
    }

    #[test]
    fn test_mir_stmt_debug() {
        let stmt = MirStmt::Expr(MirExpr::Literal {
            value: MirLiteral::Null,
            span: span1(),
        });
        let s = format!("{stmt:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirPat — pattern forms
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_pat_wildcard() {
        let p = MirPat::Wildcard;
        assert_eq!(p, MirPat::Wildcard);
    }

    #[test]
    fn test_mir_pat_literal() {
        let p = MirPat::Literal(MirLiteral::Int(42));
        assert_eq!(p, MirPat::Literal(MirLiteral::Int(42)));
    }

    #[test]
    fn test_mir_pat_variable() {
        let p = MirPat::Variable("binding".into());
        assert_eq!(p, MirPat::Variable("binding".into()));
    }

    #[test]
    fn test_mir_pat_variant_with_arg() {
        let p = MirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(MirPat::Variable("inner".into()))),
        };
        if let MirPat::Variant { name, arg } = &p {
            assert_eq!(name, "Some");
            assert!(arg.is_some());
        } else {
            panic!("expected Variant variant");
        }
    }

    #[test]
    fn test_mir_pat_variant_no_arg() {
        let p = MirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(
            p,
            MirPat::Variant {
                name: "None".into(),
                arg: None
            }
        );
    }

    #[test]
    fn test_mir_pat_record_no_rest() {
        let p = MirPat::Record {
            fields: vec![("x".into(), MirPat::Wildcard)],
            rest: false,
        };
        if let MirPat::Record { fields, rest } = &p {
            assert_eq!(fields.len(), 1);
            assert!(!rest);
        } else {
            panic!("expected Record variant");
        }
    }

    #[test]
    fn test_mir_pat_record_with_rest() {
        let p = MirPat::Record {
            fields: vec![],
            rest: true,
        };
        assert_eq!(
            p,
            MirPat::Record {
                fields: vec![],
                rest: true
            }
        );
    }

    #[test]
    fn test_mir_pat_clone() {
        let p = MirPat::Variable("x".into());
        assert_eq!(p, p.clone());
    }

    #[test]
    fn test_mir_pat_debug() {
        let p = MirPat::Wildcard;
        let s = format!("{p:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirBinaryOp
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_binary_op_all_variants() {
        // Grouped by category to validate the full set.
        use MirBinaryOp::*;
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
            let _ = op; // just verify the name resolves
        }
    }

    #[test]
    fn test_mir_binary_op_equality() {
        assert_eq!(MirBinaryOp::Add, MirBinaryOp::Add);
        assert_ne!(MirBinaryOp::Add, MirBinaryOp::Sub);
        assert_ne!(MirBinaryOp::Eq, MirBinaryOp::Ne);
    }

    #[test]
    fn test_mir_binary_op_clone() {
        let op = MirBinaryOp::And;
        assert_eq!(op, op.clone());
    }

    // ------------------------------------------------------------------
    // MirUnaryOp
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_unary_op_variants() {
        assert_eq!(MirUnaryOp::Neg as u8, 0);
        assert_eq!(MirUnaryOp::Not as u8, 1);
    }

    #[test]
    fn test_mir_unary_op_equality() {
        assert_eq!(MirUnaryOp::Neg, MirUnaryOp::Neg);
        assert_ne!(MirUnaryOp::Neg, MirUnaryOp::Not);
    }

    #[test]
    fn test_mir_unary_op_clone() {
        assert_eq!(MirUnaryOp::Not, MirUnaryOp::Not.clone());
    }

    // ------------------------------------------------------------------
    // MirParam
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_param_untyped() {
        let p = MirParam {
            name: "x".into(),
            type_: None,
        };
        assert_eq!(p.name, "x");
        assert!(p.type_.is_none());
    }

    #[test]
    fn test_mir_param_typed() {
        let p = MirParam {
            name: "y".into(),
            type_: Some(Type::Named("Int".into())),
        };
        assert_eq!(p.name, "y");
        assert_eq!(p.type_, Some(Type::Named("Int".into())));
    }

    #[test]
    fn test_mir_param_clone() {
        let p = MirParam {
            name: "z".into(),
            type_: None,
        };
        assert_eq!(p, p.clone());
    }

    #[test]
    fn test_mir_param_debug() {
        let p = MirParam {
            name: "p".into(),
            type_: None,
        };
        let s = format!("{p:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirArm
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_arm_no_guard() {
        let arm = MirArm {
            pattern: MirPat::Wildcard,
            guard: None,
            body: MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            },
        };
        assert_eq!(arm.pattern, MirPat::Wildcard);
        assert!(arm.guard.is_none());
    }

    #[test]
    fn test_mir_arm_with_guard() {
        let arm = MirArm {
            pattern: MirPat::Variable("x".into()),
            guard: Some(MirExpr::Literal {
                value: MirLiteral::Bool(true),
                span: span1(),
            }),
            body: MirExpr::Literal {
                value: MirLiteral::Int(1),
                span: span1(),
            },
        };
        assert!(arm.guard.is_some());
    }

    #[test]
    fn test_mir_arm_clone() {
        let arm = MirArm {
            pattern: MirPat::Wildcard,
            guard: None,
            body: MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            },
        };
        assert_eq!(arm, arm.clone());
    }

    #[test]
    fn test_mir_arm_debug() {
        let arm = MirArm {
            pattern: MirPat::Wildcard,
            guard: None,
            body: MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            },
        };
        let s = format!("{arm:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirDecl — top-level declarations
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_decl_function() {
        let decl = MirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Literal {
                value: MirLiteral::Int(0),
                span: span1(),
            },
            is_pub: true,
            is_generator: false,
            span: span1(),
        };
        if let MirDecl::Function { name, is_pub, .. } = &decl {
            assert_eq!(name, "main");
            assert!(*is_pub);
        } else {
            panic!("expected Function variant");
        }
    }

    #[test]
    fn test_mir_decl_type_def() {
        let decl = MirDecl::TypeDef {
            name: "MyInt".into(),
            type_: Type::Named("Int".into()),
            is_pub: false,
            span: span1(),
        };
        if let MirDecl::TypeDef { name, type_, .. } = &decl {
            assert_eq!(name, "MyInt");
            assert_eq!(*type_, Type::Named("Int".into()));
        } else {
            panic!("expected TypeDef variant");
        }
    }

    #[test]
    fn test_mir_decl_record_def() {
        let decl = MirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                MirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                MirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: span1(),
        };
        if let MirDecl::RecordDef { name, fields, .. } = &decl {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("expected RecordDef variant");
        }
    }

    #[test]
    fn test_mir_decl_union_def() {
        let decl = MirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![
                MirVariant {
                    name: "Some".into(),
                    arg: Some(Type::Named("Int".into())),
                },
                MirVariant {
                    name: "None".into(),
                    arg: None,
                },
            ],
            is_pub: true,
            span: span1(),
        };
        if let MirDecl::UnionDef { name, variants, .. } = &decl {
            assert_eq!(name, "Option");
            assert_eq!(variants.len(), 2);
        } else {
            panic!("expected UnionDef variant");
        }
    }

    #[test]
    fn test_mir_decl_clone() {
        let decl = MirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            },
            is_pub: false,
            is_generator: false,
            span: span1(),
        };
        assert_eq!(decl, decl.clone());
    }

    #[test]
    fn test_mir_decl_debug() {
        let decl = MirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Literal {
                value: MirLiteral::Null,
                span: span1(),
            },
            is_pub: false,
            is_generator: false,
            span: span1(),
        };
        let s = format!("{decl:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirField
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_field() {
        let f = MirField {
            name: "x".into(),
            type_: Type::Named("Int".into()),
        };
        assert_eq!(f.name, "x");
        assert_eq!(f.type_, Type::Named("Int".into()));
    }

    #[test]
    fn test_mir_field_clone() {
        let f = MirField {
            name: "y".into(),
            type_: Type::Named("String".into()),
        };
        assert_eq!(f, f.clone());
    }

    #[test]
    fn test_mir_field_debug() {
        let f = MirField {
            name: "z".into(),
            type_: Type::Named("Float".into()),
        };
        let s = format!("{f:?}");
        assert!(!s.is_empty());
    }

    // ------------------------------------------------------------------
    // MirVariant
    // ------------------------------------------------------------------

    #[test]
    fn test_mir_variant_with_arg() {
        let v = MirVariant {
            name: "Some".into(),
            arg: Some(Type::Named("Int".into())),
        };
        assert_eq!(v.name, "Some");
        assert!(v.arg.is_some());
    }

    #[test]
    fn test_mir_variant_without_arg() {
        let v = MirVariant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(v.name, "None");
        assert!(v.arg.is_none());
    }

    #[test]
    fn test_mir_variant_clone() {
        let v = MirVariant {
            name: "Ok".into(),
            arg: Some(Type::Named("String".into())),
        };
        assert_eq!(v, v.clone());
    }

    #[test]
    fn test_mir_variant_debug() {
        let v = MirVariant {
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
    fn test_mir_literal_serde_roundtrip() {
        let original = MirLiteral::Str("hello".into());
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirLiteral = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_mir_expr_serde_roundtrip() {
        let original = MirExpr::Binary {
            op: MirBinaryOp::Eq,
            lhs: Box::new(MirExpr::Variable {
                name: "a".into(),
                span: span1(),
            }),
            rhs: Box::new(MirExpr::Variable {
                name: "b".into(),
                span: span1(),
            }),
            span: span2(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirExpr = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_mir_pat_serde_roundtrip() {
        let original = MirPat::Record {
            fields: vec![("key".into(), MirPat::Variable("v".into()))],
            rest: true,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirPat = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_mir_decl_serde_roundtrip() {
        let original = MirDecl::Function {
            name: "add".into(),
            params: vec![
                MirParam {
                    name: "a".into(),
                    type_: Some(Type::Named("Int".into())),
                },
                MirParam {
                    name: "b".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            return_type: Some(Type::Named("Int".into())),
            body: MirExpr::Binary {
                op: MirBinaryOp::Add,
                lhs: Box::new(MirExpr::Variable {
                    name: "a".into(),
                    span: span1(),
                }),
                rhs: Box::new(MirExpr::Variable {
                    name: "b".into(),
                    span: span1(),
                }),
                span: span1(),
            },
            is_pub: true,
            is_generator: false,
            span: span1(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirDecl = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_mir_stmt_serde_roundtrip() {
        let original = MirStmt::Let {
            pat: MirPat::Variable("x".into()),
            value: MirExpr::Literal {
                value: MirLiteral::Int(99),
                span: span1(),
            },
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirStmt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_mir_arm_serde_roundtrip() {
        let original = MirArm {
            pattern: MirPat::Literal(MirLiteral::Int(42)),
            guard: None,
            body: MirExpr::Literal {
                value: MirLiteral::Bool(true),
                span: span1(),
            },
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: MirArm = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }
}
