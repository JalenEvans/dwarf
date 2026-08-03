//! Tests for JSON serialization/deserialization of HIR types.
//!
//! These tests verify that serde `#[derive(Serialize, Deserialize)]`
//! on HIR types works correctly — the types round-trip through JSON
//! losslessly and the output is deterministic.

use dwarf_syntax::hir::*;
use dwarf_syntax::span::Span;

#[test]
fn test_json_roundtrip_literal() {
    let expr = Expr::Literal {
        value: LiteralValue::Int(42),
        span: Span::default(),
    };
    let json = serde_json::to_string_pretty(&expr).unwrap();
    let deserialized: Expr = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, deserialized);
}

#[test]
fn test_json_roundtrip_function_decl() {
    let decl = Decl::Function {
        name: "add".to_string(),
        params: vec![Param {
            name: "a".to_string(),
            type_: Some(Type::Named("i32".to_string())),
        }],
        return_type: Some(Type::Named("i32".to_string())),
        body: Expr::Literal {
            value: LiteralValue::Int(0),
            span: Span::default(),
        },
        is_pub: false,
        span: Span::default(),
    };
    let json = serde_json::to_string_pretty(&decl).unwrap();
    let deserialized: Decl = serde_json::from_str(&json).unwrap();
    assert_eq!(decl, deserialized);
}

#[test]
fn test_json_output_is_deterministic() {
    let decl = Decl::Function {
        name: "main".to_string(),
        params: vec![],
        return_type: None,
        body: Expr::Block {
            stmts: vec![Stmt::Expr(Expr::Literal {
                value: LiteralValue::Int(1),
                span: Span::default(),
            })],
            span: Span::default(),
        },
        is_pub: false,
        span: Span::default(),
    };
    let json1 = serde_json::to_string_pretty(&decl).unwrap();
    let json2 = serde_json::to_string_pretty(&decl).unwrap();
    assert_eq!(json1, json2);
}

// ============================================================================
// TYPE-LEVEL OPERATOR JSON ROUNDTRIP TESTS (RED Phase — expected to fail)
//
// Type::KeyOf and Type::IndexedAccess do not exist yet. These tests verify
// that once added, they derive Serialize/Deserialize correctly and round-trip
// through JSON losslessly.
// ============================================================================

#[test]
fn test_json_roundtrip_keyof_type() {
    let type_ = Type::KeyOf(Box::new(Type::Named("Person".to_string())));
    let json = serde_json::to_string_pretty(&type_).unwrap();
    let deserialized: Type = serde_json::from_str(&json).unwrap();
    assert_eq!(type_, deserialized);
}

#[test]
fn test_json_roundtrip_indexed_access_type() {
    let type_ = Type::IndexedAccess {
        obj: Box::new(Type::Named("Person".to_string())),
        key: "name".to_string(),
    };
    let json = serde_json::to_string_pretty(&type_).unwrap();
    let deserialized: Type = serde_json::from_str(&json).unwrap();
    assert_eq!(type_, deserialized);
}

#[test]
fn test_json_roundtrip_keyof_in_typedef() {
    let decl = Decl::TypeDef {
        name: "PersonKeys".to_string(),
        type_: Type::KeyOf(Box::new(Type::Named("Person".to_string()))),
        is_pub: false,
        span: Span::default(),
    };
    let json = serde_json::to_string_pretty(&decl).unwrap();
    let deserialized: Decl = serde_json::from_str(&json).unwrap();
    assert_eq!(decl, deserialized);
}

#[test]
fn test_json_roundtrip_indexed_access_in_typedef() {
    let decl = Decl::TypeDef {
        name: "PersonName".to_string(),
        type_: Type::IndexedAccess {
            obj: Box::new(Type::Named("Person".to_string())),
            key: "name".to_string(),
        },
        is_pub: false,
        span: Span::default(),
    };
    let json = serde_json::to_string_pretty(&decl).unwrap();
    let deserialized: Decl = serde_json::from_str(&json).unwrap();
    assert_eq!(decl, deserialized);
}

#[test]
fn test_json_keyof_output_is_deterministic() {
    let type_ = Type::KeyOf(Box::new(Type::Named("Person".to_string())));
    let json1 = serde_json::to_string_pretty(&type_).unwrap();
    let json2 = serde_json::to_string_pretty(&type_).unwrap();
    assert_eq!(json1, json2);
}

#[test]
fn test_json_indexed_access_output_is_deterministic() {
    let type_ = Type::IndexedAccess {
        obj: Box::new(Type::Named("Person".to_string())),
        key: "name".to_string(),
    };
    let json1 = serde_json::to_string_pretty(&type_).unwrap();
    let json2 = serde_json::to_string_pretty(&type_).unwrap();
    assert_eq!(json1, json2);
}
