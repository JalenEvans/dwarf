//! MCP ServerHandler implementation for the Dwarf compiler.
//!
//! `DwarfMcpHandler` receives JSON-RPC method calls from an MCP client
//! (e.g. an LLM-powered IDE) and translates them into compiler operations
//! via `dwarf_lib` and `dwarf_gen`.

use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::*;
use std::sync::Arc;

/// The MCP server handler for the Dwarf compiler.
///
/// This struct implements `rust_mcp_sdk::mcp_server::ServerHandler`.
///
/// # Green phase
///
/// The implementation of `ServerHandler` provides:
/// - `handle_initialize_request` — returns server metadata (name, version, capabilities)
/// - Default implementations for all other handlers (returns "method not found")
///
/// # Future phases
///
/// Future phases will add:
/// - A `compile` tool that accepts Dwarf source, options, and a target
/// - Resources such as `dwarf://schema/types` for type introspection
/// - Prompts for common compilation workflows
pub struct DwarfMcpHandler;

impl DwarfMcpHandler {
    /// Create a new handler with default configuration.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DwarfMcpHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ServerHandler for DwarfMcpHandler {
    /// Handle the MCP initialize handshake.
    ///
    /// Returns server metadata including protocol version, capabilities,
    /// and server info (name + version).
    async fn handle_initialize_request(
        &self,
        _params: InitializeRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        Ok(InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                resources: Some(ServerCapabilitiesResources {
                    subscribe: Some(false),
                    list_changed: None,
                }),
                tools: Some(ServerCapabilitiesTools {
                    list_changed: None,
                }),
                prompts: Some(ServerCapabilitiesPrompts {
                    list_changed: None,
                }),
                completions: None,
                experimental: None,
                logging: None,
                tasks: None,
            },
            server_info: Implementation {
                name: "dwarf-mcp".to_string(),
                version: "0.1.0".to_string(),
                description: Some("Standalone MCP server for the Dwarf compiler".to_string()),
                title: Some("Dwarf MCP Server".to_string()),
                icons: vec![],
                website_url: None,
            },
            instructions: None,
            meta: None,
        })
    }
}
