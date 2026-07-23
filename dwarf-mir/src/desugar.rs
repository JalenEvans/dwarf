//! Desugaring passes for MIR lowering.
//!
//! This module handles the desugaring of syntactic sugar from the HIR
//! into simpler MIR forms. Currently supports:
//! - Pipe operator (`|>`) desugaring

use dwarf_syntax::hir::{Expr, LiteralValue, BinaryOp, UnaryOp, Stmt, Pat, MatchArm};
use crate::*;

// ---------------------------------------------------------------------------
// Helper: literal value conversion
// ---------------------------------------------------------------------------

/// Convert an HIR literal value to its MIR equivalent.
///
/// `RawStr` is desugared to `Str` since the raw-string distinction is
/// purely syntactic and not needed in the MIR.
fn convert_literal(value: &LiteralValue) -> MirLiteral {
    match value {
        LiteralValue::Int(i) => MirLiteral::Int(*i),
        LiteralValue::Float(f) => MirLiteral::Float(*f),
        LiteralValue::Str(s) | LiteralValue::RawStr(s) => MirLiteral::Str(s.clone()),
        LiteralValue::Bool(b) => MirLiteral::Bool(*b),
        LiteralValue::Null => MirLiteral::Null,
    }
}

// ---------------------------------------------------------------------------
// Helper: operator conversion
// ---------------------------------------------------------------------------

/// Convert an HIR binary operator to its MIR equivalent.
fn convert_binary_op(op: BinaryOp) -> MirBinaryOp {
    match op {
        BinaryOp::Add => MirBinaryOp::Add,
        BinaryOp::Sub => MirBinaryOp::Sub,
        BinaryOp::Mul => MirBinaryOp::Mul,
        BinaryOp::Div => MirBinaryOp::Div,
        BinaryOp::Eq => MirBinaryOp::Eq,
        BinaryOp::Ne => MirBinaryOp::Ne,
        BinaryOp::Lt => MirBinaryOp::Lt,
        BinaryOp::Gt => MirBinaryOp::Gt,
        BinaryOp::Le => MirBinaryOp::Le,
        BinaryOp::Ge => MirBinaryOp::Ge,
        BinaryOp::And => MirBinaryOp::And,
        BinaryOp::Or => MirBinaryOp::Or,
    }
}

/// Convert an HIR unary operator to its MIR equivalent.
fn convert_unary_op(op: UnaryOp) -> MirUnaryOp {
    match op {
        UnaryOp::Neg => MirUnaryOp::Neg,
        UnaryOp::Not => MirUnaryOp::Not,
    }
}

// ---------------------------------------------------------------------------
// Helper: pattern conversion
// ---------------------------------------------------------------------------

/// Convert an HIR pattern to its MIR equivalent.
fn convert_pat(pat: Pat) -> MirPat {
    match pat {
        Pat::Wildcard => MirPat::Wildcard,
        Pat::Literal(v) => MirPat::Literal(convert_literal(&v)),
        Pat::Variable(name) => MirPat::Variable(name),
        Pat::Variant { name, arg } => MirPat::Variant {
            name,
            arg: arg.map(|a| Box::new(convert_pat(*a))),
        },
        Pat::Record { fields, rest } => MirPat::Record {
            fields: fields.into_iter().map(|(n, p)| (n, convert_pat(p))).collect(),
            rest,
        },
    }
}

// ---------------------------------------------------------------------------
// Helper: match arm conversion
// ---------------------------------------------------------------------------

/// Convert an HIR match arm to its MIR equivalent, recursively desugaring
/// the guard and body expressions.
fn convert_arm(arm: MatchArm) -> MirArm {
    MirArm {
        pattern: convert_pat(arm.pattern),
        guard: arm.guard.map(|g| desugar_pipe(&g)),
        body: desugar_pipe(&arm.body),
    }
}

// ---------------------------------------------------------------------------
// Helper: statement conversion
// ---------------------------------------------------------------------------

