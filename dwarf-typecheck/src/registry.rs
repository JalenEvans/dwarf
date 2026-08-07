//! TypeRegistry — stores and resolves all type definitions.

use std::collections::HashMap;

use crate::types::{PrimitiveType, TypeDef, TypeId};

/// The central store for type definitions.
///
/// Pre-registers five primitive types at construction:
/// - 0: Int
/// - 1: Float
/// - 2: Str
/// - 3: Bool
/// - 4: Null
///
/// User-defined types are assigned IDs starting from 5.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TypeRegistry {
    types: Vec<TypeDef>,
    /// Maps a method-signature key to the TypeId of the method's
    /// `TypeDef::Func` signature (implicit `self` excluded). The key is
    /// `format!("{owner}:{name}")` — a single `String` rather than a
    /// `(TypeId, String)` tuple because tuple keys are not valid JSON object
    /// keys, so a registry with populated methods would panic on
    /// serde_json serialization.
    /// Populated by `resolve::register_decls`; used by inference to resolve
    /// `self.method(...)` calls and by interface conformance checking.
    #[serde(default)]
    method_sigs: HashMap<String, TypeId>,
}

impl TypeRegistry {
    /// Create a new registry with pre-registered primitives and built-in generics.
    ///
    /// IDs 0-4: Primitives (Int, Float, Str, Bool, Null)
    /// IDs 5-8: Built-in generics (Option, Result, List, Map)
    pub fn new() -> Self {
        Self {
            types: vec![
                TypeDef::Primitive(PrimitiveType::Int),   // 0
                TypeDef::Primitive(PrimitiveType::Float), // 1
                TypeDef::Primitive(PrimitiveType::Str),   // 2
                TypeDef::Primitive(PrimitiveType::Bool),  // 3
                TypeDef::Primitive(PrimitiveType::Null),  // 4
                TypeDef::BuiltinGeneric {
                    name: "Option".to_string(),
                }, // 5
                TypeDef::BuiltinGeneric {
                    name: "Result".to_string(),
                }, // 6
                TypeDef::BuiltinGeneric {
                    name: "List".to_string(),
                }, // 7
                TypeDef::BuiltinGeneric {
                    name: "Map".to_string(),
                }, // 8
            ],
            method_sigs: HashMap::new(),
        }
    }

    /// Get the List base type used for array inference.
    ///
    /// List is permanently registered as built-in generic ID 7.
    /// This method is kept for backward compatibility.
    pub fn get_or_create_list_base(&mut self) -> TypeId {
        7
    }

    /// Get the Option base type used for non-null assertion inference.
    ///
    /// Option is permanently registered as built-in generic ID 5.
    pub fn get_or_create_option_base(&mut self) -> TypeId {
        5
    }

    /// Register an anonymous union type from a list of variants.
    pub fn register_anonymous_union(&mut self, variants: Vec<crate::types::VariantDef>) -> TypeId {
        self.register(TypeDef::Union(variants))
    }

    /// Returns the TypeId for a built-in generic type constructor by name.
    pub fn get_builtin_id(&self, name: &str) -> Option<TypeId> {
        self.types
            .iter()
            .position(|def| matches!(def, TypeDef::BuiltinGeneric { name: n } if n == name))
    }

    /// Register a new type definition, returning its TypeId.
    /// TypeIds are assigned sequentially starting from 5.
    pub fn register(&mut self, def: TypeDef) -> TypeId {
        let id = self.types.len();
        self.types.push(def);
        id
    }

    /// Register a method signature for an owning record/interface type.
    ///
    /// `func_id` must be the TypeId of a `TypeDef::Func` whose parameter list
    /// excludes the implicit `self`.
    pub fn register_method_sig(&mut self, owner: TypeId, name: String, func_id: TypeId) {
        self.method_sigs.insert(format!("{owner}:{name}"), func_id);
    }

    /// Look up the `TypeDef::Func` TypeId registered for a method on an
    /// owning record/interface type, or `None` if the type has no such method.
    pub fn lookup_method_sig(&self, owner: TypeId, name: &str) -> Option<TypeId> {
        self.method_sigs.get(&format!("{owner}:{name}")).copied()
    }

    /// Get a type definition by ID. Returns None if ID is out of bounds.
    pub fn get(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.get(id)
    }

    /// Follow alias chains to find the canonical TypeId.
    /// Detects cycles to prevent infinite loops — if a cycle is detected,
    /// returns the given `id` unchanged.
    pub fn resolve(&self, id: TypeId) -> TypeId {
        let mut visited = std::collections::HashSet::new();
        let mut current = id;
        visited.insert(current);
        loop {
            match self.types.get(current) {
                Some(TypeDef::Alias(target)) => {
                    if !visited.insert(*target) {
                        // Cycle detected — return the current ID
                        return current;
                    }
                    current = *target;
                }
                _ => return current,
            }
        }
    }

