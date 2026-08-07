//! WebAssembly backend — emits a minimal subset of WebAssembly text (WAT)
//! format from LIR declarations.
//!
//! [`WasmBackend`](backend::WasmBackend) is a fully implemented
//! [`EmitterBackend`](crate::backend::EmitterBackend) that produces WAT text
//! `wat::parse_str` can compile into module bytes for the wasmtime test
//! runner. The supported i32-only subset is documented on the backend type in
//! [`backend`](backend).

pub mod backend;
