//! Backend emitter framework for the Dwarf compiler.
//!
//! This crate defines the [`EmitterBackend`] trait and related error types
//! that concrete backends (JavaScript, WebAssembly, etc.) implement to
//! produce code from LIR declarations.

pub mod backend;
pub mod error;
pub mod imports;
pub mod naming;
pub mod types;
