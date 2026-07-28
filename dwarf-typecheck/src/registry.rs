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
/// The List base type for array inference is registered lazily on first use
/// via [`get_or_create_list_base`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeRegistry {
    types: Vec<TypeDef>,
    /// Lazily-initialised cache of the List base type ID for array inference.
    #[serde(skip)]
    list_base_id: Option<TypeId>,
}

// Manual PartialEq — skips the list_base_id cache field since it's derived
// state that is semantically implied by the types vector.
impl PartialEq for TypeRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.types == other.types
    }
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
            list_base_id: None,
        }
    }

    /// Get or create the List base type used for array inference.
    ///
    /// The List base is registered as an empty record `{}` the first time this
    /// method is called. Its ID is cached for subsequent calls so that all
    /// `GenericInstance { base: List, … }` types share the same base.
    pub fn get_or_create_list_base(&mut self) -> TypeId {
        if let Some(id) = self.list_base_id {
            return id;
        }
        let id = self.types.len();
        self.types.push(TypeDef::Record(Vec::new()));
        self.list_base_id = Some(id);
        id
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
