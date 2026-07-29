//! RED-phase tests for Java extern FFI codegen (Chunk 4).
//!
//! These tests verify that `LirDecl::Extern` declarations with `java:` sources
//! emit real Java import statements instead of stub comments, and that
//! non-java sources (npm:, py:) are silently ignored by the Java backend.
//!
//! **All tests in this file are expected to FAIL** until the extern codegen
//! is implemented in `JavaBackend`. The current implementation emits
//! `// extern: <source> fn <name>` for all extern declarations.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::java::backend::JavaBackend;
use dwarf_lir::LirDecl;

// ------------------------------------------------------------------
// Helpers (mirror ts_extern_tests.rs patterns)
// ------------------------------------------------------------------

fn emit_program(decls: Vec<LirDecl>) -> String {
    let mut backend = JavaBackend::new("dwarf.gen", "0.1.0");
    backend.emit_module(&decls).unwrap()
}

// ==================================================================
// Test 1: Emit import for java extern
// ==================================================================
//
// An extern with source "java:java.util" should cause the Java backend
// to emit a real `import java.util.*` (or `import java.util.ArrayList`)
// statement instead of the current stub comment
// `// extern: java:java.util fn ArrayList`.

#[test]
fn test_emit_extern_java_import() {
    let decl = LirDecl::Extern {
        source: "java:java.util".into(),
        name: "ArrayList".into(),
        params: vec![],
        return_type: None,
        is_pub: true,
    };
    let result = emit_program(vec![decl]);

    // Should contain a real Java import statement
    assert!(
        result.contains("import"),
        "java extern should produce an import statement, got:\n{result}"
    );
    assert!(
        result.contains("java.util"),
        "import should reference 'java.util', got:\n{result}"
    );

    // Should NOT contain the stub comment
    assert!(
        !result.contains("// extern:"),
        "java extern should NOT produce a stub comment, got:\n{result}"
    );
}

// ==================================================================
// Test 2: npm extern ignored in Java backend
// ==================================================================
//
// An extern with source "npm:express" targets TypeScript, not Java.
// The Java backend should emit NOTHING for this declaration —
// no import, no comment, no function signature.

#[test]
fn test_emit_extern_npm_ignored_in_java() {
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
        "Java backend should not emit anything for npm: extern, got:\n{result}"
    );
    assert!(
        !result.contains("npm:express"),
        "Java backend should not reference npm: source, got:\n{result}"
    );
    assert!(
        !result.contains("// extern:"),
        "Java backend should not emit stub comment for npm: extern, got:\n{result}"
    );
}

// ==================================================================
// Test 3: py extern ignored in Java backend
// ==================================================================
//
// An extern with source "py:json" targets Python, not Java.
// The Java backend should emit NOTHING for this declaration.

#[test]
fn test_emit_extern_py_ignored_in_java() {
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
        "Java backend should not emit anything for py: extern, got:\n{result}"
    );
    assert!(
        !result.contains("py:json"),
        "Java backend should not reference py: source, got:\n{result}"
    );
    assert!(
        !result.contains("// extern:"),
        "Java backend should not emit stub comment for py: extern, got:\n{result}"
    );
}
