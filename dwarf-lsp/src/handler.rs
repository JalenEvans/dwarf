//! LSP request/notification handler for the Dwarf language server.
//!
//! `DwarfLspHandler` receives JSON-RPC messages from an LSP client
//! (e.g. VS Code, Neovim) and dispatches them to the appropriate
//! compiler operations via `dwarf_lib`.

use lsp_server::{Notification, Request, Response};
use lsp_types::*;

/// The LSP server handler for the Dwarf compiler.
///
/// This struct manages client capabilities and holds a reference to
/// compiler state for fulfilling LSP requests such as diagnostics,
/// completions, hover information, and go-to-definition.
pub struct DwarfLspHandler {
    /// Capabilities advertised by the LSP client during initialization.
    client_capabilities: ClientCapabilities,
}

impl DwarfLspHandler {
    /// Create a new handler with the given client capabilities.
    pub fn new(client_capabilities: ClientCapabilities) -> Self {
        Self {
            client_capabilities,
        }
    }

    /// Handle an incoming LSP request and return an optional response.
    ///
    /// Returns `Ok(None)` for requests that are handled asynchronously
    /// or don't require a response. Returns `Err` for unhandled methods.
    pub fn handle_request(&mut self, req: &Request) -> Result<Option<Response>, String> {
        match req.method.as_str() {
            // Known methods we don't implement yet — return empty/default results.
            "textDocument/completion" => Ok(Some(Response::new_ok(
                req.id.clone(),
                CompletionResponse::Array(vec![]),
            ))),
            "textDocument/hover" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            "textDocument/definition" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            "textDocument/references" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            "textDocument/documentSymbol" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            "textDocument/formatting" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            _ => Err(format!("Unhandled request method: {}", req.method)),
        }
    }

    /// Handle an incoming LSP notification.
    ///
    /// Notifications are fire-and-forget messages from the client. Common
    /// notifications include `textDocument/didOpen`, `textDocument/didChange`,
    /// and `textDocument/didSave`.
    pub fn handle_notification(&mut self, notif: &Notification) {
        match notif.method.as_str() {
            "textDocument/didOpen" => {
                // Document opened — parse and publish diagnostics.
                tracing::info!("Document opened");
            }
            "textDocument/didChange" => {
                // Document changed — re-parse and publish diagnostics.
                tracing::info!("Document changed");
            }
            "textDocument/didSave" => {
                // Document saved — optional re-validation.
                tracing::info!("Document saved");
            }
            "textDocument/didClose" => {
                // Document closed — clean up state.
                tracing::info!("Document closed");
            }
            _ => {
                tracing::debug!("Unhandled notification: {}", notif.method);
            }
        }
    }
}
