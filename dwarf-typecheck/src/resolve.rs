//! HIR type resolution — registers parsed type declarations into a TypeRegistry.
//!
//! This module provides [`register_decls`], which takes a list of HIR
//! declarations and populates a [`TypeRegistry`] with resolved type definitions.
//! Named types are processed in declaration order; anonymous types (inline
//! records, function types) are registered before the named type that contains
//! them, which is the order the tests expect.

use std::collections::HashMap;

use dwarf_syntax::hir::{Decl, Type as HirType};

use crate::error::TypeCheckError;
use crate::registry::TypeRegistry;
use crate::types::{
    FieldDef, LiteralType, RefConstraint, TypeDef, TypeId, VariantDef, ANY_TYPE_ID, NEVER_TYPE_ID,
};

/// The result of resolving HIR declarations into a TypeRegistry.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub registry: TypeRegistry,
    /// Maps user-defined type names to their TypeIds.
    /// Does NOT include built-in primitive names.
    pub name_map: HashMap<String, TypeId>,
    /// Maps extern function names to their registered Func TypeIds.
    pub extern_map: HashMap<String, TypeId>,
    /// Type errors discovered during resolution (e.g. unknown type names in
    /// extern signatures).
    pub errors: Vec<TypeCheckError>,
}

/// Register all type declarations from parsed HIR into a TypeRegistry.
///
/// Returns the [`ResolutionResult`] with the populated registry and name map.
///
/// Processing is a single pass over the declarations: for each named type,
/// the body types are resolved first (which may register anonymous types),
/// then the named type itself is registered. Unknown type names resolve to
/// the Null primitive (ID 4) rather than creating new placeholder entries.
pub fn register_decls(registry: &mut TypeRegistry, decls: &[Decl]) -> ResolutionResult {
    // Internal name map includes built-in primitives for resolution.
    let mut name_map: HashMap<String, TypeId> = HashMap::new();
    name_map.insert("int".to_string(), 0);
    name_map.insert("Int".to_string(), 0);
    name_map.insert("float".to_string(), 1);
    name_map.insert("Float".to_string(), 1);
    name_map.insert("str".to_string(), 2);
    name_map.insert("Str".to_string(), 2);
    name_map.insert("bool".to_string(), 3);
    name_map.insert("Bool".to_string(), 3);
    name_map.insert("null".to_string(), 4);
    name_map.insert("Null".to_string(), 4);
    name_map.insert("string".to_string(), 2); // alias for str
    name_map.insert("String".to_string(), 2); // alias for str

    // Built-in generic type constructors
    name_map.insert("Option".to_string(), 5);
    name_map.insert("Result".to_string(), 6);
    name_map.insert("List".to_string(), 7);
    name_map.insert("Map".to_string(), 8);
    // Lowercase variants for case-insensitive matching
    name_map.insert("option".to_string(), 5);
    name_map.insert("result".to_string(), 6);
    name_map.insert("list".to_string(), 7);
    name_map.insert("map".to_string(), 8);

    // The Any type (virtual, not registered in registry but used as a TypeId)
    name_map.insert("Any".to_string(), ANY_TYPE_ID);
    name_map.insert("any".to_string(), ANY_TYPE_ID);

    // The returned name_map will only contain user-defined names.
    let mut user_name_map: HashMap<String, TypeId> = HashMap::new();
    let mut extern_map: HashMap<String, TypeId> = HashMap::new();
    let mut errors: Vec<TypeCheckError> = Vec::new();

    for decl in decls {
        match decl {
            Decl::RecordDef {
                name,
                fields,
                methods: _,
                implements: _,
                is_pub: _,
                span: _,
            } => {
                // Resolve field types first (may register anonymous types),
                // then register the named record.
                let resolved_fields: Vec<FieldDef> = fields
                    .iter()
                    .map(|f| FieldDef {
                        name: f.name.clone(),
                        type_id: resolve_hir_type(&f.type_, registry, &mut name_map),
                    })
                    .collect();
                let name_str = name.clone();
                let id = registry.register(TypeDef::Record(resolved_fields));
                name_map.insert(name_str.clone(), id);
                user_name_map.insert(name_str, id);
            }
            Decl::UnionDef {
                name,
                variants,
                type_params: _,
                is_pub: _,
                span: _,
            } => {
                // Resolve variant arg types first (may register anonymous types),
                // then register the named union.
                let resolved_variants: Vec<VariantDef> = variants
                    .iter()
                    .map(|v| VariantDef {
                        name: v.name.clone(),
                        type_id: v
                            .arg
                            .as_ref()
                            .map(|t| resolve_hir_type(t, registry, &mut name_map)),
                    })
                    .collect();
                let name_str = name.clone();
                let id = registry.register(TypeDef::Union(resolved_variants));
                name_map.insert(name_str.clone(), id);
                user_name_map.insert(name_str, id);
            }
            Decl::TypeDef {
                name,
                type_,
                is_pub: _,
                span: _,
            } => {
                // Resolve the alias target first (may register anonymous types),
                // then register the named alias.
                let resolved_id = resolve_hir_type(type_, registry, &mut name_map);
                let name_str = name.clone();
                let id = registry.register(TypeDef::Alias(resolved_id));
                name_map.insert(name_str.clone(), id);
                user_name_map.insert(name_str, id);
            }
            Decl::Decorator { target, .. } => {
                // Recursively register types from the decorated target.
                // This handles nested decorators as well (Decorator wrapping
                // Decorator wrapping ... wrapping a function).
                let inner_result = register_decls(registry, std::slice::from_ref(target.as_ref()));
                user_name_map.extend(inner_result.name_map);
                extern_map.extend(inner_result.extern_map);
                errors.extend(inner_result.errors);
            }
            Decl::Extern {
                name,
                params,
                return_type,
                span,
                ..
            } => {
                // Resolve each parameter's type and the return type,
                // then register a Func type for this extern declaration.
                // Unknown type names in extern signatures are reported as
                // errors (rather than silently falling back to Null) so that
                // typos like `Itn` instead of `Int` are caught early.
                let mut resolved_params: Vec<TypeId> = Vec::with_capacity(params.len());
                for p in params {
                    match p.type_.as_ref() {
                        Some(t) => match resolve_hir_type_strict(t, registry, &mut name_map) {
                            Ok(id) => resolved_params.push(id),
                            Err(msg) => {
                                errors.push(TypeCheckError::new(
                                    "DWARF-E-TYPE-0002",
                                    format!(
                                        "unknown type in extern '{}' parameter '{}': {}",
                                        name, p.name, msg
                                    ),
                                    *span,
                                ));
                                resolved_params.push(4); // Null fallback
                            }
                        },
                        None => resolved_params.push(4), // Null for untyped params
                    }
                }
                let resolved_return = match return_type.as_ref() {
                    Some(t) => match resolve_hir_type_strict(t, registry, &mut name_map) {
                        Ok(id) => id,
                        Err(msg) => {
                            errors.push(TypeCheckError::new(
                                "DWARF-E-TYPE-0002",
                                format!("unknown type in extern '{}' return type: {}", name, msg),
                                *span,
                            ));
                            4 // Null fallback
                        }
                    },
                    None => 4, // Null for void return
                };
                let func_type = TypeDef::Func(resolved_params, resolved_return);
                let func_id = registry.register(func_type);
                extern_map.insert(name.clone(), func_id);
            }
            // Const declarations are value bindings, not type definitions.
            // They don't contribute to the type registry.
            Decl::Const { .. } => {}
            _ => {}
        }
    }

    ResolutionResult {
        registry: registry.clone(),
        name_map: user_name_map,
        extern_map,
        errors,
    }
}

