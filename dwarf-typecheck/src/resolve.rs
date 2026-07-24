//! HIR type resolution — registers parsed type declarations into a TypeRegistry.
//!
//! This module provides [`register_decls`], which takes a list of HIR
//! declarations and populates a [`TypeRegistry`] with resolved type definitions.
//! Named types are processed in declaration order; anonymous types (inline
//! records, function types) are registered before the named type that contains
//! them, which is the order the tests expect.

use std::collections::HashMap;

use dwarf_syntax::hir::{Decl, Type as HirType};

use crate::registry::TypeRegistry;
use crate::types::{FieldDef, TypeDef, TypeId, VariantDef};

/// The result of resolving HIR declarations into a TypeRegistry.
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub registry: TypeRegistry,
    /// Maps user-defined type names to their TypeIds.
    /// Does NOT include built-in primitive names.
    pub name_map: HashMap<String, TypeId>,
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
    name_map.insert("float".to_string(), 1);
    name_map.insert("str".to_string(), 2);
    name_map.insert("bool".to_string(), 3);
    name_map.insert("string".to_string(), 2); // alias for str

    // The returned name_map will only contain user-defined names.
    let mut user_name_map: HashMap<String, TypeId> = HashMap::new();

    for decl in decls {
        match decl {
            Decl::RecordDef {
                name,
                fields,
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
            }
            _ => {}
        }
    }

    ResolutionResult {
        registry: registry.clone(),
        name_map: user_name_map,
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
    }
}
