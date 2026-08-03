//! LIR Walker Architecture
//!
//! This crate provides the `LirBackend` trait and a shared tree walker engine
//! for traversing LIR (Low-level Intermediate Representation) trees. Backends
//! implement the trait to process LIR nodes without writing traversal code.

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirField, LirLiteral, LirParam, LirPat, LirStmt,
    LirUnaryOp, LirVariant, TargetHint,
};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error type returned by all backend hooks.
#[derive(Debug, Clone)]
pub struct BackendError {
    message: String,
}

impl BackendError {
    /// Create a new `BackendError` from any string-like value.
    pub fn msg(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

// ---------------------------------------------------------------------------
// ReducedArm — simplified match arm (already reduced by the walker)
// ---------------------------------------------------------------------------

/// A match arm after the walker has reduced its sub-expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct ReducedArm<R> {
    pub pattern: R,
    pub guard: Option<R>,
    pub body: R,
}

// ---------------------------------------------------------------------------
// LirBackend trait
// ---------------------------------------------------------------------------

/// Backend trait for processing LIR trees.
///
/// Each method corresponds to a LIR node category. The generic parameter `R`
/// is the reduction type — backends choose what each node reduces to (e.g.
/// `()` for side-effect-only backends, `String` for pretty-printers, an AST
/// node for code generators, etc.).
pub trait LirBackend<R> {
    // ------ Expression hooks (20) ------

