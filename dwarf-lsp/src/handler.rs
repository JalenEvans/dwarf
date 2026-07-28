//! LSP request/notification handler for the Dwarf language server.
//!
//! `DwarfLspHandler` receives JSON-RPC messages from an LSP client
//! (e.g. VS Code, Neovim) and dispatches them to the appropriate
//! compiler operations via `dwarf_lib`.

use crossbeam_channel::Sender;
use dwarf_parser::pass::ParsePass;
use dwarf_syntax::diagnostic::byte_to_line_col;
use dwarf_syntax::hir::{Decl, Expr, Pat, Stmt, Type as HirType};
use dwarf_syntax::span::Span;
use dwarf_typecheck::infer::{infer_expr, TypeEnv};
use dwarf_typecheck::pass::TypeCheckPass;
use dwarf_typecheck::registry::TypeRegistry;
use dwarf_typecheck::types::{TypeDef, TypeId};
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
                    hover_provider: Some(HoverProviderCapability::Simple(true)),
                    definition_provider: Some(OneOf::Left(true)),
                    document_symbol_provider: Some(OneOf::Left(true)),
                    document_formatting_provider: Some(OneOf::Left(true)),
                    completion_provider: Some(CompletionOptions {
                        resolve_provider: Some(false),
                        ..Default::default()
                    }),
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
            "textDocument/completion" => self.handle_completion(req),
            "textDocument/hover" => self.handle_hover(req),
            "textDocument/definition" => self.handle_definition(req),
            "textDocument/references" => Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::json!(null),
            ))),
            "textDocument/documentSymbol" => self.handle_document_symbol(req),
            "textDocument/formatting" => self.handle_formatting(req),
            _ => Err(format!("Unhandled request method: {}", req.method)),
        }
    }

    /// Handle a `textDocument/hover` request.
    fn handle_hover(&mut self, req: &Request) -> Result<Option<Response>, String> {
        let params: HoverParams = serde_json::from_value(req.params.clone())
            .map_err(|e| format!("Invalid hover params: {}", e))?;
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = self.get_document_text(&uri).ok_or("Document not found")?;
        let offset = Self::position_to_offset(&source, position);

        let parse_pass = ParsePass;
        let (decls, _) = parse_pass
            .parse(&source)
            .map_err(|e| format!("Parse error: {}", e))?;

        let type_pass = TypeCheckPass::new();
        let mut registry = type_pass.check(&decls).0;

        // Try to find an expression at the requested position and infer its type.
        if let Some((func, expr)) = find_expr_at_offset(&decls, offset) {
            let mut env = TypeEnv::new();
            if let Decl::Function { params, .. } = func {
                for param in params {
                    if let Some(type_id) = param.type_.as_ref().and_then(resolve_hir_type_simple) {
                        env.bind(param.name.clone(), type_id);
                    }
                }
            }
            if let Ok(type_id) = infer_expr(expr, &env, &mut registry) {
                let contents = format!("type: {}", type_id_to_name(&registry, type_id));
                return Ok(Some(Response::new_ok(
                    req.id.clone(),
                    Hover {
                        contents: HoverContents::Scalar(MarkedString::String(contents)),
                        range: None,
                    },
                )));
            }
        }

        // If no expression was found, check whether the cursor is on a
        // function parameter in the signature.
        if let Some(type_name) = find_param_hover(&decls, &source, offset) {
            let contents = format!("type: {type_name}");
            return Ok(Some(Response::new_ok(
                req.id.clone(),
                Hover {
                    contents: HoverContents::Scalar(MarkedString::String(contents)),
                    range: None,
                },
            )));
        }

        // Fallback for positions inside a function body.
        let contents = format!(
            "expression at line {}, column {}",
            position.line, position.character
        );
        Ok(Some(Response::new_ok(
            req.id.clone(),
            Hover {
                contents: HoverContents::Scalar(MarkedString::String(contents)),
                range: None,
            },
        )))
    }

    /// Handle a `textDocument/completion` request.
    fn handle_completion(&mut self, req: &Request) -> Result<Option<Response>, String> {
        let params: CompletionParams = serde_json::from_value(req.params.clone())
            .map_err(|e| format!("Invalid completion params: {}", e))?;
        let uri = params.text_document_position.text_document.uri;
        let source = self.get_document_text(&uri).ok_or("Document not found")?;

        let mut items: Vec<CompletionItem> = vec![
            ("fn", CompletionItemKind::KEYWORD),
            ("let", CompletionItemKind::KEYWORD),
            ("if", CompletionItemKind::KEYWORD),
            ("else", CompletionItemKind::KEYWORD),
            ("return", CompletionItemKind::KEYWORD),
            ("match", CompletionItemKind::KEYWORD),
            ("for", CompletionItemKind::KEYWORD),
            ("while", CompletionItemKind::KEYWORD),
            ("struct", CompletionItemKind::KEYWORD),
            ("enum", CompletionItemKind::KEYWORD),
            ("import", CompletionItemKind::KEYWORD),
            ("pub", CompletionItemKind::KEYWORD),
            ("true", CompletionItemKind::KEYWORD),
            ("false", CompletionItemKind::KEYWORD),
            ("null", CompletionItemKind::KEYWORD),
        ]
        .into_iter()
        .map(|(label, kind)| CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            ..Default::default()
        })
        .collect();

        // Add symbols defined in the current document.
        let parse_pass = ParsePass;
        if let Ok((decls, _)) = parse_pass.parse(&source) {
            for decl in &decls {
                match decl {
                    Decl::Function { name, .. } => items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        ..Default::default()
                    }),
                    Decl::RecordDef { name, .. } => items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        ..Default::default()
                    }),
                    Decl::UnionDef { name, .. } => items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::ENUM),
                        ..Default::default()
                    }),
                    Decl::TypeDef { name, .. } => items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        ..Default::default()
                    }),
                    _ => {}
                }
            }
        }

        Ok(Some(Response::new_ok(
            req.id.clone(),
            CompletionResponse::Array(items),
        )))
    }

    /// Handle a `textDocument/definition` request.
    fn handle_definition(&mut self, req: &Request) -> Result<Option<Response>, String> {
        let params: GotoDefinitionParams = serde_json::from_value(req.params.clone())
            .map_err(|e| format!("Invalid definition params: {}", e))?;
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = self.get_document_text(&uri).ok_or("Document not found")?;
        let offset = Self::position_to_offset(&source, position);

        let parse_pass = ParsePass;
        let (decls, _) = parse_pass
            .parse(&source)
            .map_err(|e| format!("Parse error: {}", e))?;

        let ident = identifier_at_offset(&source, offset);
        if ident.is_empty() {
            return Ok(Some(Response::new_ok(
                req.id.clone(),
                serde_json::Value::Null,
            )));
        }

        // Top-level declaration (function, record, union, type alias).
        if let Some(decl) = find_decl_by_name(&decls, &ident) {
            let range = span_to_range(&source, decl_span(decl));
            return Ok(Some(Response::new_ok(
                req.id.clone(),
                GotoDefinitionResponse::Scalar(Location { uri, range }),
            )));
        }

        // Local `let` binding inside a function body.
        if let Some(span) = find_local_var_decl(&decls, &ident) {
            let range = span_to_range(&source, span);
            return Ok(Some(Response::new_ok(
                req.id.clone(),
                GotoDefinitionResponse::Scalar(Location { uri, range }),
            )));
        }

        Ok(Some(Response::new_ok(
            req.id.clone(),
            serde_json::Value::Null,
        )))
    }

    /// Handle a `textDocument/documentSymbol` request.
    fn handle_document_symbol(&mut self, req: &Request) -> Result<Option<Response>, String> {
        let params: DocumentSymbolParams = serde_json::from_value(req.params.clone())
            .map_err(|e| format!("Invalid document symbol params: {}", e))?;
        let uri = params.text_document.uri;
        let source = self.get_document_text(&uri).ok_or("Document not found")?;

        let parse_pass = ParsePass;
        let (decls, _) = parse_pass
            .parse(&source)
            .map_err(|e| format!("Parse error: {}", e))?;

        let mut symbols: Vec<SymbolInformation> = Vec::new();
        for decl in &decls {
            let (name, kind) = match decl {
                Decl::Function { name, .. } => (name.clone(), SymbolKind::FUNCTION),
                Decl::RecordDef { name, .. } => (name.clone(), SymbolKind::STRUCT),
                Decl::UnionDef { name, .. } => (name.clone(), SymbolKind::ENUM),
                Decl::TypeDef { name, .. } => (name.clone(), SymbolKind::STRUCT),
                Decl::Import { .. } | Decl::Decorator { .. } => continue,
            };
            let range = span_to_range(&source, decl_span(decl));
            symbols.push(SymbolInformation {
                name,
                kind,
                location: Location {
                    uri: uri.clone(),
                    range,
                },
                container_name: None,
                #[allow(deprecated)]
                deprecated: None,
                tags: None,
            });
        }

        Ok(Some(Response::new_ok(
            req.id.clone(),
            DocumentSymbolResponse::Flat(symbols),
        )))
    }

    /// Handle a `textDocument/formatting` request.
    fn handle_formatting(&self, req: &Request) -> Result<Option<Response>, String> {
        let params: DocumentFormattingParams = serde_json::from_value(req.params.clone())
            .map_err(|e| format!("Invalid formatting params: {e}"))?;
        let uri = params.text_document.uri;
        let source = self.get_document_text(&uri).ok_or("Document not found")?;

        // Validate the document via a parse pass. When parsing succeeds we
        // run a lightweight formatting pass over the source so that the
        // returned edit contains cleaner spacing.
        let formatted = match ParsePass.parse(&source) {
            Ok(_) => format_source(&source),
            Err(_) => source,
        };

        let range = Self::whole_document_range(&formatted);
        Ok(Some(Response::new_ok(
            req.id.clone(),
            vec![TextEdit {
                range,
                new_text: formatted,
            }],
        )))
    }

    /// Get the source text of an open document by URI.
    fn get_document_text(&self, uri: &Uri) -> Option<String> {
        self.open_documents.get(uri).cloned()
    }

    /// Build a range that covers the entire document text.
    fn whole_document_range(source: &str) -> Range {
        let lines: Vec<&str> = source.split('\n').collect();
        let last_line = lines.len().saturating_sub(1);
        let last_len = lines.last().map(|l| l.len()).unwrap_or(0);
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: last_line as u32,
                character: last_len as u32,
            },
        }
    }

    /// Convert an LSP position to a byte offset into the source text.
    ///
    /// This assumes ASCII source text; for non-ASCII input the offset will
    /// correspond to the byte index of the requested character position.
    fn position_to_offset(source: &str, position: Position) -> usize {
        let mut offset = 0;
        let mut current_line = 0;
        let bytes = source.as_bytes();

        while current_line < position.line && offset < bytes.len() {
            if bytes[offset] == b'\n' {
                current_line += 1;
            }
            offset += 1;
        }

        let line_start = offset;
        let mut current_char: usize = 0;
        let target_char = position.character as usize;
        while current_char < target_char && offset < bytes.len() {
            if bytes[offset] == b'\n' {
                break;
            }
            offset += 1;
            current_char += 1;
        }

        line_start + current_char.min(target_char)
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
                if let Ok(params) =
                    serde_json::from_value::<DidSaveTextDocumentParams>(notif.params.clone())
                {
                    let uri = params.text_document.uri;
                    if let Some(text) = self.get_document_text(&uri) {
                        self.publish_diagnostics(&uri, &text);
                    }
                }
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

/// Lightweight source formatter that normalizes whitespace around common
/// Dwarf punctuation so the returned text is easier to read.
fn format_source(source: &str) -> String {
    let mut result = source.to_string();

    // Collapse runs of whitespace into a single space.
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    // Add spacing around common punctuation.
    result = result.replace(',', ", ");
    result = result.replace(':', ": ");
    result = result.replace("->", " -> ");
    result = result.replace('{', " { ");
    result = result.replace('}', " } ");
    result = result.replace('+', " + ");
    result = result.replace('=', " = ");

    // Collapse any whitespace introduced by the replacements.
    result.split_whitespace().collect::<Vec<_>>().join(" ")
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

// ---------------------------------------------------------------------------
// Hover helpers
// ---------------------------------------------------------------------------

/// Find the innermost expression (and its containing function declaration)
/// whose span contains the given byte offset.
fn find_expr_at_offset(decls: &[Decl], offset: usize) -> Option<(&Decl, &Expr)> {
    for decl in decls {
        if let Decl::Function { body, .. } = decl {
            if let Some(expr) = find_expr_in_expr(body, offset) {
                return Some((decl, expr));
            }
        }
    }
    None
}

/// Recurse into `expr` and return the innermost expression whose span contains
/// `offset`, or `None` if the offset is outside the expression.
fn find_expr_in_expr(expr: &Expr, offset: usize) -> Option<&Expr> {
    let span = expr.span();
    if offset < span.start || offset >= span.end {
        return None;
    }

    let child = match expr {
        Expr::Binary { lhs, rhs, .. } => {
            find_expr_in_expr(lhs, offset).or_else(|| find_expr_in_expr(rhs, offset))
        }
        Expr::Unary { expr: inner, .. } => find_expr_in_expr(inner, offset),
        Expr::Call { func, args, .. } => find_expr_in_expr(func, offset)
            .or_else(|| args.iter().find_map(|arg| find_expr_in_expr(arg, offset))),
        Expr::Member { obj, .. } => find_expr_in_expr(obj, offset),
        Expr::If {
            cond, then, else_, ..
        } => find_expr_in_expr(cond, offset)
            .or_else(|| find_expr_in_expr(then, offset))
            .or_else(|| else_.as_deref().and_then(|e| find_expr_in_expr(e, offset))),
        Expr::Match { expr, arms, .. } => find_expr_in_expr(expr, offset).or_else(|| {
            arms.iter().find_map(|arm| {
                arm.guard
                    .as_ref()
                    .and_then(|g| find_expr_in_expr(g, offset))
                    .or_else(|| find_expr_in_expr(&arm.body, offset))
            })
        }),
        Expr::Block { stmts, .. } => stmts.iter().find_map(|stmt| match stmt {
            Stmt::Expr(e) => find_expr_in_expr(e, offset),
            Stmt::Let(_, e) => find_expr_in_expr(e, offset),
        }),
        Expr::Pipe { lhs, rhs, .. } => {
            find_expr_in_expr(lhs, offset).or_else(|| find_expr_in_expr(rhs, offset))
        }
        Expr::Propagate { expr: inner, .. } => find_expr_in_expr(inner, offset),
        Expr::Try {
            body,
            guard,
            handler,
            ..
        } => find_expr_in_expr(body, offset)
            .or_else(|| guard.as_deref().and_then(|g| find_expr_in_expr(g, offset)))
            .or_else(|| find_expr_in_expr(handler, offset)),
        Expr::Throw { expr: inner, .. } => find_expr_in_expr(inner, offset),
        Expr::For { iterable, body, .. } => {
            find_expr_in_expr(iterable, offset).or_else(|| find_expr_in_expr(body, offset))
        }
        Expr::Assign { target, value, .. } => {
            find_expr_in_expr(target, offset).or_else(|| find_expr_in_expr(value, offset))
        }
        Expr::Lambda { body, .. } => find_expr_in_expr(body, offset),
        Expr::Record { fields, .. } => fields
            .iter()
            .find_map(|(_, e)| find_expr_in_expr(e, offset)),
        Expr::Variant { arg, .. } => arg.as_deref().and_then(|a| find_expr_in_expr(a, offset)),
        Expr::Array { items, .. } => items
            .iter()
            .find_map(|item| find_expr_in_expr(item, offset)),
        Expr::ForAll { property, .. } => find_expr_in_expr(property, offset),
        Expr::AssertConsistent { expr: inner, .. } => find_expr_in_expr(inner, offset),
        _ => None,
    };

    child.or(Some(expr))
}

/// If the cursor is on a function parameter name inside a function signature,
/// return the parameter's type as a display string.
fn find_param_hover(decls: &[Decl], source: &str, offset: usize) -> Option<String> {
    for decl in decls {
        if let Decl::Function {
            params, body, span, ..
        } = decl
        {
            if offset >= span.start && offset < span.end && offset < body.span().start {
                let ident = identifier_at_offset(source, offset);
                for param in params {
                    if param.name == ident {
                        return param.type_.as_ref().map(hir_type_to_string);
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Definition / document-symbol helpers
// ---------------------------------------------------------------------------

/// Convert a HIR byte span to an LSP zero-indexed range.
fn span_to_range(source: &str, span: Span) -> Range {
    let (start_line, start_col) = byte_to_line_col(source, span.start).unwrap_or((1, 1));
    let (end_line, end_col) = byte_to_line_col(source, span.end).unwrap_or((start_line, start_col));

    Range {
        start: Position {
            line: start_line.saturating_sub(1) as u32,
            character: start_col.saturating_sub(1) as u32,
        },
        end: Position {
            line: end_line.saturating_sub(1) as u32,
            character: end_col.saturating_sub(1) as u32,
        },
    }
}

/// Return the source span for a top-level declaration.
fn decl_span(decl: &Decl) -> Span {
    match decl {
        Decl::Import { span, .. } => *span,
        Decl::Function { span, .. } => *span,
        Decl::TypeDef { span, .. } => *span,
        Decl::RecordDef { span, .. } => *span,
        Decl::UnionDef { span, .. } => *span,
        Decl::Decorator { span, .. } => *span,
    }
}

/// Find a top-level declaration whose name matches `ident`.
fn find_decl_by_name<'a>(decls: &'a [Decl], ident: &str) -> Option<&'a Decl> {
    decls.iter().find(|decl| {
        let name_matches = |decl: &Decl| match decl {
            Decl::Function { name, .. } => name == ident,
            Decl::RecordDef { name, .. } => name == ident,
            Decl::UnionDef { name, .. } => name == ident,
            Decl::TypeDef { name, .. } => name == ident,
            _ => false,
        };

        match decl {
            Decl::Decorator { target, .. } => name_matches(target),
            _ => name_matches(decl),
        }
    })
}

/// Find a local `let` binding named `ident` in any function body.
fn find_local_var_decl(decls: &[Decl], ident: &str) -> Option<Span> {
    for decl in decls {
        if let Decl::Function { body, .. } = decl {
            if let Some(span) = find_let_in_expr(body, ident) {
                return Some(span);
            }
        }
    }
    None
}

fn find_let_in_expr(expr: &Expr, ident: &str) -> Option<Span> {
    if let Expr::Block { stmts, .. } = expr {
        for stmt in stmts {
            match stmt {
                Stmt::Let(Pat::Variable(name), init) if name == ident => {
                    return Some(init.span());
                }
                Stmt::Expr(e) => {
                    if let Some(span) = find_let_in_expr(e, ident) {
                        return Some(span);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Extract the identifier (word) at the given byte offset.
fn identifier_at_offset(source: &str, offset: usize) -> String {
    let bytes = source.as_bytes();
    let mut start = offset;
    while start > 0 && is_ident_char(bytes.get(start - 1).copied().unwrap_or(0)) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_ident_char(bytes.get(end).copied().unwrap_or(0)) {
        end += 1;
    }
    source[start..end].to_string()
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Resolve a simple HIR type annotation to a primitive type ID.
fn resolve_hir_type_simple(t: &HirType) -> Option<TypeId> {
    match t {
        HirType::Named(name) => match name.as_str() {
            "int" | "Int" => Some(0),
            "float" | "Float" => Some(1),
            "str" | "Str" | "string" | "String" => Some(2),
            "bool" | "Bool" => Some(3),
            "null" | "Null" => Some(4),
            _ => None,
        },
        _ => None,
    }
}

/// Render a HIR type as a human-readable string.
fn hir_type_to_string(t: &HirType) -> String {
    match t {
        HirType::Named(name) => name.clone(),
        HirType::Generic { base, args } => {
            let args_str = args
                .iter()
                .map(hir_type_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{base}<{args_str}>")
        }
        _ => format!("{t:?}"),
    }
}

/// Convert a type ID into a human-readable type name.
fn type_id_to_name(registry: &TypeRegistry, type_id: TypeId) -> String {
    match type_id {
        0 => "Int".to_string(),
        1 => "Float".to_string(),
        2 => "Str".to_string(),
        3 => "Bool".to_string(),
        4 => "Null".to_string(),
        usize::MAX => "Never".to_string(),
        _ => match registry.get(type_id) {
            Some(TypeDef::Primitive(p)) => format!("{p:?}"),
            Some(TypeDef::BuiltinGeneric { name }) => name.clone(),
            Some(TypeDef::GenericInstance { base, args }) => {
                let base_name = type_id_to_name(registry, *base);
                let args_str = args
                    .iter()
                    .map(|a| type_id_to_name(registry, *a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{base_name}<{args_str}>")
            }
            _ => format!("type#{type_id}"),
        },
    }
}
