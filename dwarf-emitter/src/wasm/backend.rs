//! A minimal WebAssembly backend that emits WAT (WebAssembly Text) from LIR.
//!
//! DWARF-129 — GREEN phase. The [`WasmBackend`] struct implements the full
//! [`EmitterBackend`](crate::backend::EmitterBackend) trait and produces real
//! WAT text that `wat::parse_str` can compile and the wasmtime test runner can
//! execute.
//!
//! # Target subset (the spec the tests pin down)
//!
//! * Functions emit as `(func $name (param ...) (result i32) ...)`.
//! * Only `i32` is supported for `Int`/`Bool`. Floats, strings, records,
//!   unions, lambdas, and arrays must return
//!   [`EmitterError::UnsupportedFeature`].
//! * `Int` literals emit `i32.const N`; `Bool true`/`false` emit
//!   `i32.const 1`/`i32.const 0`.
//! * Binary operators map to `i32.add`/`i32.sub`/`i32.mul`/`i32.eq`/`i32.ne`/
//!   `i32.lt_s`/`i32.gt_s`/`i32.le_s`/`i32.ge_s`/`i32.and`/`i32.or`.
//! * Calls emit `call $name`; variables emit `local.get`.
//! * `LirExpr::AssertConsistent` emits `unreachable` (a trap) so the wasmtime
//!   runner maps trap → `passed: false`.
//! * `test_*` (or `is_pub`) functions are exported via the WAT func field
//!   `(export "name")`.

use std::collections::HashMap;

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;

/// A backend that emits WAT text from LIR declarations.
///
/// `Output` is a `String` containing the complete WAT module.
///
/// # Local-allocation model
///
/// WAT gives every parameter an index `0..n` and every extra local an index
/// starting at `n`. To keep `local.get`/`local.set` indices correct across
/// `let` bindings, the backend tracks a name→index map and a running counter
/// (*params first, then block-scoped lets*). State is reset once per top-level
/// function in [`EmitterBackend::emit_decl`].
pub struct WasmBackend {
    /// Maps a variable/parameter name to its wasm local index.
    locals: HashMap<String, usize>,
    /// Index of the next available local (params occupy 0..n).
    next_local: usize,
}

impl WasmBackend {
    /// Create a new `WasmBackend`.
    pub fn new() -> Self {
        Self {
            locals: HashMap::new(),
            next_local: 0,
        }
    }

    /// Reset the local-allocator state for a fresh function.
    fn reset_locals(&mut self) {
        self.locals.clear();
        self.next_local = 0;
    }

    /// Allocate a fresh local index and record it under `name`.
    fn allocate_local(&mut self, name: &str) -> usize {
        let idx = self.next_local;
        self.locals.insert(name.to_string(), idx);
        self.next_local += 1;
        idx
    }

    /// Pre-assign local indices for every `let` binding reachable through
    /// `Block` nodes in `expr`, in depth-first order. This lets the emitter
    /// declare the locals in the function header before rendering the body.
    fn collect_lets(&mut self, expr: &LirExpr) {
        match expr {
            LirExpr::Block { stmts, .. } => {
                for stmt in stmts {
                    match stmt {
                        LirStmt::Let { pat, value } => {
                            // Only variable patterns get named bindings; other
                            // patterns still consume a slot but stay unnamed.
                            match pat {
                                LirPat::Variable(name) => {
                                    self.allocate_local(name);
                                }
                                _ => {
                                    self.allocate_local("_let");
                                }
                            }
                            self.collect_lets(value);
                        }
                        LirStmt::Expr(e) => self.collect_lets(e),
                    }
                }
            }
            LirExpr::Call { func, args, .. } => {
                self.collect_lets(func);
                for a in args {
                    self.collect_lets(a);
                }
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                self.collect_lets(cond);
                self.collect_lets(then);
                if let Some(e) = else_ {
                    self.collect_lets(e);
                }
            }
            LirExpr::Binary { lhs, rhs, .. } => {
                self.collect_lets(lhs);
                self.collect_lets(rhs);
            }
            LirExpr::Unary { expr, .. } => self.collect_lets(expr),
            LirExpr::Assign { target, value, .. } => {
                self.collect_lets(target);
                self.collect_lets(value);
            }
            _ => {}
        }
    }

