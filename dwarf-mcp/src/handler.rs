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
/// - `handle_list_resources_request` — returns all language resources
/// - `handle_read_resource_request` — returns content for a specific resource URI
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

// ---------------------------------------------------------------------------
// Resource definitions
// ---------------------------------------------------------------------------

/// Build the list of all known Dwarf language resources.
fn all_resources() -> Vec<Resource> {
    vec![
        Resource {
            uri: "dwarf://syntax/overview".to_string(),
            name: "Dwarf Syntax Overview".to_string(),
            title: Some("Dwarf Syntax Overview".to_string()),
            description: Some("Language philosophy, key design decisions, and syntax fundamentals.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/functions".to_string(),
            name: "Functions".to_string(),
            title: Some("Functions".to_string()),
            description: Some("Function declarations, parameters, return types, and the effects system (pure/io/async).".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/types".to_string(),
            name: "Types".to_string(),
            title: Some("Types".to_string()),
            description: Some("Records, unions, generics, type aliases, and refinement types like Int(0..100).".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/modules".to_string(),
            name: "Modules".to_string(),
            title: Some("Modules".to_string()),
            description: Some("Module declarations, imports via `import \"x\"`, and path resolution.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/expressions".to_string(),
            name: "Expressions".to_string(),
            title: Some("Expressions".to_string()),
            description: Some("If/match/block/loop — everything is an expression. Pipe operator `|>`.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/testing".to_string(),
            name: "Testing".to_string(),
            title: Some("Testing".to_string()),
            description: Some("@test decorator, assertions, property-based testing with `forAll`.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://stdlib/reference".to_string(),
            name: "Standard Library".to_string(),
            title: Some("Standard Library".to_string()),
            description: Some("Built-in types and common functions available in every Dwarf program.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/basic".to_string(),
            name: "Basic Examples".to_string(),
            title: Some("Basic Examples".to_string()),
            description: Some("Hello world, arithmetic, string manipulation, and I/O examples.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/types".to_string(),
            name: "Type Examples".to_string(),
            title: Some("Type Examples".to_string()),
            description: Some("Record, union, and generic type patterns with real Dwarf code.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/testing".to_string(),
            name: "Testing Examples".to_string(),
            title: Some("Testing Examples".to_string()),
            description: Some("Unit tests, property-based tests, and edge-case patterns.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
    ]
}

/// Return the markdown content for a known resource URI, or `None` if unknown.
fn resource_content(uri: &str) -> Option<&'static str> {
    match uri {
        "dwarf://syntax/overview" => Some(include_str!("../resources/syntax/overview.md")),
        "dwarf://syntax/functions" => Some(include_str!("../resources/syntax/functions.md")),
        "dwarf://syntax/types" => Some(include_str!("../resources/syntax/types.md")),
        "dwarf://syntax/modules" => Some(include_str!("../resources/syntax/modules.md")),
        "dwarf://syntax/expressions" => Some(include_str!("../resources/syntax/expressions.md")),
        "dwarf://syntax/testing" => Some(include_str!("../resources/syntax/testing.md")),
        "dwarf://stdlib/reference" => Some(include_str!("../resources/stdlib/reference.md")),
        "dwarf://examples/basic" => Some(include_str!("../resources/examples/basic.md")),
        "dwarf://examples/types" => Some(include_str!("../resources/examples/types.md")),
        "dwarf://examples/testing" => Some(include_str!("../resources/examples/testing.md")),
        _ => None,
    }
}

#[async_trait]
impl ServerHandler for DwarfMcpHandler {
    /// Handle the MCP initialize handshake.
    ///
    /// Returns server metadata including protocol version, capabilities,
    /// and server info (name + version).  Stores the client details on the
    /// runtime so that `is_initialized()` returns `true` after the handshake.
    async fn handle_initialize_request(
        &self,
        params: InitializeRequestParams,
        runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        // Persist client details so the runtime knows the session is initialised.
        // Without this, the SDK's error-handling path (which checks
        // `is_initialized()`) will drop `RpcError` responses instead of
        // forwarding them to the client.
        runtime
            .set_client_details(params)
            .await
            .map_err(|e| RpcError::internal_error().with_message(e.to_string()))?;

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

    /// Handle `resources/list` — return all known Dwarf language resources.
    async fn handle_list_resources_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            resources: all_resources(),
            next_cursor: None,
            meta: None,
        })
    }

    /// Handle `resources/read` — return the markdown content for a resource URI.
    ///
    /// Returns a JSON-RPC error with code `-32602` when the URI is unknown.
    async fn handle_read_resource_request(
        &self,
        params: ReadResourceRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        match resource_content(&params.uri) {
            Some(text) => Ok(ReadResourceResult {
                contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                    uri: params.uri,
                    mime_type: Some("text/markdown".to_string()),
                    text: text.to_string(),
                    meta: None,
                })],
                meta: None,
            }),
            None => Err(RpcError {
                code: -32602,
                message: format!("Resource not found: {}", params.uri),
                data: None,
            }),
        }
    }
}
