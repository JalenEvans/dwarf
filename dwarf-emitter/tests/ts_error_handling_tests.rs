//! TypeScript error-handling codegen tests.
//!
//! These tests verify the generated TypeScript output for Dwarf `try/catch`,
//! `throw`, and `?` propagation.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::ts::backend::TypeScriptBackend;
use dwarf_lir::{Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, TargetHint};
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror the existing ts_gen_tests.rs / ts_integration.rs)
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
    let mut backend = TypeScriptBackend::new("0.1.0");
    backend.emit_expr(expr).unwrap()
}

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = TypeScriptBackend::new("0.1.0");
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
fn emit_try_catch() {
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
        result.contains("try {"),
        "try/catch should emit a try block, got: {result}"
    );
    assert!(
        result.contains("catch (e)"),
        "try/catch should emit a catch clause with the binding, got: {result}"
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
fn emit_try_catch_block_body() {
    let body = LirExpr::Block {
        stmts: vec![
            LirStmt::Let {
                pat: LirPat::Variable("x".to_string()),
                value: int_lit(1),
            },
            LirStmt::Expr(str_lit("ok")),
        ],
        hint: no_hint(),
        span: s(),
    };
    let expr = LirExpr::Try {
        body: Box::new(body),
        binding: LirPat::Variable("e".to_string()),
        guard: None,
        handler: Box::new(str_lit("fallback")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("try {"),
        "try/catch should emit a try block, got: {result}"
    );
    assert!(
        result.contains("catch (e)"),
        "try/catch should emit a catch clause with the binding, got: {result}"
    );
    assert!(
        result.contains("let x = 1"),
        "try body should emit the let statement, got: {result}"
    );
    assert!(
        result.contains("; return \"ok\";"),
        "try body should emit semicolon-separated statements ending with return, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "catch handler should emit the fallback expression, got: {result}"
    );
}

#[test]
fn emit_try_catch_with_guard() {
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
        result.contains("try {"),
        "guarded try/catch should emit a try block, got: {result}"
    );
    assert!(
        result.contains("catch (e)"),
        "guarded try/catch should emit a catch clause, got: {result}"
    );
    assert!(
        result.contains("if (e.code === 1)"),
        "guard should be emitted as a conditional, got: {result}"
    );
    assert!(
        result.contains("throw e"),
        "guard fallback should re-throw the error, got: {result}"
    );
    assert!(
        result.contains("\"handled\""),
        "handler body should be emitted, got: {result}"
    );
}

#[test]
fn emit_throw() {
    let expr = LirExpr::Throw {
        expr: Box::new(call("Error", vec![str_lit("msg")])),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("throw Error(\"msg\")"),
        "throw should emit a throw statement, got: {result}"
    );
}

#[test]
fn emit_propagate() {
    let expr = LirExpr::Propagate {
        expr: Box::new(member(var("result"), "value")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("isErr("),
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
fn e2e_try_catch_compiles() {
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
        result.contains("export function safe"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("try {"),
        "emitted function should contain a try block, got: {result}"
    );
    assert!(
        result.contains("catch (e)"),
        "emitted function should contain a catch clause, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "emitted function should contain the fallback, got: {result}"
    );
}

#[test]
fn e2e_throw_compiles() {
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
        result.contains("export function blows"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("throw Error(\"boom\")"),
        "emitted function should throw an error, got: {result}"
    );
}

#[test]
fn e2e_propagate_compiles() {
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
        result.contains("export function unwrap"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("import { isErr } from 'dwarf-runtime/result.js'"),
        "propagate should import the Result runtime helper, got: {result}"
    );
    assert!(
        result.contains("isErr("),
        "emitted function should check isErr, got: {result}"
    );
    assert!(
        result.contains("return"),
        "emitted function should early-return on error, got: {result}"
    );
}