/// Resolve an HIR type expression to a TypeId.
///
/// Registers anonymous types (inline records, unions, function types) in the
/// registry and returns their IDs. For named types, looks up the name in the
/// name map. Unknown type names resolve to the existing Null primitive (ID 4)
/// rather than registering a new placeholder.
fn resolve_hir_type(
    hir_type: &HirType,
    registry: &mut TypeRegistry,
    name_map: &mut HashMap<String, TypeId>,
) -> TypeId {
    match hir_type {
        HirType::Named(name) => match name_map.get(name) {
            Some(id) => *id,
            None => {
                // Unknown type name — use the existing Null primitive (ID 4)
                // instead of registering a new placeholder.
                4
            }
        },
        HirType::Record(fields) => {
            let resolved_fields: Vec<FieldDef> = fields
                .iter()
                .map(|(name, type_)| FieldDef {
                    name: name.clone(),
                    type_id: resolve_hir_type(type_, registry, name_map),
                })
                .collect();
            registry.register(TypeDef::Record(resolved_fields))
        }
        HirType::Union(types) => {
            // Unnamed union — create synthetic variant names
            let resolved_variants: Vec<VariantDef> = types
                .iter()
                .enumerate()
                .map(|(i, t)| VariantDef {
                    name: format!("V{}", i),
                    type_id: Some(resolve_hir_type(t, registry, name_map)),
                })
                .collect();
            registry.register(TypeDef::Union(resolved_variants))
        }
        HirType::Func { params, return_ } => {
            let resolved_params: Vec<TypeId> = params
                .iter()
                .map(|p| resolve_hir_type(p, registry, name_map))
                .collect();
            let resolved_return = resolve_hir_type(return_, registry, name_map);
            registry.register(TypeDef::Func(resolved_params, resolved_return))
        }
        HirType::Generic { base, args } => match name_map.get(base).copied() {
            Some(base_id) => {
                let resolved_args: Vec<TypeId> = args
                    .iter()
                    .map(|arg| resolve_hir_type(arg, registry, name_map))
                    .collect();
                registry.register(TypeDef::GenericInstance {
                    base: base_id,
                    args: resolved_args,
                })
            }
            None => 4,
        },
        HirType::Refined { base, .. } => resolve_hir_type(base, registry, name_map),
        HirType::KeyOf(inner) => {
            let target_id = resolve_hir_type(inner, registry, name_map);
            let resolved_target = registry.resolve(target_id);
            // Clone field names out of the registry to avoid borrow conflict
            // when registering new literal types below.
            let field_names: Option<Vec<String>> = match registry.get(resolved_target) {
                Some(TypeDef::Record(fields)) => {
                    Some(fields.iter().map(|f| f.name.clone()).collect())
                }
                _ => None,
            };
            match field_names {
                Some(names) if names.is_empty() => {
                    // keyof {} → empty union
                    registry.register(TypeDef::Union(vec![]))
                }
                Some(names) => {
                    // Map field names to Literal(String(_)) TypeDefs in the registry
                    let literal_ids: Vec<TypeId> = names
                        .iter()
                        .map(|name| {
                            registry.register(TypeDef::Literal(LiteralType::String(name.clone())))
                        })
                        .collect();
                    // Wrap in a union with synthetic variant names
                    registry.register(TypeDef::Union(
                        literal_ids
                            .iter()
                            .enumerate()
                            .map(|(i, id)| VariantDef {
                                name: format!("K{}", i),
                                type_id: Some(*id),
                            })
                            .collect(),
                    ))
                }
                None => {
                    // keyof on non-record — fallback to empty union
                    registry.register(TypeDef::Union(vec![]))
                }
            }
        }
        HirType::IndexedAccess { obj, key } => {
            let obj_id = resolve_hir_type(obj, registry, name_map);
            let resolved_obj = registry.resolve(obj_id);
            match registry.get(resolved_obj) {
                Some(TypeDef::Record(fields)) => fields
                    .iter()
                    .find(|f| f.name == *key)
                    .map(|f| f.type_id)
                    .unwrap_or(NEVER_TYPE_ID),
                _ => NEVER_TYPE_ID,
            }
        }
    }
}