    /// Number of registered types (including primitives).
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Render a TypeId as a human-readable type name for error messages
    /// (mirrors `dwarf-lsp`'s `type_id_to_name`).
    ///
    /// Primitive types and built-in generics use their source-level names;
    /// generic instances render as `Base<Arg1, Arg2>`; anything else falls
    /// back to `type#<id>`.
    pub fn type_name(&self, type_id: TypeId) -> String {
        use crate::types::{
            ANY_TYPE_ID, BOOL_TYPE_ID, FLOAT_TYPE_ID, INT_TYPE_ID, LIST_TYPE_ID, MAP_TYPE_ID,
            NEVER_TYPE_ID, NULL_TYPE_ID, OPTION_TYPE_ID, RESULT_TYPE_ID, STR_TYPE_ID,
        };
        match type_id {
            INT_TYPE_ID => "Int".to_string(),
            FLOAT_TYPE_ID => "Float".to_string(),
            STR_TYPE_ID => "Str".to_string(),
            BOOL_TYPE_ID => "Bool".to_string(),
            NULL_TYPE_ID => "Null".to_string(),
            OPTION_TYPE_ID => "Option".to_string(),
            RESULT_TYPE_ID => "Result".to_string(),
            LIST_TYPE_ID => "List".to_string(),
            MAP_TYPE_ID => "Map".to_string(),
            NEVER_TYPE_ID => "Never".to_string(),
            ANY_TYPE_ID => "Any".to_string(),
            _ => match self.types.get(type_id) {
                Some(TypeDef::Primitive(p)) => format!("{p:?}"),
                Some(TypeDef::BuiltinGeneric { name }) => name.clone(),
                Some(TypeDef::GenericInstance { base, args }) => {
                    let base_name = self.type_name(*base);
                    let args_str = args
                        .iter()
                        .map(|a| self.type_name(*a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{base_name}<{args_str}>")
                }
                _ => format!("type#{type_id}"),
            },
        }
    }

    /// Returns true if only built-in types (primitives + built-in generics) are registered.
    pub fn is_empty(&self) -> bool {
        self.types.len() == 9
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrimitiveType;

    // ------------------------------------------------------------------
    // Built-in generic registration tests (DWARF-GENERICS)
    //
    // WILL FAIL — RED PHASE
    //
    // These tests verify that Option<T>, Result<T,E>, List<T>, and Map<K,V>
    // are registered as built-in generic type constructors in TypeRegistry.
    //
    // They will fail until:
    //   1. TypeDef::BuiltinGeneric { name: String } variant is added
    //   2. TypeRegistry::new() registers the 4 built-in generics after primitives
    //   3. TypeRegistry::get_builtin_id(&self, name: &str) -> Option<TypeId> is added
    // ------------------------------------------------------------------

    #[test]
    fn test_option_registered_as_builtin() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        let option_id = registry.get_builtin_id("Option");
        assert!(
            option_id.is_some(),
            "Option should be registered as built-in"
        );
        let def = registry.get(option_id.unwrap());
        assert!(matches!(def, Some(TypeDef::BuiltinGeneric { name }) if name == "Option"));
    }

    #[test]
    fn test_result_registered_as_builtin() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        let result_id = registry.get_builtin_id("Result");
        assert!(
            result_id.is_some(),
            "Result should be registered as built-in"
        );
    }

    #[test]
    fn test_list_registered_as_builtin() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        let list_id = registry.get_builtin_id("List");
        assert!(list_id.is_some(), "List should be registered as built-in");
    }

    #[test]
    fn test_map_registered_as_builtin() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        let map_id = registry.get_builtin_id("Map");
        assert!(map_id.is_some(), "Map should be registered as built-in");
    }

    #[test]
    fn test_option_int_generic_instance() {
        // WILL FAIL — RED PHASE
        let mut registry = TypeRegistry::new();
        let option_id = registry.get_builtin_id("Option").unwrap();
        let int_id = 0; // Int is always ID 0
        let instance_id = registry.register(TypeDef::GenericInstance {
            base: option_id,
            args: vec![int_id],
        });
        let def = registry.get(instance_id);
        match def {
            Some(TypeDef::GenericInstance { base, args }) => {
                assert_eq!(*base, option_id);
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], int_id);
            }
            other => panic!("Expected GenericInstance, got: {:?}", other),
        }
    }

    #[test]
    fn test_primitives_unchanged() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        assert_eq!(registry.len(), 9); // 5 primitives + 4 builtin generics
        assert!(matches!(
            registry.get(0),
            Some(TypeDef::Primitive(PrimitiveType::Int))
        ));
        assert!(matches!(
            registry.get(1),
            Some(TypeDef::Primitive(PrimitiveType::Float))
        ));
        assert!(matches!(
            registry.get(2),
            Some(TypeDef::Primitive(PrimitiveType::Str))
        ));
        assert!(matches!(
            registry.get(3),
            Some(TypeDef::Primitive(PrimitiveType::Bool))
        ));
        assert!(matches!(
            registry.get(4),
            Some(TypeDef::Primitive(PrimitiveType::Null))
        ));
    }

    #[test]
    fn test_builtin_generics_have_correct_ids() {
        // WILL FAIL — RED PHASE
        let registry = TypeRegistry::new();
        // Built-in generics start at ID 5 (after 5 primitives)
        let option_id = registry.get_builtin_id("Option").unwrap();
        let result_id = registry.get_builtin_id("Result").unwrap();
        let list_id = registry.get_builtin_id("List").unwrap();
        let map_id = registry.get_builtin_id("Map").unwrap();
        assert_eq!(option_id, 5);
        assert_eq!(result_id, 6);
        assert_eq!(list_id, 7);
        assert_eq!(map_id, 8);
    }
}
