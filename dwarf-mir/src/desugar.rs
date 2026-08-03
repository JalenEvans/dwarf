//! Desugaring passes for MIR lowering.
//!
//! This module handles the desugaring of syntactic sugar from the HIR
//! into simpler MIR forms. Currently supports:
//! - Pipe operator (`|>`) desugaring

use crate::*;
use dwarf_syntax::hir::{BinaryOp, Decl, Expr, LiteralValue, MatchArm, Pat, Stmt, UnaryOp};

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
            fields: fields
                .into_iter()
                .map(|(n, p)| (n, convert_pat(p)))
                .collect(),
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
                MirExpr::Variable {
                    name,
                    span: rhs_span,
                } => MirExpr::Call {
                    func: Box::new(MirExpr::Variable {
                        name,
                        span: rhs_span,
                    }),
                    args: vec![desugared_lhs],
                    span: *span,
                },

                // a |> f(b)            →  f(a, b)
                // a |> (f |> g)(b)     →  g(f(a), b)   (chained pipe as callee)
                MirExpr::Call {
                    func,
                    args,
                    span: _rhs_span,
                } => MirExpr::Call {
                    func,
                    args: std::iter::once(desugared_lhs).chain(args).collect(),
                    span: *span,
                },

                // x |> obj.method      →  obj.method(x)
                MirExpr::Member {
                    obj,
                    field,
                    span: rhs_span,
                } => MirExpr::Call {
                    func: Box::new(MirExpr::Member {
                        obj,
                        field,
                        span: rhs_span,
                    }),
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

        Expr::If {
            cond,
            then,
            else_,
            span,
        } => MirExpr::If {
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
            fields: fields
                .iter()
                .map(|(n, e)| (n.clone(), desugar_pipe(e)))
                .collect(),
            span: *span,
        },

        Expr::Variant { name, arg, span } => MirExpr::Variant {
            name: name.clone(),
            arg: arg.as_ref().map(|a| Box::new(desugar_pipe(a))),
            span: *span,
        },

        Expr::Assign {
            target,
            value,
            span,
        } => MirExpr::Assign {
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

        // Propagate is preserved through MIR so the backend can emit
        // target-specific error propagation.
        Expr::Propagate { expr, span } => MirExpr::Propagate {
            expr: Box::new(desugar_pipe(expr)),
            span: *span,
        },

        // For doesn't have a direct MirExpr equivalent yet; it is desugared
        // in a separate pass. For now, recursively desugar the inner
        // expressions so the function is total over Expr.
        Expr::For {
            binding: _,
            iterable,
            body,
            span,
        } => {
            // For-loops are desugared in a separate pass. For pipe desugaring,
            // recursively desugar the iterable and body.
            let _ = span;
            MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "__for_loop".into(),
                    span: *span,
                }),
                args: vec![desugar_pipe(iterable), desugar_pipe(body)],
                span: *span,
            }
        }

        Expr::ForAll {
            type_,
            binding,
            property,
            span,
        } => MirExpr::ForAll {
            type_: type_.clone(),
            binding: convert_pat(binding.clone()),
            property: Box::new(desugar_pipe(property)),
            span: *span,
        },

        Expr::AssertConsistent { expr, span } => MirExpr::AssertConsistent {
            expr: Box::new(desugar_pipe(expr)),
            span: *span,
        },

        // Try/Throw expressions are preserved through MIR so the backend can
        // emit target-specific error handling.
        Expr::Try {
            body,
            binding,
            guard,
            handler,
            span,
        } => MirExpr::Try {
            body: Box::new(desugar_pipe(body)),
            binding: convert_pat(binding.clone()),
            guard: guard.as_ref().map(|g| Box::new(desugar_pipe(g))),
            handler: Box::new(desugar_pipe(handler)),
            span: *span,
        },
        Expr::Throw { expr, span } => MirExpr::Throw {
            expr: Box::new(desugar_pipe(expr)),
            span: *span,
        },
    }
}

// ---------------------------------------------------------------------------
// Propagation desugaring
// ---------------------------------------------------------------------------

/// Desugar the propagation operator (`?`) in an expression.
///
/// The `?` operator unwraps Result/Option types, propagating the error
/// case upward. It is preserved as `MirExpr::Propagate` so the backend can
/// emit target-specific error propagation (e.g. `isErr` checks in
/// TypeScript).
///
/// Non-propagate expressions are converted to their MIR equivalent unchanged
/// (passthrough to `desugar_pipe`).
pub fn desugar_propagate(expr: &Expr) -> MirExpr {
    match expr {
        Expr::Propagate { expr: inner, span } => MirExpr::Propagate {
            expr: Box::new(desugar_pipe(inner)),
            span: *span,
        },

        // Non-propagate expressions pass through unchanged.
        other => desugar_pipe(other),
    }
}

// ---------------------------------------------------------------------------
// For-loop desugaring
// ---------------------------------------------------------------------------

