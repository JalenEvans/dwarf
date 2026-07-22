//! Integration tests for the HIR type resolution module.
//!
//! These tests validate the public API of `resolve::register_decls()` and
//! the `ResolutionResult` type.
//!
//! All tests are expected to fail (Red phase) because the resolve module
//! is not yet implemented — only stubs exist.

use dwarf_syntax::hir::*;
use dwarf_syntax::span::Span;
use dwarf_typecheck::registry::TypeRegistry;
use dwarf_typecheck::resolve::{self, *};
use dwarf_typecheck::types::*;

// ---------------------------------------------------------------------------
// Helper: create a dummy Span for synthetic HIR nodes
// ---------------------------------------------------------------------------

fn dummy_span() -> Span {
    Span::new(0, 0, 0)
}

// ===========================================================================
// 1. Register a RecordDef
// ===========================================================================

#[test]
fn test_register_record_def() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::RecordDef {
        name: "Point".to_string(),
        fields: vec![
            Field {
                name: "x".to_string(),
                type_: Type::Named("int".to_string()),
            },
            Field {
                name: "y".to_string(),
                type_: Type::Named("int".to_string()),
            },
        ],
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    // Registry should now have 5 primitives + 1 user type = 6 entries
    assert_eq!(result.registry.len(), 6);

    // The Point record should be at ID 5 with resolved field types
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "x".to_string(),
                type_id: 0, // int
            },
            FieldDef {
                name: "y".to_string(),
                type_id: 0, // int
            },
        ]))
    );

    // name_map should contain "Point" -> 5
    assert_eq!(result.name_map.get("Point"), Some(&5));
    assert_eq!(result.name_map.len(), 1);
}

// ===========================================================================
// 2. Register a UnionDef
// ===========================================================================

#[test]
fn test_register_union_def() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::UnionDef {
        name: "Option".to_string(),
        variants: vec![
            Variant {
                name: "None".to_string(),
                arg: None,
            },
            Variant {
                name: "Some".to_string(),
                arg: Some(Type::Named("int".to_string())),
            },
        ],
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    // Registry should have 5 primitives + 1 user type = 6 entries
    assert_eq!(result.registry.len(), 6);

    // The Option union should be at ID 5
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Union(vec![
            VariantDef {
                name: "None".to_string(),
                type_id: None,
            },
            VariantDef {
                name: "Some".to_string(),
                type_id: Some(0), // int
            },
        ]))
    );

    assert_eq!(result.name_map.get("Option"), Some(&5));
}

// ===========================================================================
// 3. Register a type alias
// ===========================================================================

#[test]
fn test_register_type_alias() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::TypeDef {
        name: "Age".to_string(),
        type_: Type::Named("int".to_string()),
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    // Registry should have 5 primitives + 1 alias = 6 entries
    assert_eq!(result.registry.len(), 6);

    // The Age alias should resolve to Int (ID 0)
    assert_eq!(result.registry.get(5), Some(&TypeDef::Alias(0)));

    assert_eq!(result.name_map.get("Age"), Some(&5));
}

// ===========================================================================
// 4. Multiple declarations
// ===========================================================================

#[test]
fn test_multiple_declarations() {
    let mut registry = TypeRegistry::new();
    let decls = vec![
        Decl::RecordDef {
            name: "Point".to_string(),
            fields: vec![
                Field {
                    name: "x".to_string(),
                    type_: Type::Named("int".to_string()),
                },
                Field {
                    name: "y".to_string(),
                    type_: Type::Named("int".to_string()),
                },
            ],
            is_pub: true,
            span: dummy_span(),
        },
        Decl::UnionDef {
            name: "Option".to_string(),
            variants: vec![
                Variant {
                    name: "None".to_string(),
                    arg: None,
                },
                Variant {
                    name: "Some".to_string(),
                    arg: Some(Type::Named("int".to_string())),
                },
            ],
            is_pub: true,
            span: dummy_span(),
        },
    ];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 2 user types = 7 entries
    assert_eq!(result.registry.len(), 7);

    // Point at ID 5
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "x".to_string(),
                type_id: 0,
            },
            FieldDef {
                name: "y".to_string(),
                type_id: 0,
            },
        ]))
    );

    // Option at ID 6
    assert_eq!(
        result.registry.get(6),
        Some(&TypeDef::Union(vec![
            VariantDef {
                name: "None".to_string(),
                type_id: None,
            },
            VariantDef {
                name: "Some".to_string(),
                type_id: Some(0),
            },
        ]))
    );

    // Both names should be in name_map
    assert_eq!(result.name_map.get("Point"), Some(&5));
    assert_eq!(result.name_map.get("Option"), Some(&6));
    assert_eq!(result.name_map.len(), 2);
}

