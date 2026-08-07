//! Gungnir — DWARF-120 formal verification engine (pure logic).
//!
//! This module provides the SMT-LIB2 translator and verification-query
//! builder for `@gungnir`-annotated functions. It is intentionally *pure*: it
//! produces SMT-LIB2 text and parses Z3's output but never spawns a
//! subprocess. The Z3 process bridge lives in the `forge` CLI.
//!
//! # Discovery
//!
//! [`discover_gungnir`] scans top-level declarations for functions carrying the
//! `@gungnir` marker and attaches their `@requires` / `@ensures` /
//! `@invariant` conditions (which the parser stringifies) back into `Expr`
//! form via `dwarf_parser::parse_expr_str`.
//!
//! # Query shape
//! For `fn f(params...) -> T { body }` with `@requires(PRE)`,
//! `@ensures(POST)`, `@invariant(INV)` the emitted script is, in order:
//!
//! ```text
//! (declare-const <param> <sort>)...          // Int for Int params
//! (declare-const <param>@pre <sort>)...      // ONLY for params referenced by old()
//! (declare-const result <sort>)
//! (assert (= <param> <param>@pre))...        // ONLY for params referenced by old()
//! (assert <INV resolved to record param>)    // when @invariant present (entry)
//! (assert <PRE>)                             // when @requires present
//! (assert (= result <body>))
//! (assert (not <POST>))
//! (check-sat)
//! (get-model)
//! ```

use dwarf_syntax::hir::{
    BinaryOp, Decl, Decorator, Expr, LiteralValue, Param, Stmt, Type, UnaryOp,
};

/// The contract attached to a `@gungnir` function.
#[derive(Debug, Clone)]
pub struct GungnirContract {
    /// Parsed `@requires(condition)`.
    pub pre: Option<Expr>,
    /// Parsed `@ensures(condition)`.
    pub post: Option<Expr>,
    /// Parsed `@invariant(condition)`.
    pub invariant: Option<Expr>,
}

/// A `@gungnir`-annotated function together with its contract.
#[derive(Debug, Clone)]
pub struct GungnirFunction {
    /// Function name.
    pub name: String,
    /// Declared parameters.
    pub params: Vec<Param>,
    /// Return type, if declared.
    pub return_type: Option<Type>,
    /// Function body expression.
    pub body: Expr,
    /// The attached contract.
    pub contract: GungnirContract,
}

/// Outcome of a verification query.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The contract holds (Z3 answered `unsat`).
    Proved,
    /// The contract is violated; carries the counterexample model.
    Counterexample { model: String },
    /// Z3 answered `unknown` (timeout / undecidable).
    Unproven { reason: String },
    /// The output was unrecognisable / the toolchain failed.
    Error { reason: String },
}

impl Verdict {
    /// A stable, short human label for the verdict.
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Proved => "proved",
            Verdict::Counterexample { .. } => "counterexample",
            Verdict::Unproven { .. } => "unproven",
            Verdict::Error { .. } => "error",
        }
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discover all `@gungnir`-annotated functions in a declaration slice.
///
/// Plain (undecorated) functions are ignored. Each discovered function has its
/// `@requires` / `@ensures` / `@invariant` conditions re-parsed from their
/// stringified forms back into `Expr`.
pub fn discover_gungnir(decls: &[Decl]) -> Vec<GungnirFunction> {
    let mut out = Vec::new();

    for decl in decls {
        let (name, params, return_type, body, decorators) = match decl {
            Decl::Function {
                name,
                params,
                return_type,
                body,
                decorators,
                ..
            } => (name, params, return_type, body, decorators),
            _ => continue,
        };

        if !decorators.iter().any(|d| matches!(d, Decorator::Gungnir)) {
            continue;
        }

        let mut pre = None;
        let mut post = None;
        let mut invariant = None;

        for dec in decorators {
            match dec {
                Decorator::Requires { condition } => {
                    pre = parse_condition(condition);
                }
                Decorator::Ensures { condition } => {
                    post = parse_condition(condition);
                }
                Decorator::Invariant { condition } => {
                    invariant = parse_condition(condition);
                }
                _ => {}
            }
        }

        out.push(GungnirFunction {
            name: name.clone(),
            params: params.clone(),
            return_type: return_type.clone(),
            body: body.clone(),
            contract: GungnirContract {
                pre,
                post,
                invariant,
            },
        });
    }

    out
}

