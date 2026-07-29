//! MIR → LIR lowering.
//!
//! This module converts MIR declarations to LIR declarations by annotating
//! every expression with [`TargetHint::None`] and setting function effects
//! to [`Effect::Pure`] by default.
//!
//! `MirDecl::TypeDef` has no LIR equivalent and is silently skipped.

use crate::{
    Effect, LirArm, LirBinaryOp, LirDecl, LirExpr, LirField, LirLiteral, LirParam, LirPat, LirStmt,
    LirUnaryOp, LirVariant, TargetHint,
};
use dwarf_mir::{
    MirArm, MirBinaryOp, MirDecl, MirExpr, MirLiteral, MirParam, MirPat, MirStmt, MirUnaryOp,
};

/// Lower a slice of MIR declarations to LIR declarations.
///
/// Each MIR declaration is converted to its LIR counterpart. Function
/// declarations receive `effect: Effect::Pure` and `hint: TargetHint::None`
/// by default. Every expression is annotated with `TargetHint::None`.
///
/// `MirDecl::TypeDef` variants have no LIR equivalent and are silently
/// skipped.
pub fn lower_to_lir(mir_decls: &[MirDecl]) -> Vec<LirDecl> {
    mir_decls
        .iter()
        .filter_map(|decl| match decl {
            MirDecl::TypeDef { .. } => None,
            MirDecl::Function {
                name,
                params,
                return_type,
                body,
                is_pub,
                is_generator,
                span,
            } => Some(LirDecl::Function {
                name: name.clone(),
                params: lower_params(params),
                return_type: return_type.clone(),
                body: lower_expr(body),
                effect: Effect::Pure,
                hint: TargetHint::None,
                is_pub: *is_pub,
                is_generator: *is_generator,
                span: *span,
            }),
            MirDecl::RecordDef {
                name,
                fields,
                is_pub,
                span,
            } => Some(LirDecl::RecordDef {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|f| LirField {
                        name: f.name.clone(),
                        type_: f.type_.clone(),
                    })
                    .collect(),
                is_pub: *is_pub,
                span: *span,
            }),
            MirDecl::UnionDef {
                name,
                variants,
                is_pub,
                span,
            } => Some(LirDecl::UnionDef {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|v| LirVariant {
                        name: v.name.clone(),
                        arg: v.arg.clone(),
                    })
                    .collect(),
                is_pub: *is_pub,
                span: *span,
            }),
            MirDecl::Extern {
                source,
                name,
                params,
                return_type,
                is_pub,
            } => Some(LirDecl::Extern {
                source: source.clone(),
                name: name.clone(),
                params: lower_params(params),
                return_type: return_type.clone(),
                is_pub: *is_pub,
            }),
        })
        .collect()
}