// ===========================================================================
// 5. Named type resolution (primitive name -> TypeId mapping)
// ===========================================================================

#[test]
fn test_named_type_resolution_primitives() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::RecordDef {
        name: "Fields".to_string(),
        fields: vec![
            Field {
                name: "a".to_string(),
                type_: Type::Named("int".to_string()),
            },
            Field {
                name: "b".to_string(),
                type_: Type::Named("float".to_string()),
            },
            Field {
                name: "c".to_string(),
                type_: Type::Named("str".to_string()),
            },
            Field {
                name: "d".to_string(),
                type_: Type::Named("bool".to_string()),
            },
            Field {
                name: "e".to_string(),
                type_: Type::Named("string".to_string()),
            },
        ],
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    assert_eq!(result.registry.len(), 6);

    // Field types should resolve to the correct primitive TypeIds:
    //   int   -> 0
    //   float -> 1
    //   str   -> 2
    //   bool  -> 3
    //   string -> 2 (alias for str)
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "a".to_string(),
                type_id: 0, // int
            },
            FieldDef {
                name: "b".to_string(),
                type_id: 1, // float
            },
            FieldDef {
                name: "c".to_string(),
                type_id: 2, // str
            },
            FieldDef {
                name: "d".to_string(),
                type_id: 3, // bool
            },
            FieldDef {
                name: "e".to_string(),
                type_id: 2, // string -> Str
            },
        ]))
    );
}

#[test]
fn test_named_type_resolution_cross_reference() {
    // A record referencing another type that was declared earlier in the list
    let mut registry = TypeRegistry::new();
    let decls = vec![
        Decl::RecordDef {
            name: "Address".to_string(),
            fields: vec![
                Field {
                    name: "street".to_string(),
                    type_: Type::Named("str".to_string()),
                },
                Field {
                    name: "zip".to_string(),
                    type_: Type::Named("int".to_string()),
                },
            ],
            is_pub: true,
            span: dummy_span(),
        },
        Decl::RecordDef {
            name: "Person".to_string(),
            fields: vec![
                Field {
                    name: "name".to_string(),
                    type_: Type::Named("str".to_string()),
                },
                Field {
                    name: "address".to_string(),
                    type_: Type::Named("Address".to_string()),
                },
            ],
            is_pub: true,
            span: dummy_span(),
        },
    ];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 2 records = 7 entries
    assert_eq!(result.registry.len(), 7);

    // Address at ID 5, Person at ID 6
    assert_eq!(result.name_map.get("Address"), Some(&5));
    assert_eq!(result.name_map.get("Person"), Some(&6));

    // Address has street: Str, zip: Int
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "street".to_string(),
                type_id: 2, // str
            },
            FieldDef {
                name: "zip".to_string(),
                type_id: 0, // int
            },
        ]))
    );

    // Person has name: Str, address: Address (ID 5)
    assert_eq!(
        result.registry.get(6),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "name".to_string(),
                type_id: 2, // str
            },
            FieldDef {
                name: "address".to_string(),
                type_id: 5, // Address record
            },
        ]))
    );
}

// ===========================================================================
// 6. Anonymous record types (inline Type::Record in field definitions)
// ===========================================================================

