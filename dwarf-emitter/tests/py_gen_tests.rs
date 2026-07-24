//! Tests for Python backend property-based testing generator emission.
//!
//! These tests verify that `LirExpr::ForAll` with various built-in types
//! emits the correct Hypothesis generators (`st.integers()`, `st.text()`,
//! etc.) with `@given` decorators.
//!
//! All tests are currently in the **RED phase** — the Python backend
//! emits `ForAll` as a comment (`# forAll<...>`) rather than real
//! Hypothesis output. These tests assert the *desired* output and will
//! fail until the emitter is extended.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::py::backend::PythonBackend;
use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, TargetHint,
};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror py_integration.rs)
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

fn var(name: &str) -> LirExpr {
    LirExpr::Variable {
        name: name.to_string(),
        hint: no_hint(),
        span: s(),
    }
}

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = PythonBackend::new();
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: forAll Int emits st.integers()
// ==================================================================

#[test]
fn test_forall_int_emits_st_integers() {
    let decl = LirDecl::Function {
        name: "test_prop".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::ForAll {
            type_: Type::Named("Int".into()),
            binding: LirPat::Variable("x".into()),
            property: Box::new(LirExpr::Binary {
                op: LirBinaryOp::Gt,
                lhs: Box::new(var("x")),
                rhs: Box::new(int_lit(0)),
                hint: no_hint(),
                span: s(),
            }),
            hint: no_hint(),
            span: s(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("st.integers()"),
        "Should use Hypothesis st.integers() generator for Int type, got: {result}"
    );
    assert!(
        result.contains("@given"),
        "Should use @given decorator, got: {result}"
    );
}

// ==================================================================
// Test 2: forAll String emits st.text()
// ==================================================================

#[test]
fn test_forall_string_emits_st_text() {
    let decl = LirDecl::Function {
        name: "test_prop".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::ForAll {
            type_: Type::Named("String".into()),
            binding: LirPat::Variable("s".into()),
            property: Box::new(var("s")),
            hint: no_hint(),
            span: s(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("st.text()"),
        "Should use Hypothesis st.text() generator for String type, got: {result}"
    );
    assert!(
        result.contains("@given"),
        "Should use @given decorator, got: {result}"
    );
}

// ==================================================================
// Test 3: forAll Bool emits st.booleans()
// ==================================================================

#[test]
fn test_forall_bool_emits_st_booleans() {
    let decl = LirDecl::Function {
        name: "test_prop".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::ForAll {
            type_: Type::Named("Bool".into()),
            binding: LirPat::Variable("b".into()),
            property: Box::new(var("b")),
            hint: no_hint(),
            span: s(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("st.booleans()"),
        "Should use Hypothesis st.booleans() generator for Bool type, got: {result}"
    );
    assert!(
        result.contains("@given"),
        "Should use @given decorator, got: {result}"
    );
}
