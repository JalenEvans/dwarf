//! Integration tests for the TypeRegistry module.
//!
//! These tests validate the public API of `TypeRegistry` and the
//! supporting type definitions (`TypeDef`, `PrimitiveType`, etc.).
//!
//! All tests are expected to fail (Red phase) because the types and
//! registry are not yet implemented — only stubs exist.

use dwarf_typecheck::registry::*;
use dwarf_typecheck::types::*;

// ---------------------------------------------------------------------------
// Primitives are pre-registered
// ---------------------------------------------------------------------------

#[test]
fn test_primitives_pre_registered_len() {
    let registry = TypeRegistry::new();
    // A new registry should have exactly 5 primitives: Int, Float, Str, Bool, Null
    assert_eq!(registry.len(), 5);
}

#[test]
fn test_primitives_pre_registered_int() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.get(0),
        Some(&TypeDef::Primitive(PrimitiveType::Int))
    );
}

#[test]
fn test_primitives_pre_registered_float() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.get(1),
        Some(&TypeDef::Primitive(PrimitiveType::Float))
    );
}

#[test]
fn test_primitives_pre_registered_str() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.get(2),
        Some(&TypeDef::Primitive(PrimitiveType::Str))
    );
}

#[test]
fn test_primitives_pre_registered_bool() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.get(3),
        Some(&TypeDef::Primitive(PrimitiveType::Bool))
    );
}

#[test]
fn test_primitives_pre_registered_null() {
    let registry = TypeRegistry::new();
    assert_eq!(
        registry.get(4),
        Some(&TypeDef::Primitive(PrimitiveType::Null))
    );
}

#[test]
fn test_is_empty_true_only_primitives() {
    let registry = TypeRegistry::new();
    // With only primitives, is_empty() should return true
    assert!(registry.is_empty());
}

#[test]
fn test_is_empty_false_after_register() {
    let mut registry = TypeRegistry::new();
    registry.register(TypeDef::Primitive(PrimitiveType::Int));
    // After registering a user type, is_empty() should return false
    assert!(!registry.is_empty());
}

// ---------------------------------------------------------------------------
// Register new types
// ---------------------------------------------------------------------------

#[test]
fn test_register_record_returns_id_5() {
    let mut registry = TypeRegistry::new();
    let record = TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
    ]);
    let id = registry.register(record);
    assert_eq!(id, 5);
}

#[test]
fn test_register_alias_returns_id_6() {
    let mut registry = TypeRegistry::new();
    // Register a record first (ID 5), then an alias to it (ID 6)
    let record = TypeDef::Record(vec![FieldDef {
        name: "point".to_string(),
        type_id: 0,
    }]);
    registry.register(record);
    let id = registry.register(TypeDef::Alias(5));
    assert_eq!(id, 6);
}

#[test]
fn test_register_union_returns_id_7() {
    let mut registry = TypeRegistry::new();
    // Register record (5), alias (6), then union (7)
    registry.register(TypeDef::Record(vec![]));
    registry.register(TypeDef::Alias(5));
    let union = TypeDef::Union(vec![
        VariantDef {
            name: "None".to_string(),
            type_id: None,
        },
        VariantDef {
            name: "Some".to_string(),
            type_id: Some(0),
        },
    ]);
    let id = registry.register(union);
    assert_eq!(id, 7);
}

#[test]
fn test_register_func_returns_id_8() {
    let mut registry = TypeRegistry::new();
    // Register record (5), alias (6), union (7), then func (8)
    registry.register(TypeDef::Record(vec![]));
    registry.register(TypeDef::Alias(5));
    registry.register(TypeDef::Union(vec![]));
    let func = TypeDef::Func(vec![0, 1], 2);
    let id = registry.register(func);
    assert_eq!(id, 8);
}

#[test]
fn test_register_increases_len() {
    let mut registry = TypeRegistry::new();
    assert_eq!(registry.len(), 5);

    registry.register(TypeDef::Record(vec![]));
    assert_eq!(registry.len(), 6);

    registry.register(TypeDef::Alias(5));
    assert_eq!(registry.len(), 7);

    registry.register(TypeDef::Union(vec![]));
    assert_eq!(registry.len(), 8);

    registry.register(TypeDef::Func(vec![], 0));
    assert_eq!(registry.len(), 9);
}