/// Desugar a for-loop expression into a let + loop + match.
///
/// `for x in iterable { body }` becomes:
///
/// ```ignore
/// {
///     let __iter = iterable;
///     loop {
///         match __iter.next() {
///             Some(x) => { body },
///             None => break,
///         }
///     }
/// }
/// ```
///
/// The `__break` variable is used as a marker for the break expression
/// (analogous to `__propagate` in the propagation desugaring pass).
pub fn desugar_for_loop(expr: &Expr) -> MirExpr {
    match expr {
        Expr::For {
            binding,
            iterable,
            body,
            span,
        } => {
            let span = *span;

            // Desugar the iterable and body through the full pipeline
            // (pipe → propagate).
            let mir_iterable = desugar_propagate(iterable);
            let mir_body = desugar_propagate(body);

            // Convert the for-loop binding pattern
            let mir_binding = convert_pat(binding.clone());

            // Build the desugared form:
            //
            //   {
            //       let __iter = iterable;
            //       loop {
            //           match __iter.next() {
            //               Some(binding) => body,
            //               None => __break,
            //           }
            //       }
            //   }
            MirExpr::Block {
                span,
                stmts: vec![
                    // let __iter = iterable
                    MirStmt::Let {
                        pat: MirPat::Variable("__iter".into()),
                        value: mir_iterable,
                    },
                    // loop { match __iter.next() { Some(binding) => body, None => __break } }
                    MirStmt::Expr(MirExpr::Loop {
                        span,
                        body: Box::new(MirExpr::Match {
                            span,
                            expr: Box::new(MirExpr::Call {
                                span,
                                func: Box::new(MirExpr::Member {
                                    span,
                                    obj: Box::new(MirExpr::Variable {
                                        name: "__iter".into(),
                                        span,
                                    }),
                                    field: "next".into(),
                                }),
                                args: vec![],
                            }),
                            arms: vec![
                                MirArm {
                                    pattern: MirPat::Variant {
                                        name: "Some".into(),
                                        arg: Some(Box::new(mir_binding)),
                                    },
                                    guard: None,
                                    body: mir_body,
                                },
                                MirArm {
                                    pattern: MirPat::Variant {
                                        name: "None".into(),
                                        arg: None,
                                    },
                                    guard: None,
                                    body: MirExpr::Variable {
                                        name: "__break".into(),
                                        span,
                                    },
                                },
                            ],
                        }),
                    }),
                ],
            }
        }

        // Non-for-loop expressions pass through unchanged via the full
        // desugaring pipeline (pipe → propagate).
        other => desugar_propagate(other),
    }
}

// ---------------------------------------------------------------------------
// Type alias expansion
// ---------------------------------------------------------------------------

/// Convert an HIR parameter to its MIR equivalent.
fn convert_param(p: &dwarf_syntax::hir::Param) -> MirParam {
    MirParam {
        name: p.name.clone(),
        type_: p.type_.clone(),
    }
}

