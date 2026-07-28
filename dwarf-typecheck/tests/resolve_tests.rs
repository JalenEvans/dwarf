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
use dwarf_typecheck::resolve::*;
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

    // Registry should now have 5 primitives + 4 built-in generics + 1 user type = 10 entries
    assert_eq!(result.registry.len(), 10);

    // The Point record should be at ID 9 with resolved field types
    assert_eq!(
        result.registry.get(9),
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

    // name_map should contain "Point" -> 9
    assert_eq!(result.name_map.get("Point"), Some(&9));
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

    // Registry should have 5 primitives + 4 built-in generics + 1 user type = 10 entries
    assert_eq!(result.registry.len(), 10);

    // The Option union should be at ID 9
    // (built-in generic Option is at ID 5; user-defined Option shadows it)
    assert_eq!(
        result.registry.get(9),
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

    assert_eq!(result.name_map.get("Option"), Some(&9));
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

    // Registry should have 5 primitives + 4 built-in generics + 1 alias = 10 entries
    assert_eq!(result.registry.len(), 10);

    // The Age alias should resolve to Int (ID 0) at ID 9
    assert_eq!(result.registry.get(9), Some(&TypeDef::Alias(0)));

    assert_eq!(result.name_map.get("Age"), Some(&9));
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

    // 5 primitives + 4 built-in generics + 2 user types = 11 entries
    assert_eq!(result.registry.len(), 11);

    // Point at ID 9
    assert_eq!(
        result.registry.get(9),
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

    // Option at ID 10 (user-defined, shadows built-in Option at ID 5)
    assert_eq!(
        result.registry.get(10),
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
    assert_eq!(result.name_map.get("Point"), Some(&9));
    assert_eq!(result.name_map.get("Option"), Some(&10));
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

    assert_eq!(result.registry.len(), 10);

    // Field types should resolve to the correct primitive TypeIds:
    //   int   -> 0
    //   float -> 1
    //   str   -> 2
    //   bool  -> 3
    //   string -> 2 (alias for str)
    assert_eq!(
        result.registry.get(9),
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

    // 5 primitives + 4 built-in generics + 2 records = 11 entries
    assert_eq!(result.registry.len(), 11);

    // Address at ID 9, Person at ID 10
    assert_eq!(result.name_map.get("Address"), Some(&9));
    assert_eq!(result.name_map.get("Person"), Some(&10));

    // Address has street: Str, zip: Int
    assert_eq!(
        result.registry.get(9),
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

    // Person has name: Str, address: Address (ID 9)
    assert_eq!(
        result.registry.get(10),
        Some(&TypeDef::Record(vec![
            FieldDef {
                name: "name".to_string(),
                type_id: 2, // str
            },
            FieldDef {
                name: "address".to_string(),
                type_id: 9, // Address record
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

    // 5 primitives + 4 built-in generics + 1 anonymous record + 1 named record = 11 entries
    assert_eq!(result.registry.len(), 11);

    // Anonymous record should be registered at ID 9
    assert_eq!(
        result.registry.get(9),
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

    // Container record at ID 10 with a field referencing the anonymous record
    assert_eq!(
        result.registry.get(10),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "inner".to_string(),
            type_id: 9, // anonymous record
        }]))
    );

    // Named types in name_map: only "Container", not the anonymous record
    assert_eq!(result.name_map.get("Container"), Some(&10));
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

    // 5 primitives + 4 built-in generics + 1 func type + 1 record = 11 entries
    assert_eq!(result.registry.len(), 11);

    // Func type should be at ID 9
    assert_eq!(
        result.registry.get(9),
        Some(&TypeDef::Func(vec![0], 3)) // (int) -> bool
    );

    // Callback record should be at ID 10
    assert_eq!(
        result.registry.get(10),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "handler".to_string(),
            type_id: 9, // func type
        }]))
    );

    // Only the named type appears in name_map
    assert_eq!(result.name_map.get("Callback"), Some(&10));
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

    // Registry should still have only the 5 primitives + 4 built-in generics
    assert_eq!(result.registry.len(), 9);
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

    // The record should still be registered at ID 9
    assert_eq!(result.registry.len(), 10);
    assert_eq!(result.name_map.get("Foo"), Some(&9));

    // The field 'bar' references a type; the exact type_id is
    // implementation-defined (could be a placeholder or a best-effort
    // lookup). For now we just verify the record exists and has a field
    // named "bar" — the exact type_id value is not asserted.
    match result.registry.get(9) {
        Some(TypeDef::Record(fields)) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "bar");
            // The type_id for an unknown type is implementation-defined,
            // but must exist (not panic).
            let _ = fields[0].type_id;
        }
        other => panic!("Expected Record at ID 9, got {:?}", other),
    }
}

// ===========================================================================
// 10. Generic type resolution
//     A `Generic { base, args }` HIR type is registered as a
//     `GenericInstance` TypeDef in the registry (if the base is known).
// ===========================================================================

#[test]
fn test_resolve_generic_type_simple() {
    let mut registry = TypeRegistry::new();
    let decls = vec![
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
        Decl::RecordDef {
            name: "Container".to_string(),
            fields: vec![Field {
                name: "value".to_string(),
                type_: Type::Generic {
                    base: "Option".to_string(),
                    args: vec![Type::Named("int".to_string())],
                },
            }],
            is_pub: true,
            span: dummy_span(),
        },
    ];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 4 built-in generics + 1 union + 1 GenericInstance + 1 record = 12 entries
    assert_eq!(result.registry.len(), 12);

    // User-defined Option at ID 9 (built-in generic Option is at ID 5)
    assert_eq!(result.name_map.get("Option"), Some(&9));

    // GenericInstance { base: Option(9), args: [Int(0)] } at ID 10
    assert_eq!(
        result.registry.get(10),
        Some(&TypeDef::GenericInstance {
            base: 9,
            args: vec![0],
        })
    );

    // Container at ID 11 referencing the GenericInstance at ID 10
    assert_eq!(
        result.registry.get(11),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "value".to_string(),
            type_id: 10,
        }]))
    );

    assert_eq!(result.name_map.get("Container"), Some(&11));
    assert_eq!(result.name_map.get("Option"), Some(&9));
}

