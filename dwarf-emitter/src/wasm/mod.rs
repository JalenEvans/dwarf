//! WebAssembly backend — emits a minimal subset of WebAssembly text (WAT)
//! format from LIR declarations.
//!
//! DWARF-129: This backend is in its RED phase. Only the structural scaffolding
//! (the [`WasmBackend`](backend::WasmBackend) type and its [`EmitterBackend`]
//! implementation as an honest stub) exists. The real WAT emission is specified
//! by the tests in [`backend`](backend) and implemented in the Green phase.

pub mod backend;