//! LSP (Language Server Protocol) server for the Dwarf compiler.
//!
//! This crate exposes Dwarf's compiler capabilities as an LSP server,
//! providing IDE features such as diagnostics, completions, hover info,
//! go-to-definition, and more for Dwarf source code.
//!
//! # Architecture
//!
//! The server communicates over JSON-RPC via stdio transport, using the
//! `lsp-server` crate for the protocol framework and `lsp-types` for the
//! type definitions. The `DwarfLspHandler` struct dispatches each LSP
//! method to the appropriate compiler pipeline.

pub mod handler;
