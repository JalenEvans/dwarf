//! Integration tests for the structural type compatibility module.
//!
//! These tests validate the public API of `compat::check()` and the
//! supporting types (`CompatibilityResult`, `CompatDetail`, etc.).
//!
//! All tests are expected to fail (Red phase) because the compat module
//! is not yet implemented — only stubs exist.

use dwarf_typecheck::compat::{self, *};
use dwarf_typecheck::registry::TypeRegistry;
use dwarf_typecheck::types::*;

// ---------------------------------------------------------------------------
// Helper: build a registry with some common types
// ---------------------------------------------------------------------------

/// Creates a fresh registry pre-populated with useful types for testing.
///
/// IDs (after the 5 primitives):
///   5: Record { x: Int, y: Int }
///   6: Record { y: Int, x: Int }             (same fields, swapped order)
///   7: Record { x: Int, y: Float }            (field y has different type)
///   8: Record { x: Int }                      (missing field y)
///   9: Record { x: Int, y: Int, z: Bool }     (extra field z)
///  10: Union { Ok: Int, Err: Str }
///  11: Union { Err: Str, Ok: Int }            (same variants, swapped)
///  12: Union { Ok: Float, Err: Str }          (Ok carries Float instead)
///  13: Union { Ok: Int }                      (missing Err variant)
///  14: Union { Ok: Int, Err: Str, Warn: Bool } (extra Warn variant)
///  15: Func(Int, Float) -> Bool
///  16: Func(Int) -> Bool                      (fewer params)
///  17: Func(Int, Float) -> Str                (different return type)
///  18: Alias -> 5                              (alias to the xy record)
///  19: Alias -> 18                             (alias chain: 19 -> 18 -> 5)
///  20: Record { nested: 5 }                   (record with nested record field)
///  21: Record { nested: 7 }                   (mismatching nested record)
///  22: Record { x: Int, y: Int }              (same shape as ID 5, new ID)
///  23: Union { Nil: Null }                    (unit variant)
///  24: Union { Nil: Null, Some: Int }         (mix of unit + payload)
///  25: Func(Int, Float) -> Bool               (same shape as ID 15, new ID)
///  26: Alias -> 10                             (alias to Ok/Err union)
fn build_registry() -> TypeRegistry {
    let mut r = TypeRegistry::new();

    // ID 5: Record { x: Int, y: Int }
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
    ]));

    // ID 6: Record { y: Int, x: Int }  (swapped field order)
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
    ]));

    // ID 7: Record { x: Int, y: Float }
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 1,
        },
    ]));

    // ID 8: Record { x: Int }  (missing y)
    r.register(TypeDef::Record(vec![FieldDef {
        name: "x".to_string(),
        type_id: 0,
    }]));

    // ID 9: Record { x: Int, y: Int, z: Bool }
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "z".to_string(),
            type_id: 3,
        },
    ]));

    // ID 10: Union { Ok: Int, Err: Str }
    r.register(TypeDef::Union(vec![
        VariantDef {
            name: "Ok".to_string(),
            type_id: Some(0),
        },
        VariantDef {
            name: "Err".to_string(),
            type_id: Some(2),
        },
    ]));

    // ID 11: Union { Err: Str, Ok: Int }  (swapped order)
    r.register(TypeDef::Union(vec![
        VariantDef {
            name: "Err".to_string(),
            type_id: Some(2),
        },
        VariantDef {
            name: "Ok".to_string(),
            type_id: Some(0),
        },
    ]));

    // ID 12: Union { Ok: Float, Err: Str }  (Ok has Float instead of Int)
    r.register(TypeDef::Union(vec![
        VariantDef {
            name: "Ok".to_string(),
            type_id: Some(1),
        },
        VariantDef {
            name: "Err".to_string(),
            type_id: Some(2),
        },
    ]));

    // ID 13: Union { Ok: Int }  (missing Err)
    r.register(TypeDef::Union(vec![VariantDef {
        name: "Ok".to_string(),
        type_id: Some(0),
    }]));

    // ID 14: Union { Ok: Int, Err: Str, Warn: Bool }
    r.register(TypeDef::Union(vec![
        VariantDef {
            name: "Ok".to_string(),
            type_id: Some(0),
        },
        VariantDef {
            name: "Err".to_string(),
            type_id: Some(2),
        },
        VariantDef {
            name: "Warn".to_string(),
            type_id: Some(3),
        },
    ]));

    // ID 15: Func(Int, Float) -> Bool
    r.register(TypeDef::Func(vec![0, 1], 3));

    // ID 16: Func(Int) -> Bool
    r.register(TypeDef::Func(vec![0], 3));

    // ID 17: Func(Int, Float) -> Str
    r.register(TypeDef::Func(vec![0, 1], 2));

    // ID 18: Alias -> 5
    r.register(TypeDef::Alias(5));

    // ID 19: Alias -> 18  (alias chain)
    r.register(TypeDef::Alias(18));

    // ID 20: Record { nested: 5 }
    r.register(TypeDef::Record(vec![FieldDef {
        name: "nested".to_string(),
        type_id: 5,
    }]));

    // ID 21: Record { nested: 7 }
    r.register(TypeDef::Record(vec![FieldDef {
        name: "nested".to_string(),
        type_id: 7,
    }]));

    // ID 22: Record { x: Int, y: Int }  (duplicate of ID 5)
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "x".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "y".to_string(),
            type_id: 0,
        },
    ]));

    // ID 23: Union { Nil: Null }
    r.register(TypeDef::Union(vec![VariantDef {
        name: "Nil".to_string(),
        type_id: Some(4),
    }]));

    // ID 24: Union { Nil: Null, Some: Int }
    r.register(TypeDef::Union(vec![
        VariantDef {
            name: "Nil".to_string(),
            type_id: Some(4),
        },
        VariantDef {
            name: "Some".to_string(),
            type_id: Some(0),
        },
    ]));

    // ID 25: Func(Int, Float) -> Bool  (duplicate of ID 15)
    r.register(TypeDef::Func(vec![0, 1], 3));

    // ID 26: Alias -> 10
    r.register(TypeDef::Alias(10));

    r
}