    fn visit_expr_literal(
        &mut self,
        value: &LirLiteral,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_variable(
        &mut self,
        name: &str,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_call(
        &mut self,
        func: R,
        args: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_member(
        &mut self,
        obj: R,
        field: &str,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_optional_access(
        &mut self,
        obj: R,
        field: &str,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_if(
        &mut self,
        cond: R,
        then: R,
        else_: Option<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_match(
        &mut self,
        expr: R,
        arms: Vec<ReducedArm<R>>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_block(
        &mut self,
        stmts: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_assign(
        &mut self,
        target: R,
        value: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_lambda(
        &mut self,
        params: &[LirParam],
        body: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_record(
        &mut self,
        fields: Vec<(String, R)>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_variant(
        &mut self,
        name: &str,
        arg: Option<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_array(
        &mut self,
        items: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_binary(
        &mut self,
        op: LirBinaryOp,
        lhs: R,
        rhs: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_unary(
        &mut self,
        op: LirUnaryOp,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_wildcard(&mut self, hint: &TargetHint, span: Span) -> Result<R, BackendError>;

    fn visit_expr_for_all(
        &mut self,
        type_: &Type,
        binding: R,
        property: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_assert_consistent(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_try(
        &mut self,
        body: R,
        binding: R,
        guard: Option<R>,
        handler: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_throw(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_propagate(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    // ------ Statement hooks (2) ------

    fn visit_stmt_let(&mut self, pat: R, value: R) -> Result<R, BackendError>;

    fn visit_stmt_expr(&mut self, expr: R) -> Result<R, BackendError>;

    // ------ Pattern hooks (5) ------

    fn visit_pat_wildcard(&mut self) -> Result<R, BackendError>;

    fn visit_pat_literal(&mut self, value: &LirLiteral) -> Result<R, BackendError>;

    fn visit_pat_variable(&mut self, name: &str) -> Result<R, BackendError>;

    fn visit_pat_variant(&mut self, name: &str, arg: Option<R>) -> Result<R, BackendError>;

    fn visit_pat_record(&mut self, fields: Vec<(String, R)>, rest: bool)
        -> Result<R, BackendError>;

    // ------ Declaration hooks (4) ------

    #[allow(clippy::too_many_arguments)]
    fn visit_decl_function(
        &mut self,
        name: &str,
        params: &[LirParam],
        return_type: &Option<Type>,
        body: R,
        effect: &Effect,
        hint: &TargetHint,
        is_pub: bool,
        is_generator: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_record_def(
        &mut self,
        name: &str,
        fields: &[LirField],
        is_pub: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_union_def(
        &mut self,
        name: &str,
        variants: &[LirVariant],
        is_pub: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_extern(
        &mut self,
        source: &str,
        name: &str,
        params: &[LirParam],
        return_type: &Option<Type>,
        is_pub: bool,
    ) -> Result<R, BackendError>;

    // ------ Lifecycle hooks (4) ------

    fn enter_module(&mut self) -> Result<(), BackendError>;

    fn exit_module(&mut self, decls: Vec<R>) -> Result<R, BackendError>;

    fn enter_function(&mut self, name: &str) -> Result<(), BackendError>;

    fn exit_function(&mut self, name: &str, body: R) -> Result<R, BackendError>;
}

// ---------------------------------------------------------------------------
// Tree walker engine
// ---------------------------------------------------------------------------

/// Walk a pattern node, reducing it bottom-up.
fn walk_pat<R>(backend: &mut impl LirBackend<R>, pat: &LirPat) -> Result<R, BackendError> {
    match pat {
        LirPat::Wildcard => backend.visit_pat_wildcard(),
        LirPat::Literal(value) => backend.visit_pat_literal(value),
        LirPat::Variable(name) => backend.visit_pat_variable(name),
        LirPat::Variant { name, arg } => {
            let reduced_arg = match arg {
                Some(inner) => Some(walk_pat(backend, inner)?),
                None => None,
            };
            backend.visit_pat_variant(name, reduced_arg)
        }
        LirPat::Record { fields, rest } => {
            let mut reduced_fields = Vec::with_capacity(fields.len());
            for (name, field_pat) in fields {
                let reduced = walk_pat(backend, field_pat)?;
                reduced_fields.push((name.clone(), reduced));
            }
            backend.visit_pat_record(reduced_fields, *rest)
        }
    }
}

/// Walk a statement node, reducing children first.
fn walk_stmt<R>(backend: &mut impl LirBackend<R>, stmt: &LirStmt) -> Result<R, BackendError> {
    match stmt {
        LirStmt::Let { pat, value } => {
            let reduced_value = walk_expr(backend, value)?;
            let reduced_pat = walk_pat(backend, pat)?;
            backend.visit_stmt_let(reduced_pat, reduced_value)
        }
        LirStmt::Expr(expr) => {
            let reduced_expr = walk_expr(backend, expr)?;
            backend.visit_stmt_expr(reduced_expr)
        }
    }
}

/// Walk an expression node, recursively reducing children first (bottom-up).
pub fn walk_expr<R>(backend: &mut impl LirBackend<R>, expr: &LirExpr) -> Result<R, BackendError> {
    match expr {
        LirExpr::Literal { value, hint, span } => backend.visit_expr_literal(value, hint, *span),
        LirExpr::Variable { name, hint, span } => backend.visit_expr_variable(name, hint, *span),
        LirExpr::Call {
            func,
            args,
            hint,
            span,
        } => {
            let reduced_func = walk_expr(backend, func)?;
            let mut reduced_args = Vec::with_capacity(args.len());
            for arg in args {
                reduced_args.push(walk_expr(backend, arg)?);
            }
            backend.visit_expr_call(reduced_func, reduced_args, hint, *span)
        }
        LirExpr::Member {
            obj,
            field,
            hint,
            span,
        } => {
            let reduced_obj = walk_expr(backend, obj)?;
            backend.visit_expr_member(reduced_obj, field, hint, *span)
        }
        LirExpr::OptionalAccess {
            obj,
            field,
            hint,
            span,
        } => {
            let reduced_obj = walk_expr(backend, obj)?;
            backend.visit_expr_optional_access(reduced_obj, field, hint, *span)
        }
        LirExpr::If {
            cond,
            then,
            else_,
            hint,
            span,
        } => {
            let reduced_cond = walk_expr(backend, cond)?;
            let reduced_then = walk_expr(backend, then)?;
            let reduced_else = match else_ {
                Some(e) => Some(walk_expr(backend, e)?),
                None => None,
            };
            backend.visit_expr_if(reduced_cond, reduced_then, reduced_else, hint, *span)
        }
        LirExpr::Match {
            expr,
            arms,
            hint,
            span,
        } => {
            let reduced_expr = walk_expr(backend, expr)?;
            let mut reduced_arms = Vec::with_capacity(arms.len());
            for arm in arms {
                let reduced_pattern = walk_pat(backend, &arm.pattern)?;
                let reduced_guard = match &arm.guard {
                    Some(g) => Some(walk_expr(backend, g)?),
                    None => None,
                };
                let reduced_body = walk_expr(backend, &arm.body)?;
                reduced_arms.push(ReducedArm {
                    pattern: reduced_pattern,
                    guard: reduced_guard,
                    body: reduced_body,
                });
            }
            backend.visit_expr_match(reduced_expr, reduced_arms, hint, *span)
        }
        LirExpr::Block { stmts, hint, span } => {
            let mut reduced_stmts = Vec::with_capacity(stmts.len());
            for stmt in stmts {
                reduced_stmts.push(walk_stmt(backend, stmt)?);
            }
            backend.visit_expr_block(reduced_stmts, hint, *span)
        }
        LirExpr::Assign {
            target,
            value,
            hint,
            span,
        } => {
            let reduced_target = walk_expr(backend, target)?;
            let reduced_value = walk_expr(backend, value)?;
            backend.visit_expr_assign(reduced_target, reduced_value, hint, *span)
        }
        LirExpr::Lambda {
            params,
            body,
            hint,
            span,
        } => {
            let reduced_body = walk_expr(backend, body)?;
            backend.visit_expr_lambda(params, reduced_body, hint, *span)
        }
        LirExpr::Record { fields, hint, span } => {
            let mut reduced_fields = Vec::with_capacity(fields.len());
            for (name, field_expr) in fields {
                let reduced = walk_expr(backend, field_expr)?;
                reduced_fields.push((name.clone(), reduced));
            }
            backend.visit_expr_record(reduced_fields, hint, *span)
        }
        LirExpr::Variant {
            name,
            arg,
            hint,
            span,
        } => {
            let reduced_arg = match arg {
                Some(a) => Some(walk_expr(backend, a)?),
                None => None,
            };
            backend.visit_expr_variant(name, reduced_arg, hint, *span)
        }
        LirExpr::Array { items, hint, span } => {
            let mut reduced_items = Vec::with_capacity(items.len());
            for item in items {
                reduced_items.push(walk_expr(backend, item)?);
            }
            backend.visit_expr_array(reduced_items, hint, *span)
        }
        LirExpr::Binary {
            op,
            lhs,
            rhs,
            hint,
            span,
        } => {
            let reduced_lhs = walk_expr(backend, lhs)?;
            let reduced_rhs = walk_expr(backend, rhs)?;
            backend.visit_expr_binary(*op, reduced_lhs, reduced_rhs, hint, *span)
        }
        LirExpr::Unary {
            op,
            expr,
            hint,
            span,
        } => {
            let reduced_expr = walk_expr(backend, expr)?;
            backend.visit_expr_unary(*op, reduced_expr, hint, *span)
        }
        LirExpr::Wildcard { hint, span } => backend.visit_expr_wildcard(hint, *span),
        LirExpr::ForAll {
            type_,
            binding,
            property,
            hint,
            span,
        } => {
            let reduced_binding = walk_pat(backend, binding)?;
            let reduced_property = walk_expr(backend, property)?;
            backend.visit_expr_for_all(type_, reduced_binding, reduced_property, hint, *span)
        }
        LirExpr::AssertConsistent { expr, hint, span } => {
            let reduced_expr = walk_expr(backend, expr)?;
            backend.visit_expr_assert_consistent(reduced_expr, hint, *span)
        }
        LirExpr::Try {
            body,
            binding,
            guard,
            handler,
            hint,
            span,
        } => {
            let reduced_body = walk_expr(backend, body)?;
            let reduced_binding = walk_pat(backend, binding)?;
            let reduced_guard = match guard {
                Some(g) => Some(walk_expr(backend, g)?),
                None => None,
            };
            let reduced_handler = walk_expr(backend, handler)?;
            backend.visit_expr_try(
                reduced_body,
                reduced_binding,
                reduced_guard,
                reduced_handler,
                hint,
                *span,
            )
        }
        LirExpr::Throw { expr, hint, span } => {
            let reduced_expr = walk_expr(backend, expr)?;
            backend.visit_expr_throw(reduced_expr, hint, *span)
        }
        LirExpr::Propagate { expr, hint, span } => {
            let reduced_expr = walk_expr(backend, expr)?;
            backend.visit_expr_propagate(reduced_expr, hint, *span)
        }
    }
}

/// Walk a single declaration. Function decls are wrapped in function lifecycle hooks.
pub fn walk_decl<R>(backend: &mut impl LirBackend<R>, decl: &LirDecl) -> Result<R, BackendError> {
    match decl {
        LirDecl::Function {
            name,
            params,
            return_type,
            body,
            effect,
            hint,
            is_pub,
            is_generator,
            span,
        } => {
            backend.enter_function(name)?;
            let reduced_body = walk_expr(backend, body)?;
            let exited_body = backend.exit_function(name, reduced_body)?;
            backend.visit_decl_function(
                name,
                params,
                return_type,
                exited_body,
                effect,
                hint,
                *is_pub,
                *is_generator,
                *span,
            )
        }
        LirDecl::RecordDef {
            name,
            fields,
            is_pub,
            span,
        } => backend.visit_decl_record_def(name, fields, *is_pub, *span),
        LirDecl::UnionDef {
            name,
            variants,
            is_pub,
            span,
        } => backend.visit_decl_union_def(name, variants, *is_pub, *span),
        LirDecl::Extern {
            source,
            name,
            params,
            return_type,
            is_pub,
        } => backend.visit_decl_extern(source, name, params, return_type, *is_pub),
    }
}

/// Walk a complete module (slice of declarations).
pub fn walk_module<R>(
    backend: &mut impl LirBackend<R>,
    decls: &[LirDecl],
) -> Result<R, BackendError> {
    backend.enter_module()?;
    let mut results = Vec::with_capacity(decls.len());
    for decl in decls {
        results.push(walk_decl(backend, decl)?);
    }
    backend.exit_module(results)
}

// ---------------------------------------------------------------------------
// DebugBackend — reference test backend producing S-expression debug output
// ---------------------------------------------------------------------------

/// Reference test backend that produces S-expression-style debug output.
/// Used to validate walker correctness and serve as a reference implementation.
pub struct DebugBackend;

/// Format a literal value for debug output (extracts inner value from Debug repr).
fn fmt_literal(v: &LirLiteral) -> String {
    match v {
        LirLiteral::Int(n) => format!("{n}"),
        LirLiteral::Float(f) => format!("{f}"),
        LirLiteral::Str(s) => format!("\"{s}\""),
        LirLiteral::Bool(b) => format!("{b}"),
        LirLiteral::Null => "null".to_string(),
    }
}

/// Format a binary operator as its symbolic representation.
fn fmt_binop(op: LirBinaryOp) -> &'static str {
    match op {
        LirBinaryOp::Add => "+",
        LirBinaryOp::Sub => "-",
        LirBinaryOp::Mul => "*",
        LirBinaryOp::Div => "/",
        LirBinaryOp::Eq => "==",
        LirBinaryOp::Ne => "!=",
        LirBinaryOp::Lt => "<",
        LirBinaryOp::Gt => ">",
        LirBinaryOp::Le => "<=",
        LirBinaryOp::Ge => ">=",
        LirBinaryOp::And => "&&",
        LirBinaryOp::Or => "||",
    }
}

/// Format a unary operator as its symbolic representation.
fn fmt_unop(op: LirUnaryOp) -> &'static str {
    match op {
        LirUnaryOp::Neg => "-",
        LirUnaryOp::Not => "!",
    }
}

impl LirBackend<String> for DebugBackend {
    // ------ Expression hooks (20) ------

    fn visit_expr_literal(
        &mut self,
        value: &LirLiteral,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(literal {})", fmt_literal(value)))
    }

    fn visit_expr_variable(
        &mut self,
        name: &str,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(var {name})"))
    }

    fn visit_expr_call(
        &mut self,
        func: String,
        args: Vec<String>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        let mut parts = vec![func];
        parts.extend(args);
        Ok(format!("(call {})", parts.join(" ")))
    }

    fn visit_expr_member(
        &mut self,
        obj: String,
        field: &str,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(member {obj} {field})"))
    }

    fn visit_expr_optional_access(
        &mut self,
        obj: String,
        field: &str,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(optional_member {obj} {field})"))
    }

    fn visit_expr_if(
        &mut self,
        cond: String,
        then: String,
        else_: Option<String>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        match else_ {
            Some(e) => Ok(format!("(if {cond} {then} {e})")),
            None => Ok(format!("(if {cond} {then})")),
        }
    }

    fn visit_expr_match(
        &mut self,
        expr: String,
        arms: Vec<ReducedArm<String>>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        let arm_strs: Vec<String> = arms
            .iter()
            .map(|arm| match &arm.guard {
                Some(g) => format!("(arm {} {} {})", arm.pattern, g, arm.body),
                None => format!("(arm {} {})", arm.pattern, arm.body),
            })
            .collect();
        let mut parts = vec![expr];
        parts.extend(arm_strs);
        Ok(format!("(match {})", parts.join(" ")))
    }

    fn visit_expr_block(
        &mut self,
        stmts: Vec<String>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(block {})", stmts.join(" ")))
    }

    fn visit_expr_assign(
        &mut self,
        target: String,
        value: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(assign {target} {value})"))
    }

    fn visit_expr_lambda(
        &mut self,
        params: &[LirParam],
        body: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        Ok(format!("(lambda ({}) {body})", param_names.join(" ")))
    }

    fn visit_expr_record(
        &mut self,
        fields: Vec<(String, String)>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        let field_strs: Vec<String> = fields.iter().map(|(k, v)| format!("({k} {v})")).collect();
        Ok(format!("(record {})", field_strs.join(" ")))
    }

    fn visit_expr_variant(
        &mut self,
        name: &str,
        arg: Option<String>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        match arg {
            Some(a) => Ok(format!("(variant {name} {a})")),
            None => Ok(format!("(variant {name})")),
        }
    }

    fn visit_expr_array(
        &mut self,
        items: Vec<String>,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(array {})", items.join(" ")))
    }

    fn visit_expr_binary(
        &mut self,
        op: LirBinaryOp,
        lhs: String,
        rhs: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(binary {} {lhs} {rhs})", fmt_binop(op)))
    }

    fn visit_expr_unary(
        &mut self,
        op: LirUnaryOp,
        expr: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(unary {} {expr})", fmt_unop(op)))
    }

    fn visit_expr_wildcard(
        &mut self,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok("(wildcard)".to_string())
    }

    fn visit_expr_for_all(
        &mut self,
        _type_: &Type,
        binding: String,
        property: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(for-all {binding} {property})"))
    }

    fn visit_expr_assert_consistent(
        &mut self,
        expr: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(assert-consistent {expr})"))
    }

    fn visit_expr_try(
        &mut self,
        body: String,
        binding: String,
        guard: Option<String>,
        handler: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        match guard {
            Some(g) => Ok(format!("(try {body} {binding} {g} {handler})")),
            None => Ok(format!("(try {body} {binding} {handler})")),
        }
    }

    fn visit_expr_throw(
        &mut self,
        expr: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(throw {expr})"))
    }

    fn visit_expr_propagate(
        &mut self,
        expr: String,
        _hint: &TargetHint,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(propagate {expr})"))
    }

    // ------ Statement hooks (2) ------

    fn visit_stmt_let(&mut self, pat: String, value: String) -> Result<String, BackendError> {
        Ok(format!("(let {pat} {value})"))
    }

    fn visit_stmt_expr(&mut self, expr: String) -> Result<String, BackendError> {
        Ok(expr)
    }

    // ------ Pattern hooks (5) ------

    fn visit_pat_wildcard(&mut self) -> Result<String, BackendError> {
        Ok("_".to_string())
    }

    fn visit_pat_literal(&mut self, value: &LirLiteral) -> Result<String, BackendError> {
        Ok(format!("(lit {})", fmt_literal(value)))
    }

    fn visit_pat_variable(&mut self, name: &str) -> Result<String, BackendError> {
        Ok(name.to_string())
    }

    fn visit_pat_variant(
        &mut self,
        name: &str,
        arg: Option<String>,
    ) -> Result<String, BackendError> {
        match arg {
            Some(a) => Ok(format!("(variant {name} {a})")),
            None => Ok(format!("(variant {name})")),
        }
    }

    fn visit_pat_record(
        &mut self,
        fields: Vec<(String, String)>,
        rest: bool,
    ) -> Result<String, BackendError> {
        let field_strs: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        let mut s = format!("(record {})", field_strs.join(" "));
        if rest {
            s.push_str(" ..");
        }
        Ok(s)
    }

    // ------ Declaration hooks (4) ------

    #[allow(clippy::too_many_arguments)]
    fn visit_decl_function(
        &mut self,
        name: &str,
        _params: &[LirParam],
        _return_type: &Option<Type>,
        body: String,
        _effect: &Effect,
        _hint: &TargetHint,
        _is_pub: bool,
        _is_generator: bool,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(function {name} {body})"))
    }

    fn visit_decl_record_def(
        &mut self,
        name: &str,
        _fields: &[LirField],
        _is_pub: bool,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(record-def {name})"))
    }

    fn visit_decl_union_def(
        &mut self,
        name: &str,
        _variants: &[LirVariant],
        _is_pub: bool,
        _span: Span,
    ) -> Result<String, BackendError> {
        Ok(format!("(union-def {name})"))
    }

    fn visit_decl_extern(
        &mut self,
        source: &str,
        name: &str,
        _params: &[LirParam],
        _return_type: &Option<Type>,
        _is_pub: bool,
    ) -> Result<String, BackendError> {
        Ok(format!("(extern {source} {name})"))
    }

    // ------ Lifecycle hooks (4) ------

    fn enter_module(&mut self) -> Result<(), BackendError> {
        Ok(())
    }

    fn exit_module(&mut self, decls: Vec<String>) -> Result<String, BackendError> {
        Ok(decls.join("\n"))
    }

    fn enter_function(&mut self, _name: &str) -> Result<(), BackendError> {
        Ok(())
    }

    fn exit_function(&mut self, _name: &str, body: String) -> Result<String, BackendError> {
        Ok(body)
    }
}

// ------ Tests ------

#[cfg(test)]
mod tests {
    use crate::{BackendError, LirBackend, ReducedArm};
    use dwarf_lir::{
        Effect, LirArm, LirBinaryOp, LirDecl, LirExpr, LirField, LirLiteral, LirParam, LirPat,
        LirStmt, LirUnaryOp, LirVariant, TargetHint,
    };
    use dwarf_syntax::hir::Type;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn s() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint() -> TargetHint {
        TargetHint::None
    }

    fn make_literal(val: i64) -> LirExpr {
        LirExpr::Literal {
            value: LirLiteral::Int(val),
            hint: hint(),
            span: s(),
        }
    }

    fn make_var(name: &str) -> LirExpr {
        LirExpr::Variable {
            name: name.into(),
            hint: hint(),
            span: s(),
        }
    }

    // ------------------------------------------------------------------
    // BackendError — error type must exist and implement std::error::Error
    // ------------------------------------------------------------------

    #[test]
    fn test_backend_error_is_std_error() {
        let err = BackendError::msg("something went wrong");
        // Must implement std::error::Error (which requires Debug + Display).
        let _: &dyn std::error::Error = &err;
        // Debug must work.
        let debug_str = format!("{err:?}");
        assert!(!debug_str.is_empty());
        // Display must work.
        let display_str = format!("{err}");
        assert!(!display_str.is_empty());
    }

    #[test]
    fn test_backend_error_construction() {
        let err1 = BackendError::msg("test error");
        assert!(format!("{err1}").contains("test error"));

        let err2 = BackendError::msg("another error");
        assert!(format!("{err2}").contains("another error"));

        // Two different messages should produce different Display output.
        assert_ne!(format!("{err1}"), format!("{err2}"));
    }

    // ------------------------------------------------------------------
    // MockBackend — implements LirBackend<()> to prove the trait exists
    // and has the expected shape.
    // ------------------------------------------------------------------

    struct MockBackend;

    impl LirBackend<()> for MockBackend {
        // ------ Expression hooks (20) ------

        fn visit_expr_literal(
            &mut self,
            _value: &LirLiteral,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_variable(
            &mut self,
            _name: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_call(
            &mut self,
            _func: (),
            _args: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_member(
            &mut self,
            _obj: (),
            _field: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_optional_access(
            &mut self,
            _obj: (),
            _field: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_if(
            &mut self,
            _cond: (),
            _then: (),
            _else_: Option<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_match(
            &mut self,
            _expr: (),
            _arms: Vec<ReducedArm<()>>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_block(
            &mut self,
            _stmts: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_assign(
            &mut self,
            _target: (),
            _value: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_lambda(
            &mut self,
            _params: &[LirParam],
            _body: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_record(
            &mut self,
            _fields: Vec<(String, ())>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_variant(
            &mut self,
            _name: &str,
            _arg: Option<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_array(
            &mut self,
            _items: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_binary(
            &mut self,
            _op: LirBinaryOp,
            _lhs: (),
            _rhs: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_unary(
            &mut self,
            _op: LirUnaryOp,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_wildcard(
            &mut self,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_for_all(
            &mut self,
            _type_: &Type,
            _binding: (),
            _property: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_assert_consistent(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_try(
            &mut self,
            _body: (),
            _binding: (),
            _guard: Option<()>,
            _handler: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_throw(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_propagate(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Statement hooks (2) ------

        fn visit_stmt_let(&mut self, _pat: (), _value: ()) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_stmt_expr(&mut self, _expr: ()) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Pattern hooks (5) ------

        fn visit_pat_wildcard(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_literal(&mut self, _value: &LirLiteral) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_variable(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_variant(&mut self, _name: &str, _arg: Option<()>) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_record(
            &mut self,
            _fields: Vec<(String, ())>,
            _rest: bool,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Declaration hooks (4) ------

        #[allow(clippy::too_many_arguments)]
        fn visit_decl_function(
            &mut self,
            _name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            _body: (),
            _effect: &Effect,
            _hint: &TargetHint,
            _is_pub: bool,
            _is_generator: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_record_def(
            &mut self,
            _name: &str,
            _fields: &[LirField],
            _is_pub: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_union_def(
            &mut self,
            _name: &str,
            _variants: &[LirVariant],
            _is_pub: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_extern(
            &mut self,
            _source: &str,
            _name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            _is_pub: bool,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Lifecycle hooks (4) ------

        fn enter_module(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn exit_module(&mut self, _decls: Vec<()>) -> Result<(), BackendError> {
            Ok(())
        }

        fn enter_function(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }

        fn exit_function(&mut self, _name: &str, _body: ()) -> Result<(), BackendError> {
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Trait can be implemented — proves LirBackend<R> exists
    // ------------------------------------------------------------------

    #[test]
    fn test_trait_impl_compiles() {
        let _backend = MockBackend;
        // If this compiles, the trait exists and can be implemented.
    }

    // ------------------------------------------------------------------
    // Expression hooks — every LirExpr variant has a corresponding hook
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_expr_literal_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_literal(&LirLiteral::Int(42), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_variable_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_variable("x", &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_call_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_call((), vec![(), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_member_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_member((), "field", &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_if_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_if((), (), Some(()), &hint(), s());
        assert!(result.is_ok());

        // Also test without else branch.
        let result2 = b.visit_expr_if((), (), None, &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_match_hook() {
        let mut b = MockBackend;
        let arms = vec![ReducedArm {
            pattern: (),
            guard: None,
            body: (),
        }];
        let result = b.visit_expr_match((), arms, &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_block_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_block(vec![(), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_assign_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_assign((), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_lambda_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "x".into(),
            type_: None,
        }];
        let result = b.visit_expr_lambda(&params, (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_record_hook() {
        let mut b = MockBackend;
        let fields = vec![("x".into(), ()), ("y".into(), ())];
        let result = b.visit_expr_record(fields, &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_variant_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_variant("Some", Some(()), &hint(), s());
        assert!(result.is_ok());

        let result2 = b.visit_expr_variant("None", None, &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_array_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_array(vec![(), (), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_binary_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_binary(LirBinaryOp::Add, (), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_unary_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_unary(LirUnaryOp::Neg, (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_wildcard_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_wildcard(&hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_for_all_hook() {
        let mut b = MockBackend;
        let ty = Type::Named("Int".into());
        let result = b.visit_expr_for_all(&ty, (), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_assert_consistent_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_assert_consistent((), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_try_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_try((), (), None, (), &hint(), s());
        assert!(result.is_ok());

        // Also test with guard.
        let result2 = b.visit_expr_try((), (), Some(()), (), &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_throw_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_throw((), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_propagate_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_propagate((), &hint(), s());
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Statement hooks — both LirStmt variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_stmt_let_hook() {
        let mut b = MockBackend;
        let result = b.visit_stmt_let((), ());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_stmt_expr_hook() {
        let mut b = MockBackend;
        let result = b.visit_stmt_expr(());
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Pattern hooks — all 5 LirPat variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_pat_wildcard_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_wildcard();
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_literal_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_literal(&LirLiteral::Int(7));
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_variable_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_variable("binding");
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_variant_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_variant("Some", Some(()));
        assert!(result.is_ok());

        let result2 = b.visit_pat_variant("None", None);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_pat_record_hook() {
        let mut b = MockBackend;
        let fields = vec![("x".into(), ()), ("y".into(), ())];
        let result = b.visit_pat_record(fields, false);
        assert!(result.is_ok());

        // Also test with rest.
        let result2 = b.visit_pat_record(vec![], true);
        assert!(result2.is_ok());
    }

    // ------------------------------------------------------------------
    // Declaration hooks — all 4 LirDecl variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_decl_function_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "a".into(),
            type_: None,
        }];
        let result = b.visit_decl_function(
            "main",
            &params,
            &None,
            (),
            &Effect::Pure,
            &hint(),
            true,
            false,
            s(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_record_def_hook() {
        let mut b = MockBackend;
        let fields = vec![LirField {
            name: "x".into(),
            type_: Type::Named("Int".into()),
        }];
        let result = b.visit_decl_record_def("Point", &fields, true, s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_union_def_hook() {
        let mut b = MockBackend;
        let variants = vec![
            LirVariant {
                name: "Some".into(),
                arg: Some(Type::Named("Int".into())),
            },
            LirVariant {
                name: "None".into(),
                arg: None,
            },
        ];
        let result = b.visit_decl_union_def("Option", &variants, true, s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_extern_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "fd".into(),
            type_: None,
        }];
        let result = b.visit_decl_extern("libc", "read", &params, &None, true);
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Lifecycle hooks — module and function enter/exit
    // ------------------------------------------------------------------

    #[test]
    fn test_lifecycle_hooks() {
        let mut b = MockBackend;

        // Module lifecycle.
        assert!(b.enter_module().is_ok());
        assert!(b.exit_module(vec![(), ()]).is_ok());

        // Function lifecycle.
        assert!(b.enter_function("main").is_ok());
        assert!(b.exit_function("main", ()).is_ok());
    }

    // ------------------------------------------------------------------
    // Generic return type — trait works with different R types
    // ------------------------------------------------------------------

    struct StringBackend;

    impl LirBackend<String> for StringBackend {
        fn visit_expr_literal(
            &mut self,
            v: &LirLiteral,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{v:?}"))
        }
        fn visit_expr_variable(
            &mut self,
            name: &str,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(name.to_string())
        }
        fn visit_expr_call(
            &mut self,
            func: String,
            args: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("call({func}, [{}])", args.join(", ")))
        }
        fn visit_expr_member(
            &mut self,
            obj: String,
            field: &str,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{obj}.{field}"))
        }
        fn visit_expr_optional_access(
            &mut self,
            obj: String,
            field: &str,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{obj}?.{field}"))
        }
        fn visit_expr_if(
            &mut self,
            c: String,
            t: String,
            e: Option<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            match e {
                Some(el) => Ok(format!("if {c} then {t} else {el}")),
                None => Ok(format!("if {c} then {t}")),
            }
        }
        fn visit_expr_match(
            &mut self,
            expr: String,
            _arms: Vec<ReducedArm<String>>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("match {expr}"))
        }
        fn visit_expr_block(
            &mut self,
            stmts: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{{ {} }}", stmts.join("; ")))
        }
        fn visit_expr_assign(
            &mut self,
            target: String,
            value: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{target} = {value}"))
        }
        fn visit_expr_lambda(
            &mut self,
            params: &[LirParam],
            body: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            Ok(format!("|{}| {body}", names.join(", ")))
        }
        fn visit_expr_record(
            &mut self,
            fields: Vec<(String, String)>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            Ok(format!("{{ {} }}", pairs.join(", ")))
        }
        fn visit_expr_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.to_string()),
            }
        }
        fn visit_expr_array(
            &mut self,
            items: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("[{}]", items.join(", ")))
        }
        fn visit_expr_binary(
            &mut self,
            op: LirBinaryOp,
            lhs: String,
            rhs: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("({lhs} {op:?} {rhs})"))
        }
        fn visit_expr_unary(
            &mut self,
            op: LirUnaryOp,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("({op:?} {expr})"))
        }
        fn visit_expr_wildcard(
            &mut self,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok("_".to_string())
        }
        fn visit_expr_for_all(
            &mut self,
            _ty: &Type,
            _binding: String,
            property: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("forAll {property}"))
        }
        fn visit_expr_assert_consistent(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("assertConsistent({expr})"))
        }
        fn visit_expr_try(
            &mut self,
            body: String,
            _binding: String,
            _guard: Option<String>,
            handler: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("try {body} catch {handler}"))
        }
        fn visit_expr_throw(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("throw {expr}"))
        }
        fn visit_expr_propagate(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{expr}?"))
        }

        fn visit_stmt_let(&mut self, pat: String, value: String) -> Result<String, BackendError> {
            Ok(format!("let {pat} = {value}"))
        }
        fn visit_stmt_expr(&mut self, expr: String) -> Result<String, BackendError> {
            Ok(expr)
        }

        fn visit_pat_wildcard(&mut self) -> Result<String, BackendError> {
            Ok("_".into())
        }
        fn visit_pat_literal(&mut self, v: &LirLiteral) -> Result<String, BackendError> {
            Ok(format!("{v:?}"))
        }
        fn visit_pat_variable(&mut self, name: &str) -> Result<String, BackendError> {
            Ok(name.into())
        }
        fn visit_pat_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
        ) -> Result<String, BackendError> {
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.into()),
            }
        }
        fn visit_pat_record(
            &mut self,
            fields: Vec<(String, String)>,
            rest: bool,
        ) -> Result<String, BackendError> {
            let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            let mut s = format!("{{ {} }}", pairs.join(", "));
            if rest {
                s.push_str(" ..");
            }
            Ok(s)
        }

        #[allow(clippy::too_many_arguments)]
        fn visit_decl_function(
            &mut self,
            name: &str,
            _params: &[LirParam],
            _ret: &Option<Type>,
            body: String,
            _effect: &Effect,
            _hint: &TargetHint,
            _is_pub: bool,
            _is_gen: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("fn {name} = {body}"))
        }
        fn visit_decl_record_def(
            &mut self,
            name: &str,
            _fields: &[LirField],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("record {name}"))
        }
        fn visit_decl_union_def(
            &mut self,
            name: &str,
            _variants: &[LirVariant],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("union {name}"))
        }
        fn visit_decl_extern(
            &mut self,
            source: &str,
            name: &str,
            _params: &[LirParam],
            _ret: &Option<Type>,
            _is_pub: bool,
        ) -> Result<String, BackendError> {
            Ok(format!("extern {source} {name}"))
        }

        fn enter_module(&mut self) -> Result<(), BackendError> {
            Ok(())
        }
        fn exit_module(&mut self, decls: Vec<String>) -> Result<String, BackendError> {
            Ok(decls.join("\n"))
        }
        fn enter_function(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }
        fn exit_function(&mut self, _name: &str, body: String) -> Result<String, BackendError> {
            Ok(body)
        }
    }

    #[test]
    fn test_generic_return_type_string() {
        let mut b = StringBackend;
        let lit = b
            .visit_expr_literal(&LirLiteral::Int(42), &hint(), s())
            .unwrap();
        assert!(
            !lit.is_empty(),
            "String backend should produce non-empty output"
        );

        let var = b.visit_expr_variable("x", &hint(), s()).unwrap();
        assert_eq!(var, "x");

        let call = b.visit_expr_call(var, vec![lit], &hint(), s()).unwrap();
        assert!(call.contains("call"), "should contain 'call'");
    }

    #[test]
    fn test_generic_return_type_i32() {
        // Prove the trait works with a numeric return type too.
        struct CountBackend;

        impl LirBackend<i32> for CountBackend {
            fn visit_expr_literal(
                &mut self,
                _v: &LirLiteral,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_variable(
                &mut self,
                _n: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_call(
                &mut self,
                f: i32,
                args: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(f + args.into_iter().sum::<i32>())
            }
            fn visit_expr_member(
                &mut self,
                obj: i32,
                _f: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(obj)
            }
            fn visit_expr_optional_access(
                &mut self,
                obj: i32,
                _f: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(obj)
            }
            fn visit_expr_if(
                &mut self,
                c: i32,
                t: i32,
                e: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(c + t + e.unwrap_or(0))
            }
            fn visit_expr_match(
                &mut self,
                expr: i32,
                _arms: Vec<ReducedArm<i32>>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(expr)
            }
            fn visit_expr_block(
                &mut self,
                stmts: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(stmts.into_iter().sum())
            }
            fn visit_expr_assign(
                &mut self,
                t: i32,
                v: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(t + v)
            }
            fn visit_expr_lambda(
                &mut self,
                _p: &[LirParam],
                body: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_expr_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            fn visit_expr_variant(
                &mut self,
                _n: &str,
                arg: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(arg.unwrap_or(0))
            }
            fn visit_expr_array(
                &mut self,
                items: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(items.into_iter().sum())
            }
            fn visit_expr_binary(
                &mut self,
                _op: LirBinaryOp,
                l: i32,
                r: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(l + r)
            }
            fn visit_expr_unary(
                &mut self,
                _op: LirUnaryOp,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_wildcard(
                &mut self,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_expr_for_all(
                &mut self,
                _t: &Type,
                _b: i32,
                p: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(p)
            }
            fn visit_expr_assert_consistent(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_try(
                &mut self,
                body: i32,
                _b: i32,
                _g: Option<i32>,
                h: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body + h)
            }
            fn visit_expr_throw(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_propagate(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_stmt_let(&mut self, _p: i32, v: i32) -> Result<i32, BackendError> {
                Ok(v)
            }
            fn visit_stmt_expr(&mut self, e: i32) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_pat_wildcard(&mut self) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_pat_literal(&mut self, _v: &LirLiteral) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variable(&mut self, _n: &str) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variant(&mut self, _n: &str, a: Option<i32>) -> Result<i32, BackendError> {
                Ok(a.unwrap_or(0))
            }
            fn visit_pat_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _rest: bool,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            #[allow(clippy::too_many_arguments)]
            fn visit_decl_function(
                &mut self,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                body: i32,
                _e: &Effect,
                _h: &TargetHint,
                _pub: bool,
                _gen: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_decl_record_def(
                &mut self,
                _n: &str,
                _f: &[LirField],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_union_def(
                &mut self,
                _n: &str,
                _v: &[LirVariant],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_extern(
                &mut self,
                _s: &str,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                _pub: bool,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn enter_module(&mut self) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_module(&mut self, decls: Vec<i32>) -> Result<i32, BackendError> {
                Ok(decls.into_iter().sum())
            }
            fn enter_function(&mut self, _n: &str) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_function(&mut self, _n: &str, body: i32) -> Result<i32, BackendError> {
                Ok(body)
            }
        }

        let mut b = CountBackend;
        let count = b
            .visit_expr_binary(LirBinaryOp::Add, 1, 2, &hint(), s())
            .unwrap();
        assert_eq!(count, 3, "i32 backend should sum children");
    }

    // ------------------------------------------------------------------
    // Full walk — exercise every hook category in sequence
    // ------------------------------------------------------------------

    #[test]
    fn test_full_walk_all_categories() {
        let mut b = MockBackend;

        // Module enter.
        b.enter_module().unwrap();

        // Declaration: function.
        b.enter_function("main").unwrap();
        b.visit_decl_function(
            "main",
            &[],
            &None,
            (),
            &Effect::Pure,
            &hint(),
            true,
            false,
            s(),
        )
        .unwrap();

        // Expression hooks.
        b.visit_expr_literal(&LirLiteral::Int(1), &hint(), s())
            .unwrap();
        b.visit_expr_variable("x", &hint(), s()).unwrap();
        b.visit_expr_call((), vec![], &hint(), s()).unwrap();
        b.visit_expr_member((), "f", &hint(), s()).unwrap();
        b.visit_expr_if((), (), None, &hint(), s()).unwrap();
        b.visit_expr_match((), vec![], &hint(), s()).unwrap();
        b.visit_expr_block(vec![], &hint(), s()).unwrap();
        b.visit_expr_assign((), (), &hint(), s()).unwrap();
        b.visit_expr_lambda(&[], (), &hint(), s()).unwrap();
        b.visit_expr_record(vec![], &hint(), s()).unwrap();
        b.visit_expr_variant("None", None, &hint(), s()).unwrap();
        b.visit_expr_array(vec![], &hint(), s()).unwrap();
        b.visit_expr_binary(LirBinaryOp::Add, (), (), &hint(), s())
            .unwrap();
        b.visit_expr_unary(LirUnaryOp::Not, (), &hint(), s())
            .unwrap();
        b.visit_expr_wildcard(&hint(), s()).unwrap();
        b.visit_expr_for_all(&Type::Named("Int".into()), (), (), &hint(), s())
            .unwrap();
        b.visit_expr_assert_consistent((), &hint(), s()).unwrap();
        b.visit_expr_try((), (), None, (), &hint(), s()).unwrap();
        b.visit_expr_throw((), &hint(), s()).unwrap();
        b.visit_expr_propagate((), &hint(), s()).unwrap();

        // Statement hooks.
        b.visit_stmt_let((), ()).unwrap();
        b.visit_stmt_expr(()).unwrap();

        // Pattern hooks.
        b.visit_pat_wildcard().unwrap();
        b.visit_pat_literal(&LirLiteral::Null).unwrap();
        b.visit_pat_variable("x").unwrap();
        b.visit_pat_variant("Some", None).unwrap();
        b.visit_pat_record(vec![], false).unwrap();

        // Declaration hooks (non-function).
        b.visit_decl_record_def("Pt", &[], true, s()).unwrap();
        b.visit_decl_union_def("Opt", &[], true, s()).unwrap();
        b.visit_decl_extern("libc", "write", &[], &None, false)
            .unwrap();

        // Function/module exit.
        b.exit_function("main", ()).unwrap();
        b.exit_module(vec![()]).unwrap();
    }

    // ==================================================================
    // Walker engine tests
    // ==================================================================

    // ------------------------------------------------------------------
    // SpyBackend — records which hooks were called for verification
    // ------------------------------------------------------------------

    struct SpyBackend {
        calls: Vec<String>,
    }

    impl SpyBackend {
        fn new() -> Self {
            Self { calls: Vec::new() }
        }

        fn record(&mut self, name: &str) {
            self.calls.push(name.to_string());
        }
    }

    impl LirBackend<String> for SpyBackend {
        // ------ Expression hooks ------

        fn visit_expr_literal(
            &mut self,
            value: &LirLiteral,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_literal");
            Ok(format!("literal({value:?})"))
        }

        fn visit_expr_variable(
            &mut self,
            name: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_variable");
            Ok(format!("var({name})"))
        }

        fn visit_expr_call(
            &mut self,
            func: String,
            args: Vec<String>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_call");
            Ok(format!("call({func}, [{}])", args.join(", ")))
        }

        fn visit_expr_member(
            &mut self,
            obj: String,
            field: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_member");
            Ok(format!("{obj}.{field}"))
        }
        fn visit_expr_optional_access(
            &mut self,
            obj: String,
            field: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_optional_access");
            Ok(format!("{obj}?.{field}"))
        }

        fn visit_expr_if(
            &mut self,
            cond: String,
            then: String,
            else_: Option<String>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_if");
            match else_ {
                Some(e) => Ok(format!("if {cond} then {then} else {e}")),
                None => Ok(format!("if {cond} then {then}")),
            }
        }

        fn visit_expr_match(
            &mut self,
            expr: String,
            arms: Vec<ReducedArm<String>>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_match");
            Ok(format!("match {expr} with {} arms", arms.len()))
        }

        fn visit_expr_block(
            &mut self,
            stmts: Vec<String>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_block");
            Ok(format!("block({} stmts)", stmts.len()))
        }

        fn visit_expr_assign(
            &mut self,
            target: String,
            value: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_assign");
            Ok(format!("{target} = {value}"))
        }

        fn visit_expr_lambda(
            &mut self,
            params: &[LirParam],
            body: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_lambda");
            Ok(format!("lambda({} params, {body})", params.len()))
        }

        fn visit_expr_record(
            &mut self,
            fields: Vec<(String, String)>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_record");
            Ok(format!("record({} fields)", fields.len()))
        }

        fn visit_expr_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_variant");
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.to_string()),
            }
        }

        fn visit_expr_array(
            &mut self,
            items: Vec<String>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_array");
            Ok(format!("[{} items]", items.len()))
        }

        fn visit_expr_binary(
            &mut self,
            op: LirBinaryOp,
            lhs: String,
            rhs: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_binary");
            Ok(format!("({lhs} {op:?} {rhs})"))
        }

        fn visit_expr_unary(
            &mut self,
            op: LirUnaryOp,
            expr: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_unary");
            Ok(format!("({op:?} {expr})"))
        }

        fn visit_expr_wildcard(
            &mut self,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_wildcard");
            Ok("_".to_string())
        }

        fn visit_expr_for_all(
            &mut self,
            _type_: &Type,
            binding: String,
            property: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_for_all");
            Ok(format!("forAll {binding} -> {property}"))
        }

        fn visit_expr_assert_consistent(
            &mut self,
            expr: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_assert_consistent");
            Ok(format!("assertConsistent({expr})"))
        }

        fn visit_expr_try(
            &mut self,
            body: String,
            binding: String,
            guard: Option<String>,
            handler: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_try");
            match guard {
                Some(g) => Ok(format!("try {body} catch {binding} if {g} => {handler}")),
                None => Ok(format!("try {body} catch {binding} => {handler}")),
            }
        }

        fn visit_expr_throw(
            &mut self,
            expr: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_throw");
            Ok(format!("throw {expr}"))
        }

        fn visit_expr_propagate(
            &mut self,
            expr: String,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_expr_propagate");
            Ok(format!("{expr}?"))
        }

        // ------ Statement hooks ------

        fn visit_stmt_let(&mut self, pat: String, value: String) -> Result<String, BackendError> {
            self.record("visit_stmt_let");
            Ok(format!("let {pat} = {value}"))
        }

        fn visit_stmt_expr(&mut self, expr: String) -> Result<String, BackendError> {
            self.record("visit_stmt_expr");
            Ok(expr)
        }

        // ------ Pattern hooks ------

        fn visit_pat_wildcard(&mut self) -> Result<String, BackendError> {
            self.record("visit_pat_wildcard");
            Ok("_".into())
        }

        fn visit_pat_literal(&mut self, value: &LirLiteral) -> Result<String, BackendError> {
            self.record("visit_pat_literal");
            Ok(format!("pat_lit({value:?})"))
        }

        fn visit_pat_variable(&mut self, name: &str) -> Result<String, BackendError> {
            self.record("visit_pat_variable");
            Ok(name.into())
        }

        fn visit_pat_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
        ) -> Result<String, BackendError> {
            self.record("visit_pat_variant");
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.into()),
            }
        }

        fn visit_pat_record(
            &mut self,
            fields: Vec<(String, String)>,
            rest: bool,
        ) -> Result<String, BackendError> {
            self.record("visit_pat_record");
            Ok(format!("pat_record({} fields, rest={rest})", fields.len()))
        }

        // ------ Declaration hooks ------

        #[allow(clippy::too_many_arguments)]
        fn visit_decl_function(
            &mut self,
            name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            body: String,
            _effect: &Effect,
            _hint: &TargetHint,
            _is_pub: bool,
            _is_generator: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_decl_function");
            Ok(format!("fn {name} = {body}"))
        }

        fn visit_decl_record_def(
            &mut self,
            name: &str,
            _fields: &[LirField],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_decl_record_def");
            Ok(format!("record {name}"))
        }

        fn visit_decl_union_def(
            &mut self,
            name: &str,
            _variants: &[LirVariant],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            self.record("visit_decl_union_def");
            Ok(format!("union {name}"))
        }

        fn visit_decl_extern(
            &mut self,
            source: &str,
            name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            _is_pub: bool,
        ) -> Result<String, BackendError> {
            self.record("visit_decl_extern");
            Ok(format!("extern {source} {name}"))
        }

        // ------ Lifecycle hooks ------

        fn enter_module(&mut self) -> Result<(), BackendError> {
            self.record("enter_module");
            Ok(())
        }

        fn exit_module(&mut self, decls: Vec<String>) -> Result<String, BackendError> {
            self.record("exit_module");
            Ok(format!("module({} decls)", decls.len()))
        }

        fn enter_function(&mut self, name: &str) -> Result<(), BackendError> {
            self.record(&format!("enter_function({name})"));
            Ok(())
        }

        fn exit_function(&mut self, name: &str, body: String) -> Result<String, BackendError> {
            self.record(&format!("exit_function({name})"));
            Ok(body)
        }
    }

    // ------------------------------------------------------------------
    // Test 1: walk_decl on Function calls enter_module and exit_module
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_decl_function_calls_enter_exit() {
        let mut spy = SpyBackend::new();
        let func_decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            body: make_literal(0),
            effect: Effect::Pure,
            hint: hint(),
            is_pub: true,
            is_generator: false,
            span: s(),
        };

        let result = crate::walk_decl(&mut spy, &func_decl);
        assert!(result.is_ok());

        // Verify lifecycle hooks were called.
        assert!(
            spy.calls.iter().any(|c| c.starts_with("enter_function")),
            "enter_function should have been called"
        );
        assert!(
            spy.calls.iter().any(|c| c.starts_with("exit_function")),
            "exit_function should have been called"
        );
    }

    // ------------------------------------------------------------------
    // Test 2: walk_expr on Literal reaches the hook
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_literal_reaches_hook() {
        let mut spy = SpyBackend::new();
        let lit_expr = make_literal(42);

        let result = crate::walk_expr(&mut spy, &lit_expr);
        assert!(result.is_ok());

        // Verify the literal hook was called.
        assert!(
            spy.calls.contains(&"visit_expr_literal".to_string()),
            "visit_expr_literal should have been called"
        );
    }

    // ------------------------------------------------------------------
    // Test 3: walk_expr on Binary walks children before parent (bottom-up)
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_binary_recursion() {
        let mut spy = SpyBackend::new();
        let binary_expr = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(make_literal(1)),
            rhs: Box::new(make_literal(2)),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut spy, &binary_expr);
        assert!(result.is_ok());

        // Verify bottom-up walk: literals must be visited BEFORE binary.
        let lit_positions: Vec<usize> = spy
            .calls
            .iter()
            .enumerate()
            .filter(|(_, c)| *c == "visit_expr_literal")
            .map(|(i, _)| i)
            .collect();
        let bin_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_binary")
            .expect("visit_expr_binary should have been called");

        assert_eq!(
            lit_positions.len(),
            2,
            "both literal children should be visited"
        );
        for &lit_pos in &lit_positions {
            assert!(
                lit_pos < bin_position,
                "literal hooks should be called before binary hook (bottom-up walk)"
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 4: walk_expr on Call walks func and args before call hook
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_call_with_args() {
        let mut spy = SpyBackend::new();
        let call_expr = LirExpr::Call {
            func: Box::new(make_var("f")),
            args: vec![make_literal(10), make_literal(20)],
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut spy, &call_expr);
        assert!(result.is_ok());

        // Verify all children walked before call hook.
        let call_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_call")
            .expect("visit_expr_call should have been called");

        let var_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_variable")
            .expect("visit_expr_variable should have been called for func");

        let lit_positions: Vec<usize> = spy
            .calls
            .iter()
            .enumerate()
            .filter(|(_, c)| *c == "visit_expr_literal")
            .map(|(i, _)| i)
            .collect();

        assert!(
            var_position < call_position,
            "func variable should be walked before call hook"
        );
        assert_eq!(
            lit_positions.len(),
            2,
            "both arg literals should be visited"
        );
        for &lit_pos in &lit_positions {
            assert!(
                lit_pos < call_position,
                "arg literals should be walked before call hook"
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 5: walk_expr on Block walks statements in order
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_block_statement_order() {
        let mut spy = SpyBackend::new();
        let block_expr = LirExpr::Block {
            stmts: vec![
                LirStmt::Let {
                    pat: LirPat::Variable("x".into()),
                    value: make_literal(1),
                },
                LirStmt::Expr(make_literal(2)),
            ],
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut spy, &block_expr);
        assert!(result.is_ok());

        // Verify statements walked in order: let before expr.
        let let_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_stmt_let")
            .expect("visit_stmt_let should have been called");
        let expr_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_stmt_expr")
            .expect("visit_stmt_expr should have been called");
        let block_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_block")
            .expect("visit_expr_block should have been called");

        assert!(
            let_position < expr_position,
            "let statement should be walked before expr statement"
        );
        assert!(
            expr_position < block_position,
            "all statements should be walked before block hook"
        );
    }

    // ------------------------------------------------------------------
    // Test 6: walk_expr on If walks both branches
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_if_both_branches() {
        let mut spy = SpyBackend::new();
        let if_expr = LirExpr::If {
            cond: Box::new(make_literal(1)),
            then: Box::new(make_literal(2)),
            else_: Some(Box::new(make_literal(3))),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut spy, &if_expr);
        assert!(result.is_ok());

        // Verify all three sub-expressions walked before if hook.
        let if_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_if")
            .expect("visit_expr_if should have been called");

        let lit_positions: Vec<usize> = spy
            .calls
            .iter()
            .enumerate()
            .filter(|(_, c)| *c == "visit_expr_literal")
            .map(|(i, _)| i)
            .collect();

        assert_eq!(
            lit_positions.len(),
            3,
            "cond, then, and else literals should all be visited"
        );
        for &lit_pos in &lit_positions {
            assert!(
                lit_pos < if_position,
                "all branch literals should be walked before if hook"
            );
        }
    }

    // ------------------------------------------------------------------
    // Test 7: walk_expr on Match walks all arms (pattern, guard, body)
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_expr_match_arms() {
        let mut spy = SpyBackend::new();
        let match_expr = LirExpr::Match {
            expr: Box::new(make_var("x")),
            arms: vec![
                LirArm {
                    pattern: LirPat::Literal(LirLiteral::Int(1)),
                    guard: Some(make_literal(10)),
                    body: make_literal(100),
                },
                LirArm {
                    pattern: LirPat::Wildcard,
                    guard: None,
                    body: make_literal(200),
                },
            ],
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut spy, &match_expr);
        assert!(result.is_ok());

        // Verify match hook was called.
        let match_position = spy
            .calls
            .iter()
            .position(|c| c == "visit_expr_match")
            .expect("visit_expr_match should have been called");

        // Verify patterns were walked.
        let pat_lit_count = spy
            .calls
            .iter()
            .filter(|c| *c == "visit_pat_literal")
            .count();
        let pat_wild_count = spy
            .calls
            .iter()
            .filter(|c| *c == "visit_pat_wildcard")
            .count();

        assert_eq!(
            pat_lit_count, 1,
            "first arm's literal pattern should be walked"
        );
        assert_eq!(
            pat_wild_count, 1,
            "second arm's wildcard pattern should be walked"
        );

        // Verify guard was walked (only first arm has guard).
        let guard_count = spy
            .calls
            .iter()
            .filter(|c| *c == "visit_expr_literal")
            .count();
        // Should have: guard literal (10) + body literal (100) + body literal (200) = 3
        assert_eq!(
            guard_count, 3,
            "guard and body literals should all be walked"
        );

        // All arm components should be walked before match hook.
        let pat_lit_pos = spy
            .calls
            .iter()
            .position(|c| c == "visit_pat_literal")
            .expect("visit_pat_literal should have been called");
        assert!(
            pat_lit_pos < match_position,
            "arm patterns should be walked before match hook"
        );
    }

    // ------------------------------------------------------------------
    // Test 8: walk_decl on Extern calls visit_decl_extern directly
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_decl_extern_direct() {
        let mut spy = SpyBackend::new();
        let extern_decl = LirDecl::Extern {
            source: "libc".into(),
            name: "read".into(),
            params: vec![],
            return_type: None,
            is_pub: false,
        };

        let result = crate::walk_decl(&mut spy, &extern_decl);
        assert!(result.is_ok());

        // Verify visit_decl_extern was called.
        assert!(
            spy.calls.contains(&"visit_decl_extern".to_string()),
            "visit_decl_extern should have been called"
        );
    }

    // ------------------------------------------------------------------
    // Test 9: walk_decl on full module (RecordDef + Function)
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_full_module() {
        let mut spy = SpyBackend::new();
        let module = vec![
            LirDecl::RecordDef {
                name: "Point".into(),
                fields: vec![LirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                }],
                is_pub: true,
                span: s(),
            },
            LirDecl::Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                body: LirExpr::Block {
                    stmts: vec![
                        LirStmt::Let {
                            pat: LirPat::Variable("x".into()),
                            value: make_literal(1),
                        },
                        LirStmt::Expr(make_literal(2)),
                    ],
                    hint: hint(),
                    span: s(),
                },
                effect: Effect::Pure,
                hint: hint(),
                is_pub: true,
                is_generator: false,
                span: s(),
            },
        ];

        crate::walk_module(&mut spy, &module).unwrap();

        // Verify lifecycle order: enter_module → record_def → enter_function → ... → exit_function → exit_module
        let enter_mod_pos = spy
            .calls
            .iter()
            .position(|c| c == "enter_module")
            .expect("enter_module should have been called");
        let record_pos = spy
            .calls
            .iter()
            .position(|c| c == "visit_decl_record_def")
            .expect("visit_decl_record_def should have been called");
        let enter_func_pos = spy
            .calls
            .iter()
            .position(|c| c.starts_with("enter_function"))
            .expect("enter_function should have been called");
        let exit_func_pos = spy
            .calls
            .iter()
            .position(|c| c.starts_with("exit_function"))
            .expect("exit_function should have been called");
        let exit_mod_pos = spy
            .calls
            .iter()
            .position(|c| c == "exit_module")
            .expect("exit_module should have been called");

        assert!(enter_mod_pos < record_pos, "enter_module before record_def");
        assert!(
            record_pos < enter_func_pos,
            "record_def before enter_function"
        );
        assert!(
            enter_func_pos < exit_func_pos,
            "enter_function before exit_function"
        );
        assert!(
            exit_func_pos < exit_mod_pos,
            "exit_function before exit_module"
        );
    }

    // ------------------------------------------------------------------
    // Test 10: walk preserves reduced types (R=i32)
    // ------------------------------------------------------------------

    #[test]
    fn test_walk_preserves_reduced_types() {
        // Use a CountBackend that returns i32 (node count).
        struct CountBackend;

        impl LirBackend<i32> for CountBackend {
            fn visit_expr_literal(
                &mut self,
                _v: &LirLiteral,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_variable(
                &mut self,
                _n: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_call(
                &mut self,
                f: i32,
                args: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(f + args.into_iter().sum::<i32>())
            }
            fn visit_expr_member(
                &mut self,
                obj: i32,
                _f: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(obj)
            }
            fn visit_expr_optional_access(
                &mut self,
                obj: i32,
                _f: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(obj)
            }
            fn visit_expr_if(
                &mut self,
                c: i32,
                t: i32,
                e: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(c + t + e.unwrap_or(0))
            }
            fn visit_expr_match(
                &mut self,
                expr: i32,
                _arms: Vec<ReducedArm<i32>>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(expr)
            }
            fn visit_expr_block(
                &mut self,
                stmts: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(stmts.into_iter().sum())
            }
            fn visit_expr_assign(
                &mut self,
                t: i32,
                v: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(t + v)
            }
            fn visit_expr_lambda(
                &mut self,
                _p: &[LirParam],
                body: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_expr_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            fn visit_expr_variant(
                &mut self,
                _n: &str,
                arg: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(arg.unwrap_or(0))
            }
            fn visit_expr_array(
                &mut self,
                items: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(items.into_iter().sum())
            }
            fn visit_expr_binary(
                &mut self,
                _op: LirBinaryOp,
                l: i32,
                r: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(l + r + 1)
            }
            fn visit_expr_unary(
                &mut self,
                _op: LirUnaryOp,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_wildcard(
                &mut self,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_expr_for_all(
                &mut self,
                _t: &Type,
                _b: i32,
                p: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(p)
            }
            fn visit_expr_assert_consistent(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_try(
                &mut self,
                body: i32,
                _b: i32,
                _g: Option<i32>,
                h: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body + h)
            }
            fn visit_expr_throw(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_propagate(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_stmt_let(&mut self, _p: i32, v: i32) -> Result<i32, BackendError> {
                Ok(v)
            }
            fn visit_stmt_expr(&mut self, e: i32) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_pat_wildcard(&mut self) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_pat_literal(&mut self, _v: &LirLiteral) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variable(&mut self, _n: &str) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variant(&mut self, _n: &str, a: Option<i32>) -> Result<i32, BackendError> {
                Ok(a.unwrap_or(0))
            }
            fn visit_pat_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _rest: bool,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            #[allow(clippy::too_many_arguments)]
            fn visit_decl_function(
                &mut self,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                body: i32,
                _e: &Effect,
                _h: &TargetHint,
                _pub: bool,
                _gen: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_decl_record_def(
                &mut self,
                _n: &str,
                _f: &[LirField],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_union_def(
                &mut self,
                _n: &str,
                _v: &[LirVariant],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_extern(
                &mut self,
                _s: &str,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                _pub: bool,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn enter_module(&mut self) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_module(&mut self, decls: Vec<i32>) -> Result<i32, BackendError> {
                Ok(decls.into_iter().sum())
            }
            fn enter_function(&mut self, _n: &str) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_function(&mut self, _n: &str, body: i32) -> Result<i32, BackendError> {
                Ok(body)
            }
        }

        let mut backend = CountBackend;
        // Walk a binary expression: 1 + 2 should reduce to 3 (1 + 2 = 3 nodes).
        let binary_expr = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(make_literal(1)),
            rhs: Box::new(make_literal(2)),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &binary_expr);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            3,
            "binary with two literals should reduce to 3"
        );
    }

    // ==================================================================
    // DebugBackend (reference test backend) tests
    // ==================================================================

    // ------------------------------------------------------------------
    // Test 1: DebugBackend renders a literal integer
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_literal() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("(literal 42)"),
            "expected output to contain '(literal 42)', got: {output}"
        );
    }

    // ------------------------------------------------------------------
    // Test 2: DebugBackend renders a variable reference
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_variable() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::Variable {
            name: "x".into(),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(
            output, "(var x)",
            "variable expression should render as '(var x)'"
        );
    }

    // ------------------------------------------------------------------
    // Test 3: DebugBackend renders a function call with args
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_call() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::Call {
            func: Box::new(make_var("f")),
            args: vec![make_literal(1), make_literal(2)],
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("(call"),
            "output should contain '(call', got: {output}"
        );
        assert!(
            output.contains("(var f)"),
            "output should contain the func '(var f)', got: {output}"
        );
        assert!(
            output.contains("(literal 1)"),
            "output should contain first arg '(literal 1)', got: {output}"
        );
        assert!(
            output.contains("(literal 2)"),
            "output should contain second arg '(literal 2)', got: {output}"
        );
    }

    // ------------------------------------------------------------------
    // Test 4: DebugBackend renders if-then-else with both branches
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_if_then_else() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::If {
            cond: Box::new(make_var("cond")),
            then: Box::new(make_literal(1)),
            else_: Some(Box::new(make_literal(2))),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("(if"),
            "output should contain '(if', got: {output}"
        );
        assert!(
            output.contains("(var cond)"),
            "output should contain condition '(var cond)', got: {output}"
        );
        assert!(
            output.contains("(literal 1)"),
            "output should contain then-branch '(literal 1)', got: {output}"
        );
        assert!(
            output.contains("(literal 2)"),
            "output should contain else-branch '(literal 2)', got: {output}"
        );
    }

    // ------------------------------------------------------------------
    // Test 5: DebugBackend renders a block with let-binding and final expr
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_block_with_let() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::Block {
            stmts: vec![
                LirStmt::Let {
                    pat: LirPat::Variable("x".into()),
                    value: make_literal(10),
                },
                LirStmt::Expr(make_var("x")),
            ],
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("(block"),
            "output should contain '(block', got: {output}"
        );
        assert!(
            output.contains("(let x"),
            "output should contain '(let x' for the binding, got: {output}"
        );
        assert!(
            output.contains("(literal 10)"),
            "output should contain the assigned value '(literal 10)', got: {output}"
        );
        assert!(
            output.contains("(var x)"),
            "output should contain the final expression '(var x)', got: {output}"
        );

        // Verify ordering: let should appear before the final var x.
        let let_pos = output
            .find("(let x")
            .expect("should find '(let x' in output");
        let var_pos = output
            .find("(var x)")
            .expect("should find '(var x)' in output");
        assert!(
            let_pos < var_pos,
            "let-binding should appear before the final variable reference"
        );
    }

    // ------------------------------------------------------------------
    // Test 6: DebugBackend renders a binary operation with operator
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_binary() {
        let mut backend = crate::DebugBackend;
        let expr = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(make_literal(1)),
            rhs: Box::new(make_literal(2)),
            hint: hint(),
            span: s(),
        };

        let result = crate::walk_expr(&mut backend, &expr);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("(binary"),
            "output should contain '(binary', got: {output}"
        );
        assert!(
            output.contains("+"),
            "output should contain the '+' operator for Add, got: {output}"
        );
        assert!(
            output.contains("(literal 1)"),
            "output should contain lhs '(literal 1)', got: {output}"
        );
        assert!(
            output.contains("(literal 2)"),
            "output should contain rhs '(literal 2)', got: {output}"
        );
    }

    // ------------------------------------------------------------------
    // Test 7: DebugBackend renders a function declaration with body
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_function_decl() {
        let mut backend = crate::DebugBackend;
        let func_decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            body: make_literal(99),
            effect: Effect::Pure,
            hint: hint(),
            is_pub: false,
            is_generator: false,
            span: s(),
        };

        let result = crate::walk_decl(&mut backend, &func_decl);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("main"),
            "output should contain function name 'main', got: {output}"
        );
        assert!(
            output.contains("(literal 99)"),
            "output should contain the body '(literal 99)', got: {output}"
        );
    }

    // ------------------------------------------------------------------
    // Test 8: DebugBackend renders a full module with record + function
    // ------------------------------------------------------------------

    #[test]
    fn test_debug_backend_full_module() {
        let mut backend = crate::DebugBackend;
        let decls = vec![
            LirDecl::RecordDef {
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
            },
            LirDecl::Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                body: make_literal(0),
                effect: Effect::Pure,
                hint: hint(),
                is_pub: true,
                is_generator: false,
                span: s(),
            },
        ];

        let result = crate::walk_module(&mut backend, &decls);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(
            output.contains("Point"),
            "output should contain record name 'Point', got: {output}"
        );
        assert!(
            output.contains("main"),
            "output should contain function name 'main', got: {output}"
        );
        assert!(
            output.contains("(literal 0)"),
            "output should contain the function body '(literal 0)', got: {output}"
        );
    }
}
