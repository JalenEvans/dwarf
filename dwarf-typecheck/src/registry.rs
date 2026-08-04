//! TypeRegistry — stores and resolves all type definitions.

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