// ===========================================================================
// 1. Primitives — match same kind
// ===========================================================================

#[test]
fn test_primitives_int_int_compatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 0, 0);
    assert!(result.compatible, "Int should be compatible with Int");
    // At minimum there should be one detail entry
    assert!(!result.details.is_empty(), "Should have detail entries");
    assert!(
        result.details.iter().all(|d| *d == CompatDetail::Ok),
        "All details should be Ok"
    );
}

#[test]
fn test_primitives_float_float_compatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 1, 1);
    assert!(result.compatible, "Float should be compatible with Float");
    assert!(!result.details.is_empty());
}

#[test]
fn test_primitives_str_str_compatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 2, 2);
    assert!(result.compatible, "Str should be compatible with Str");
    assert!(!result.details.is_empty());
}

#[test]
fn test_primitives_bool_bool_compatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 3, 3);
    assert!(result.compatible, "Bool should be compatible with Bool");
    assert!(!result.details.is_empty());
}

#[test]
fn test_primitives_null_null_compatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 4, 4);
    assert!(result.compatible, "Null should be compatible with Null");
    assert!(!result.details.is_empty());
}

#[test]
fn test_alias_to_primitive_compatible_with_primitive() {
    let mut r = TypeRegistry::new();
    // Register an alias to Int (ID 5)
    r.register(TypeDef::Alias(0));

    // Alias(5) -> Int(0) should be compatible with Int(0)
    let result = compat::check(&r, 5, 0);
    assert!(
        result.compatible,
        "Alias to Int should be compatible with Int"
    );
}

// ===========================================================================
// 2. Primitives — mismatch
// ===========================================================================

#[test]
fn test_primitives_int_float_incompatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 0, 1);
    assert!(
        !result.compatible,
        "Int should NOT be compatible with Float"
    );
    assert!(
        result.details.contains(&CompatDetail::PrimitiveMismatch {
            expected: PrimitiveType::Int,
            actual: PrimitiveType::Float,
        }),
        "Should report PrimitiveMismatch(Int, Float)"
    );
}

#[test]
fn test_primitives_bool_str_incompatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 3, 2);
    assert!(!result.compatible, "Bool should NOT be compatible with Str");
    assert!(
        result.details.contains(&CompatDetail::PrimitiveMismatch {
            expected: PrimitiveType::Bool,
            actual: PrimitiveType::Str,
        }),
        "Should report PrimitiveMismatch(Bool, Str)"
    );
}

