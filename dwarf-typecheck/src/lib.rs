//! Type-checking crate for the Dwarf compiler.
//! Implements the type system: TypeRegistry, structural compatibility,
//! local type inference, generics, and the TypeCheckPass.

pub mod compat;
pub mod error;
pub mod infer;
pub mod pass;
pub mod passes;
pub mod registry;
pub mod resolve;
pub mod types;
