//! RED-phase tests for Python error-handling codegen.
//!
//! These tests assert the desired Python output for Dwarf `try/catch`,
//! `throw`, and `?` propagation. They reference `LirExpr::Try`,
//! `LirExpr::Throw`, and `LirExpr::Propagate` variants.
//!
//! The Python backend currently returns `UnsupportedFeature` for `Try` and
//! `Propagate`, so these tests are expected to fail until the emitter is
//! extended.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::py::backend::PythonBackend;
use dwarf_lir::{Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, TargetHint};
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror py_gen_tests.rs / py_integration.rs)
// ------------------------------------------------------------------

fn s() -> Span {
    Span::new(0, 0, 0)
}

fn no_hint() -> TargetHint {
    TargetHint::None
}

fn int_lit(v: i64) -> LirExpr {
    LirExpr::Literal {
        value: LirLiteral::Int(v),
        hint: no_hint(),
        span: s(),
    }
}

fn str_lit(v: &str) -> LirExpr {
    LirExpr::Literal {
        value: LirLiteral::Str(v.to_string()),
        hint: no_hint(),
        span: s(),
    }
}

fn var(name: &str) -> LirExpr {
    LirExpr::Variable {
        name: name.to_string(),
        hint: no_hint(),
        span: s(),
    }
}

fn member(obj: LirExpr, field: &str) -> LirExpr {
    LirExpr::Member {
        obj: Box::new(obj),
        field: field.to_string(),
        hint: no_hint(),
        span: s(),
    }
}

fn call(name: &str, args: Vec<LirExpr>) -> LirExpr {
    LirExpr::Call {
        func: Box::new(var(name)),
        args,
        hint: no_hint(),
        span: s(),
    }
}

fn emit_expr(expr: &LirExpr) -> String {
    let mut backend = PythonBackend::new();
    backend.emit_expr(expr).unwrap()
}

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = PythonBackend::new();
    backend.emit_module(&decls).unwrap()
}

fn function(name: &str, body: LirExpr) -> LirDecl {
    LirDecl::Function {
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body,
        effect: Effect::Pure,
        hint: no_hint(),
        is_pub: true,
        is_generator: false,
        span: s(),
    }
}

// ------------------------------------------------------------------
// Unit tests: emit_expr for Try / Throw / Propagate
// ------------------------------------------------------------------

#[test]
fn py_emit_try_catch() {
    let expr = LirExpr::Try {
        body: Box::new(str_lit("ok")),
        binding: LirPat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(str_lit("fallback")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("try:"),
        "try/catch should emit a try block, got: {result}"
    );
    assert!(
        result.contains("except Exception as e:"),
        "try/catch should emit an except clause with the binding, got: {result}"
    );
    assert!(
        result.contains("\"ok\""),
        "try body should emit the original expression, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "catch handler should emit the fallback expression, got: {result}"
    );
}

#[test]
fn py_emit_try_catch_with_guard() {
    let guard = LirExpr::Binary {
        op: LirBinaryOp::Eq,
        lhs: Box::new(member(var("e"), "code")),
        rhs: Box::new(int_lit(1)),
        hint: no_hint(),
        span: s(),
    };
    let expr = LirExpr::Try {
        body: Box::new(str_lit("ok")),
        binding: LirPat::Variable("e".to_string()),
        guard: Some(Box::new(guard)),
        handler: Box::new(str_lit("handled")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("try:"),
        "guarded try/catch should emit a try block, got: {result}"
    );
    assert!(
        result.contains("except Exception as e:"),
        "guarded try/catch should emit an except clause, got: {result}"
    );
    assert!(
        result.contains("if e.code == 1:"),
        "guard should be emitted as a conditional, got: {result}"
    );
    assert!(
        result.contains("raise e"),
        "guard fallback should re-raise the error, got: {result}"
    );
    assert!(
        result.contains("\"handled\""),
        "handler body should be emitted, got: {result}"
    );
}

#[test]
fn py_emit_throw() {
    let expr = LirExpr::Throw {
        expr: Box::new(call("Error", vec![str_lit("msg")])),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("raise Error(\"msg\")"),
        "throw should emit a raise statement, got: {result}"
    );
}

#[test]
fn py_emit_propagate() {
    let expr = LirExpr::Propagate {
        expr: Box::new(member(var("result"), "value")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("is_err("),
        "propagate should check for an error case, got: {result}"
    );
    assert!(
        result.contains("return"),
        "propagate should early-return on error, got: {result}"
    );
    assert!(
        result.contains("result.value"),
        "propagate should access the original expression, got: {result}"
    );
}

// ------------------------------------------------------------------
// Integration tests: full module emission
// ------------------------------------------------------------------

#[test]
fn python_e2e_try_catch() {
    let decl = function(
        "safe",
        LirExpr::Try {
            body: Box::new(str_lit("ok")),
            binding: LirPat::Variable("e".to_string()),
            guard: None,
            handler: Box::new(str_lit("fallback")),
            hint: no_hint(),
            span: s(),
        },
    );
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("def safe():"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("try:"),
        "emitted function should contain a try block, got: {result}"
    );
    assert!(
        result.contains("except Exception as e:"),
        "emitted function should contain an except clause, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "emitted function should contain the fallback, got: {result}"
    );
}

#[test]
fn python_e2e_throw() {
    let decl = function(
        "blows",
        LirExpr::Throw {
            expr: Box::new(call("Error", vec![str_lit("boom")])),
            hint: no_hint(),
            span: s(),
        },
    );
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("def blows():"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("raise Error(\"boom\")"),
        "emitted function should raise an error, got: {result}"
    );
}

#[test]
fn python_e2e_propagate() {
    let decl = function(
        "unwrap",
        LirExpr::Propagate {
            expr: Box::new(member(var("result"), "value")),
            hint: no_hint(),
            span: s(),
        },
    );
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("def unwrap():"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("from dwarf_runtime.result import"),
        "propagate should import the Result runtime helper, got: {result}"
    );
    assert!(
        result.contains("is_err("),
        "emitted function should check is_err, got: {result}"
    );
    assert!(
        result.contains("return"),
        "emitted function should early-return on error, got: {result}"
    );
}
