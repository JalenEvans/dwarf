//! Tests for Java backend property-based testing generator emission.
//!
//! These tests verify that `LirExpr::ForAll` with various built-in types
//! emits the correct jqwik property testing annotations (`@ForAll`,
//! `@IntGenerator`, `@StringGenerator`, etc.).
//!
//! All tests are currently in the **RED phase** — the Java backend
//! emits `ForAll` as a comment (`/* forAll<...> */`) rather than real
//! jqwik annotation output. These tests assert the *desired* output
//! and will fail until the emitter is extended.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::java::backend::JavaBackend;
use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, TargetHint,
};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror java_integration.rs)
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
    let mut backend = JavaBackend::default();
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: forAll Int has @ForAll annotation
// ==================================================================

#[test]
fn test_forall_int_has_forall_annotation() {
    let decl = LirDecl::Function {
        name: "testProp".into(),
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
        is_generator: false,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("@ForAll"),
        "Should use @ForAll annotation for property-based testing, got: {result}"
    );
    assert!(
        result.contains("@Property"),
        "Should use @Property annotation on the test method, got: {result}"
    );
    assert!(
        result.contains("int") || result.contains("@IntGenerator"),
        "Should include int type or int generator annotation, got: {result}"
    );
}

// ==================================================================
// Test 2: forAll String has @ForAll annotation
// ==================================================================

#[test]
fn test_forall_string_has_forall_annotation() {
    let decl = LirDecl::Function {
        name: "testProp".into(),
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
        is_generator: false,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("@ForAll"),
        "Should use @ForAll annotation for property-based testing, got: {result}"
    );
    assert!(
        result.contains("@Property"),
        "Should use @Property annotation on the test method, got: {result}"
    );
    assert!(
        result.contains("String"),
        "Should include String type annotation, got: {result}"
    );
}