#[test]
fn test_primitives_float_null_incompatible() {
    let r = TypeRegistry::new();
    let result = compat::check(&r, 1, 4);
    assert!(
        !result.compatible,
        "Float should NOT be compatible with Null"
    );
    assert!(
        result.details.contains(&CompatDetail::PrimitiveMismatch {
            expected: PrimitiveType::Float,
            actual: PrimitiveType::Null,
        }),
        "Should report PrimitiveMismatch(Float, Null)"
    );
}

// ===========================================================================
// 3. Records — exact match
// ===========================================================================

#[test]
fn test_record_exact_match_compatible() {
    let r = build_registry();
    // Compare record { x: Int, y: Int } against itself
    let result = compat::check(&r, 5, 5);
    assert!(result.compatible, "Record should be compatible with itself");
    assert_eq!(
        result.details.len(),
        2,
        "Should have 2 detail entries (x, y)"
    );
    assert!(
        result.details.iter().all(|d| *d == CompatDetail::Ok),
        "All details should be Ok"
    );
}

#[test]
fn test_record_same_fields_different_order_compatible() {
    let r = build_registry();
    // { x: Int, y: Int }  vs  { y: Int, x: Int }
    let result = compat::check(&r, 5, 6);
    assert!(
        result.compatible,
        "Records with same fields in different order should be compatible"
    );
}

#[test]
fn test_record_separate_definitions_same_shape_compatible() {
    let r = build_registry();
    // ID 5 and ID 22 both define { x: Int, y: Int } at different positions
    let result = compat::check(&r, 5, 22);
    assert!(
        result.compatible,
        "Two separately-defined records with same shape should be compatible"
    );
}

// ===========================================================================
// 4. Records — field mismatch
// ===========================================================================

#[test]
fn test_record_missing_field_incompatible() {
    let r = build_registry();
    // expected: { x: Int, y: Int }, actual: { x: Int }  (missing y)
    let result = compat::check(&r, 5, 8);
    assert!(
        !result.compatible,
        "Should be incompatible when field is missing"
    );
    assert!(
        result.details.contains(&CompatDetail::MissingField {
            field: "y".to_string(),
        }),
        "Should report MissingField for 'y'"
    );
}

#[test]
fn test_record_extra_field_incompatible() {
    let r = build_registry();
    // expected: { x: Int, y: Int }, actual: { x: Int, y: Int, z: Bool }  (extra z)
    let result = compat::check(&r, 5, 9);
    assert!(
        !result.compatible,
        "Should be incompatible when actual has extra field"
    );
    assert!(
        result.details.contains(&CompatDetail::ExtraField {
            field: "z".to_string(),
        }),
        "Should report ExtraField for 'z'"
    );
}

#[test]
fn test_record_field_type_mismatch_incompatible() {
    let r = build_registry();
    // expected: { x: Int, y: Int }, actual: { x: Int, y: Float }
    let result = compat::check(&r, 5, 7);
    assert!(
        !result.compatible,
        "Should be incompatible when field type differs"
    );
    assert!(
        result.details.contains(&CompatDetail::FieldTypeMismatch {
            field: "y".to_string(),
            expected: 0,
            actual: 1,
        }),
        "Should report FieldTypeMismatch for field 'y'"
    );
}

// ===========================================================================
// 5. Record field type depth
// ===========================================================================

#[test]
fn test_record_nested_field_mismatch_incompatible() {
    let r = build_registry();
    // expected: { nested: 5 (Record { x: Int, y: Int }) }
    // actual:   { nested: 7 (Record { x: Int, y: Float }) }
    let result = compat::check(&r, 20, 21);
    assert!(
        !result.compatible,
        "Should be incompatible when nested record fields differ"
    );
}

#[test]
fn test_record_nested_field_match_compatible() {
    let r = build_registry();
    // expected: { nested: 5 }, actual: { nested: 22 }
    // Both are Record { x: Int, y: Int } at different IDs
    let result = compat::check(&r, 20, 20);
    assert!(
        result.compatible,
        "Should be compatible when nested record fields match"
    );
}