#[test]
fn test_anonymous_record_type() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::RecordDef {
        name: "Container".to_string(),
        fields: vec![Field {
            name: "inner".to_string(),
            type_: Type::Record(vec![
                ("name".to_string(), Box::new(Type::Named("str".to_string()))),
                (
                    "value".to_string(),
                    Box::new(Type::Named("int".to_string())),
                ),
            ]),
        }],
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 1 anonymous record + 1 named record = 7 entries
    assert_eq!(result.registry.len(), 7);

    // Anonymous record should be registered at ID 5
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "name".to_string(),
                type_id: 2, // str
            },
            FieldDef {
                name: "value".to_string(),
                type_id: 0, // int
            },
        ]))
    );

    // Container record at ID 6 with a field referencing the anonymous record
    assert_eq!(
        result.registry.get(6),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "inner".to_string(),
            type_id: 5, // anonymous record
        }]))
    );

    // Named types in name_map: only "Container", not the anonymous record
    assert_eq!(result.name_map.get("Container"), Some(&6));
    assert_eq!(result.name_map.len(), 1);
}

// ===========================================================================
// 7. Function types in annotations
// ===========================================================================

#[test]
fn test_function_type_in_field() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::RecordDef {
        name: "Callback".to_string(),
        fields: vec![Field {
            name: "handler".to_string(),
            type_: Type::Func {
                params: vec![Type::Named("int".to_string())],
                return_: Box::new(Type::Named("bool".to_string())),
            },
        }],
        is_pub: true,
        span: dummy_span(),
    }];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 1 func type + 1 record = 7 entries
    assert_eq!(result.registry.len(), 7);

    // Func type should be at ID 5
    assert_eq!(
        result.registry.get(5),
        Some(&TypeDef::Func(vec![0], 3)) // (int) -> bool
    );

    // Callback record should be at ID 6
    assert_eq!(
        result.registry.get(6),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "handler".to_string(),
            type_id: 5, // func type
        }]))
    );

    // Only the named type appears in name_map
    assert_eq!(result.name_map.get("Callback"), Some(&6));
    assert_eq!(result.name_map.len(), 1);
}

// ===========================================================================
// 8. Empty declarations
// ===========================================================================

#[test]
fn test_empty_declarations() {
    let mut registry = TypeRegistry::new();
    let decls: Vec<Decl> = vec![];

    let result = register_decls(&mut registry, &decls);

    // Registry should still have only the 5 primitives
    assert_eq!(result.registry.len(), 5);
    assert!(result.registry.is_empty());

    // name_map should be empty
    assert!(result.name_map.is_empty());
    assert_eq!(result.name_map.len(), 0);
}

// ===========================================================================
// 9. Missing type name
// ===========================================================================

#[test]
fn test_missing_type_name() {
    let mut registry = TypeRegistry::new();
    let decls = vec![Decl::RecordDef {
        name: "Foo".to_string(),
        fields: vec![Field {
            name: "bar".to_string(),
            type_: Type::Named("UnknownType".to_string()),
        }],
        is_pub: true,
        span: dummy_span(),
    }];

    // Should not panic — the resolver should create the record entry even
    // when a field references a type name that hasn't been registered.
    let result = register_decls(&mut registry, &decls);

    // The record should still be registered at ID 5
    assert_eq!(result.registry.len(), 6);
    assert_eq!(result.name_map.get("Foo"), Some(&5));

    // The field 'bar' references a type; the exact type_id is
    // implementation-defined (could be a placeholder or a best-effort
    // lookup). For now we just verify the record exists and has a field
    // named "bar" — the exact type_id value is not asserted.
    match result.registry.get(5) {
        Some(TypeDef::Record(fields)) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "bar");
            // The type_id for an unknown type is implementation-defined,
            // but must exist (not panic).
            let _ = fields[0].type_id;
        }
        other => panic!("Expected Record at ID 5, got {:?}", other),
    }
}
