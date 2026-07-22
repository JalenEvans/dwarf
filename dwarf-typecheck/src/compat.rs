//! Structural type compatibility checking.
//!
//! Provides the [`check`] function which determines whether two types are
//! structurally compatible (same shape, same field/variant types, etc.)
//! after resolving all aliases.

use crate::registry::TypeRegistry;
use crate::types::{FieldDef, PrimitiveType, TypeDef, TypeId};
use std::collections::{BTreeSet, HashMap};

/// The result of a structural compatibility check.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompatibilityResult {
    /// Whether the two types are compatible.
    pub compatible: bool,
    /// A list of details describing the comparison at the field/variant/param
    /// level. All entries are [`CompatDetail::Ok`] when compatible.
    pub details: Vec<CompatDetail>,
}

/// A single compatibility detail describing one aspect of the comparison.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CompatDetail {
    /// The corresponding part matched.
    Ok,
    /// A record field exists in both types but has a different type.
    FieldTypeMismatch {
        field: String,
        expected: TypeId,
        actual: TypeId,
    },
    /// A field that exists in the expected type is missing from the actual type.
    MissingField { field: String },
    /// An unexpected field exists in the actual type but not in the expected.
    ExtraField { field: String },
    /// A union variant exists in both types but has a different payload type.
    VariantTypeMismatch {
        variant: String,
        expected: Option<TypeId>,
        actual: Option<TypeId>,
    },
    /// A variant that exists in the expected union is missing from the actual.
    MissingVariant { variant: String },
    /// An unexpected variant exists in the actual union but not in the expected.
    ExtraVariant { variant: String },
    /// Two primitive types do not match.
    PrimitiveMismatch {
        expected: PrimitiveType,
        actual: PrimitiveType,
    },
    /// Two function types have a different number of parameters.
    ParamCountMismatch { expected: usize, actual: usize },
    /// Two function types have different return types.
    ReturnTypeMismatch { expected: TypeId, actual: TypeId },
}

/// Check whether two types are structurally compatible.
///
/// Aliases are resolved before comparison (see [`TypeRegistry::resolve`]).
pub fn check(registry: &TypeRegistry, expected: TypeId, actual: TypeId) -> CompatibilityResult {
    let expected_id = registry.resolve(expected);
    let actual_id = registry.resolve(actual);

    let expected_def = registry.get(expected_id);
    let actual_def = registry.get(actual_id);

    match (expected_def, actual_def) {
        (Some(TypeDef::Primitive(e)), Some(TypeDef::Primitive(a))) => check_primitives(e, a),
        (Some(TypeDef::Record(e_fields)), Some(TypeDef::Record(a_fields))) => {
            check_records(registry, e_fields, a_fields)
        }
        (Some(TypeDef::Union(e_variants)), Some(TypeDef::Union(a_variants))) => {
            check_unions(registry, e_variants, a_variants)
        }
        (Some(TypeDef::Func(e_params, e_ret)), Some(TypeDef::Func(a_params, a_ret))) => {
            check_funcs(registry, e_params, e_ret, a_params, a_ret)
        }
        (
            Some(TypeDef::GenericInstance { base, args }),
            Some(TypeDef::GenericInstance {
                base: actual_base,
                args: actual_args,
            }),
        ) => check_generic_instances(registry, base, args, actual_base, actual_args),
        // Cross-kind (or unresolved types) are always incompatible.
        _ => CompatibilityResult {
            compatible: false,
            details: vec![],
        },
    }
}

fn check_primitives(expected: &PrimitiveType, actual: &PrimitiveType) -> CompatibilityResult {
    if expected == actual {
        CompatibilityResult {
            compatible: true,
            details: vec![CompatDetail::Ok],
        }
    } else {
        CompatibilityResult {
            compatible: false,
            details: vec![CompatDetail::PrimitiveMismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            }],
        }
    }
}

fn check_records(
    registry: &TypeRegistry,
    expected: &[FieldDef],
    actual: &[FieldDef],
) -> CompatibilityResult {
    let mut details = Vec::new();

    let e_fields: HashMap<&str, TypeId> = expected
        .iter()
        .map(|f| (f.name.as_str(), f.type_id))
        .collect();
    let a_fields: HashMap<&str, TypeId> = actual
        .iter()
        .map(|f| (f.name.as_str(), f.type_id))
        .collect();

    let all_names: BTreeSet<&str> = e_fields
        .keys()
        .copied()
        .chain(a_fields.keys().copied())
        .collect();

    for name in all_names {
        match (e_fields.get(name), a_fields.get(name)) {
            (Some(_), None) => {
                details.push(CompatDetail::MissingField {
                    field: name.to_string(),
                });
            }
            (None, Some(_)) => {
                details.push(CompatDetail::ExtraField {
                    field: name.to_string(),
                });
            }
            (Some(e_tid), Some(a_tid)) => {
                let result = check(registry, *e_tid, *a_tid);
                if result.compatible {
                    details.push(CompatDetail::Ok);
                } else {
                    details.push(CompatDetail::FieldTypeMismatch {
                        field: name.to_string(),
                        expected: *e_tid,
                        actual: *a_tid,
                    });
                }
            }
            (None, None) => unreachable!(),
        }
    }

    let compatible = details.iter().all(|d| *d == CompatDetail::Ok);
    CompatibilityResult {
        compatible,
        details,
    }
}

