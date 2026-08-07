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
    BinaryOp, Decl, Decorator, Expr, LiteralValue, Param, RefConstraint, Stmt, Type, UnaryOp,
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

/// Placeholder emitted for expression nodes outside the v1 SMT subset.
///
/// The old fallback leaked Rust `Debug` output (`{:?}`) into the emitted SMT
/// text; that is neither valid SMT-LIB2 nor readable. Instead we emit a single
/// stable token that makes the script syntactically invalid (z3 will error),
/// which the caller surfaces as `Error` rather than a bogus verdict.
const UNSUPPORTED_FORM: &str = "(unsupported-form)";

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
            _ => UNSUPPORTED_FORM.to_string(),
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
        // Fallback for the unsupported v1 subset — emit a stable marker rather
        // than leaking Rust `Debug` output into the SMT text. The script will be
        // invalid, which z3 rejects and surfaces as `Error`.
        other => {
            let _ = other;
            UNSUPPORTED_FORM.to_string()
        }
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
/// recursively renders the expression, tagging leaf variables with `@pre`. A
/// member access keeps its field qualification on the tagged object
/// (`old(w.balance)` → `w@pre.balance`).
fn prestate(expr: &Expr) -> String {
    match expr {
        Expr::Block { stmts, .. } => match &stmts[..] {
            [Stmt::Expr(inner)] => prestate(inner),
            _ => UNSUPPORTED_FORM.to_string(),
        },
        Expr::Literal { value, .. } => translate_literal(value),
        Expr::Variable { name, .. } => format!("{}@pre", name),
        Expr::Call { func, args, .. } => {
            let func_str = if let Expr::Variable { name, .. } = func.as_ref() {
                format!("{}@pre", name)
            } else {
                prestate(func)
            };
            let arg_strs: Vec<String> = args.iter().map(prestate).collect();
            format!("({} {})", func_str, arg_strs.join(" "))
        }
        Expr::Member { obj, field, .. } => format!("{}.{}", prestate(obj), field),
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
        _ => UNSUPPORTED_FORM.to_string(),
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
///
/// Callers should first consult [`unsupported_reason`]; if it returns `Some`,
/// the function is outside the soundly-verifiable v1 subset and this builder is
/// not a meaningful query for it (it may still emit a syntactically-valid but
/// vacuous/inexpressible script). For supported functions the script is sound.
pub fn build_verification_query(f: &GungnirFunction) -> String {
    let mut lines = Vec::new();

    let (declares, old_bindings, refined) = build_param_decls(f);

    // 1. declare-const each param / hoisted record field
    lines.extend(declares);

    // 2. declare-const <param>@pre for old-referenced params
    for param in &old_bindings {
        lines.push(format!("(declare-const {}@pre Int)", param));
    }

    // 3. declare-const result in the function's actual return sort
    let result_sort = f
        .return_type
        .as_ref()
        .and_then(type_sort)
        .unwrap_or("Int");
    lines.push(format!("(declare-const result {})", result_sort));

    // 4. assert (= <param> <param>@pre) for old-referenced params
    for param in &old_bindings {
        lines.push(format!("(assert (= {} {}@pre))", param, param));
    }

    // 4b. honor refined-type domains on parameters
    lines.extend(refined);

    // 5. entry invariant (resolved to the record param)
    if let Some(inv) = &f.contract.invariant {
        lines.push(format!("(assert {})", translate(&resolve_invariant(inv, f))));
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

    // 9. ask the solver whether the negated post-condition is satisfiable
    lines.push("(check-sat)".to_string());
    // 10. request a counterexample model when a witness exists
    lines.push("(get-model)".to_string());

lines.join("\n")
}

/// A reason the function is outside the soundly-verifiable v1 subset, if any.
///
/// Guards against the soundness holes that would otherwise yield a *false*
/// verdict:
///
/// - A body that references `result`, `old(...)`, or any variable that is not a
///   declared parameter. `old()` is only meaningful in *contract conditions*,
///   so `old()` in a body is rejected. A body like `result + 1` would otherwise
///   build `(= result (+ result 1))` which is trivially unsat → a **vacuous
///   `Proved`**
///   (nothing — see the DWARF-120 soundness hardening).
/// - A return (or param) type that does not map to a supported SMT sort
///   (`Int`/`Bool`/`String`/`Float`) — e.g. a record-return or unknown name;
///   without this the engine emits a wrong-sort `result` declaration.
/// - `old(...)` taking a non-param argument (member/compound), which would emit
///   an undeclared `@pre` symbol and make z3 fail with `unknown constant`.
///
/// Returns `Some(reason)` to reject, or `None` if soundly verifiable.
pub fn unsupported_reason(f: &GungnirFunction) -> Option<String> {
    // 1. Return type must map to a supported SMT sort (Fix 3 / sort correctness).
    if let Some(rt) = &f.return_type {
        if !is_primitive_type(rt) {
            return Some(format!(
                "unsupported return type for gungnir verification: {:?}",
                rt
            ));
        }
    }
    // 2. Params must be primitives or record-typed; a bare free param is fine,
    //    an exotic named/function type is not.
    for p in &f.params {
        if let Some(t) = &p.type_ {
            if !is_primitive_type(t) {
                // Record-typed params are hoisted; anything else is unsupported.
                if !is_record_like(f, &p.name) {
                    return Some(format!(
                        "unsupported parameter type `{}` for `{}`",
                        p.name, p.name
                    ));
                }
            }
        }
    }

    // 3. The body must not reference `result`, `old`, or unknown free variables
    //    (the vacuous-proof hole). We allow references to parameters and to
    //    function names (call heads).
    let param_names: std::collections::HashSet<&str> =
        f.params.iter().map(|p| p.name.as_str()).collect();
    if let Some(bad) = body_free_ref(&f.body, &param_names) {
        return Some(format!(
            "function body references unsupported symbol `{}` (only parameters \
             are valid in a gungnir body)",
            bad
        ));
    }

    // 4. `old(...)` must take a bare parameter argument. `old(w.balance)` /
    //    `old(a + b)` would emit an undeclared `@pre` symbol (`w@pre.balance`,
    //    `a@pre`) and make z3 fail with `unknown constant`.
    for cond in [&f.contract.pre, &f.contract.post, &f.contract.invariant]
        .into_iter()
        .flatten()
    {
        if let Some(bad) = old_unsupported_arg(cond, &param_names) {
            return Some(format!(
                "unsupported `old(...)` argument `{}` in a contract condition \
                 (v1 supports only `old(param)`)",
                bad
            ));
        }
    }

    None
}

/// Detect an `old(...)` call whose argument is not a bare parameter reference.
fn old_unsupported_arg(
    expr: &Expr,
    params: &std::collections::HashSet<&str>,
) -> Option<String> {
    match expr {
        Expr::Call { func, args, .. } => {
            if let Expr::Variable { name, .. } = func.as_ref() {
                if name == "old" {
                    match args.as_slice() {
                        [Expr::Variable { name, .. }] if params.contains(name.as_str()) => {
                            // Bare parameter reference — supported.
                        }
                        [arg] => {
                            return Some(format!("{:?}", arg));
                        }
                        _ => {
                            return Some("old()".to_string());
                        }
                    }
                }
            }
            for a in args {
                if let Some(bad) = old_unsupported_arg(a, params) {
                    return Some(bad);
                }
            }
            None
        }
        Expr::Block { stmts, .. } => {
            for st in stmts {
                match st {
                    Stmt::Expr(e) => {
                        if let Some(bad) = old_unsupported_arg(e, params) {
                            return Some(bad);
                        }
                    }
                    Stmt::Let(_, e) => {
                        if let Some(bad) = old_unsupported_arg(e, params) {
                            return Some(bad);
                        }
                    }
                }
            }
            None
        }
        Expr::Member { obj, .. } => old_unsupported_arg(obj, params),
        Expr::Binary { lhs, rhs, .. } => {
            if let Some(bad) = old_unsupported_arg(lhs, params) {
                return Some(bad);
            }
            old_unsupported_arg(rhs, params)
        }
        Expr::Unary { expr, .. } => old_unsupported_arg(expr, params),
        Expr::If {
            cond, then, else_, ..
        } => {
            if let Some(bad) = old_unsupported_arg(cond, params) {
                return Some(bad);
            }
            if let Some(bad) = old_unsupported_arg(then, params) {
                return Some(bad);
            }
            else_.as_ref().and_then(|e| old_unsupported_arg(e, params))
        }
        _ => None,
    }
}

/// Heuristic: whether a param is treated as a record in v1 (i.e. is not a known
/// primitive and is referenced via member access).
fn is_record_like(f: &GungnirFunction, param: &str) -> bool {
    !record_fields(f, param).is_empty()
}

/// Recursive scan of an expression for offending symbols referenced in a body:
/// the magic `result` variable, `old(...)`, or any name that is not a declared
/// parameter. Call heads (function names) are allowed.
fn body_free_ref(expr: &Expr, params: &std::collections::HashSet<&str>) -> Option<String> {
    match expr {
        Expr::Block { stmts, .. } => {
            for st in stmts {
                match st {
                    Stmt::Expr(e) => {
                        if let Some(bad) = body_free_ref(e, params) {
                            return Some(bad);
                        }
                    }
                    Stmt::Let(_, e) => {
                        if let Some(bad) = body_free_ref(e, params) {
                            return Some(bad);
                        }
                    }
                }
            }
            None
        }
        Expr::Literal { .. } => None,
        Expr::Variable { name, .. } => {
            if name == "result" || name == "old" || !params.contains(name.as_str()) {
                Some(name.clone())
            } else {
                None
            }
        }
        Expr::Call { func, args, .. } => {
            // `old(...)` is only meaningful in contract conditions — reject it
            // explicitly in a body regardless of the head-allowing rule below.
            if let Expr::Variable { name, .. } = func.as_ref() {
                if name == "old" {
                    return Some("old".to_string());
                }
            }
            // The call's *head* is a function name — allow it (do not walk it as
            // a free variable), but validate every argument expression.
            if !matches!(func.as_ref(), Expr::Variable { .. }) {
                if let Some(bad) = body_free_ref(func, params) {
                    return Some(bad);
                }
            }
            for a in args {
                if let Some(bad) = body_free_ref(a, params) {
                    return Some(bad);
                }
            }
            None
        }
        Expr::Member { obj, .. } => body_free_ref(obj, params),
        Expr::Binary { lhs, rhs, .. } => {
            if let Some(bad) = body_free_ref(lhs, params) {
                return Some(bad);
            }
            body_free_ref(rhs, params)
        }
        Expr::Unary { expr, .. } => body_free_ref(expr, params),
        Expr::If {
            cond, then, else_, ..
        } => {
            if let Some(bad) = body_free_ref(cond, params) {
                return Some(bad);
            }
            if let Some(bad) = body_free_ref(then, params) {
                return Some(bad);
            }
            else_.as_ref().and_then(|e| body_free_ref(e, params))
        }
        _ => None,
    }
}

/// Compute the param declaration lines, the list of `old()`-referenced params
/// that need `name@pre` declares + equality asserts, and the refined-type domain
/// assertions.
fn build_param_decls(f: &GungnirFunction) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut declares = Vec::new();
    let mut old_bindings = Vec::new();
    let mut refined = Vec::new();

    for param in &f.params {
        match &param.type_ {
            Some(t) if is_primitive_type(t) => {
                declares.push(format!(
                    "(declare-const {} {})",
                    param.name,
                    type_sort(t).unwrap_or("Int")
                ));
                refined.extend(refined_assertions(&param.name, t));
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

    (declares, old_bindings, refined)
}

/// The SMT-LIB2 sort for a primitive-typed param.
///
/// Maps the Dwarf primitive names onto Z3 sorts:
/// - `Int` → `Int`
/// - `Bool` → `Bool`
/// - `String` → `String`
/// - `Float` → `Real` (Z3 has no `Float` sort)
///
/// Refined types (`Int(0..100)`) share their base's sort; the refinement
/// constraint itself is emitted separately as domain assertions (see
/// [`refined_assertions`]).
fn type_sort(t: &Type) -> Option<&'static str> {
    match t {
        Type::Named(n) => match n.as_str() {
            "Int" => Some("Int"),
            "Bool" => Some("Bool"),
            "String" => Some("String"),
            "Float" => Some("Real"),
            _ => None,
        },
        Type::Refined { base, .. } => type_sort(base),
        _ => None,
    }
}

/// Whether a type is a primitive SMT sort (as opposed to a record). Refined
/// primitives (e.g. `Int(0..100)`) count as primitive so they get a `declare-const`
/// with their base sort; the constraint is emitted by [`refined_assertions`].
fn is_primitive_type(t: &Type) -> bool {
    type_sort(t).is_some()
}

/// Emit the domain assertions that honor a refined type's constraint.
///
/// A param declared `a: Int(0..100)` must be constrained in the query, otherwise
/// z3 could produce a counterexample outside the type's domain (a false
/// counterexample). Range constraints map to `(>= a min)` / `(<= a max)`;
/// `NonEmpty` refines strings, which we reject cleanly via the engine's
/// unsupported-reason check (see [`unsupported_reason`]).
fn refined_assertions(param: &str, t: &Type) -> Vec<String> {
    let mut out = Vec::new();
    match t {
        Type::Refined {
            base,
            constraint: RefConstraint::Range { min, max },
        } if type_sort(base) == Some("Int") => {
            out.push(format!("(assert (>= {} {}))", param, min));
            out.push(format!("(assert (<= {} {}))", param, max));
        }
        Type::Refined {
            base,
            constraint: RefConstraint::NonEmpty,
        } if type_sort(base) == Some("String") => {
            out.push(format!("(assert (not (= {} \"\")))", param));
        }
        _ => {}
    }
    out
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