    /// Emit a full WAT function body (instructions only) for `expr`.
    fn emit_expr_inner(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        match expr {
            LirExpr::Literal { value, .. } => self.emit_literal(value),
            LirExpr::Variable { name, .. } => {
                let idx = self.locals.get(name).ok_or_else(|| {
                    EmitterError::UnsupportedFeature(format!(
                        "variable `{name}` not bound to a wasm local"
                    ))
                })?;
                Ok(format!("local.get {idx}"))
            }
            LirExpr::Binary { op, lhs, rhs, .. } => {
                let lhs = self.emit_expr_inner(lhs)?;
                let rhs = self.emit_expr_inner(rhs)?;
                let op = self.emit_binary_op(op)?;
                Ok(format!("{lhs}\n{rhs}\n{op}"))
            }
            LirExpr::Unary { op, expr, .. } => {
                let inner = self.emit_expr_inner(expr)?;
                match op {
                    LirUnaryOp::Neg => Ok(format!("{inner}\ni32.const 0\ni32.sub")),
                    LirUnaryOp::Not => Ok(format!("{inner}\ni32.eqz")),
                }
            }
            LirExpr::Call { func, args, .. } => {
                let mut parts = Vec::new();
                for a in args {
                    parts.push(self.emit_expr_inner(a)?);
                }
                let callee = match func.as_ref() {
                    LirExpr::Variable { name, .. } => name.clone(),
                    LirExpr::Literal {
                        value: LirLiteral::Str(name),
                        ..
                    } => name.clone(),
                    other => {
                        return Err(EmitterError::UnsupportedFeature(format!(
                            "call target {other:?} is outside the minimal subset"
                        )));
                    }
                };
                parts.push(format!("call ${}", sanitize(&callee)));
                Ok(parts.join("\n"))
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                let cond = self.emit_expr_inner(cond)?;
                let then = self.emit_expr_inner(then)?;
                match else_ {
                    Some(else_expr) => {
                        let else_body = self.emit_expr_inner(else_expr)?;
                        Ok(format!(
                            "{cond}\n\
                             (if (result i32) (then {then}) (else {else_body}))"
                        ))
                    }
                    None => Ok(format!("{cond}\n(if (then {then}))")),
                }
            }
            LirExpr::Block { stmts, .. } => self.emit_block(stmts),
            LirExpr::AssertConsistent { .. } => Ok("unreachable".to_string()),
            // -- everything outside the minimal i32 subset -- //
            other => Err(EmitterError::UnsupportedFeature(format!(
                "{other:?} is outside the minimal WAT subset"
            ))),
        }
    }

    /// Emit the statements of a `Block` as an instruction sequence.
    fn emit_block(&mut self, stmts: &[LirStmt]) -> Result<String, EmitterError> {
        let mut parts = Vec::new();
        for stmt in stmts {
            match stmt {
                LirStmt::Let { pat, value } => {
                    let val = self.emit_expr_inner(value)?;
                    let idx = match pat {
                        LirPat::Variable(name) => self.locals.get(name).copied().ok_or_else(|| {
                            EmitterError::UnsupportedFeature(format!(
                                "let binding `{name}` missing local index"
                            ))
                        })?,
                        other => {
                            return Err(EmitterError::UnsupportedFeature(format!(
                                "let pattern {other:?} is outside the supported subset"
                            )));
                        }
                    };
                    parts.push(format!("{val}\nlocal.set {idx}"));
                }
                LirStmt::Expr(e) => parts.push(self.emit_expr_inner(e)?),
            }
        }
        Ok(parts.join("\n"))
    }
}

