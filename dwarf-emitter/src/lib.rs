//! Backend emitter framework for the Dwarf compiler.
//!
//! This crate defines the [`EmitterBackend`] trait and related error types
//! that concrete backends (JavaScript, WebAssembly, etc.) implement to
//! produce code from LIR declarations.

pub mod backend;
pub mod debug_backend;
pub mod error;
pub mod format;
pub mod naming;
pub mod sourcemap;
pub mod types;

pub mod ts;
