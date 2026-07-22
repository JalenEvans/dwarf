//! Type definitions and data structures for the Dwarf type system.

/// A handle/index into the TypeRegistry.
pub type TypeId = usize;

/// Built-in primitive types.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveType {
    Int,
    Float,
    Str,
    Bool,
    Null,
}

/// A named field in a record type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub type_id: TypeId,
}

/// A variant in a union type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VariantDef {
    pub name: String,
    /// The type of the variant's payload, if any.
    /// `None` for unit variants like `None` or `Nil`.
    pub type_id: Option<TypeId>,
}

/// A resolved type definition stored in the TypeRegistry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeDef {
    /// A primitive type (int, float, str, bool, null).
    Primitive(PrimitiveType),
    /// An alias to another type by its TypeId.
    Alias(TypeId),
    /// A record type with named fields.
    Record(Vec<FieldDef>),
    /// A union type with named variants.
    Union(Vec<VariantDef>),
    /// A function type: (param_types...) -> return_type.
    Func(Vec<TypeId>, TypeId),
    /// A concrete instantiation of a generic type.
    /// e.g., `Option<int>` where base is the TypeId of `Option` and args is `[0]` (Int).
    GenericInstance { base: TypeId, args: Vec<TypeId> },
}
