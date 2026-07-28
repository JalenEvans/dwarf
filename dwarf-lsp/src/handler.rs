//! LSP request/notification handler for the Dwarf language server.
//!
//! `DwarfLspHandler` receives JSON-RPC messages from an LSP client
//! (e.g. VS Code, Neovim) and dispatches them to the appropriate
//! compiler operations via `dwarf_lib`.

use crossbeam_channel::Sender;
use dwarf_parser::pass::ParsePass;
use dwarf_syntax::diagnostic::byte_to_line_col;
use dwarf_typecheck::pass::TypeCheckPass;
use lsp_server::{Message, Notification, Request, Response};
use lsp_types::notification::{Notification as LspNotification, PublishDiagnostics};
use lsp_types::*;
use std::collections::HashMap;

/// The LSP server handler for the Dwarf compiler.
///
/// This struct manages client capabilities, open document state, and the
/// outbound message channel for publishing diagnostics and other LSP
/// notifications.
pub struct DwarfLspHandler {
    /// Capabilities advertised by the LSP client during initialization.
    client_capabilities: ClientCapabilities,
    /// Outbound channel for sending LSP notifications to the client.
    sender: Sender<Message>,
    /// Open documents tracked by the server (uri → source text).
    open_documents: HashMap<Uri, String>,
    /// Root URI provided by the client during initialization.
    root_uri: Option<Uri>,
}

impl DwarfLspHandler {
    /// Create a new handler with the given client capabilities and outbound
    /// message channel.
    pub fn new(client_capabilities: ClientCapabilities, sender: Sender<Message>) -> Self {
        Self {
            client_capabilities,
            sender,
            open_documents: HashMap::new(),
            root_uri: None,
        }
    }

    /// Handle an incoming LSP request and return an optional response.
    ///
    /// Returns `Ok(None)` for requests that are handled asynchronously
    /// or don't require a response. Returns `Err` for unhandled methods.
    pub fn handle_request(&mut self, req: &Request) -> Result<Option<Response>, String> {
        match req.method.as_str() {
            "initialize" => {
                let params: InitializeParams = serde_json::from_value(req.params.clone())
                    .map_err(|e| format!("Invalid initialize params: {}", e))?;
                self.client_capabilities = params.capabilities;
                #[allow(deprecated)]
                {
                    self.root_uri = params.root_uri;
                }

                let server_capabilities = ServerCapabilities {
                    text_document_sync: Some(TextDocumentSyncCapability::Kind(
                        TextDocumentSyncKind::FULL,
                    )),
                    ..Default::default()
                };
                let result = InitializeResult {
                    capabilities: server_capabilities,
                    server_info: None,
                };
                Ok(Some(Response::new_ok(req.id.clone(), result)))
            }
            "shutdown" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::Value::Null,
            ))),
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
                if let Ok(params) =
                    serde_json::from_value::<DidOpenTextDocumentParams>(notif.params.clone())
                {
                    let uri = params.text_document.uri;
                    let text = params.text_document.text;
                    self.open_documents.insert(uri.clone(), text.clone());
                    self.publish_diagnostics(&uri, &text);
                }
            }
            "textDocument/didChange" => {
                if let Ok(params) =
                    serde_json::from_value::<DidChangeTextDocumentParams>(notif.params.clone())
                {
                    let uri = params.text_document.uri;
                    // Full document sync: the last content change carries the
                    // complete updated source text.
                    if let Some(change) = params.content_changes.into_iter().last() {
                        let text = change.text;
                        self.open_documents.insert(uri.clone(), text.clone());
                        self.publish_diagnostics(&uri, &text);
                    }
                }
            }
            "textDocument/didSave" => {
                // Document saved — optional re-validation.
                tracing::info!("Document saved");
            }
            "textDocument/didClose" => {
                if let Ok(params) =
                    serde_json::from_value::<DidCloseTextDocumentParams>(notif.params.clone())
                {
                    let uri = params.text_document.uri;
                    self.open_documents.remove(&uri);
                    self.clear_diagnostics(&uri);
                }
            }
            _ => {
                tracing::debug!("Unhandled notification: {}", notif.method);
            }
        }
    }

    /// Parse the document text and publish the resulting diagnostics to the
    /// client.
    fn publish_diagnostics(&self, uri: &Uri, text: &str) {
        let parse_pass = ParsePass;
        let diagnostics = match parse_pass.parse(text) {
            Ok((decls, errors)) => {
                let mut diagnostics: Vec<Diagnostic> = errors
                    .iter()
                    .map(|err| parse_error_to_diagnostic(text, err))
                    .collect();

                let type_pass = TypeCheckPass::new();
                let (_, type_errors) = type_pass.check(&decls);
                diagnostics.extend(
                    type_errors
                        .iter()
                        .map(|err| typecheck_error_to_diagnostic(text, err)),
                );

                diagnostics
            }
            Err(message) => vec![Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("dwarf".to_string()),
                message,
                related_information: None,
                tags: None,
                data: None,
            }],
        };
        self.send_diagnostics(uri, diagnostics);
    }

    /// Clear diagnostics for a closed document by publishing an empty
    /// diagnostic list.
    fn clear_diagnostics(&self, uri: &Uri) {
        self.send_diagnostics(uri, vec![]);
    }

    /// Send a `textDocument/publishDiagnostics` notification.
    fn send_diagnostics(&self, uri: &Uri, diagnostics: Vec<Diagnostic>) {
        let params = PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: None,
        };
        let notif = Notification::new(
            PublishDiagnostics::METHOD.to_string(),
            serde_json::to_value(params).expect("diagnostics params serialize"),
        );
        let _ = self.sender.send(notif.into());
    }
}

/// Convert a Dwarf parser error into an LSP diagnostic.
fn parse_error_to_diagnostic(source: &str, err: &dwarf_parser::ParseError) -> Diagnostic {
    let (start_line, start_col) = byte_to_line_col(source, err.span.start).unwrap_or((1, 1));
    let (end_line, end_col) =
        byte_to_line_col(source, err.span.end).unwrap_or((start_line, start_col));

    let start = Position {
        line: start_line.saturating_sub(1) as u32,
        character: start_col.saturating_sub(1) as u32,
    };
    let end = Position {
        line: end_line.saturating_sub(1) as u32,
        character: end_col.saturating_sub(1) as u32,
    };

    Diagnostic {
        range: Range { start, end },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(err.code.to_string())),
        code_description: None,
        source: Some("dwarf".to_string()),
        message: err.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert a Dwarf type-checking error into an LSP diagnostic.
fn typecheck_error_to_diagnostic(
    source: &str,
    err: &dwarf_typecheck::error::TypeCheckError,
) -> Diagnostic {
    let (start_line, start_col) = byte_to_line_col(source, err.span.start).unwrap_or((1, 1));
    let (end_line, end_col) =
        byte_to_line_col(source, err.span.end).unwrap_or((start_line, start_col));

    let start = Position {
        line: start_line.saturating_sub(1) as u32,
        character: start_col.saturating_sub(1) as u32,
    };
    let end = Position {
        line: end_line.saturating_sub(1) as u32,
        character: end_col.saturating_sub(1) as u32,
    };

    Diagnostic {
        range: Range { start, end },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(err.code.to_string())),
        code_description: None,
        source: Some("dwarf".to_string()),
        message: err.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}