// ===========================================================================
// 6. Unions — exact match
// ===========================================================================

#[test]
fn test_union_exact_match_compatible() {
    let r = build_registry();
    // Compare union { Ok: Int, Err: Str } against itself
    let result = compat::check(&r, 10, 10);
    assert!(result.compatible, "Union should be compatible with itself");
    assert_eq!(
        result.details.len(),
        2,
        "Should have 2 detail entries (Ok, Err)"
    );
    assert!(
        result.details.iter().all(|d| *d == CompatDetail::Ok),
        "All details should be Ok"
    );
}

#[test]
fn test_union_same_variants_different_order_compatible() {
    let r = build_registry();
    // { Ok: Int, Err: Str }  vs  { Err: Str, Ok: Int }
    let result = compat::check(&r, 10, 11);
    assert!(
        result.compatible,
        "Unions with same variants in different order should be compatible"
    );
}

#[test]
fn test_union_unit_variant_exact_match_compatible() {
    let r = build_registry();
    let result = compat::check(&r, 23, 23);
    assert!(
        result.compatible,
        "Union with unit variant should be compatible with itself"
    );
}

// ===========================================================================
// 7. Unions — variant mismatch
// ===========================================================================

#[test]
fn test_union_missing_variant_incompatible() {
    let r = build_registry();
    // expected: { Ok: Int, Err: Str }, actual: { Ok: Int }  (missing Err)
    let result = compat::check(&r, 10, 13);
    assert!(
        !result.compatible,
        "Should be incompatible when variant is missing"
    );
    assert!(
        result.details.contains(&CompatDetail::MissingVariant {
            variant: "Err".to_string(),
        }),
        "Should report MissingVariant for 'Err'"
    );
}

#[test]
fn test_union_extra_variant_incompatible() {
    let r = build_registry();
    // expected: { Ok: Int, Err: Str }, actual: { Ok: Int, Err: Str, Warn: Bool }
    let result = compat::check(&r, 10, 14);
    assert!(
        !result.compatible,
        "Should be incompatible when actual has extra variant"
    );
    assert!(
        result.details.contains(&CompatDetail::ExtraVariant {
            variant: "Warn".to_string(),
        }),
        "Should report ExtraVariant for 'Warn'"
    );
}

#[test]
fn test_union_variant_payload_type_mismatch_incompatible() {
    let r = build_registry();
    // expected: { Ok: Int, Err: Str }, actual: { Ok: Float, Err: Str }
    let result = compat::check(&r, 10, 12);
    assert!(
        !result.compatible,
        "Should be incompatible when variant payload type differs"
    );
    assert!(
        result.details.contains(&CompatDetail::VariantTypeMismatch {
            variant: "Ok".to_string(),
            expected: Some(0),
            actual: Some(1),
        }),
        "Should report VariantTypeMismatch for 'Ok'"
    );
}

#[test]
fn test_union_variant_payload_none_vs_some_mismatch() {
    let mut r = TypeRegistry::new();
    // Register union { Nil: Null }  (ID 5)
    r.register(TypeDef::Union(vec![VariantDef {
        name: "Nil".to_string(),
        type_id: Some(4),
    }]));
    // Register union { Nil: None }  (ID 6) — unit variant, no payload
    r.register(TypeDef::Union(vec![VariantDef {
        name: "Nil".to_string(),
        type_id: None,
    }]));

    let result = compat::check(&r, 5, 6);
    assert!(
        !result.compatible,
        "Should be incompatible when variant payload type differs (Some vs None)"
    );
}

// ===========================================================================
// 8. Function types
// ===========================================================================

#[test]
fn test_func_exact_match_compatible() {
    let r = build_registry();
    // Func(Int, Float) -> Bool  vs  itself
    let result = compat::check(&r, 15, 15);
    assert!(result.compatible, "Func should be compatible with itself");
    assert!(
        result.details.iter().all(|d| *d == CompatDetail::Ok),
        "All details should be Ok"
    );
}

#[test]
fn test_func_same_shape_different_id_compatible() {
    let r = build_registry();
    // ID 15 and ID 25 both define Func(Int, Float) -> Bool
    let result = compat::check(&r, 15, 25);
    assert!(
        result.compatible,
        "Two separately-defined funcs with same signature should be compatible"
    );
}