fn check_unions(
    registry: &TypeRegistry,
    expected: &[crate::types::VariantDef],
    actual: &[crate::types::VariantDef],
) -> CompatibilityResult {
    let mut details = Vec::new();

    let e_variants: HashMap<&str, &crate::types::VariantDef> =
        expected.iter().map(|v| (v.name.as_str(), v)).collect();
    let a_variants: HashMap<&str, &crate::types::VariantDef> =
        actual.iter().map(|v| (v.name.as_str(), v)).collect();

    let all_names: BTreeSet<&str> = e_variants
        .keys()
        .copied()
        .chain(a_variants.keys().copied())
        .collect();

    for name in all_names {
        match (e_variants.get(name), a_variants.get(name)) {
            (Some(_), None) => {
                details.push(CompatDetail::MissingVariant {
                    variant: name.to_string(),
                });
            }
            (None, Some(_)) => {
                details.push(CompatDetail::ExtraVariant {
                    variant: name.to_string(),
                });
            }
            (Some(ev), Some(av)) => match (ev.type_id, av.type_id) {
                (Some(et), Some(at)) => {
                    let result = check(registry, et, at);
                    if result.compatible {
                        details.push(CompatDetail::Ok);
                    } else {
                        details.push(CompatDetail::VariantTypeMismatch {
                            variant: name.to_string(),
                            expected: Some(et),
                            actual: Some(at),
                        });
                    }
                }
                (None, None) => {
                    // Both are unit variants (no payload) — they match.
                    details.push(CompatDetail::Ok);
                }
                (et, at) => {
                    // One has a payload and the other doesn't — mismatch.
                    details.push(CompatDetail::VariantTypeMismatch {
                        variant: name.to_string(),
                        expected: et,
                        actual: at,
                    });
                }
            },
            (None, None) => unreachable!(),
        }
    }

    let compatible = details.iter().all(|d| *d == CompatDetail::Ok);
    CompatibilityResult {
        compatible,
        details,
    }
}

fn check_funcs(
    registry: &TypeRegistry,
    e_params: &[TypeId],
    e_ret: &TypeId,
    a_params: &[TypeId],
    a_ret: &TypeId,
) -> CompatibilityResult {
    let mut details = Vec::new();

    if e_params.len() != a_params.len() {
        details.push(CompatDetail::ParamCountMismatch {
            expected: e_params.len(),
            actual: a_params.len(),
        });
    }

    // Compare each overlapping param position.
    let min_len = e_params.len().min(a_params.len());
    for i in 0..min_len {
        let result = check(registry, e_params[i], a_params[i]);
        if result.compatible {
            details.push(CompatDetail::Ok);
        } else {
            // No dedicated ParamTypeMismatch variant; include the recursive
            // details so the caller still sees the incompatibility.
            details.extend(result.details);
        }
    }

    // Compare return types.
    let ret_result = check(registry, *e_ret, *a_ret);
    if ret_result.compatible {
        details.push(CompatDetail::Ok);
    } else {
        details.push(CompatDetail::ReturnTypeMismatch {
            expected: *e_ret,
            actual: *a_ret,
        });
    }

    let compatible = details.iter().all(|d| *d == CompatDetail::Ok);
    CompatibilityResult {
        compatible,
        details,
    }
}

fn check_generic_instances(
    registry: &TypeRegistry,
    base: &TypeId,
    args: &[TypeId],
    actual_base: &TypeId,
    actual_args: &[TypeId],
) -> CompatibilityResult {
    let mut details = Vec::new();

    if base != actual_base {
        return CompatibilityResult {
            compatible: false,
            details: vec![],
        };
    }

    if args.len() != actual_args.len() {
        return CompatibilityResult {
            compatible: false,
            details: vec![],
        };
    }

    for (a, b) in args.iter().zip(actual_args.iter()) {
        let result = check(registry, *a, *b);
        if result.compatible {
            details.push(CompatDetail::Ok);
        } else {
            details.extend(result.details);
        }
    }

    let compatible = details.iter().all(|d| *d == CompatDetail::Ok);
    CompatibilityResult {
        compatible,
        details,
    }
}
