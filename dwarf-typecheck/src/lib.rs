//! Type-checking crate for the Dwarf compiler.
//! Implements the type system: TypeRegistry, structural compatibility,
//! local type inference, generics, and the TypeCheckPass.

pub mod compat;
pub mod error;
pub mod registry;
pub mod types;