/// Strict variant of [`resolve_hir_type`] that reports unknown type names as
/// errors instead of silently falling back to Null (ID 4).
///
/// Used when resolving extern signatures, where a typo like `Itn` instead of
/// `Int` must be surfaced to the user rather than masked by a Null default.
fn resolve_hir_type_strict(
    hir_type: &HirType,
    registry: &mut TypeRegistry,
    name_map: &mut HashMap<String, TypeId>,
) -> Result<TypeId, String> {
    match hir_type {
        HirType::Named(name) => name_map
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown type: {}", name)),
        HirType::Record(fields) => {
            let mut resolved_fields = Vec::with_capacity(fields.len());
            for (name, type_) in fields {
                let type_id = resolve_hir_type_strict(type_, registry, name_map)?;
                resolved_fields.push(FieldDef {
                    name: name.clone(),
                    type_id,
                });
            }
            Ok(registry.register(TypeDef::Record(resolved_fields)))
        }
        HirType::Union(types) => {
            let mut resolved_variants = Vec::with_capacity(types.len());
            for (i, t) in types.iter().enumerate() {
                let type_id = resolve_hir_type_strict(t, registry, name_map)?;
                resolved_variants.push(VariantDef {
                    name: format!("V{}", i),
                    type_id: Some(type_id),
                });
            }
            Ok(registry.register(TypeDef::Union(resolved_variants)))
        }
        HirType::Func { params, return_ } => {
            let mut resolved_params = Vec::with_capacity(params.len());
            for p in params {
                resolved_params.push(resolve_hir_type_strict(p, registry, name_map)?);
            }
            let resolved_return = resolve_hir_type_strict(return_, registry, name_map)?;
            Ok(registry.register(TypeDef::Func(resolved_params, resolved_return)))
        }
        HirType::Generic { base, args } => {
            let base_id = name_map
                .get(base)
                .copied()
                .ok_or_else(|| format!("unknown type: {}", base))?;
            let mut resolved_args = Vec::with_capacity(args.len());
            for arg in args {
                resolved_args.push(resolve_hir_type_strict(arg, registry, name_map)?);
            }
            Ok(registry.register(TypeDef::GenericInstance {
                base: base_id,
                args: resolved_args,
            }))
        }
        HirType::Refined { base, .. } => resolve_hir_type_strict(base, registry, name_map),
        HirType::KeyOf(inner) => {
            let target_id = resolve_hir_type_strict(inner, registry, name_map)?;
            let resolved_target = registry.resolve(target_id);
            let field_names: Option<Vec<String>> = match registry.get(resolved_target) {
                Some(TypeDef::Record(fields)) => {
                    Some(fields.iter().map(|f| f.name.clone()).collect())
                }
                _ => None,
            };
            match field_names {
                Some(names) if names.is_empty() => Ok(registry.register(TypeDef::Union(vec![]))),
                Some(names) => {
                    let literal_ids: Vec<TypeId> = names
                        .iter()
                        .map(|name| {
                            registry.register(TypeDef::Literal(LiteralType::String(name.clone())))
                        })
                        .collect();
                    Ok(registry.register(TypeDef::Union(
                        literal_ids
                            .iter()
                            .enumerate()
                            .map(|(i, id)| VariantDef {
                                name: format!("K{}", i),
                                type_id: Some(*id),
                            })
                            .collect(),
                    )))
                }
                None => Ok(registry.register(TypeDef::Union(vec![]))),
            }
        }
        HirType::IndexedAccess { obj, key } => {
            let obj_id = resolve_hir_type_strict(obj, registry, name_map)?;
            let resolved_obj = registry.resolve(obj_id);
            match registry.get(resolved_obj) {
                Some(TypeDef::Record(fields)) => Ok(fields
                    .iter()
                    .find(|f| f.name == *key)
                    .map(|f| f.type_id)
                    .unwrap_or(NEVER_TYPE_ID)),
                _ => Ok(NEVER_TYPE_ID),
            }
        }
    }
}

/// Convert an HIR `RefConstraint` to the typecheck `RefConstraint`.
pub fn convert_ref_constraint(constraint: &dwarf_syntax::hir::RefConstraint) -> RefConstraint {
    match constraint {
        dwarf_syntax::hir::RefConstraint::Range { min, max } => RefConstraint::Range {
            min: *min,
            max: *max,
        },
        dwarf_syntax::hir::RefConstraint::NonEmpty => RefConstraint::NonEmpty,
    }
}