/// Lower a MIR expression to an LIR expression, adding [`TargetHint::None`].
pub fn lower_expr(expr: &MirExpr) -> LirExpr {
    match expr {
        MirExpr::Literal { value, span } => LirExpr::Literal {
            value: lower_literal(value),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Variable { name, span } => LirExpr::Variable {
            name: name.clone(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Call { func, args, span } => LirExpr::Call {
            func: Box::new(lower_expr(func)),
            args: args.iter().map(lower_expr).collect(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Member { obj, field, span } => LirExpr::Member {
            obj: Box::new(lower_expr(obj)),
            field: field.clone(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::If {
            cond,
            then,
            else_,
            span,
        } => LirExpr::If {
            cond: Box::new(lower_expr(cond)),
            then: Box::new(lower_expr(then)),
            else_: else_.as_ref().map(|e| Box::new(lower_expr(e))),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Loop { body, .. } => lower_expr(body),

        MirExpr::Match {
            expr: match_expr,
            arms,
            span,
        } => LirExpr::Match {
            expr: Box::new(lower_expr(match_expr)),
            arms: arms.iter().map(lower_arm).collect(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Block { stmts, span } => LirExpr::Block {
            stmts: stmts.iter().map(lower_stmt).collect(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Assign {
            target,
            value,
            span,
        } => LirExpr::Assign {
            target: Box::new(lower_expr(target)),
            value: Box::new(lower_expr(value)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Lambda { params, body, span } => LirExpr::Lambda {
            params: lower_params(params),
            body: Box::new(lower_expr(body)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Record { fields, span } => LirExpr::Record {
            fields: fields
                .iter()
                .map(|(name, expr)| (name.clone(), lower_expr(expr)))
                .collect(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Variant { name, arg, span } => LirExpr::Variant {
            name: name.clone(),
            arg: arg.as_ref().map(|a| Box::new(lower_expr(a))),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Array { items, span } => LirExpr::Array {
            items: items.iter().map(lower_expr).collect(),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Binary { op, lhs, rhs, span } => LirExpr::Binary {
            op: lower_binary_op(op),
            lhs: Box::new(lower_expr(lhs)),
            rhs: Box::new(lower_expr(rhs)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Unary {
            op,
            expr: unary_expr,
            span,
        } => LirExpr::Unary {
            op: lower_unary_op(op),
            expr: Box::new(lower_expr(unary_expr)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Wildcard { span } => LirExpr::Wildcard {
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::ForAll {
            type_,
            binding,
            property,
            span,
        } => LirExpr::ForAll {
            type_: type_.clone(),
            binding: lower_pat(binding),
            property: Box::new(lower_expr(property)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::AssertConsistent { expr, span } => LirExpr::AssertConsistent {
            expr: Box::new(lower_expr(expr)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Try {
            body,
            binding,
            guard,
            handler,
            span,
        } => LirExpr::Try {
            body: Box::new(lower_expr(body)),
            binding: lower_pat(binding),
            guard: guard.as_ref().map(|g| Box::new(lower_expr(g))),
            handler: Box::new(lower_expr(handler)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Throw { expr, span } => LirExpr::Throw {
            expr: Box::new(lower_expr(expr)),
            hint: TargetHint::None,
            span: *span,
        },
        MirExpr::Propagate { expr, span } => LirExpr::Propagate {
            expr: Box::new(lower_expr(expr)),
            hint: TargetHint::None,
            span: *span,
        },
    }
}

/// Lower a MIR match arm to an LIR match arm.
fn lower_arm(arm: &MirArm) -> LirArm {
    LirArm {
        pattern: lower_pat(&arm.pattern),
        guard: arm.guard.as_ref().map(lower_expr),
        body: lower_expr(&arm.body),
    }
}

/// Lower a MIR pattern to an LIR pattern.
pub fn lower_pat(pat: &MirPat) -> LirPat {
    match pat {
        MirPat::Wildcard => LirPat::Wildcard,
        MirPat::Literal(lit) => LirPat::Literal(lower_literal(lit)),
        MirPat::Variable(name) => LirPat::Variable(name.clone()),
        MirPat::Variant { name, arg } => LirPat::Variant {
            name: name.clone(),
            arg: arg.as_ref().map(|a| Box::new(lower_pat(a))),
        },
        MirPat::Record { fields, rest } => LirPat::Record {
            fields: fields
                .iter()
                .map(|(n, p)| (n.clone(), lower_pat(p)))
                .collect(),
            rest: *rest,
        },
    }
}

/// Lower a MIR statement to an LIR statement.
pub fn lower_stmt(stmt: &MirStmt) -> LirStmt {
    match stmt {
        MirStmt::Let { pat, value } => LirStmt::Let {
            pat: lower_pat(pat),
            value: lower_expr(value),
        },
        MirStmt::Expr(expr) => LirStmt::Expr(lower_expr(expr)),
    }
}

/// Lower a MIR literal to an LIR literal.
pub fn lower_literal(lit: &MirLiteral) -> LirLiteral {
    match lit {
        MirLiteral::Int(i) => LirLiteral::Int(*i),
        MirLiteral::Float(f) => LirLiteral::Float(*f),
        MirLiteral::Str(s) => LirLiteral::Str(s.clone()),
        MirLiteral::Bool(b) => LirLiteral::Bool(*b),
        MirLiteral::Null => LirLiteral::Null,
    }
}

/// Lower a MIR binary operator to an LIR binary operator (identity mapping).
pub fn lower_binary_op(op: &MirBinaryOp) -> LirBinaryOp {
    match op {
        MirBinaryOp::Add => LirBinaryOp::Add,
        MirBinaryOp::Sub => LirBinaryOp::Sub,
        MirBinaryOp::Mul => LirBinaryOp::Mul,
        MirBinaryOp::Div => LirBinaryOp::Div,
        MirBinaryOp::Eq => LirBinaryOp::Eq,
        MirBinaryOp::Ne => LirBinaryOp::Ne,
        MirBinaryOp::Lt => LirBinaryOp::Lt,
        MirBinaryOp::Gt => LirBinaryOp::Gt,
        MirBinaryOp::Le => LirBinaryOp::Le,
        MirBinaryOp::Ge => LirBinaryOp::Ge,
        MirBinaryOp::And => LirBinaryOp::And,
        MirBinaryOp::Or => LirBinaryOp::Or,
    }
}

/// Lower a MIR unary operator to an LIR unary operator (identity mapping).
pub fn lower_unary_op(op: &MirUnaryOp) -> LirUnaryOp {
    match op {
        MirUnaryOp::Neg => LirUnaryOp::Neg,
        MirUnaryOp::Not => LirUnaryOp::Not,
    }
}

/// Lower MIR parameters to LIR parameters (identity conversion).
fn lower_params(params: &[MirParam]) -> Vec<LirParam> {
    params
        .iter()
        .map(|p| LirParam {
            name: p.name.clone(),
            type_: p.type_.clone(),
        })
        .collect()
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
    // lower_to_lir — top-level declaration lowering
    // ------------------------------------------------------------------

    #[test]
    fn test_lower_function_decl() {
        let mir_fn = MirDecl::Function {
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
        let result = lower_to_lir(&[mir_fn]);
        assert_eq!(result.len(), 1);
        match &result[0] {
            LirDecl::Function {
                name,
                effect,
                hint,
                is_pub,
                ..
            } => {
                assert_eq!(name, "main");
                assert_eq!(*effect, Effect::Pure);
                assert_eq!(*hint, TargetHint::None);
                assert!(*is_pub);
            }
            other => panic!("expected Function variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_record_decl() {
        let mir_record = MirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                dwarf_mir::MirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                dwarf_mir::MirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: span1(),
        };
        let result = lower_to_lir(&[mir_record]);
        assert_eq!(result.len(), 1);
        match &result[0] {
            LirDecl::RecordDef { name, fields, .. } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            other => panic!("expected RecordDef variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_union_decl() {
        let mir_union = MirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![
                dwarf_mir::MirVariant {
                    name: "Some".into(),
                    arg: Some(Type::Named("Int".into())),
                },
                dwarf_mir::MirVariant {
                    name: "None".into(),
                    arg: None,
                },
            ],
            is_pub: true,
            span: span1(),
        };
        let result = lower_to_lir(&[mir_union]);
        assert_eq!(result.len(), 1);
        match &result[0] {
            LirDecl::UnionDef { name, variants, .. } => {
                assert_eq!(name, "Option");
                assert_eq!(variants.len(), 2);
            }
            other => panic!("expected UnionDef variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_empty_input() {
        let result = lower_to_lir(&[]);
        assert!(result.is_empty());
    }

    // ------------------------------------------------------------------
    // lower_expr — expression lowering
    // ------------------------------------------------------------------

    #[test]
    fn test_lower_literal_expr() {
        let mir_expr = MirExpr::Literal {
            value: MirLiteral::Int(42),
            span: span1(),
        };
        let result = lower_expr(&mir_expr);
        match &result {
            LirExpr::Literal { value, hint, .. } => {
                assert_eq!(*value, LirLiteral::Int(42));
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected Literal variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_call_expr() {
        let mir_expr = MirExpr::Call {
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
        let result = lower_expr(&mir_expr);
        match &result {
            LirExpr::Call {
                func, args, hint, ..
            } => {
                assert!(
                    matches!(func.as_ref(), LirExpr::Variable { name, .. } if name == "f"),
                    "expected func to be Variable(\"f\")"
                );
                assert_eq!(args.len(), 1);
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected Call variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_binary_expr() {
        let mir_expr = MirExpr::Binary {
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
        let result = lower_expr(&mir_expr);
        match &result {
            LirExpr::Binary {
                op, lhs, rhs, hint, ..
            } => {
                assert_eq!(*op, LirBinaryOp::Add);
                assert_eq!(
                    lhs.as_ref(),
                    &LirExpr::Literal {
                        value: LirLiteral::Int(1),
                        hint: TargetHint::None,
                        span: span1(),
                    }
                );
                assert_eq!(
                    rhs.as_ref(),
                    &LirExpr::Literal {
                        value: LirLiteral::Int(2),
                        hint: TargetHint::None,
                        span: span1(),
                    }
                );
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected Binary variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_lambda_expr() {
        let mir_expr = MirExpr::Lambda {
            params: vec![
                dwarf_mir::MirParam {
                    name: "x".into(),
                    type_: None,
                },
                dwarf_mir::MirParam {
                    name: "y".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            body: Box::new(MirExpr::Variable {
                name: "x".into(),
                span: span1(),
            }),
            span: span2(),
        };
        let result = lower_expr(&mir_expr);
        match &result {
            LirExpr::Lambda {
                params, body, hint, ..
            } => {
                assert_eq!(params.len(), 2);
                assert_eq!(
                    params[0],
                    LirParam {
                        name: "x".into(),
                        type_: None
                    }
                );
                assert_eq!(
                    params[1],
                    LirParam {
                        name: "y".into(),
                        type_: Some(Type::Named("Int".into()))
                    }
                );
                assert!(
                    matches!(body.as_ref(), LirExpr::Variable { name, .. } if name == "x"),
                    "expected body to be Variable(\"x\")"
                );
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected Lambda variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lower_preserves_spans() {
        let mir_expr = MirExpr::Literal {
            value: MirLiteral::Int(99),
            span: span2(),
        };
        let result = lower_expr(&mir_expr);
        assert_eq!(result.span(), span2(), "span should be preserved from MIR");
    }

    // ------------------------------------------------------------------
    // AssertConsistent lowering tests (DWARF-41)
    //
    // These tests verify that lower_expr correctly handles
    // MirExpr::AssertConsistent by wrapping the lowered inner expression
    // in LirExpr::AssertConsistent. They will fail to compile until
    // both MirExpr::AssertConsistent and LirExpr::AssertConsistent are
    // implemented (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_lower_assert_consistent_expr() {
        let mir_expr = MirExpr::AssertConsistent {
            expr: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(42),
                span: span1(),
            }),
            span: span1(),
        };
        let result = lower_expr(&mir_expr);
        match &result {
            LirExpr::AssertConsistent { expr, hint, .. } => {
                assert!(
                    matches!(
                        expr.as_ref(),
                        LirExpr::Literal {
                            value: LirLiteral::Int(42),
                            ..
                        }
                    ),
                    "inner expression should be preserved as literal 42"
                );
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected AssertConsistent variant, got {other:?}"),
        }
    }
}
