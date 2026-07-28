//! Java error-handling codegen tests.
//!
//! These tests verify the generated Java output for Dwarf `try/catch`,
//! `throw`, and `?` propagation.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::java::backend::JavaBackend;
use dwarf_lir::{Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, TargetHint};
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror java_gen_tests.rs / java_integration.rs)
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
    let mut backend = JavaBackend::default();
    backend.emit_expr(expr).unwrap()
}

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = JavaBackend::default();
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
fn java_emit_try_catch() {
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
        result.contains("catch (Exception e)"),
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
fn java_emit_try_catch_block_body() {
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
        result.contains("catch (Exception e)"),
        "try/catch should emit a catch clause with Exception binding, got: {result}"
    );
    assert!(
        result.contains("x = 1"),
        "try body should emit the let statement, got: {result}"
    );
    assert!(
        result.contains("return \"ok\""),
        "try body should end with the return statement, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "catch handler should emit the fallback expression, got: {result}"
    );
}

#[test]
fn java_emit_try_catch_with_guard() {
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
        result.contains("catch (Exception e)"),
        "guarded try/catch should emit a catch clause, got: {result}"
    );
    assert!(
        result.contains("if (e.code == 1)"),
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
fn java_emit_throw() {
    let expr = LirExpr::Throw {
        expr: Box::new(call("Error", vec![str_lit("msg")])),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.contains("throw new Error(\"msg\")"),
        "throw should emit a throw new statement for constructor calls, got: {result}"
    );

    let var_expr = LirExpr::Throw {
        expr: Box::new(var("e")),
        hint: no_hint(),
        span: s(),
    };
    let var_result = emit_expr(&var_expr);
    assert!(
        var_result.contains("throw e"),
        "throw should emit a plain throw statement for variables, got: {var_result}"
    );
    assert!(
        !var_result.contains("throw new e"),
        "throw should not emit `new` for variable expressions, got: {var_result}"
    );
}

#[test]
fn java_emit_propagate() {
    let expr = LirExpr::Propagate {
        expr: Box::new(member(var("result"), "value")),
        hint: no_hint(),
        span: s(),
    };
    let result = emit_expr(&expr);
    assert!(
        result.to_lowercase().contains("iserr") || result.contains("is_err"),
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
fn java_e2e_try_catch() {
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
        result.contains("public static void safe()"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("try {"),
        "emitted function should contain a try block, got: {result}"
    );
    assert!(
        result.contains("catch (Exception e)"),
        "emitted function should contain a catch clause, got: {result}"
    );
    assert!(
        result.contains("\"fallback\""),
        "emitted function should contain the fallback, got: {result}"
    );
}

#[test]
fn java_e2e_throw() {
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
        result.contains("public static void blows()"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("throw new Error(\"boom\")"),
        "emitted function should throw a new error, got: {result}"
    );
}

#[test]
fn java_e2e_propagate() {
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
        result.contains("public static void unwrap()"),
        "emitted module should contain the function, got: {result}"
    );
    assert!(
        result.contains("import dwarf.gen.Result;"),
        "propagate should import the Result runtime helper, got: {result}"
    );
    assert!(
        result.to_lowercase().contains("iserr") || result.contains("is_err"),
        "emitted function should check for an error case, got: {result}"
    );
    assert!(
        result.contains("return"),
        "emitted function should early-return on error, got: {result}"
    );
}