#[test]
fn test_resolve_generic_type_nested() {
    let mut registry = TypeRegistry::new();
    let decls = vec![
        Decl::RecordDef {
            name: "HashMap".to_string(),
            fields: vec![],
            is_pub: true,
            span: dummy_span(),
        },
        Decl::RecordDef {
            name: "List".to_string(),
            fields: vec![],
            is_pub: true,
            span: dummy_span(),
        },
        Decl::RecordDef {
            name: "Container".to_string(),
            fields: vec![Field {
                name: "map".to_string(),
                type_: Type::Generic {
                    base: "HashMap".to_string(),
                    args: vec![
                        Type::Named("str".to_string()),
                        Type::Generic {
                            base: "List".to_string(),
                            args: vec![Type::Named("int".to_string())],
                        },
                    ],
                },
            }],
            is_pub: true,
            span: dummy_span(),
        },
    ];

    let result = register_decls(&mut registry, &decls);

    // 5 primitives + 4 built-in generics + 2 named records + 2 GenericInstances + 1 container = 14
    assert_eq!(result.registry.len(), 14);

    assert_eq!(result.name_map.get("HashMap"), Some(&9));
    assert_eq!(result.name_map.get("List"), Some(&10));

    // Inner GenericInstance: List<int> at ID 11
    // (user-defined List at ID 10 shadows built-in List at ID 7)
    assert_eq!(
        result.registry.get(11),
        Some(&TypeDef::GenericInstance {
            base: 10,      // user-defined List
            args: vec![0], // Int
        })
    );

    // Outer GenericInstance: HashMap<str, List<int>> at ID 12
    assert_eq!(
        result.registry.get(12),
        Some(&TypeDef::GenericInstance {
            base: 9,           // HashMap
            args: vec![2, 11], // str, List<int>
        })
    );

    // Container at ID 13
    assert_eq!(
        result.registry.get(13),
        Some(&TypeDef::Record(vec![FieldDef {
            name: "map".to_string(),
            type_id: 12,
        }]))
    );
}

#[test]
fn test_resolve_generic_unknown_base() {
    let mut registry = TypeRegistry::new();
    let result = register_decls(
        &mut registry,
        &[Decl::RecordDef {
            name: "Foo".to_string(),
            fields: vec![Field {
                name: "x".to_string(),
                type_: Type::Generic {
                    base: "NonExistent".to_string(),
                    args: vec![Type::Named("int".to_string())],
                },
            }],
            is_pub: true,
            span: dummy_span(),
        }],
    );

    // Should not panic — the record should still be registered
    assert_eq!(result.registry.len(), 10);
    assert_eq!(result.name_map.get("Foo"), Some(&9));
    match result.registry.get(9) {
        Some(TypeDef::Record(fields)) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "x");
        }
        other => panic!("Expected Record at ID 9, got {:?}", other),
    }
}