// ---------------------------------------------------------------------------
// Get registered types
// ---------------------------------------------------------------------------

#[test]
fn test_get_valid_id_returns_some() {
    let mut registry = TypeRegistry::new();
    let record = TypeDef::Record(vec![FieldDef {
        name: "name".to_string(),
        type_id: 2, // Str
    }]);
    let id = registry.register(record);

    let retrieved = registry.get(id);
    assert!(retrieved.is_some());

    // Verify we can read back the registered definition
    match retrieved.unwrap() {
        TypeDef::Record(fields) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "name");
            assert_eq!(fields[0].type_id, 2);
        }
        _ => panic!("Expected a Record variant"),
    }
}

#[test]
fn test_get_invalid_id_returns_none() {
    let registry = TypeRegistry::new();
    // IDs 0-4 are valid (primitives), 5+ are out of bounds
    assert!(registry.get(5).is_none());
    assert!(registry.get(100).is_none());
}

#[test]
fn test_get_missing_type_id_returns_none() {
    let registry = TypeRegistry::new();
    // A very large ID should also return None
    assert!(registry.get(usize::MAX).is_none());
}

// ---------------------------------------------------------------------------
// Alias resolution
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_single_alias() {
    let mut registry = TypeRegistry::new();
    // Register a record (ID 5), then an alias to it (ID 6)
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "point".to_string(),
        type_id: 0,
    }]));
    registry.register(TypeDef::Alias(5));

    // Resolving the alias should give the canonical ID 5
    let canonical = registry.resolve(6);
    assert_eq!(canonical, 5);
}

#[test]
fn test_resolve_alias_chain() {
    let mut registry = TypeRegistry::new();
    // Register record (ID 5)
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "value".to_string(),
        type_id: 0,
    }]));
    // Chain: 6 -> 5, 7 -> 6, 8 -> 7
    registry.register(TypeDef::Alias(5)); // ID 6
    registry.register(TypeDef::Alias(6)); // ID 7
    registry.register(TypeDef::Alias(7)); // ID 8

    // Resolving the end of the chain should give the canonical ID 5
    let canonical = registry.resolve(8);
    assert_eq!(canonical, 5);

    // Intermediate aliases should also resolve to 5
    assert_eq!(registry.resolve(6), 5);
    assert_eq!(registry.resolve(7), 5);
}

#[test]
fn test_resolve_non_alias_returns_same_id() {
    let registry = TypeRegistry::new();
    // Primitives are not aliases, so resolve should be identity
    assert_eq!(registry.resolve(0), 0);
    assert_eq!(registry.resolve(1), 1);
    assert_eq!(registry.resolve(2), 2);
    assert_eq!(registry.resolve(3), 3);
    assert_eq!(registry.resolve(4), 4);
}

#[test]
fn test_resolve_record_returns_same_id() {
    let mut registry = TypeRegistry::new();
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "data".to_string(),
        type_id: 2,
    }]));

    // Non-alias user types should return the same ID
    assert_eq!(registry.resolve(5), 5);
}

// ---------------------------------------------------------------------------
// JSON serialization (type round-trips)
// ---------------------------------------------------------------------------