/// Re-parse a stringified condition (`result >= 0`) into an `Expr`.
fn parse_condition(condition: &str) -> Option<Expr> {
    dwarf_parser::parse_expr_str(condition).ok()
}

// ---------------------------------------------------------------------------
// SMT-LIB2 expression translator
// ---------------------------------------------------------------------------

/// Translate a Dwarf `Expr` into SMT-LIB2 text.
pub fn translate_smt2(expr: &Expr) -> String {
    translate(expr)
}

/// Core recursive SMT-LIB2 translator.
fn translate(expr: &Expr) -> String {
    match expr {
        // A function body is a `Block` with a single trailing expression; if it
        // has exactly one Expression statement, translate that directly.
        Expr::Block { stmts, .. } => match &stmts[..] {
            [Stmt::Expr(inner)] => translate(inner),
            _ => format!("{:?}", expr),
        },
        Expr::Literal { value, .. } => translate_literal(value),
        Expr::Variable { name, .. } => name.clone(),
        Expr::Call { func, args, .. } => translate_call(func, args),
        Expr::Member { obj, field, .. } => format!("{}.{}", translate(obj), field),
        Expr::If {
            cond,
            then,
            else_,
            ..
        } => {
            if let Some(else_expr) = else_ {
                format!(
                    "(ite {} {} {})",
                    translate(cond),
                    translate(then),
                    translate(else_expr)
                )
            } else {
                format!("(ite {} {} false)", translate(cond), translate(then))
            }
        }
        Expr::Binary {
            op, lhs, rhs, ..
        } => translate_binary(op, &translate(lhs), &translate(rhs)),
        Expr::Unary { op, expr, .. } => {
            let inner = translate(expr);
            match op {
                UnaryOp::Neg => format!("(- {})", inner),
                UnaryOp::Not => format!("(not {})", inner),
            }
        }
        // Fallback for the unsupported v1 subset — render the debug form so
        // query building stays total (never panics).
        other => format!("{:?}", other),
    }
}

/// Translate a literal value for SMT-LIB2.
fn translate_literal(value: &LiteralValue) -> String {
    match value {
        LiteralValue::Int(i) => i.to_string(),
        LiteralValue::Float(f) => format!("{}", f),
        LiteralValue::Str(_) | LiteralValue::RawStr(_) => "\"\"".to_string(),
        LiteralValue::Bool(b) => b.to_string(),
        LiteralValue::Null => "null".to_string(),
    }
}

/// Translate a call expression, special-casing `old(e)` → `e@pre`.
fn translate_call(func: &Expr, args: &[Expr]) -> String {
    // Special-case the pre-state marker `old(e)`.
    if let Expr::Variable { name, .. } = func {
        if name == "old" {
            if let Some(arg) = args.first() {
                return prestate(arg);
            }
            return "null@pre".to_string();
        }
    }

    let func_str = translate(func);
    let arg_strs: Vec<String> = args.iter().map(translate).collect();
    format!("({} {})", func_str, arg_strs.join(" "))
}

/// Translate an expression into its pre-state namespace (`e@pre`).
///
/// This is used to materialise `old(e)` as a distinct, observable symbol. It
/// recursively renders the expression, tagging leaf variables with `@pre`.
fn prestate(expr: &Expr) -> String {
    match expr {
        Expr::Block { stmts, .. } => match &stmts[..] {
            [Stmt::Expr(inner)] => prestate(inner),
            _ => format!("{:?}", expr),
        },
        Expr::Literal { value, .. } => translate_literal(value),
        Expr::Variable { name, .. } => format!("{}@pre", name),
        Expr::Call { func, args, .. } => {
            let func_str = if let Expr::Variable { name, .. } = func.as_ref() {
                format!("{}@pre", name)
            } else {
                translate(func)
            };
            let arg_strs: Vec<String> = args.iter().map(prestate).collect();
            format!("({} {})", func_str, arg_strs.join(" "))
        }
        Expr::Member { obj, field, .. } => format!("{}@pre.{}", prestate(obj), field),
        Expr::Binary {
            op, lhs, rhs, ..
        } => translate_binary(op, &prestate(lhs), &prestate(rhs)),
        Expr::Unary { op, expr, .. } => {
            let inner = prestate(expr);
            match op {
                UnaryOp::Neg => format!("(- {})", inner),
                UnaryOp::Not => format!("(not {})", inner),
            }
        }
        Expr::If {
            cond,
            then,
            else_,
            ..
        } => {
            if let Some(else_expr) = else_ {
                format!(
                    "(ite {} {} {})",
                    prestate(cond),
                    prestate(then),
                    prestate(else_expr)
                )
            } else {
                format!("(ite {} {} false)", prestate(cond), prestate(then))
            }
        }
        other => format!("{:?}", other),
    }
}