impl Default for WasmBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterBackend for WasmBackend {
    type Output = String;

    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
        let mut funcs = Vec::new();
        for decl in decls {
            let s = self.emit_decl(decl)?;
            if !s.is_empty() {
                funcs.push(s);
            }
        }
        Ok(format!("(module\n{}\n)", funcs.join("\n\n")))
    }

    fn emit_decl(&mut self, decl: &LirDecl) -> Result<String, EmitterError> {
        match decl {
            LirDecl::Function {
                name,
                params,
                return_type,
                body,
                is_pub,
                is_generator,
                ..
            } => {
                if *is_generator {
                    return Ok(String::new());
                }
                // Start a fresh local-allocator: params are locals 0..n.
                self.reset_locals();
                for p in params {
                    self.allocate_local(&p.name);
                }

                // Allocate locals for any block-scoped `let` bindings up front
                // so a `(local ...)` declaration can be emitted before the body.
                self.collect_lets(body);
                let num_lets = self.next_local.saturating_sub(params.len());

                let param_attrs: Vec<String> = params
                    .iter()
                    .map(|p| format!("(param ${} i32)", sanitize(&p.name)))
                    .collect();
                let result_attr = match return_type {
                    Some(ty) if is_i32(ty) => " (result i32)".to_string(),
                    Some(ty) => {
                        return Err(EmitterError::UnsupportedFeature(format!(
                            "return type {ty:?} is outside the supported i32 subset"
                        )));
                    }
                    None => String::new(),
                };
                let local_attr = if num_lets > 0 {
                    let mut s = String::from(" (local");
                    for _ in 0..num_lets {
                        s.push_str(" i32");
                    }
                    s.push(')');
                    s
                } else {
                    String::new()
                };

                // Whether the body leaves a value on the stack must be decided
                // against the LIR *before* it is rendered into instruction text.
                let body_leaves = leaves_value(body);

                let body = self.emit_expr_inner(body)?;
                // Implicit return when the declared type expects an i32 value
                // but the body leaves none; conversely drop a stray value when
                // no result is declared.
                let body = match return_type {
                    Some(ty) if is_i32(ty) && !body_leaves => {
                        format!("{body}\ni32.const 0")
                    }
                    Some(ty) if is_i32(ty) => body,
                    _ if body_leaves => format!("{body}\ndrop"),
                    _ => body,
                };

                let export = if *is_pub || name.starts_with("test_") {
                    format!(" (export \"{name}\")")
                } else {
                    String::new()
                };
                let fn_name = sanitize(name);
                let params_attr = if param_attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", param_attrs.join(" "))
                };
                // Concatenate result and local annotations, space-separated.
                let mut type_attrs: Vec<&str> = Vec::new();
                if !result_attr.is_empty() {
                    type_attrs.push(result_attr.trim());
                }
                if !local_attr.is_empty() {
                    type_attrs.push(local_attr.trim());
                }
                let type_attr = if type_attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", type_attrs.join(" "))
                };
                let indented = indent_body(&body);
                // The export is a func *field*, so it precedes the body.
                Ok(format!(
                    "(func ${fn_name}{params_attr}{type_attr}{export}\n{indented}\n)"
                ))
            }
            // Non-function declarations are not part of the minimal WAT subset.
            _ => Ok(String::new()),
        }
    }

    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        self.emit_expr_inner(expr)
    }

    fn emit_pat(&mut self, pat: &LirPat) -> Result<String, EmitterError> {
        match pat {
            LirPat::Wildcard => Ok("_".to_string()),
            LirPat::Literal(lit) => self.emit_literal(lit),
            LirPat::Variable(name) => Ok(sanitize(name)),
            other => Err(EmitterError::UnsupportedFeature(format!(
                "pattern {other:?} is outside the supported subset"
            ))),
        }
    }

    fn emit_type(&mut self, ty: &Type) -> Result<String, EmitterError> {
        if is_i32(ty) {
            Ok("i32".to_string())
        } else {
            Err(EmitterError::UnsupportedFeature(
                "type is outside the supported i32 subset".to_string(),
            ))
        }
    }

    fn emit_literal(&mut self, lit: &LirLiteral) -> Result<String, EmitterError> {
        match lit {
            LirLiteral::Int(v) => Ok(format!("i32.const {v}")),
            LirLiteral::Bool(true) => Ok("i32.const 1".to_string()),
            LirLiteral::Bool(false) => Ok("i32.const 0".to_string()),
            LirLiteral::Null => Ok("i32.const 0".to_string()),
            LirLiteral::Float(_) | LirLiteral::Str(_) => Err(EmitterError::UnsupportedFeature(
                "literals outside Int/Bool/Null are not supported".to_string(),
            )),
        }
    }

    fn emit_binary_op(&mut self, op: &LirBinaryOp) -> Result<String, EmitterError> {
        Ok(match op {
            LirBinaryOp::Add => "i32.add",
            LirBinaryOp::Sub => "i32.sub",
            LirBinaryOp::Mul => "i32.mul",
            LirBinaryOp::Div => "i32.div_s",
            LirBinaryOp::Eq => "i32.eq",
            LirBinaryOp::Ne => "i32.ne",
            LirBinaryOp::Lt => "i32.lt_s",
            LirBinaryOp::Gt => "i32.gt_s",
            LirBinaryOp::Le => "i32.le_s",
            LirBinaryOp::Ge => "i32.ge_s",
            LirBinaryOp::And => "i32.and",
            LirBinaryOp::Or => "i32.or",
        }
        .to_string())
    }

    fn emit_unary_op(&mut self, op: &LirUnaryOp) -> Result<String, EmitterError> {
        Ok(match op {
            LirUnaryOp::Neg => "i32.sub".to_string(),
            LirUnaryOp::Not => "i32.eqz".to_string(),
        })
    }

    fn emit_target_hint(&mut self, _hint: &TargetHint) -> Result<String, EmitterError> {
        Ok(String::new())
    }

    fn emit_effect(&mut self, _effect: &Effect) -> Result<String, EmitterError> {
        Ok(String::new())
    }
}