#[test]
fn test_func_param_count_mismatch_incompatible() {
    let r = build_registry();
    // expected: Func(Int, Float) -> Bool, actual: Func(Int) -> Bool
    let result = compat::check(&r, 15, 16);
    assert!(
        !result.compatible,
        "Should be incompatible when param count differs"
    );
    assert!(
        result.details.contains(&CompatDetail::ParamCountMismatch {
            expected: 2,
            actual: 1,
        }),
        "Should report ParamCountMismatch(2, 1)"
    );
}

#[test]
fn test_func_return_type_mismatch_incompatible() {
    let r = build_registry();
    // expected: Func(Int, Float) -> Bool, actual: Func(Int, Float) -> Str
    let result = compat::check(&r, 15, 17);
    assert!(
        !result.compatible,
        "Should be incompatible when return type differs"
    );
    assert!(
        result.details.contains(&CompatDetail::ReturnTypeMismatch {
            expected: 3,
            actual: 2,
        }),
        "Should report ReturnTypeMismatch(Bool, Str)"
    );
}

#[test]
fn test_func_param_type_mismatch_incompatible() {
    let mut r = TypeRegistry::new();
    // Func(Int, Float) -> Bool   (ID 5)
    r.register(TypeDef::Func(vec![0, 1], 3));
    // Func(Int, Str) -> Bool     (ID 6) — second param is Str instead of Float
    r.register(TypeDef::Func(vec![0, 2], 3));

    let result = compat::check(&r, 5, 6);
    assert!(
        !result.compatible,
        "Should be incompatible when a param type differs"
    );
}

// ===========================================================================
// 9. Alias resolution
// ===========================================================================

#[test]
fn test_alias_to_record_compatible_with_record() {
    let r = build_registry();
    // Alias(5) -> Record { x: Int, y: Int }  vs  Record { x: Int, y: Int }
    let result = compat::check(&r, 18, 5);
    assert!(
        result.compatible,
        "Alias to a record should be compatible with the record itself"
    );
}

#[test]
fn test_alias_chain_resolves_to_target() {
    let r = build_registry();
    // 19 -> 18 -> 5 (Record { x: Int, y: Int })
    // Compare against the record directly
    let result = compat::check(&r, 19, 5);
    assert!(
        result.compatible,
        "Alias chain should resolve to the target record and be compatible"
    );
}

#[test]
fn test_alias_to_union_compatible_with_union() {
    let r = build_registry();
    // Alias(10) -> Union { Ok: Int, Err: Str }  vs  itself
    let result = compat::check(&r, 26, 10);
    assert!(
        result.compatible,
        "Alias to a union should be compatible with the union itself"
    );
}

#[test]
fn test_alias_mismatch_after_resolution() {
    let r = build_registry();
    // Alias(5) -> Record { x: Int, y: Int }  vs  Record { x: Int, y: Float }
    let result = compat::check(&r, 18, 7);
    assert!(
        !result.compatible,
        "Alias to record should report mismatch when resolved record differs"
    );
}

// ===========================================================================
// 10. Edge cases
// ===========================================================================

#[test]
fn test_empty_record_vs_empty_record_compatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Record(vec![])); // ID 5
    r.register(TypeDef::Record(vec![])); // ID 6

    let result = compat::check(&r, 5, 6);
    assert!(result.compatible, "Two empty records should be compatible");
}

#[test]
fn test_empty_union_vs_empty_union_compatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Union(vec![])); // ID 5
    r.register(TypeDef::Union(vec![])); // ID 6

    let result = compat::check(&r, 5, 6);
    assert!(result.compatible, "Two empty unions should be compatible");
}

#[test]
fn test_empty_func_vs_empty_func_compatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Func(vec![], 4)); // () -> Null, ID 5
    r.register(TypeDef::Func(vec![], 4)); // () -> Null, ID 6

    let result = compat::check(&r, 5, 6);
    assert!(
        result.compatible,
        "Two identical empty-param funcs should be compatible"
    );
}