/// Translate a binary operation.
fn translate_binary(op: &dwarf_syntax::hir::BinaryOp, l: &str, r: &str) -> String {
    match op {
        BinaryOp::Add => format!("(+ {} {})", l, r),
        BinaryOp::Sub => format!("(- {} {})", l, r),
        BinaryOp::Mul => format!("(* {} {})", l, r),
        BinaryOp::Div => format!("(div {} {})", l, r),
        BinaryOp::Eq => format!("(= {} {})", l, r),
        BinaryOp::Ne => format!("(not (= {} {}))", l, r),
        BinaryOp::Lt => format!("(< {} {})", l, r),
        BinaryOp::Gt => format!("(> {} {})", l, r),
        BinaryOp::Le => format!("(<= {} {})", l, r),
        BinaryOp::Ge => format!("(>= {} {})", l, r),
        BinaryOp::And => format!("(and {} {})", l, r),
        BinaryOp::Or => format!("(or {} {})", l, r),
    }
}

// ---------------------------------------------------------------------------
// Verification-query builder
// ---------------------------------------------------------------------------

/// Build the full SMT-LIB2 verification script for a Gungnir function.
pub fn build_verification_query(f: &GungnirFunction) -> String {
    let mut lines = Vec::new();

    let (declares, old_bindings) = build_param_decls(f);

    // 1. declare-const each param / hoisted record field
    lines.extend(declares);

    // 2. declare-const <param>@pre for old-referenced params
    for param in &old_bindings {
        lines.push(format!("(declare-const {}@pre Int)", param));
    }

    // 3. declare-const result
    lines.push("(declare-const result Int)".to_string());

    // 4. assert (= <param> <param>@pre) for old-referenced params
    for param in &old_bindings {
        lines.push(format!("(assert (= {} {}@pre))", param, param));
    }

    // 5. entry invariant (resolved to the record param)
    if let Some(inv) = &f.contract.invariant {
        let resolved = resolve_invariant(inv, f);
        lines.push(format!("(assert {})", translate(&resolved)));
    }

    // 6. pre-condition
    if let Some(pre) = &f.contract.pre {
        lines.push(format!("(assert {})", translate(pre)));
    }

    // 7. post-state: result equals body
    lines.push(format!("(assert (= result {}))", translate(&f.body)));

    // 8. negate the post-condition
    if let Some(post) = &f.contract.post {
        lines.push(format!("(assert (not {}))", translate(post)));
    }

    // 9. / 10.
    lines.push("(check-sat)".to_string());
    lines.push("(get-model)".to_string());

    lines.join("\n")
}

/// Compute the param declaration lines and the list of `old()`-referenced
/// params that need `name@pre` declares + equality asserts.
fn build_param_decls(f: &GungnirFunction) -> (Vec<String>, Vec<String>) {
    let mut declares = Vec::new();
    let mut old_bindings = Vec::new();

    for param in &f.params {
        match &param.type_ {
            Some(t) if is_primitive_type(t) => {
                declares.push(format!("(declare-const {} {})", param.name, type_sort(t)));
            }
            Some(_) => {
                // Record-typed param: hoist referenced fields.
                let fields = record_fields(f, &param.name);
                if fields.is_empty() {
                    declares.push(format!("(declare-const {} Int)", param.name));
                } else {
                    for field in fields {
                        declares.push(format!("(declare-const {}.{} Int)", param.name, field));
                    }
                }
            }
            None => {
                declares.push(format!("(declare-const {} Int)", param.name));
            }
        }
    }

    if let Some(post) = &f.contract.post {
        collect_old_refs(post, &f.params, &mut old_bindings);
    }

    (declares, old_bindings)
}

