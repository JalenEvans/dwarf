//! RED-phase tests for TypeScript extern FFI codegen (Chunk 3).
//!
//! These tests verify that `LirDecl::Extern` declarations with `npm:` sources
//! emit real TypeScript import statements instead of stub comments, and that
//! non-npm sources are silently ignored by the TypeScript backend.
//!
//! **All tests in this file are expected to FAIL** until the extern codegen
//! is implemented in `TypeScriptBackend`. The current implementation emits
//! `// extern: <source> fn <name>` for all extern declarations.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::ts::backend::TypeScriptBackend;
use dwarf_lir::{Effect, LirDecl, LirExpr, LirLiteral, LirParam, LirStmt, TargetHint};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror ts_integration.rs patterns)
// ------------------------------------------------------------------

fn s() -> Span {
    Span::new(0, 0, 0)
}

fn no_hint() -> TargetHint {
    TargetHint::None
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

fn param(name: &str, type_: Option<Type>) -> LirParam {
    LirParam {
        name: name.to_string(),
        type_,
    }
}

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = TypeScriptBackend::new("0.1.0");
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: Emit import for npm extern
// ==================================================================
//
// An extern with source "npm:express" should cause the TypeScript
// backend to emit a real `import { express } from 'express'` statement
// instead of the current stub comment `// extern: npm:express fn express`.

#[test]
fn test_emit_extern_npm_import() {
    let decl = LirDecl::Extern {
        source: "npm:express".into(),
        name: "express".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should contain a real TypeScript import statement
    assert!(
        result.contains("import"),
        "npm extern should produce an import statement, got:\n{result}"
    );
    assert!(
        result.contains("express"),
        "import should reference 'express', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("// extern:"),
        "npm extern should NOT produce a stub comment, got:\n{result}"
    );
}

// ==================================================================
// Test 2: Python extern ignored in TypeScript backend
// ==================================================================
//
// An extern with source "py:json" targets Python, not TypeScript.
// The TypeScript backend should emit NOTHING for this declaration —
// no import, no comment, no function signature.

#[test]
fn test_emit_extern_py_ignored_in_ts() {
    let decl = LirDecl::Extern {
        source: "py:json".into(),
        name: "dumps".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should NOT contain any reference to the Python extern
    assert!(
        !result.contains("dumps"),
        "TS backend should not emit anything for py: extern, got:\n{result}"
    );
    assert!(
        !result.contains("py:json"),
        "TS backend should not reference py: source, got:\n{result}"
    );
    assert!(
        !result.contains("// extern:"),
        "TS backend should not emit stub comment for py: extern, got:\n{result}"
    );
}

// ==================================================================
// Test 3: Extern with params and return type
// ==================================================================
//
// An extern with typed parameters and a return type should emit an
// import AND a `declare function` signature with properly mapped
// TypeScript types (e.g., String → string, Any → any).

#[test]
fn test_emit_extern_with_types() {
    let decl = LirDecl::Extern {
        source: "npm:axios".into(),
        name: "get".into(),
        params: vec![param("url", Some(Type::Named("String".into())))],
        return_type: Some(Type::Named("Any".into())),
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should contain an import for 'get' from 'axios'
    assert!(
        result.contains("import"),
        "typed extern should produce an import statement, got:\n{result}"
    );
    assert!(
        result.contains("axios"),
        "import should reference 'axios' module, got:\n{result}"
    );

    // Should contain TypeScript type annotations (String → string, Any → any)
    assert!(
        result.contains("string"),
        "should map String type to TypeScript 'string', got:\n{result}"
    );
    assert!(
        result.contains("any"),
        "should map Any type to TypeScript 'any', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("// extern:"),
        "typed extern should NOT produce a stub comment, got:\n{result}"
    );
}

// ==================================================================
// Test 4: Extern declaration + function that calls it
// ==================================================================
//
// A module with an extern declaration AND a function that calls the
// imported function should emit both the import and the calling
// function with the imported name.

#[test]
fn test_emit_extern_function_call() {
    let decls = vec![
        LirDecl::Extern {
            source: "npm:axios".into(),
            name: "get".into(),
            params: vec![param("url", Some(Type::Named("String".into())))],
            return_type: Some(Type::Named("Any".into())),
            is_pub: true,
        },
        LirDecl::Function {
            name: "fetchData".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Block {
                stmts: vec![LirStmt::Expr(LirExpr::Call {
                    func: Box::new(var("get")),
                    args: vec![str_lit("https://api.example.com")],
                    hint: no_hint(),
                    span: s(),
                })],
                hint: no_hint(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            is_generator: false,
            span: s(),
        },
    ];
    let result = emit_program(decls);

    // Should contain the import for the extern
    assert!(
        result.contains("import"),
        "module with extern should produce import, got:\n{result}"
    );
    assert!(
        result.contains("axios"),
        "import should reference 'axios', got:\n{result}"
    );

    // Should contain the calling function
    assert!(
        result.contains("fetchData"),
        "should contain the calling function, got:\n{result}"
    );
    assert!(
        result.contains("get("),
        "calling function should invoke the imported 'get', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("// extern:"),
        "should NOT contain stub comment, got:\n{result}"
    );
}

// ==================================================================
// Test 5: Async extern call emits await
// ==================================================================
//
// An extern declared with async effect, when called from within a
// function body, should produce an `await` on the call site. The
// import should also be present.

#[test]
fn test_emit_extern_async_call() {
    let decls = vec![
        LirDecl::Extern {
            source: "npm:axios".into(),
            name: "get".into(),
            params: vec![param("url", Some(Type::Named("String".into())))],
            return_type: Some(Type::Named("Any".into())),
            is_pub: true,
        },
        LirDecl::Function {
            name: "fetchData".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Block {
                stmts: vec![LirStmt::Expr(LirExpr::Call {
                    func: Box::new(var("get")),
                    args: vec![str_lit("https://api.example.com")],
                    hint: TargetHint::Async,
                    span: s(),
                })],
                hint: no_hint(),
                span: s(),
            },
            effect: Effect::Async,
            hint: TargetHint::Async,
            is_pub: true,
            is_generator: false,
            span: s(),
        },
    ];
    let result = emit_program(decls);

    // Should contain the import for the extern
    assert!(
        result.contains("import"),
        "async extern module should produce import, got:\n{result}"
    );

    // The calling function should be async and use await
    assert!(
        result.contains("async"),
        "calling function should be async, got:\n{result}"
    );
    assert!(
        result.contains("await"),
        "async extern call should use 'await', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("// extern:"),
        "async extern should NOT produce stub comment, got:\n{result}"
    );
}