/// Return `true` if the type maps to `i32` in the wasm target.
fn is_i32(ty: &Type) -> bool {
    matches!(ty, Type::Named(name) if name == "Int" || name == "Bool")
}

/// Does evaluating `expr` leave a value on the stack (vs. a statement-like
/// expression such as `AssertConsistent`, which traps)?
fn leaves_value(expr: &LirExpr) -> bool {
    match expr {
        LirExpr::Literal { value, .. } => match value {
            LirLiteral::Bool(_) | LirLiteral::Int(_) | LirLiteral::Null => true,
            LirLiteral::Float(_) | LirLiteral::Str(_) => false,
        },
        LirExpr::Variable { .. } => true,
        LirExpr::Call { .. } => true,
        LirExpr::Member { .. } | LirExpr::OptionalAccess { .. } => true,
        LirExpr::If { then, else_, .. } => match else_ {
            Some(_) => true,
            None => leaves_value(then),
        },
        LirExpr::Binary { .. } | LirExpr::Unary { .. } => true,
        LirExpr::Block { stmts, .. } => {
            // A block's value is the value of its final statement.
            match stmts.last() {
                Some(LirStmt::Expr(e)) => leaves_value(e),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Indent every instruction line by two spaces so it nests inside `(func`.
fn indent_body(body: &str) -> String {
    body.lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Sanitize a name into a safe wasm identifier (replace characters wasm
/// rejects with `_`).
fn sanitize(name: &str) -> String {
    let filtered: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '$' || c == '.' { c } else { '_' })
        .collect();
    if filtered.is_empty() {
        "anon".to_string()
    } else {
        filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::LirParam;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers — span/hint factories and LIR fixtures
    // ------------------------------------------------------------------

    fn s() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint() -> TargetHint {
        TargetHint::None
    }

    fn int_lit(n: i64) -> LirExpr {
        LirExpr::Literal {
            value: LirLiteral::Int(n),
            hint: hint(),
            span: s(),
        }
    }

    fn bool_lit(b: bool) -> LirExpr {
        LirExpr::Literal {
            value: LirLiteral::Bool(b),
            hint: hint(),
            span: s(),
        }
    }

    fn var(name: &str) -> LirExpr {
        LirExpr::Variable {
            name: name.to_string(),
            hint: hint(),
            span: s(),
        }
    }

    /// A `test_*` function named `name` with the given body and an `Int`
    /// return type, marked public so the backend must export it.
    fn test_fn(name: &str, params: Vec<LirParam>, body: LirExpr) -> LirDecl {
        LirDecl::Function {
            name: name.to_string(),
            params,
            return_type: Some(Type::Named("Int".into())),
            body,
            effect: Effect::Pure,
            hint: hint(),
            is_pub: true,
            is_generator: false,
            span: s(),
        }
    }

    fn int_param(name: &str) -> LirParam {
        LirParam {
            name: name.to_string(),
            type_: Some(Type::Named("Int".into())),
        }
    }

    fn emit(backend: &mut WasmBackend, decl: &[LirDecl]) -> String {
        backend
            .emit_module(decl)
            .expect("WasmBackend::emit_module must succeed")
    }

    // ------------------------------------------------------------------
    // Spec: emit_module — functions, results, literals, exports
    // ------------------------------------------------------------------

    /// A function returning the i32 literal 42 must produce a WAT function
    /// with an i32 result, the constant, and an export.
    #[test]
    fn test_emit_function_i32_literal_42() {
        let mut backend = WasmBackend::new();
        let decl = test_fn("test_passing", vec![], int_lit(42));
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("(func"),
            "expected a WAT function, got: {:?}",
            out
        );
        assert!(
            out.contains("(result i32)"),
            "expected an i32 result annotation, got: {:?}",
            out
        );
        assert!(
            out.contains("i32.const 42"),
            "expected i32.const 42 for the literal, got: {:?}",
            out
        );
        assert!(
            out.contains("(export \"test_passing\""),
            "expected the test_* function to be exported, got: {:?}",
            out
        );
    }

    /// `test_add(a, b) = a + b` must emit params, `local.get`, `i32.add`,
    /// and an export.
    #[test]
    fn test_emit_function_add_i32() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(var("a")),
            rhs: Box::new(var("b")),
            hint: hint(),
            span: s(),
        };
        let decl = test_fn(
            "test_add",
            vec![int_param("a"), int_param("b")],
            body,
        );
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("local.get"),
            "expected local.get for parameters, got: {:?}",
            out
        );
        assert!(
            out.contains("i32.add"),
            "expected i32.add for binary add, got: {:?}",
            out
        );
        assert!(
            out.contains("(export \"test_add\""),
            "expected the test_* function to be exported, got: {:?}",
            out
        );
    }

    /// `Bool true` must emit `i32.const 1`.
    #[test]
    fn test_emit_bool_true_i32_const_1() {
        let mut backend = WasmBackend::new();
        let decl = test_fn("test_true", vec![], bool_lit(true));
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("i32.const 1"),
            "expected i32.const 1 for true, got: {:?}",
            out
        );
    }

    /// `Bool false` must emit `i32.const 0`.
    #[test]
    fn test_emit_bool_false_i32_const_0() {
        let mut backend = WasmBackend::new();
        let decl = test_fn("test_false", vec![], bool_lit(false));
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("i32.const 0"),
            "expected i32.const 0 for false, got: {:?}",
            out
        );
    }

    /// Equality must emit `i32.eq`.
    #[test]
    fn test_emit_eq_i32_eq() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::Binary {
            op: LirBinaryOp::Eq,
            lhs: Box::new(var("a")),
            rhs: Box::new(var("b")),
            hint: hint(),
            span: s(),
        };
        let decl = test_fn("test_eq", vec![int_param("a"), int_param("b")], body);
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("i32.eq"),
            "expected i32.eq for binary equality, got: {:?}",
            out
        );
    }

    /// If/else must emit WAT `if` and `else`.
    #[test]
    fn test_emit_if_else() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::If {
            cond: Box::new(bool_lit(true)),
            then: Box::new(int_lit(1)),
            else_: Some(Box::new(int_lit(2))),
            hint: hint(),
            span: s(),
        };
        let decl = test_fn("test_if_else", vec![], body);
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("if"),
            "expected a WAT if expression, got: {:?}",
            out
        );
        assert!(
            out.contains("else"),
            "expected a WAT else branch, got: {:?}",
            out
        );
    }

    /// `AssertConsistent` must emit `unreachable` so the runner reports the
    /// test as failed (trap → passed: false).
    #[test]
    fn test_emit_assert_consistent_unreachable() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::AssertConsistent {
            expr: Box::new(int_lit(0)),
            hint: hint(),
            span: s(),
        };
        let decl = test_fn("test_assert", vec![], body);
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("unreachable"),
            "expected unreachable for AssertConsistent, got: {:?}",
            out
        );
    }

    /// A call must emit `call $name`.
    #[test]
    fn test_emit_call() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::Call {
            func: Box::new(var("helper")),
            args: vec![int_lit(1)],
            hint: hint(),
            span: s(),
        };
        let decl = test_fn("test_call", vec![], body);
        let out = emit(&mut backend, &[decl]);
        assert!(
            out.contains("call $helper"),
            "expected call $helper, got: {:?}",
            out
        );
    }

    // ------------------------------------------------------------------
    // Spec: unsupported features must error with the right variant
    // ------------------------------------------------------------------

    /// String literals are outside the minimal i32-only subset and must be
    /// rejected with `EmitterError::UnsupportedFeature`.
    #[test]
    fn test_emit_str_literal_unsupported() {
        let mut backend = WasmBackend::new();
        let body = LirExpr::Literal {
            value: LirLiteral::Str("hello".into()),
            hint: hint(),
            span: s(),
        };
        let decl = test_fn("test_str", vec![], body);
        let result = backend.emit_module(&[decl]);
        assert!(
            matches!(result, Err(EmitterError::UnsupportedFeature(_))),
            "expected Err(UnsupportedFeature) for a string literal, got: {:?}",
            result
        );
    }
}