#[test]
fn test_record_missing_field_and_extra_field_combined() {
    let mut r = TypeRegistry::new();
    // expected: { a: Int, b: Float }
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "a".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "b".to_string(),
            type_id: 1,
        },
    ]));
    // actual: { a: Int, c: Str }  (missing b, extra c)
    r.register(TypeDef::Record(vec![
        FieldDef {
            name: "a".to_string(),
            type_id: 0,
        },
        FieldDef {
            name: "c".to_string(),
            type_id: 2,
        },
    ]));

    let result = compat::check(&r, 5, 6);
    assert!(!result.compatible, "Should be incompatible");
    assert!(
        result.details.contains(&CompatDetail::MissingField {
            field: "b".to_string(),
        }),
        "Should report MissingField for 'b'"
    );
    assert!(
        result.details.contains(&CompatDetail::ExtraField {
            field: "c".to_string(),
        }),
        "Should report ExtraField for 'c'"
    );
    // 'a' is present in both with same type, so should be Ok or not in mismatch list
}

#[test]
fn test_mixed_type_kinds_incompatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Record(vec![])); // ID 5
    r.register(TypeDef::Union(vec![])); // ID 6
    r.register(TypeDef::Func(vec![], 0)); // ID 7

    // Record vs Union should be incompatible
    let result1 = compat::check(&r, 5, 6);
    assert!(
        !result1.compatible,
        "Record vs Union should be incompatible"
    );

    // Record vs Func should be incompatible
    let result2 = compat::check(&r, 5, 7);
    assert!(!result2.compatible, "Record vs Func should be incompatible");

    // Union vs Func should be incompatible
    let result3 = compat::check(&r, 6, 7);
    assert!(!result3.compatible, "Union vs Func should be incompatible");
}

#[test]
fn test_primitive_vs_record_incompatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Record(vec![])); // ID 5

    let result = compat::check(&r, 0, 5);
    assert!(!result.compatible, "Int vs Record should be incompatible");
}

#[test]
fn test_primitive_vs_union_incompatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Union(vec![])); // ID 5

    let result = compat::check(&r, 0, 5);
    assert!(!result.compatible, "Int vs Union should be incompatible");
}

#[test]
fn test_primitive_vs_func_incompatible() {
    let mut r = TypeRegistry::new();
    r.register(TypeDef::Func(vec![], 0)); // ID 5

    let result = compat::check(&r, 0, 5);
    assert!(!result.compatible, "Int vs Func should be incompatible");
}

// ===========================================================================
// GenericInstance compatibility tests
// ===========================================================================

#[test]
fn test_compat_generic_instance_identical() {
    let mut registry = TypeRegistry::new();
    // Register a "fake" user type at ID 5 as the base
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "dummy".to_string(),
        type_id: 0,
    }]));
    // Register two identical GenericInstances
    let id_a = registry.register(TypeDef::GenericInstance {
        base: 5,
        args: vec![0, 2],
    });
    let id_b = registry.register(TypeDef::GenericInstance {
        base: 5,
        args: vec![0, 2],
    });
    // Same TypeId always compatible
    assert!(compat::check(&registry, id_a, id_a).compatible);
    // Different TypeId with same structure should be compatible
    assert!(compat::check(&registry, id_a, id_b).compatible);
}

#[test]
fn test_compat_generic_instance_different_args() {
    let mut registry = TypeRegistry::new();
    registry.register(TypeDef::Record(vec![FieldDef {
        name: "dummy".to_string(),
        type_id: 0,
    }]));
    let id_a = registry.register(TypeDef::GenericInstance {
        base: 5,
        args: vec![0, 2],
    });
    let id_b = registry.register(TypeDef::GenericInstance {
        base: 5,
        args: vec![0, 0],
    });
    // Different type args → NOT compatible
    assert!(!compat::check(&registry, id_a, id_b).compatible);
}

#[test]
fn test_compat_generic_instance_record() {
    let mut registry = TypeRegistry::new();
    let id_record = registry.register(TypeDef::Record(vec![FieldDef {
        name: "x".to_string(),
        type_id: 0,
    }]));
    let id_generic = registry.register(TypeDef::GenericInstance {
        base: 5,
        args: vec![0],
    });
    // Different kinds → NOT compatible
    assert!(!compat::check(&registry, id_record, id_generic).compatible);
}
