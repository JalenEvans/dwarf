//! Shared syntax types for the Dwarf compiler.
//! This crate defines the token kinds, AST nodes, and diagnostic types
//! used across the compiler pipeline.

pub mod decorator;
pub mod diagnostic;
pub mod error;
pub mod hir;
pub mod span;
pub mod test_manifest;
pub mod token;
