//! Standalone MCP (Model Context Protocol) server for the Dwarf compiler.
//!
//! This crate exposes Dwarf's compiler capabilities as MCP tools, resources,
//! and prompts, enabling LLM-powered IDEs and agents to compile, analyze, and
//! transform Dwarf source code through a standardised protocol.
//!
//! # Architecture
//!
//! The server uses a standard MCP stdio transport: JSON-RPC messages are
//! received on stdin and responses are written to stdout.  The `DwarfMcpHandler`
//! struct implements `rust_mcp_sdk::mcp_server::ServerHandler` and dispatches
//! each MCP method to the appropriate Dwarf compiler pipeline.

pub mod handler;
