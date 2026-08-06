//! DWARF-129 — End-to-end spec for the minimal WAT emitter + wasmtime runner.
//!
//! These tests exercise the `WasmBackend` implementation: emitting WAT text
//! that `wat::parse_str` can turn into module bytes the
//! [`WasmTestRunner`](super::runner) executes.
//!
//! The runner's convention (see [`WasmTestRunner::run_test`]): a function that
//! completes reports `passed: true`; a function that traps (here, via the
//! `unreachable` emitted for `AssertConsistent`) reports `passed: false`.

#![cfg(test)]

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::wasm::backend::WasmBackend;
use dwarf_lir::{Effect, LirDecl, LirExpr, LirLiteral, TargetHint};
use dwarf_syntax::span::Span;

use super::runner::WasmTestRunner;

// ---------------------------------------------------------------------------
// Fixtures — build a `test_*` LIR function and emit WAT from it
// ---------------------------------------------------------------------------

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

fn assert_consistent(inner: LirExpr) -> LirExpr {
    LirExpr::AssertConsistent {
        expr: Box::new(inner),
        hint: hint(),
        span: s(),
    }
}

/// Emit a single `test_*` function with the given body and return the WAT.
fn emit_test_fn(name: &str, body: LirExpr) -> String {
    let decl = LirDecl::Function {
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body,
        effect: Effect::Pure,
        hint: hint(),
        is_pub: true,
        is_generator: false,
        span: s(),
    };
    let mut backend = WasmBackend::new();
    backend
        .emit_module(&[decl])
        .expect("WasmBackend::emit_module must succeed")
}

/// Parse emitted WAT into Wasm module bytes.
fn parse_wat(wat_src: &str) -> Vec<u8> {
    wat::parse_str(wat_src).expect("emitted WAT must be a valid Wasm module")
}

// ---------------------------------------------------------------------------
// E2E tests
// ---------------------------------------------------------------------------

/// Emit a passing `test_passing` function, compile it, and run it under the
/// wasmtime runner: the result must be `Ok(passed: true)`.
#[test]
fn test_emit_parse_run_passing_test() {
    let wat = emit_test_fn("test_passing", int_lit(42));
    assert!(
        !wat.trim().is_empty(),
        "WAT emitter must produce a module; got empty output"
    );

    let wasm = parse_wat(&wat);
    let result = WasmTestRunner::new()
        .run_test(&wasm, "test_passing")
        .expect("a valid, exported @test must execute and return Ok");

    assert!(
        result.passed,
        "the emitted passing test must report passed: true, got {:?}",
        result
    );
    assert_eq!(
        result.function_name, "test_passing",
        "result must echo the executed function name"
    );
}

/// Emit a function whose body is `AssertConsistent`, compile it, and run it:
/// the `unreachable` trap must surface as `Ok(passed: false)` — not an error,
/// and never a silent pass.
#[test]
fn test_emit_parse_run_assert_consistent_fails() {
    let wat = emit_test_fn("test_assert", assert_consistent(int_lit(0)));
    assert!(
        !wat.trim().is_empty(),
        "WAT emitter must produce a module; got empty output"
    );
    assert!(
        wat.contains("unreachable"),
        "AssertConsistent must emit unreachable so the runner sees a trap; got: {:?}",
        wat
    );

    let wasm = parse_wat(&wat);
    let result = WasmTestRunner::new()
        .run_test(&wasm, "test_assert")
        .expect("a trapping @test must still return Ok(TestResult)");

    assert!(
        !result.passed,
        "a function with AssertConsistent must be reported as failed, got {:?}",
        result
    );
    assert!(
        result.message.is_some(),
        "a failed test must carry a message, got {:?}",
        result
    );
}