/// Filter type aliases from declarations — MIR doesn't carry type aliases
/// (they're resolved in the TypeRegistry). Returns only function, record, and union declarations.
pub fn expand_type_aliases(decls: &[Decl]) -> Vec<MirDecl> {
    decls
        .iter()
        .filter_map(|decl| match decl {
            // Type aliases are resolved in the TypeRegistry — exclude from MIR.
            Decl::TypeDef { .. } => None,

            // Imports are resolved during name resolution — exclude from MIR.
            Decl::Import { .. } => None,

            // Decorators are handled by a separate decorator pass.
            Decl::Decorator { .. } => None,

            // Const declarations are value bindings — exclude from MIR for now.
            // (Future work: lower to a synthetic getter function or global.)
            Decl::Const { .. } => None,

            // Function declarations pass through with desugared bodies.
            Decl::Function {
                name,
                params,
                return_type,
                body,
                is_pub,
                span,
            } => Some(MirDecl::Function {
                name: name.clone(),
                params: params.iter().map(convert_param).collect(),
                return_type: return_type.clone(),
                body: desugar_for_loop(body),
                is_pub: *is_pub,
                is_generator: false,
                span: *span,
            }),

            // Record type definitions pass through with converted fields.
            Decl::RecordDef {
                name,
                fields,
                is_pub,
                span,
            } => Some(MirDecl::RecordDef {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|f| MirField {
                        name: f.name.clone(),
                        type_: f.type_.clone(),
                    })
                    .collect(),
                is_pub: *is_pub,
                span: *span,
            }),

            // Union type definitions pass through with converted variants.
            Decl::UnionDef {
                name,
                variants,
                is_pub,
                span,
            } => Some(MirDecl::UnionDef {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|v| MirVariant {
                        name: v.name.clone(),
                        arg: v.arg.clone(),
                    })
                    .collect(),
                is_pub: *is_pub,
                span: *span,
            }),

            // Extern declarations pass through with converted params.
            Decl::Extern {
                source,
                name,
                params,
                return_type,
                is_pub,
                span: _,
            } => Some(MirDecl::Extern {
                source: source.clone(),
                name: name.clone(),
                params: params.iter().map(convert_param).collect(),
                return_type: return_type.clone(),
                is_pub: *is_pub,
            }),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Decorator desugaring
// ---------------------------------------------------------------------------

/// Desugar decorator declarations.
///
/// `@decorator fn f(x) { body }` becomes:
///
/// ```ignore
/// fn f(x) { decorator(fn __inner(x) { body }, x) }
/// ```
///
/// Non-decorator declarations are delegated to `expand_type_aliases`.
pub fn desugar_decorators(decls: &[Decl]) -> Vec<MirDecl> {
    let mut result = Vec::new();

    for decl in decls {
        match decl {
            Decl::Decorator {
                name,
                args,
                target,
                is_pub: _,
                span,
            } => {
                // Extract the inner function from the decorated target.
                if let Decl::Function {
                    name: func_name,
                    params,
                    return_type,
                    body,
                    is_pub: func_is_pub,
                    span: func_span,
                } = target.as_ref().clone()
                {
                    // Convert the original function's parameters to MIR params.
                    let mir_params: Vec<MirParam> = params.iter().map(convert_param).collect();

                    // Build a lambda that wraps the original function body.
                    let lambda = MirExpr::Lambda {
                        params: mir_params.clone(),
                        body: Box::new(desugar_for_loop(&body)),
                        span: func_span,
                    };

                    // Convert decorator arguments (e.g., @route("/api") → "/api").
                    let mir_args: Vec<MirExpr> = args.iter().map(desugar_for_loop).collect();

                    // Pass the original function params as additional args
                    // so the decorator receives both the wrapped function and
                    // the original call-site arguments.
                    let param_vars: Vec<MirExpr> = params
                        .iter()
                        .map(|p| MirExpr::Variable {
                            name: p.name.clone(),
                            span: func_span,
                        })
                        .collect();

                    // xUnit decorators get special Jest-like desugaring.
                    // @Test → it(), @Suite → describe(), @Before → beforeEach(), etc.
                    match name.as_str() {
                        "Test" | "Suite" | "Before" | "After" | "BeforeAll" | "AfterAll" => {
                            let jest_name = match name.as_str() {
                                "Test" => "it",
                                "Suite" => "describe",
                                "Before" => "beforeEach",
                                "After" => "afterEach",
                                "BeforeAll" => "beforeAll",
                                "AfterAll" => "afterAll",
                                _ => unreachable!(),
                            };

                            let mut jest_args: Vec<MirExpr> = Vec::new();

                            // @Test and @Suite get the function name as first arg (string literal).
                            if name == "Test" || name == "Suite" {
                                jest_args.push(MirExpr::Literal {
                                    value: MirLiteral::Str(func_name.clone()),
                                    span: func_span,
                                });
                            }

                            // All xUnit decorators wrap the lambda.
                            jest_args.push(lambda);

                            result.push(MirDecl::Function {
                                name: func_name,
                                params: mir_params,
                                return_type,
                                body: MirExpr::Call {
                                    func: Box::new(MirExpr::Variable {
                                        name: jest_name.to_string(),
                                        span: *span,
                                    }),
                                    args: jest_args,
                                    span: *span,
                                },
                                is_pub: func_is_pub,
                                is_generator: false,
                                span: func_span,
                            });
                        }
                        "gen" => {
                            // @gen decorator — generator function.
                            // The function IS the generator; preserve the body as-is.
                            // The type argument from @gen(Type) becomes the return type.
                            let gen_type = mir_args.first().and_then(|arg| match arg {
                                MirExpr::Variable { name, .. } => {
                                    Some(dwarf_syntax::hir::Type::Named(name.clone()))
                                }
                                _ => None,
                            });

                            result.push(MirDecl::Function {
                                name: func_name,
                                params: mir_params,
                                return_type: gen_type.or(return_type),
                                body: desugar_for_loop(&body),
                                is_pub: func_is_pub,
                                is_generator: true,
                                span: func_span,
                            });
                        }
                        _ => {
                            // Generic decorator desugaring (unchanged for non-xUnit).
                            result.push(MirDecl::Function {
                                name: func_name,
                                params: mir_params,
                                return_type,
                                body: MirExpr::Call {
                                    func: Box::new(MirExpr::Variable {
                                        name: name.clone(),
                                        span: *span,
                                    }),
                                    args: std::iter::once(lambda)
                                        .chain(mir_args)
                                        .chain(param_vars)
                                        .collect(),
                                    span: *span,
                                },
                                is_pub: func_is_pub,
                                is_generator: false,
                                span: func_span,
                            });
                        }
                    }
                }
                // Non-function targets are currently not supported for
                // decoration — they are silently dropped.
            }
            other => {
                result.extend(expand_type_aliases(std::slice::from_ref(other)));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::desugar::{
        desugar_decorators, desugar_for_loop, desugar_pipe, desugar_propagate, expand_type_aliases,
    };
    use crate::*;
    use dwarf_syntax::hir::{
        BinaryOp, Decl, Expr, Field, LiteralValue, Param, Pat, Stmt, Type, Variant,
    };
    use dwarf_syntax::span::Span;

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
            lhs: Box::new(Expr::Variable {
                name: "a".into(),
                span: s,
            }),
            rhs: Box::new(Expr::Variable {
                name: "f".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "f".into(),
                span: s,
            }),
            args: vec![MirExpr::Variable {
                name: "a".into(),
                span: s,
            }],
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
            lhs: Box::new(Expr::Variable {
                name: "a".into(),
                span: s,
            }),
            rhs: Box::new(Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "f".into(),
                    span: s,
                }),
                args: vec![Expr::Variable {
                    name: "b".into(),
                    span: s,
                }],
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "f".into(),
                span: s,
            }),
            args: vec![
                MirExpr::Variable {
                    name: "a".into(),
                    span: s,
                },
                MirExpr::Variable {
                    name: "b".into(),
                    span: s,
                },
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
            lhs: Box::new(Expr::Variable {
                name: "a".into(),
                span: s,
            }),
            rhs: Box::new(Expr::Variable {
                name: "f".into(),
                span: s,
            }),
            span: s,
        };
        let input = Expr::Pipe {
            lhs: Box::new(inner_pipe),
            rhs: Box::new(Expr::Variable {
                name: "g".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        // g(f(a))
        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "g".into(),
                span: s,
            }),
            args: vec![MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "f".into(),
                    span: s,
                }),
                args: vec![MirExpr::Variable {
                    name: "a".into(),
                    span: s,
                }],
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
            rhs: Box::new(Expr::Variable {
                name: "f".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "f".into(),
                span: s,
            }),
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
            lhs: Box::new(Expr::Variable {
                name: "x".into(),
                span: s,
            }),
            rhs: Box::new(Expr::Member {
                obj: Box::new(Expr::Variable {
                    name: "obj".into(),
                    span: s,
                }),
                field: "method".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_pipe(&input);

        let expected = MirExpr::Call {
            func: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "obj".into(),
                    span: s,
                }),
                field: "method".into(),
                span: s,
            }),
            args: vec![MirExpr::Variable {
                name: "x".into(),
                span: s,
            }],
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

    // ------------------------------------------------------------------
    // Propagation desugaring tests
    // ------------------------------------------------------------------

    #[test]
    fn test_propagate_simple() {
        let s = span();
        let input = Expr::Propagate {
            expr: Box::new(Expr::Variable {
                name: "result".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_propagate(&input);

        let expected = MirExpr::Propagate {
            expr: Box::new(MirExpr::Variable {
                name: "result".into(),
                span: s,
            }),
            span: s,
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_propagate_nested() {
        let s = span();
        // (obj.method)? — inner expression is a member access
        let input = Expr::Propagate {
            expr: Box::new(Expr::Member {
                obj: Box::new(Expr::Variable {
                    name: "obj".into(),
                    span: s,
                }),
                field: "method".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_propagate(&input);

        let expected = MirExpr::Propagate {
            expr: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "obj".into(),
                    span: s,
                }),
                field: "method".into(),
                span: s,
            }),
            span: s,
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_propagate_non_propagate_passthrough() {
        let s = span();
        let input = Expr::Variable {
            name: "x".into(),
            span: s,
        };

        let result = desugar_propagate(&input);

        let expected = MirExpr::Variable {
            name: "x".into(),
            span: s,
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_propagate_preserves_operator() {
        let s = span();
        let input = Expr::Propagate {
            expr: Box::new(Expr::Variable {
                name: "x".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_propagate(&input);

        // Verify it's preserved as MirExpr::Propagate
        if let MirExpr::Propagate { expr, .. } = &result {
            assert_eq!(
                expr.as_ref(),
                &MirExpr::Variable {
                    name: "x".into(),
                    span: s,
                }
            );
        } else {
            panic!("desugar_propagate should produce a Propagate expression");
        }
    }

    // ------------------------------------------------------------------
    // For-loop desugaring tests
    // ------------------------------------------------------------------

    #[test]
    fn test_for_loop_simple() {
        let s = span();
        let input = Expr::For {
            binding: Pat::Variable("x".into()),
            iterable: Box::new(Expr::Variable {
                name: "iter".into(),
                span: s,
            }),
            body: Box::new(Expr::Variable {
                name: "body".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_for_loop(&input);

        // Expected: { let __iter = iter; loop { match __iter.next() { Some(x) => body, None => __break } } }
        let expected = MirExpr::Block {
            span: s,
            stmts: vec![
                MirStmt::Let {
                    pat: MirPat::Variable("__iter".into()),
                    value: MirExpr::Variable {
                        name: "iter".into(),
                        span: s,
                    },
                },
                MirStmt::Expr(MirExpr::Loop {
                    span: s,
                    body: Box::new(MirExpr::Match {
                        span: s,
                        expr: Box::new(MirExpr::Call {
                            span: s,
                            func: Box::new(MirExpr::Member {
                                span: s,
                                obj: Box::new(MirExpr::Variable {
                                    name: "__iter".into(),
                                    span: s,
                                }),
                                field: "next".into(),
                            }),
                            args: vec![],
                        }),
                        arms: vec![
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "Some".into(),
                                    arg: Some(Box::new(MirPat::Variable("x".into()))),
                                },
                                guard: None,
                                body: MirExpr::Variable {
                                    name: "body".into(),
                                    span: s,
                                },
                            },
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "None".into(),
                                    arg: None,
                                },
                                guard: None,
                                body: MirExpr::Variable {
                                    name: "__break".into(),
                                    span: s,
                                },
                            },
                        ],
                    }),
                }),
            ],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_for_loop_with_record_pattern() {
        let s = span();
        let input = Expr::For {
            binding: Pat::Record {
                fields: vec![
                    ("a".into(), Pat::Variable("x".into())),
                    ("b".into(), Pat::Variable("y".into())),
                ],
                rest: false,
            },
            iterable: Box::new(Expr::Variable {
                name: "iter".into(),
                span: s,
            }),
            body: Box::new(Expr::Variable {
                name: "body".into(),
                span: s,
            }),
            span: s,
        };

        let result = desugar_for_loop(&input);

        // The record binding pattern should be preserved inside Some(...)
        let expected = MirExpr::Block {
            span: s,
            stmts: vec![
                MirStmt::Let {
                    pat: MirPat::Variable("__iter".into()),
                    value: MirExpr::Variable {
                        name: "iter".into(),
                        span: s,
                    },
                },
                MirStmt::Expr(MirExpr::Loop {
                    span: s,
                    body: Box::new(MirExpr::Match {
                        span: s,
                        expr: Box::new(MirExpr::Call {
                            span: s,
                            func: Box::new(MirExpr::Member {
                                span: s,
                                obj: Box::new(MirExpr::Variable {
                                    name: "__iter".into(),
                                    span: s,
                                }),
                                field: "next".into(),
                            }),
                            args: vec![],
                        }),
                        arms: vec![
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "Some".into(),
                                    arg: Some(Box::new(MirPat::Record {
                                        fields: vec![
                                            ("a".into(), MirPat::Variable("x".into())),
                                            ("b".into(), MirPat::Variable("y".into())),
                                        ],
                                        rest: false,
                                    })),
                                },
                                guard: None,
                                body: MirExpr::Variable {
                                    name: "body".into(),
                                    span: s,
                                },
                            },
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "None".into(),
                                    arg: None,
                                },
                                guard: None,
                                body: MirExpr::Variable {
                                    name: "__break".into(),
                                    span: s,
                                },
                            },
                        ],
                    }),
                }),
            ],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_for_loop_empty_body() {
        let s = span();
        let input = Expr::For {
            binding: Pat::Variable("x".into()),
            iterable: Box::new(Expr::Variable {
                name: "iter".into(),
                span: s,
            }),
            body: Box::new(Expr::Block {
                stmts: vec![],
                span: s,
            }),
            span: s,
        };

        let result = desugar_for_loop(&input);

        // The body should still be an empty Block inside Some(...)
        let expected = MirExpr::Block {
            span: s,
            stmts: vec![
                MirStmt::Let {
                    pat: MirPat::Variable("__iter".into()),
                    value: MirExpr::Variable {
                        name: "iter".into(),
                        span: s,
                    },
                },
                MirStmt::Expr(MirExpr::Loop {
                    span: s,
                    body: Box::new(MirExpr::Match {
                        span: s,
                        expr: Box::new(MirExpr::Call {
                            span: s,
                            func: Box::new(MirExpr::Member {
                                span: s,
                                obj: Box::new(MirExpr::Variable {
                                    name: "__iter".into(),
                                    span: s,
                                }),
                                field: "next".into(),
                            }),
                            args: vec![],
                        }),
                        arms: vec![
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "Some".into(),
                                    arg: Some(Box::new(MirPat::Variable("x".into()))),
                                },
                                guard: None,
                                body: MirExpr::Block {
                                    span: s,
                                    stmts: vec![],
                                },
                            },
                            MirArm {
                                pattern: MirPat::Variant {
                                    name: "None".into(),
                                    arg: None,
                                },
                                guard: None,
                                body: MirExpr::Variable {
                                    name: "__break".into(),
                                    span: s,
                                },
                            },
                        ],
                    }),
                }),
            ],
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_for_loop_non_for_passthrough() {
        let s = span();
        let input = Expr::Variable {
            name: "x".into(),
            span: s,
        };

        let result = desugar_for_loop(&input);

        let expected = MirExpr::Variable {
            name: "x".into(),
            span: s,
        };
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // Type alias expansion tests
    // ------------------------------------------------------------------

    #[test]
    fn test_expand_type_alias_removes_typedef() {
        let s = span();
        let decls = vec![Decl::TypeDef {
            name: "MyInt".into(),
            type_: Type::Named("Int".into()),
            is_pub: false,
            span: s,
        }];
        let result = expand_type_aliases(&decls);
        assert!(
            result.is_empty(),
            "TypeDef should be filtered out from MIR output"
        );
    }

    #[test]
    fn test_expand_type_alias_keeps_function() {
        let s = span();
        let decls = vec![Decl::Function {
            name: "foo".into(),
            params: vec![Param {
                name: "x".into(),
                type_: Some(Type::Named("Int".into())),
            }],
            return_type: Some(Type::Named("Int".into())),
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            },
            is_pub: true,
            span: s,
        }];
        let result = expand_type_aliases(&decls);
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], MirDecl::Function { name, .. } if name == "foo"),
            "Function declarations should pass through as MirDecl::Function"
        );
    }

    #[test]
    fn test_expand_type_alias_keeps_record() {
        let s = span();
        let decls = vec![Decl::RecordDef {
            name: "Point".into(),
            fields: vec![
                Field {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                Field {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: s,
        }];
        let result = expand_type_aliases(&decls);
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], MirDecl::RecordDef { name, .. } if name == "Point"),
            "RecordDef declarations should pass through as MirDecl::RecordDef"
        );
    }

    #[test]
    fn test_expand_type_alias_keeps_union() {
        let s = span();
        let decls = vec![Decl::UnionDef {
            name: "Option".into(),
            variants: vec![
                Variant {
                    name: "Some".into(),
                    arg: Some(Type::Named("Int".into())),
                },
                Variant {
                    name: "None".into(),
                    arg: None,
                },
            ],
            is_pub: true,
            span: s,
        }];
        let result = expand_type_aliases(&decls);
        assert_eq!(result.len(), 1);
        assert!(
            matches!(&result[0], MirDecl::UnionDef { name, .. } if name == "Option"),
            "UnionDef declarations should pass through as MirDecl::UnionDef"
        );
    }

    #[test]
    fn test_expand_type_alias_empty_input() {
        let decls: Vec<Decl> = vec![];
        let result = expand_type_aliases(&decls);
        assert!(result.is_empty(), "Empty input should return empty output");
    }

    #[test]
    fn test_expand_type_alias_multiple() {
        let s = span();
        let decls = vec![
            Decl::TypeDef {
                name: "MyInt".into(),
                type_: Type::Named("Int".into()),
                is_pub: false,
                span: s,
            },
            Decl::Function {
                name: "foo".into(),
                params: vec![],
                return_type: None,
                body: Expr::Literal {
                    value: LiteralValue::Int(42),
                    span: s,
                },
                is_pub: true,
                span: s,
            },
            Decl::RecordDef {
                name: "Point".into(),
                fields: vec![],
                is_pub: true,
                span: s,
            },
        ];
        let result = expand_type_aliases(&decls);
        assert_eq!(
            result.len(),
            2,
            "TypeDef should be filtered, Function and RecordDef should remain"
        );
        assert!(
            matches!(&result[0], MirDecl::Function { name, .. } if name == "foo"),
            "First output should be the Function"
        );
        assert!(
            matches!(&result[1], MirDecl::RecordDef { name, .. } if name == "Point"),
            "Second output should be the RecordDef"
        );
    }

    // ------------------------------------------------------------------
    // Decorator desugaring tests
    // ------------------------------------------------------------------

    #[test]
    fn test_decorator_plain() {
        let s = span();
        let inner = Decl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "log".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // @log fn f() { 42 }  →  fn f() { log(fn() { 42 }) }
        let expected = vec![MirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "log".into(),
                    span: s,
                }),
                args: vec![MirExpr::Lambda {
                    params: vec![],
                    body: Box::new(MirExpr::Literal {
                        value: MirLiteral::Int(42),
                        span: s,
                    }),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decorator_with_args() {
        let s = span();
        let inner = Decl::Function {
            name: "get".into(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "route".into(),
            args: vec![Expr::Literal {
                value: LiteralValue::Str("/api".into()),
                span: s,
            }],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // @route("/api") fn get() { 42 }
        // → fn get() { route(fn() { 42 }, "/api") }
        let expected = vec![MirDecl::Function {
            name: "get".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "route".into(),
                    span: s,
                }),
                args: vec![
                    MirExpr::Lambda {
                        params: vec![],
                        body: Box::new(MirExpr::Literal {
                            value: MirLiteral::Int(42),
                            span: s,
                        }),
                        span: s,
                    },
                    MirExpr::Literal {
                        value: MirLiteral::Str("/api".into()),
                        span: s,
                    },
                ],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decorator_non_decorator() {
        let s = span();
        let input = Decl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            },
            is_pub: true,
            span: s,
        };

        let result = desugar_decorators(std::slice::from_ref(&input));

        // Non-decorator declarations delegate to expand_type_aliases.
        let expected = expand_type_aliases(&[input]);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decorator_empty_decls() {
        let input: Vec<Decl> = vec![];
        let result = desugar_decorators(&input);
        assert!(result.is_empty(), "Empty input should return empty output");
    }

    // ------------------------------------------------------------------
    // xUnit decorator desugaring (@Test, @Suite, @Before, etc.)
    // ------------------------------------------------------------------

    #[test]
    fn test_desugar_decorator_test() {
        let s = span();
        // @Test fn my_test() { assert.ok(true) }
        let inner = Decl::Function {
            name: "my_test".into(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Member {
                    obj: Box::new(Expr::Variable {
                        name: "assert".into(),
                        span: s,
                    }),
                    field: "ok".into(),
                    span: s,
                }),
                args: vec![Expr::Literal {
                    value: LiteralValue::Bool(true),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "Test".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn my_test() { it("my_test", fn() { assert.ok(true) }) }
        let desugared_body = MirExpr::Call {
            func: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "assert".into(),
                    span: s,
                }),
                field: "ok".into(),
                span: s,
            }),
            args: vec![MirExpr::Literal {
                value: MirLiteral::Bool(true),
                span: s,
            }],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "my_test".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "it".into(),
                    span: s,
                }),
                args: vec![
                    MirExpr::Literal {
                        value: MirLiteral::Str("my_test".into()),
                        span: s,
                    },
                    MirExpr::Lambda {
                        params: vec![],
                        body: Box::new(desugared_body),
                        span: s,
                    },
                ],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_decorator_suite() {
        let s = span();
        // @Suite fn math_tests() { add(1, 1); sub(2, 1) }
        let inner = Decl::Function {
            name: "math_tests".into(),
            params: vec![],
            return_type: None,
            body: Expr::Block {
                stmts: vec![
                    Stmt::Expr(Expr::Call {
                        func: Box::new(Expr::Variable {
                            name: "add".into(),
                            span: s,
                        }),
                        args: vec![
                            Expr::Literal {
                                value: LiteralValue::Int(1),
                                span: s,
                            },
                            Expr::Literal {
                                value: LiteralValue::Int(1),
                                span: s,
                            },
                        ],
                        span: s,
                    }),
                    Stmt::Expr(Expr::Call {
                        func: Box::new(Expr::Variable {
                            name: "sub".into(),
                            span: s,
                        }),
                        args: vec![
                            Expr::Literal {
                                value: LiteralValue::Int(2),
                                span: s,
                            },
                            Expr::Literal {
                                value: LiteralValue::Int(1),
                                span: s,
                            },
                        ],
                        span: s,
                    }),
                ],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "Suite".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn math_tests() { describe("math_tests", fn() { add(1,1); sub(2,1) }) }
        let desugared_body = MirExpr::Block {
            stmts: vec![
                MirStmt::Expr(MirExpr::Call {
                    func: Box::new(MirExpr::Variable {
                        name: "add".into(),
                        span: s,
                    }),
                    args: vec![
                        MirExpr::Literal {
                            value: MirLiteral::Int(1),
                            span: s,
                        },
                        MirExpr::Literal {
                            value: MirLiteral::Int(1),
                            span: s,
                        },
                    ],
                    span: s,
                }),
                MirStmt::Expr(MirExpr::Call {
                    func: Box::new(MirExpr::Variable {
                        name: "sub".into(),
                        span: s,
                    }),
                    args: vec![
                        MirExpr::Literal {
                            value: MirLiteral::Int(2),
                            span: s,
                        },
                        MirExpr::Literal {
                            value: MirLiteral::Int(1),
                            span: s,
                        },
                    ],
                    span: s,
                }),
            ],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "math_tests".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "describe".into(),
                    span: s,
                }),
                args: vec![
                    MirExpr::Literal {
                        value: MirLiteral::Str("math_tests".into()),
                        span: s,
                    },
                    MirExpr::Lambda {
                        params: vec![],
                        body: Box::new(desugared_body),
                        span: s,
                    },
                ],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_decorator_before() {
        let s = span();
        // @Before fn setup() { init() }
        let inner = Decl::Function {
            name: "setup".into(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "init".into(),
                    span: s,
                }),
                args: vec![],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "Before".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn setup() { beforeEach(fn() { init() }) }
        let desugared_body = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "init".into(),
                span: s,
            }),
            args: vec![],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "setup".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "beforeEach".into(),
                    span: s,
                }),
                args: vec![MirExpr::Lambda {
                    params: vec![],
                    body: Box::new(desugared_body),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_decorator_after() {
        let s = span();
        // @After fn cleanup() { reset() }
        let inner = Decl::Function {
            name: "cleanup".into(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "reset".into(),
                    span: s,
                }),
                args: vec![],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "After".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn cleanup() { afterEach(fn() { reset() }) }
        let desugared_body = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "reset".into(),
                span: s,
            }),
            args: vec![],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "cleanup".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "afterEach".into(),
                    span: s,
                }),
                args: vec![MirExpr::Lambda {
                    params: vec![],
                    body: Box::new(desugared_body),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_decorator_before_all() {
        let s = span();
        // @BeforeAll fn setup_all() { init_db() }
        let inner = Decl::Function {
            name: "setup_all".into(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "init_db".into(),
                    span: s,
                }),
                args: vec![],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "BeforeAll".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn setup_all() { beforeAll(fn() { init_db() }) }
        let desugared_body = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "init_db".into(),
                span: s,
            }),
            args: vec![],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "setup_all".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "beforeAll".into(),
                    span: s,
                }),
                args: vec![MirExpr::Lambda {
                    params: vec![],
                    body: Box::new(desugared_body),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_decorator_after_all() {
        let s = span();
        // @AfterAll fn cleanup_all() { close_db() }
        let inner = Decl::Function {
            name: "cleanup_all".into(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "close_db".into(),
                    span: s,
                }),
                args: vec![],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "AfterAll".into(),
            args: vec![],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn cleanup_all() { afterAll(fn() { close_db() }) }
        let desugared_body = MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: "close_db".into(),
                span: s,
            }),
            args: vec![],
            span: s,
        };
        let expected = vec![MirDecl::Function {
            name: "cleanup_all".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "afterAll".into(),
                    span: s,
                }),
                args: vec![MirExpr::Lambda {
                    params: vec![],
                    body: Box::new(desugared_body),
                    span: s,
                }],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_desugar_other_decorator_unchanged() {
        let s = span();
        // @route("/api") fn handler() { 42 }
        // Generic decorator desugaring is preserved.
        let inner = Decl::Function {
            name: "handler".into(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "route".into(),
            args: vec![Expr::Literal {
                value: LiteralValue::Str("/api".into()),
                span: s,
            }],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // Expected: fn handler() { route(fn() { 42 }, "/api") }
        // Same generic desugaring as the existing @route test.
        let expected = vec![MirDecl::Function {
            name: "handler".into(),
            params: vec![],
            return_type: None,
            body: MirExpr::Call {
                func: Box::new(MirExpr::Variable {
                    name: "route".into(),
                    span: s,
                }),
                args: vec![
                    MirExpr::Lambda {
                        params: vec![],
                        body: Box::new(MirExpr::Literal {
                            value: MirLiteral::Int(42),
                            span: s,
                        }),
                        span: s,
                    },
                    MirExpr::Literal {
                        value: MirLiteral::Str("/api".into()),
                        span: s,
                    },
                ],
                span: s,
            },
            is_pub: true,
            is_generator: false,
            span: s,
        }];
        assert_eq!(result, expected);
    }

    // ------------------------------------------------------------------
    // @gen decorator recognition tests
    //
    // These tests verify that @gen-decorated functions are recognized as
    // custom generators and are NOT desugared into generic decorator calls.
    // ------------------------------------------------------------------

    #[test]
    fn test_desugar_gen_decorator_preserves_body() {
        let s = span();
        // @gen(Color) fn gen_color() -> Color { pure_red() }
        //
        // The @gen decorator signals a custom generator. The function body
        // should be preserved as-is (the function IS the generator), NOT
        // wrapped in a call to `gen(...)`.
        let inner = Decl::Function {
            name: "gen_color".into(),
            params: vec![],
            return_type: Some(Type::Named("Color".to_string())),
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "pure_red".into(),
                    span: s,
                }),
                args: vec![],
                span: s,
            },
            is_pub: true,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "gen".into(),
            args: vec![Expr::Variable {
                name: "Color".into(),
                span: s,
            }],
            target: Box::new(inner),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        assert_eq!(result.len(), 1, "should produce exactly one declaration");
        match &result[0] {
            MirDecl::Function {
                name,
                return_type,
                body,
                ..
            } => {
                assert_eq!(name, "gen_color", "function name should be preserved");
                assert_eq!(
                    *return_type,
                    Some(Type::Named("Color".to_string())),
                    "return type should be Color"
                );
                // The body should NOT be wrapped in a gen() call.
                let body_is_wrapped_in_gen = matches!(
                    body,
                    MirExpr::Call { func, .. }
                        if matches!(func.as_ref(), MirExpr::Variable { name, .. } if name == "gen")
                );
                assert!(
                    !body_is_wrapped_in_gen,
                    "body should NOT be wrapped in gen() call"
                );
                // The body should contain the original `pure_red()` call
                match body {
                    MirExpr::Call { func, args, .. } => {
                        match func.as_ref() {
                            MirExpr::Variable { name: fn_name, .. } => {
                                assert_eq!(
                                    fn_name, "pure_red",
                                    "the original function body should be preserved"
                                );
                            }
                            other => {
                                panic!("Expected body to call 'pure_red' directly, got {other:?}")
                            }
                        }
                        assert!(args.is_empty(), "pure_red() should have no arguments");
                    }
                    other => panic!("Expected body to be the original Call expr, got {other:?}"),
                }
            }
            other => panic!("Expected MirDecl::Function, got {other:?}"),
        }
    }

    #[test]
    fn test_desugar_gen_decorator_forall_preserves_body() {
        let s = span();
        // @gen(Int) forAll Int { x -> x > 0 }
        //
        // When @gen wraps a forAll expression, the forAll body should also
        // be preserved.
        let inner = Expr::ForAll {
            type_: Type::Named("Int".to_string()),
            binding: Pat::Variable("x".to_string()),
            property: Box::new(Expr::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::Variable {
                    name: "x".into(),
                    span: s,
                }),
                rhs: Box::new(Expr::Literal {
                    value: LiteralValue::Int(0),
                    span: s,
                }),
                span: s,
            }),
            span: s,
        };

        let inner_fn = Decl::Function {
            name: String::new(),
            params: vec![],
            return_type: None,
            body: inner,
            is_pub: false,
            span: s,
        };
        let input = vec![Decl::Decorator {
            name: "gen".into(),
            args: vec![Expr::Variable {
                name: "Int".into(),
                span: s,
            }],
            target: Box::new(inner_fn),
            is_pub: true,
            span: s,
        }];

        let result = desugar_decorators(&input);

        // The @gen(Int) should wrap the synthetic function containing
        // the forAll expression. The forAll should be preserved inside
        // the resulting function body, not wrapped in a gen() call.
        assert_eq!(result.len(), 1, "should produce exactly one declaration");
        match &result[0] {
            MirDecl::Function { body, .. } => {
                // The body should NOT be a call to `gen`
                let body_is_wrapped_in_gen = matches!(
                    body,
                    MirExpr::Call { func, .. }
                        if matches!(func.as_ref(), MirExpr::Variable { name, .. } if name == "gen")
                );
                assert!(
                    !body_is_wrapped_in_gen,
                    "body should NOT be wrapped in gen() call"
                );
                // The body should contain the ForAll expression directly
                assert!(
                    matches!(body, MirExpr::ForAll { .. }),
                    "expected ForAll expression to be preserved directly in body"
                );
            }
            other => panic!("Expected MirDecl::Function, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // assert.consistent desugaring tests (DWARF-41)
    //
    // These tests verify that desugar_pipe correctly handles
    // Expr::AssertConsistent by wrapping the desugared inner expression
    // in MirExpr::AssertConsistent. They will fail to compile until
    // both Expr::AssertConsistent and MirExpr::AssertConsistent are
    // implemented (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_desugar_assert_consistent_passthrough() {
        let s = span();
        let input = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Int(42),
                span: s,
            }),
            span: s,
        };
        let result = desugar_pipe(&input);
        let expected = MirExpr::AssertConsistent {
            expr: Box::new(MirExpr::Literal {
                value: MirLiteral::Int(42),
                span: s,
            }),
            span: s,
        };
        assert_eq!(result, expected);
    }
}