/// The SMT-LIB2 sort for a primitive-typed param.
fn type_sort(t: &Type) -> String {
    match t {
        Type::Named(n) => n.clone(),
        _ => "Int".to_string(),
    }
}

/// Whether a type is a primitive SMT sort (as opposed to a record).
fn is_primitive_type(t: &Type) -> bool {
    match t {
        Type::Named(n) => matches!(n.as_str(), "Int" | "Float" | "Bool" | "String"),
        _ => false,
    }
}

/// Collect every parameter referenced under `old(...)` in an expression.
fn collect_old_refs(expr: &Expr, params: &[Param], acc: &mut Vec<String>) {
    collect_old_refs_inner(expr, params, acc);
}

fn collect_old_refs_inner(expr: &Expr, params: &[Param], acc: &mut Vec<String>) {
    match expr {
        Expr::Call { func, args, .. } => {
            if let Expr::Variable { name, .. } = func.as_ref() {
                if name == "old" {
                    for arg in args {
                        old_arg_refs(arg, params, acc);
                    }
                }
            }
            for arg in args {
                collect_old_refs_inner(arg, params, acc);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_old_refs_inner(lhs, params, acc);
            collect_old_refs_inner(rhs, params, acc);
        }
        _ => {}
    }
}

/// Record which parameter a bare `old(x)` argument refers to.
///
/// Only bare parameter references are tracked; compound/tagged `old()`
/// arguments are out of scope for v1.
fn old_arg_refs(arg: &Expr, params: &[Param], acc: &mut Vec<String>) {
    if let Expr::Variable { name, .. } = arg {
        if params.iter().any(|p| &p.name == name) && !acc.contains(name) {
            acc.push(name.to_string());
        }
    }
}

/// Discover the referenced field names of a record-typed param by walking the
/// function body and contract conditions for `param.field` member accesses.
fn record_fields(f: &GungnirFunction, param: &str) -> Vec<String> {
    let mut fields = Vec::new();
    collect_member_fields(&f.body, param, &mut fields);
    if let Some(pre) = &f.contract.pre {
        collect_member_fields(pre, param, &mut fields);
    }
    if let Some(post) = &f.contract.post {
        collect_member_fields(post, param, &mut fields);
    }
    if let Some(inv) = &f.contract.invariant {
        collect_member_fields(inv, param, &mut fields);
    }
    fields
}

/// Collect distinct `param.<field>` member names in an expression.
fn collect_member_fields(expr: &Expr, param: &str, acc: &mut Vec<String>) {
    match expr {
        Expr::Block { stmts, .. } => {
            for stmt in stmts {
                match stmt {
                    Stmt::Expr(e) => collect_member_fields(e, param, acc),
                    Stmt::Let(_, e) => collect_member_fields(e, param, acc),
                }
            }
        }
        Expr::Member { obj, field, .. } => {
            if let Expr::Variable { name, .. } = obj.as_ref() {
                if name == param && !acc.contains(field) {
                    acc.push(field.to_string());
                }
            }
            collect_member_fields(obj, param, acc);
        }
        Expr::Call { func, args, .. } => {
            collect_member_fields(func, param, acc);
            for arg in args {
                collect_member_fields(arg, param, acc);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            collect_member_fields(lhs, param, acc);
            collect_member_fields(rhs, param, acc);
        }
        Expr::If {
            cond,
            then,
            else_,
            ..
        } => {
            collect_member_fields(cond, param, acc);
            collect_member_fields(then, param, acc);
            if let Some(e) = else_ {
                collect_member_fields(e, param, acc);
            }
        }
        Expr::Unary { expr, .. } => collect_member_fields(expr, param, acc),
        _ => {}
    }
}

/// Resolve bare field references inside an `@invariant` against the
/// record-typed parameter (e.g. `balance` → `w.balance` for `w: Wallet`).
fn resolve_invariant(inv: &Expr, f: &GungnirFunction) -> Expr {
    let Some((param, field)) = record_param_alias(f) else {
        // No record-typed param with discovered fields — leave as-is.
        return inv.clone();
    };
    rewrite_bare_field(inv, &param, &field)
}

/// Find the record-typed param and its first discovered field, if any.
fn record_param_alias(f: &GungnirFunction) -> Option<(String, String)> {
    for param in &f.params {
        if let Some(t) = &param.type_ {
            if !is_primitive_type(t) {
                let fields = record_fields(f, &param.name);
                if let Some(field) = fields.first() {
                    return Some((param.name.clone(), field.clone()));
                }
            }
        }
    }
    None
}

/// Rewrite a bare identifier matching the record's primary field into a
/// qualified `param.field` member expression.
fn rewrite_bare_field(expr: &Expr, param: &str, field: &str) -> Expr {
    match expr {
        Expr::Variable { name, span } if name == field => Expr::Member {
            obj: Box::new(Expr::Variable {
                name: param.to_string(),
                span: *span,
            }),
            field: name.clone(),
            span: *span,
        },
        Expr::Binary { op, lhs, rhs, span } => Expr::Binary {
            op: op.clone(),
            lhs: Box::new(rewrite_bare_field(lhs, param, field)),
            rhs: Box::new(rewrite_bare_field(rhs, param, field)),
            span: *span,
        },
        Expr::Unary { op, expr, span } => Expr::Unary {
            op: op.clone(),
            expr: Box::new(rewrite_bare_field(expr, param, field)),
            span: *span,
        },
        Expr::If {
            cond,
            then,
            else_,
            span,
        } => Expr::If {
            cond: Box::new(rewrite_bare_field(cond, param, field)),
            then: Box::new(rewrite_bare_field(then, param, field)),
            else_: else_
                .as_ref()
                .map(|e| Box::new(rewrite_bare_field(e, param, field))),
            span: *span,
        },
        Expr::Member { obj, field, span } => Expr::Member {
            obj: Box::new(rewrite_bare_field(obj, param, field)),
            field: field.clone(),
            span: *span,
        },
        Expr::Call { func, args, span } => Expr::Call {
            func: Box::new(rewrite_bare_field(func, param, field)),
            args: args
                .iter()
                .map(|a| rewrite_bare_field(a, param, field))
                .collect(),
            span: *span,
        },
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Verdict parsing
// ---------------------------------------------------------------------------

/// Parse Z3 stdout into a `Verdict`.
///
/// - leading `unsat` → `Proved`
/// - leading `sat` → `Counterexample` (model following the verdict)
/// - leading `unknown` → `Unproven`
/// - anything else / empty → `Error`
///
/// Crucially, after `unsat` we do NOT rely on the optional `(get-model)` block:
/// Z3 prints `(error "model is not available")` there, which would otherwise be
/// mistaken for a counterexample. Only the first verdict line is authoritative.
pub fn parse_smt_output(stdout: &str) -> Verdict {
    let trimmed = stdout.trim_start();

    let first = first_line(trimmed);

    if first == "unsat" {
        return Verdict::Proved;
    }
    if first == "sat" {
        return Verdict::Counterexample {
            model: extract_model(trimmed),
        };
    }
    if first == "unknown" {
        return Verdict::Unproven {
            reason: "z3 returned `unknown` (timeout / undecidable)".to_string(),
        };
    }

    if trimmed.is_empty() {
        return Verdict::Error {
            reason: "empty SMT-LIB output".to_string(),
        };
    }

    Verdict::Error {
        reason: format!("unrecognised SMT-LIB output: {:?}", stdout),
    }
}

/// The first non-empty line.
fn first_line(s: &str) -> &str {
    s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("")
}

/// Extract the counterexample model from `sat` output: everything after the
/// `sat` verdict line, trimmed of the enclosing S-expression parens.
fn extract_model(stdout: &str) -> String {
    let mut model_lines = Vec::new();
    let mut saw_verdict = false;
    for line in stdout.lines() {
        let t = line.trim();
        if !saw_verdict {
            if t.starts_with("sat") {
                saw_verdict = true;
            }
            continue;
        }
        if t.is_empty() {
            continue;
        }
        model_lines.push(t.to_string());
    }
    model_lines.join("\n")
}

// ---------------------------------------------------------------------------
// Tests are integration-level (`dwarf-lib/tests/gungnir_tests.rs`); no unit
// tests live here to keep this module focused on the pure translator/query/
// verdict logic.
// ---------------------------------------------------------------------------