/// Convert an HIR statement to its MIR equivalent, recursively desugaring
/// any expressions inside.
fn convert_stmt(stmt: Stmt) -> MirStmt {
    match stmt {
        Stmt::Let(pat, expr) => MirStmt::Let {
            pat: convert_pat(pat),
            value: desugar_pipe(&expr),
        },
        Stmt::Expr(expr) => MirStmt::Expr(desugar_pipe(&expr)),
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Desugar the pipe operator (`|>`) in an expression.
///
/// The pipe operator passes the result of the left-hand side as the
/// first argument to the right-hand side:
///
/// - `a |> f`       → `f(a)`
/// - `a |> f(b)`    → `f(a, b)`
/// - `a |> f |> g`  → `g(f(a))`
///
/// Non-pipe expressions are converted to their MIR equivalent unchanged.
pub fn desugar_pipe(expr: &Expr) -> MirExpr {
    match expr {
        // ------------------------------------------------------------------
        // Pipe operator — the main desugaring logic
        // ------------------------------------------------------------------
        Expr::Pipe { lhs, rhs, span } => {
            // Recursively desugar both sides first (handles chained pipes).
            let desugared_lhs = desugar_pipe(lhs);
            let desugared_rhs = desugar_pipe(rhs);

            // Determine how to compose the call based on the desugared RHS form.
            match desugared_rhs {
                // a |> f               →  f(a)
                MirExpr::Variable { name, span: rhs_span } => MirExpr::Call {
                    func: Box::new(MirExpr::Variable { name, span: rhs_span }),
                    args: vec![desugared_lhs],
                    span: *span,
                },

                // a |> f(b)            →  f(a, b)
                // a |> (f |> g)(b)     →  g(f(a), b)   (chained pipe as callee)
                MirExpr::Call { func, args, span: _rhs_span } => MirExpr::Call {
                    func,
                    args: std::iter::once(desugared_lhs).chain(args).collect(),
                    span: *span,
                },

                // x |> obj.method      →  obj.method(x)
                MirExpr::Member { obj, field, span: rhs_span } => MirExpr::Call {
                    func: Box::new(MirExpr::Member { obj, field, span: rhs_span }),
                    args: vec![desugared_lhs],
                    span: *span,
                },

                // a |> <complex_expr>  →  <complex_expr>(a)
                // Fallback for any other expression form on the RHS.
                other => MirExpr::Call {
                    func: Box::new(other),
                    args: vec![desugared_lhs],
                    span: *span,
                },
            }
        }

        // ------------------------------------------------------------------
        // Passthrough — convert HIR expressions to their MIR equivalents
        // ------------------------------------------------------------------

        Expr::Literal { value, span } => MirExpr::Literal {
            value: convert_literal(value),
            span: *span,
        },

        Expr::Variable { name, span } => MirExpr::Variable {
            name: name.clone(),
            span: *span,
        },

        Expr::Call { func, args, span } => MirExpr::Call {
            func: Box::new(desugar_pipe(func)),
            args: args.iter().map(desugar_pipe).collect(),
            span: *span,
        },

        Expr::Member { obj, field, span } => MirExpr::Member {
            obj: Box::new(desugar_pipe(obj)),
            field: field.clone(),
            span: *span,
        },

        Expr::If { cond, then, else_, span } => MirExpr::If {
            cond: Box::new(desugar_pipe(cond)),
            then: Box::new(desugar_pipe(then)),
            else_: else_.as_ref().map(|e| Box::new(desugar_pipe(e))),
            span: *span,
        },

        Expr::Match { expr, arms, span } => MirExpr::Match {
            expr: Box::new(desugar_pipe(expr)),
            arms: arms.iter().map(|a| convert_arm(a.clone())).collect(),
            span: *span,
        },

        Expr::Block { stmts, span } => MirExpr::Block {
            stmts: stmts.iter().map(|s| convert_stmt(s.clone())).collect(),
            span: *span,
        },

        Expr::Binary { op, lhs, rhs, span } => MirExpr::Binary {
            op: convert_binary_op(op.clone()),
            lhs: Box::new(desugar_pipe(lhs)),
            rhs: Box::new(desugar_pipe(rhs)),
            span: *span,
        },

        Expr::Unary { op, expr, span } => MirExpr::Unary {
            op: convert_unary_op(op.clone()),
            expr: Box::new(desugar_pipe(expr)),
            span: *span,
        },

        Expr::Wildcard { span } => MirExpr::Wildcard { span: *span },

        Expr::Array { items, span } => MirExpr::Array {
            items: items.iter().map(desugar_pipe).collect(),
            span: *span,
        },

        Expr::Record { fields, span } => MirExpr::Record {
            fields: fields.iter().map(|(n, e)| (n.clone(), desugar_pipe(e))).collect(),
            span: *span,
        },

        Expr::Variant { name, arg, span } => MirExpr::Variant {
            name: name.clone(),
            arg: arg.as_ref().map(|a| Box::new(desugar_pipe(a))),
            span: *span,
        },

        Expr::Assign { target, value, span } => MirExpr::Assign {
            target: Box::new(desugar_pipe(target)),
            value: Box::new(desugar_pipe(value)),
            span: *span,
        },

        Expr::Lambda { params, body, span } => MirExpr::Lambda {
            params: params
                .iter()
                .map(|p| MirParam {
                    name: p.name.clone(),
                    type_: p.type_.clone(),
                })
                .collect(),
            body: Box::new(desugar_pipe(body)),
            span: *span,
        },

        // Propagate and For don't have direct MirExpr equivalents yet;
        // they will be desugared in separate passes. For now, recursively
        // desugar the inner expressions so the function is total over Expr.
        Expr::Propagate { expr, span: _ } => desugar_pipe(expr),

        Expr::For {
            binding,
            iterable,
            body,
            span: _,
        } => {
            // Desugar the iterable and body recursively, then wrap in a
            // temporary for-loop construct that a later pass will further
            // lower. We use a simple Call with a sentinel name to preserve
            // structure without inventing new MirExpr variants.
            let _ = binding; // consumed in a future desugaring pass
            MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "__for_loop".into(),
                    span: Span::default(),
                }),
                args: vec![desugar_pipe(iterable), desugar_pipe(body)],
                span: Span::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dwarf_syntax::hir::{Expr, LiteralValue};
    use dwarf_syntax::span::Span;
    use crate::*;
    use crate::desugar::desugar_pipe;

    /// Shared zero-length synthetic span for test expressions.
    fn span() -> Span {
        Span::new(0, 0, 0)
    }

    // ------------------------------------------------------------------
    // Pipe with Variable RHS  —  a |> f  =>  f(a)
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_variable_rhs() {
        let s = span();
        let input = Expr::Pipe {
            lhs: Box::new(Expr::Variable { name: "a".into(), span: s }),
            rhs: Box::new(Expr::Variable { name: "f".into(), span: s }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable { name: "f".into(), span: s }),
            args: vec![MirExpr::Variable { name: "a".into(), span: s }],
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Pipe with Call RHS  —  a |> f(b)  =>  f(a, b)
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_call_rhs() {
        let s = span();
        let input = Expr::Pipe {
            lhs: Box::new(Expr::Variable { name: "a".into(), span: s }),
            rhs: Box::new(Expr::Call {
                func: Box::new(Expr::Variable { name: "f".into(), span: s }),
                args: vec![Expr::Variable { name: "b".into(), span: s }],
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable { name: "f".into(), span: s }),
            args: vec![
                MirExpr::Variable { name: "a".into(), span: s },
                MirExpr::Variable { name: "b".into(), span: s },
            ],
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Pipe chain  —  a |> f |> g  =>  g(f(a))
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_chain() {
        let s = span();
        let inner_pipe = Expr::Pipe {
            lhs: Box::new(Expr::Variable { name: "a".into(), span: s }),
            rhs: Box::new(Expr::Variable { name: "f".into(), span: s }),
            span: s,
        };
        let input = Expr::Pipe {
            lhs: Box::new(inner_pipe),
            rhs: Box::new(Expr::Variable { name: "g".into(), span: s }),
            span: s,
        };

        let result = desugar_pipe(&input);

        // g(f(a))
        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable { name: "g".into(), span: s }),
            args: vec![MirExpr::Call {
                func: Box::new(MirExpr::Variable { name: "f".into(), span: s }),
                args: vec![MirExpr::Variable { name: "a".into(), span: s }],
                span: s,
            }],
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Pipe with literal LHS  —  42 |> f  =>  f(42)
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_with_literal_lhs() {
        let s = span();
        let input = Expr::Pipe {
            lhs: Box::new(Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            }),
            rhs: Box::new(Expr::Variable { name: "f".into(), span: s }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable { name: "f".into(), span: s }),
            args: vec![MirExpr::Literal {
                value: MirLiteral::Int(42),
                span: s,
            }],
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Pipe with Member RHS  —  x |> obj.method  =>  obj.method(x)
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_with_member_rhs() {
        let s = span();
        let input = Expr::Pipe {
            lhs: Box::new(Expr::Variable { name: "x".into(), span: s }),
            rhs: Box::new(Expr::Member {
                obj: Box::new(Expr::Variable { name: "obj".into(), span: s }),
                field: "method".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable { name: "obj".into(), span: s }),
                field: "method".into(),
                span: s,
            }),
            args: vec![MirExpr::Variable { name: "x".into(), span: s }],
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Non-pipe passthrough  —  a regular expression converts to MIR
    // ------------------------------------------------------------------

    #[test]
    fn test_pipe_non_pipe_passthrough() {
        let s = span();
        let input = Expr::Variable {
            name: "x".into(),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Variable {
            name: "x".into(),
            span: s,
        };
        assert_eq!(result, expected);
    }
}
