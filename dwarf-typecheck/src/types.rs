//! Type definitions and data structures for the Dwarf type system.

/// A handle/index into the TypeRegistry.
pub type TypeId = usize;

/// TypeId for the built-in `Int` primitive type.
pub const INT_TYPE_ID: TypeId = 0;
/// TypeId for the built-in `Float` primitive type.
pub const FLOAT_TYPE_ID: TypeId = 1;
/// TypeId for the built-in `Str` primitive type.
pub const STR_TYPE_ID: TypeId = 2;
/// TypeId for the built-in `Bool` primitive type.
pub const BOOL_TYPE_ID: TypeId = 3;
/// TypeId for the built-in `Null` primitive type.
pub const NULL_TYPE_ID: TypeId = 4;

/// TypeId for the built-in `Option` generic type constructor.
pub const OPTION_TYPE_ID: TypeId = 5;
/// TypeId for the built-in `Result` generic type constructor.
pub const RESULT_TYPE_ID: TypeId = 6;
/// TypeId for the built-in `List` generic type constructor.
pub const LIST_TYPE_ID: TypeId = 7;
/// TypeId for the built-in `Map` generic type constructor.
pub const MAP_TYPE_ID: TypeId = 8;
/// TypeId for the built-in `Any` type (compatible with all types).
/// This is a virtual type not registered in the TypeRegistry.
/// Uses a high value to avoid collision with dynamically registered types.
pub const ANY_TYPE_ID: TypeId = TypeId::MAX - 1;

/// Sentinel TypeId representing the bottom/never type (produced by `throw`).
pub const NEVER_TYPE_ID: TypeId = TypeId::MAX;

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

/// A constraint on a refined type.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RefConstraint {
    /// Range constraint: min..max (inclusive)
    Range { min: i64, max: i64 },
    /// Non-empty constraint: string must not be empty
    NonEmpty,
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
    /// A built-in generic type constructor (e.g., Option, Result, List, Map).
    /// These serve as the base for GenericInstance types.
    BuiltinGeneric { name: String },
    /// A refined type: a base type with a constraint.
    /// e.g., `Int(0..100)` where base is the TypeId of `Int` and constraint is `Range { min: 0, max: 100 }`.
    Refined {
        base: TypeId,
        constraint: RefConstraint,
    },
}