#[test]
fn test_json_roundtrip_primitive_int() {
    let ty = TypeDef::Primitive(PrimitiveType::Int);
    let json = serde_json::to_string(&ty).expect("serialize Int");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Int");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_primitive_float() {
    let ty = TypeDef::Primitive(PrimitiveType::Float);
    let json = serde_json::to_string(&ty).expect("serialize Float");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Float");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_primitive_str() {
    let ty = TypeDef::Primitive(PrimitiveType::Str);
    let json = serde_json::to_string(&ty).expect("serialize Str");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Str");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_primitive_bool() {
    let ty = TypeDef::Primitive(PrimitiveType::Bool);
    let json = serde_json::to_string(&ty).expect("serialize Bool");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Bool");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_primitive_null() {
    let ty = TypeDef::Primitive(PrimitiveType::Null);
    let json = serde_json::to_string(&ty).expect("serialize Null");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Null");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_record() {
    let ty = TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 1,
        },
        FieldDef {
            name: "label".to_string(),
            type_id: 2,
        },
    ]);
    let json = serde_json::to_string_pretty(&ty).expect("serialize Record");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Record");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_union() {
    let ty = TypeDef::Union(vec![
        VariantDef {
            name: "None".to_string(),
            type_id: None,
        },
        VariantDef {
            name: "Some".to_string(),
            type_id: Some(0),
        },
    ]);
    let json = serde_json::to_string(&ty).expect("serialize Union");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Union");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_func() {
    let ty = TypeDef::Func(vec![0, 1, 2], 3);
    let json = serde_json::to_string(&ty).expect("serialize Func");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Func");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_alias() {
    let ty = TypeDef::Alias(42);
    let json = serde_json::to_string(&ty).expect("serialize Alias");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize Alias");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_empty_record() {
    let ty = TypeDef::Record(vec![]);
    let json = serde_json::to_string(&ty).expect("serialize empty Record");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize empty Record");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_empty_union() {
    let ty = TypeDef::Union(vec![]);
    let json = serde_json::to_string(&ty).expect("serialize empty Union");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize empty Union");
    assert_eq!(back, ty);
}

#[test]
fn test_json_roundtrip_empty_func() {
    let ty = TypeDef::Func(vec![], 0);
    let json = serde_json::to_string(&ty).expect("serialize empty Func");
    let back: TypeDef = serde_json::from_str(&json).expect("deserialize empty Func");
    assert_eq!(back, ty);
}

// ---------------------------------------------------------------------------
// JSON serialization (registry round-trip)
// ---------------------------------------------------------------------------

#[test]
fn test_registry_json_roundtrip() {
    let mut registry = TypeRegistry::new();

    // Register a few types
    registry.register(TypeDef::Record(vec![
        FieldDef {
            name: "name".to_string(),
            type_id: 2,
        },
        FieldDef {
            name: "age".to_string(),
            type_id: 0,
        },
    ]));
    registry.register(TypeDef::Alias(5));
    registry.register(TypeDef::Union(vec![
        VariantDef {
            name: "Ok".to_string(),
            type_id: Some(5),
        },
        VariantDef {
            name: "Err".to_string(),
            type_id: Some(2),
        },
    ]));

    // The registry itself should serialize and deserialize
    let json = serde_json::to_string_pretty(&registry).expect("serialize registry");
    let back: TypeRegistry = serde_json::from_str(&json).expect("deserialize registry");

    // Verify the round-tripped registry behaves the same
    assert_eq!(back.len(), registry.len());
    assert_eq!(back.get(0), Some(&TypeDef::Primitive(PrimitiveType::Int)));
    assert_eq!(back.get(5), registry.get(5));
    assert_eq!(back.get(6), registry.get(6));
    assert_eq!(back.get(7), registry.get(7));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_self_referential_alias_does_not_loop() {
    let mut registry = TypeRegistry::new();
    // Register a record (ID 5), then an alias pointing to itself (ID 6 -> 6)
    registry.register(TypeDef::Record(vec![]));
    registry.register(TypeDef::Alias(6)); // Self-referential alias

    // resolve(6) should detect the cycle and return something sensible.
    // For now we just call it — the implementation must handle this.
    let canonical = registry.resolve(6);
    // If the implementation cannot resolve, it should at least not loop infinitely.
    // The exact behavior (return the alias ID vs. the non-existent target) is TBD.
    // We test that it *returns* rather than hangs.
    let _ = canonical;
}

#[test]
fn test_resolve_mutual_alias_chain() {
    let mut registry = TypeRegistry::new();
    // Mutual aliases: 5 -> 6, 6 -> 5
    registry.register(TypeDef::Alias(6)); // ID 5
    registry.register(TypeDef::Alias(5)); // ID 6

    // Similar to self-referential — at minimum must not loop.
    let canonical = registry.resolve(5);
    let _ = canonical;
}
