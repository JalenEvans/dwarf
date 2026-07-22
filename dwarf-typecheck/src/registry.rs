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
    /// Create a new registry with pre-registered primitives.
    pub fn new() -> Self {
        Self {
            types: vec![
                TypeDef::Primitive(PrimitiveType::Int),
                TypeDef::Primitive(PrimitiveType::Float),
                TypeDef::Primitive(PrimitiveType::Str),
                TypeDef::Primitive(PrimitiveType::Bool),
                TypeDef::Primitive(PrimitiveType::Null),
            ],
        }
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

    /// Returns true if only primitives are registered.
    pub fn is_empty(&self) -> bool {
        self.types.len() == 5
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
