//! RED-phase tests for Python extern FFI codegen (Chunk 4).
//!
//! These tests verify that `LirDecl::Extern` declarations with `py:` sources
//! emit real Python import statements instead of stub comments, and that
//! non-py sources (npm:, java:) are silently ignored by the Python backend.
//!
//! **All tests in this file are expected to FAIL** until the extern codegen
//! is implemented in `PythonBackend`. The current implementation emits
//! `# extern: <source> fn <name>` for all extern declarations.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::py::backend::PythonBackend;
use dwarf_lir::{Effect, LirDecl, LirExpr, LirLiteral, LirParam, LirStmt, TargetHint};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Helpers (mirror ts_extern_tests.rs patterns)
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
    let mut backend = PythonBackend::new();
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: Emit import for py extern
// ==================================================================
//
// An extern with source "py:json" should cause the Python backend to
// emit a real `import json` statement instead of the current stub
// comment `# extern: py:json fn dumps`.

#[test]
fn test_emit_extern_py_import() {
    let decl = LirDecl::Extern {
        source: "py:json".into(),
        name: "dumps".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should contain a real Python import statement
    assert!(
        result.contains("import"),
        "py extern should produce an import statement, got:\n{result}"
    );
    assert!(
        result.contains("json"),
        "import should reference 'json', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("# extern:"),
        "py extern should NOT produce a stub comment, got:\n{result}"
    );
}

// ==================================================================
// Test 2: npm extern ignored in Python backend
// ==================================================================
//
// An extern with source "npm:express" targets TypeScript, not Python.
// The Python backend should emit NOTHING for this declaration —
// no import, no comment, no function signature.

#[test]
fn test_emit_extern_npm_ignored_in_py() {
    let decl = LirDecl::Extern {
        source: "npm:express".into(),
        name: "express".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should NOT contain any reference to the npm extern
    assert!(
        !result.contains("express"),
        "Python backend should not emit anything for npm: extern, got:\n{result}"
    );
    assert!(
        !result.contains("npm:express"),
        "Python backend should not reference npm: source, got:\n{result}"
    );
    assert!(
        !result.contains("# extern:"),
        "Python backend should not emit stub comment for npm: extern, got:\n{result}"
    );
}

// ==================================================================
// Test 3: java extern ignored in Python backend
// ==================================================================
//
// An extern with source "java:java.util" targets Java, not Python.
// The Python backend should emit NOTHING for this declaration.

#[test]
fn test_emit_extern_java_ignored_in_py() {
    let decl = LirDecl::Extern {
        source: "java:java.util".into(),
        name: "ArrayList".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should NOT contain any reference to the java extern
    assert!(
        !result.contains("ArrayList"),
        "Python backend should not emit anything for java: extern, got:\n{result}"
    );
    assert!(
        !result.contains("java:java.util"),
        "Python backend should not reference java: source, got:\n{result}"
    );
    assert!(
        !result.contains("# extern:"),
        "Python backend should not emit stub comment for java: extern, got:\n{result}"
    );
}

// ==================================================================
// Test 4: Extern declaration + function that calls it
// ==================================================================
//
// A module with a py:+extern AND a Python function that calls the
// imported function should emit both the import and the calling
// function with the imported name.

#[test]
fn test_emit_extern_py_function_call() {
    let decls = vec![
        LirDecl::Extern {
            source: "py:json".into(),
            name: "dumps".into(),
            params: vec![param("obj", Some(Type::Named("Any".into())))],
            return_type: Some(Type::Named("String".into())),
            is_pub: true,
        },
        LirDecl::Function {
            name: "serialize".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Block {
                stmts: vec![LirStmt::Expr(LirExpr::Call {
                    func: Box::new(var("dumps")),
                    args: vec![str_lit("hello")],
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
        "module with py extern should produce import, got:\n{result}"
    );
    assert!(
        result.contains("json"),
        "import should reference 'json', got:\n{result}"
    );

    // Should contain the calling function
    assert!(
        result.contains("serialize"),
        "should contain the calling function, got:\n{result}"
    );
    assert!(
        result.contains("dumps("),
        "calling function should invoke the imported 'dumps', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("# extern:"),
        "should NOT contain stub comment, got:\n{result}"
    );
}
