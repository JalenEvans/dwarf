//! Tests for TypeScript backend property-based testing generator emission.
//!
//! These tests verify that `LirExpr::ForAll` with various built-in types
//! emits the correct fast-check generators (`fc.integer()`, `fc.string()`,
//! etc.) wrapped in `fc.property`.
//!
//! All tests are currently in the **RED phase** — the TypeScript backend
//! emits `ForAll` as a comment (`/* forAll<...> */`) rather than real
//! fast-check output. These tests assert the *desired* output and will
//! fail until the emitter is extended.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::ts::backend::TypeScriptBackend;
use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, TargetHint,
};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror ts_integration.rs)
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
    let mut backend = TypeScriptBackend::new("0.1.0");
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: forAll Int emits fc.integer()
// ==================================================================

#[test]
fn test_forall_int_emits_fc_integer() {
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
        result.contains("fc.integer()"),
        "Should use fast-check integer generator for Int type, got: {result}"
    );
    assert!(
        result.contains("fc.property"),
        "Should wrap in fc.property, got: {result}"
    );
}

// ==================================================================
// Test 2: forAll String emits fc.string()
// ==================================================================

#[test]
fn test_forall_string_emits_fc_string() {
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
        result.contains("fc.string()"),
        "Should use fast-check string generator for String type, got: {result}"
    );
    assert!(
        result.contains("fc.property"),
        "Should wrap in fc.property, got: {result}"
    );
}

// ==================================================================
// Test 3: forAll Bool emits fc.boolean()
// ==================================================================

#[test]
fn test_forall_bool_emits_fc_boolean() {
    let decl = LirDecl::Function {
        name: "testProp".into(),
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
        is_generator: false,
        span: s(),
    };
    let result = emit_program(vec![decl]);
    assert!(
        result.contains("fc.boolean()"),
        "Should use fast-check boolean generator for Bool type, got: {result}"
    );
    assert!(
        result.contains("fc.property"),
        "Should wrap in fc.property, got: {result}"
    );
}

// ==================================================================
// Test 4: forAll List<Int> emits fc.array(fc.integer())
// ==================================================================

#[test]
fn test_forall_list_int_emits_fc_array_integer() {
    let decl = LirDecl::Function {
        name: "testProp".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::ForAll {
            type_: Type::Generic {
                base: "List".into(),
                args: vec![Type::Named("Int".into())],
            },
            binding: LirPat::Variable("xs".into()),
            property: Box::new(var("xs")),
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
        result.contains("fc.array("),
        "Should use fc.array for List type, got: {result}"
    );
    assert!(
        result.contains("fc.integer()"),
        "Should use fc.integer for element type Int, got: {result}"
    );
    assert!(
        result.contains("fc.property"),
        "Should wrap in fc.property, got: {result}"
    );
}

// ==================================================================
// Test 5: forAll emits fast-check import
// ==================================================================

#[test]
fn test_forall_emits_fast_check_import() {
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
        result.contains("import * as fc from 'fast-check'"),
        "Should import fast-check with namespace import, got: {result}"
    );
}

// ==================================================================
// Test 6: forAll wraps in Jest test()
// ==================================================================

#[test]
fn test_forall_wraps_in_test_function() {
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
        result.contains("test('testProp',"),
        "Should wrap test property in Jest test() call with function name, got: {result}"
    );
}

// ==================================================================
// Test 7: forAll uses fc.assert()
// ==================================================================

#[test]
fn test_forall_uses_fc_assert() {
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
        result.contains("fc.assert("),
        "Should wrap fc.property inside fc.assert(), got: {result}"
    );
}

// ==================================================================
// Test 8: forAll full integration — import + test + fc.assert + generator
// ==================================================================

#[test]
fn test_forall_full_integration() {
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
    // Full compilation should include all of these:
    assert!(
        result.contains("import * as fc from 'fast-check'"),
        "Should have fast-check import"
    );
    assert!(
        result.contains("test('testProp',"),
        "Should have Jest test wrapper"
    );
    assert!(
        result.contains("fc.assert("),
        "Should have fc.assert wrapper"
    );
    assert!(
        result.contains("fc.integer()"),
        "Should have fc.integer() generator"
    );
}
