//! Integration tests for hover, completion, definition, and document symbol
//! LSP features in dwarf-lsp.
//!
//! These tests exercise textDocument/hover, textDocument/completion,
//! textDocument/definition, and textDocument/documentSymbol through an
//! in-process memory transport. They are expected to fail while the handler
//! returns null for definition and documentSymbol.

use std::str::FromStr;
use std::time::Duration;

use dwarf_lsp::handler::DwarfLspHandler;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidOpenTextDocument, DidSaveTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, Formatting, GotoDefinition, HoverRequest, Initialize,
    Request as _,
};
use lsp_types::{
    ClientCapabilities, CompletionItem, CompletionParams, Diagnostic, DidChangeTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentFormattingParams,
    DocumentSymbolParams, DocumentSymbolResponse, FormattingOptions, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, InitializeParams, Location, LocationLink, Position,
    PublishDiagnosticsParams, SymbolInformation, SymbolKind, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, TextEdit, Uri,
    VersionedTextDocumentIdentifier,
};

const TIMEOUT: Duration = Duration::from_secs(1);

/// Start an in-process LSP server on a memory transport and return the client
/// side of the connection.
fn start_server() -> Connection {
    let (server_conn, client_conn) = Connection::memory();
    std::thread::spawn(move || {
        let mut handler =
            DwarfLspHandler::new(ClientCapabilities::default(), server_conn.sender.clone());
        for msg in &server_conn.receiver {
            match msg {
                Message::Request(req) => {
                    let response = match handler.handle_request(&req) {
                        Ok(Some(resp)) => resp,
                        Ok(None) => continue,
                        Err(e) => {
                            Response::new_err(req.id.clone(), ErrorCode::InternalError as i32, e)
                        }
                    };
                    if server_conn.sender.send(response.into()).is_err() {
                        break;
                    }
                }
                Message::Notification(notif) => {
                    handler.handle_notification(&notif);
                }
                Message::Response(_) => {}
            }
        }
    });
    client_conn
}

fn test_uri() -> Uri {
    Uri::from_str("file:///test/main.kzd").expect("valid test URI")
}

fn send_request<P: serde::Serialize>(conn: &Connection, id: RequestId, method: &str, params: P) {
    let req = Request::new(id, method.to_string(), params);
    conn.sender
        .send(req.into())
        .expect("failed to send request");
}

fn send_notification<P: serde::Serialize>(conn: &Connection, method: &str, params: P) {
    let notif = Notification::new(method.to_string(), params);
    conn.sender
        .send(notif.into())
        .expect("failed to send notification");
}

fn expect_response(conn: &Connection, id: RequestId) -> Response {
    loop {
        match conn.receiver.recv_timeout(TIMEOUT) {
            Ok(Message::Response(resp)) if resp.id == id => return resp,
            Ok(_) => continue,
            Err(_) => panic!("timed out waiting for response to request {id}"),
        }
    }
}

fn initialize(conn: &Connection) {
    let params = InitializeParams {
        process_id: None,
        #[allow(deprecated)]
        root_path: None,
        #[allow(deprecated)]
        root_uri: None,
        capabilities: ClientCapabilities::default(),
        workspace_folders: None,
        client_info: None,
        locale: None,
        trace: None,
        work_done_progress_params: Default::default(),
        initialization_options: None,
    };
    send_request(conn, RequestId::from(1), Initialize::METHOD, params);
    let resp = expect_response(conn, RequestId::from(1));
    assert!(
        resp.error.is_none(),
        "initialize returned error: {:?}",
        resp.error
    );
    assert!(resp.result.is_some(), "initialize response missing result");
}

fn open_document(conn: &Connection, text: &str) {
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: test_uri(),
            language_id: "dwarf".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    send_notification(conn, DidOpenTextDocument::METHOD, params);
}

fn hover_at(conn: &Connection, line: u32, character: u32) -> Response {
    let text_document_position = TextDocumentPositionParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
        position: Position { line, character },
    };
    send_request(
        conn,
        RequestId::from(10),
        HoverRequest::METHOD,
        serde_json::to_value(text_document_position).expect("hover params serialize"),
    );
    expect_response(conn, RequestId::from(10))
}

fn completion_at(conn: &Connection, line: u32, character: u32) -> Response {
    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
            position: Position { line, character },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    send_request(conn, RequestId::from(20), Completion::METHOD, params);
    expect_response(conn, RequestId::from(20))
}

fn definition_at(conn: &Connection, line: u32, character: u32) -> Response {
    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: test_uri() },
            position: Position { line, character },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    send_request(conn, RequestId::from(30), GotoDefinition::METHOD, params);
    expect_response(conn, RequestId::from(30))
}

fn document_symbols(conn: &Connection) -> Response {
    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier { uri: test_uri() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    send_request(
        conn,
        RequestId::from(40),
        DocumentSymbolRequest::METHOD,
        params,
    );
    expect_response(conn, RequestId::from(40))
}

fn expect_notification(conn: &Connection, method: &str) -> Notification {
    loop {
        match conn.receiver.recv_timeout(TIMEOUT) {
            Ok(Message::Notification(notif)) if notif.method == method => return notif,
            Ok(_) => continue,
            Err(_) => panic!("timed out waiting for notification {method}"),
        }
    }
}

fn expect_diagnostics(conn: &Connection) -> Vec<Diagnostic> {
    let notif = expect_notification(conn, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notif.params).expect("publishDiagnostics params should deserialize");
    params.diagnostics
}

fn format_document(conn: &Connection) -> Response {
    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: test_uri() },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: Default::default(),
    };
    send_request(conn, RequestId::from(50), Formatting::METHOD, params);
    expect_response(conn, RequestId::from(50))
}

fn change_document(conn: &Connection, text: &str) {
    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: test_uri(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_string(),
        }],
    };
    send_notification(conn, DidChangeTextDocument::METHOD, params);
}

fn save_document(conn: &Connection) {
    let params = DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: test_uri() },
        text: None,
    };
    send_notification(conn, DidSaveTextDocument::METHOD, params);
}

#[test]
fn hover_shows_type_info() {
    let client = start_server();
    initialize(&client);

    let source = "fn add(a: Int, b: Int) -> Int { a + b }";
    open_document(&client, source);

    // Hover over the parameter `a` at line 0, character 7.
    let resp = hover_at(&client, 0, 7);
    assert!(
        resp.error.is_none(),
        "hover returned error: {:?}",
        resp.error
    );
    assert!(
        resp.result.is_some() && !resp.result.as_ref().unwrap().is_null(),
        "expected hover result with type info, got null"
    );

    let hover: Hover = serde_json::from_value(resp.result.unwrap())
        .expect("hover result should deserialize to Hover");
    let contents = hover_contents_to_string(&hover);
    assert!(
        contents.to_lowercase().contains("int"),
        "expected hover contents to mention Int type, got: {contents}"
    );
}

#[test]
fn hover_on_literal() {
    let client = start_server();
    initialize(&client);

    let source = "fn answer() -> Int { 42 }";
    open_document(&client, source);

    // Hover over the literal `42` at line 0, character 21.
    let resp = hover_at(&client, 0, 21);
    assert!(
        resp.error.is_none(),
        "hover returned error: {:?}",
        resp.error
    );
    assert!(
        resp.result.is_some() && !resp.result.as_ref().unwrap().is_null(),
        "expected hover result for literal, got null"
    );

    let hover: Hover = serde_json::from_value(resp.result.unwrap())
        .expect("hover result should deserialize to Hover");
    let contents = hover_contents_to_string(&hover);
    assert!(
        contents.to_lowercase().contains("int"),
        "expected hover contents to show Int type for literal 42, got: {contents}"
    );
}

#[test]
fn completion_returns_keywords() {
    let client = start_server();
    initialize(&client);

    open_document(&client, "");

    // Request completions at the start of the empty document.
    let resp = completion_at(&client, 0, 0);
    assert!(
        resp.error.is_none(),
        "completion returned error: {:?}",
        resp.error
    );

    let items: Vec<CompletionItem> =
        serde_json::from_value(resp.result.expect("completion response missing result"))
            .expect("completion result should deserialize to a list of items");

    let labels: Vec<String> = items.into_iter().map(|i| i.label).collect();
    let keywords = ["fn", "let", "if", "return"];
    for keyword in keywords {
        assert!(
            labels.iter().any(|l| l == keyword),
            "expected completions to include keyword '{keyword}', got: {labels:?}"
        );
    }
}

#[test]
fn completion_returns_symbols() {
    let client = start_server();
    initialize(&client);

    let source = "fn greet(name: Str) -> Str { \"hello\" }\ng";
    open_document(&client, source);

    // Request completions after `g` on the second line.
    let resp = completion_at(&client, 1, 1);
    assert!(
        resp.error.is_none(),
        "completion returned error: {:?}",
        resp.error
    );

    let items: Vec<CompletionItem> =
        serde_json::from_value(resp.result.expect("completion response missing result"))
            .expect("completion result should deserialize to a list of items");

    let labels: Vec<String> = items.into_iter().map(|i| i.label).collect();
    assert!(
        labels.iter().any(|l| l == "greet"),
        "expected completions to include symbol 'greet', got: {labels:?}"
    );
}

#[test]
fn definition_jumps_to_function() {
    let client = start_server();
    initialize(&client);

    let source =
        "fn greet(name: Str) -> Str { \"hello, \" + name }\nfn main() -> Str { greet(\"world\") }";
    open_document(&client, source);

    // Position of `greet` in `greet("world")` on line 1, character 19.
    let resp = definition_at(&client, 1, 19);
    assert!(
        resp.error.is_none(),
        "definition returned error: {:?}",
        resp.error
    );
    assert!(
        resp.result.is_some() && !resp.result.as_ref().unwrap().is_null(),
        "expected definition result, got null"
    );

    let result: GotoDefinitionResponse = serde_json::from_value(resp.result.unwrap())
        .expect("definition result should deserialize to GotoDefinitionResponse");
    let locations: Vec<Location> = match result {
        GotoDefinitionResponse::Scalar(loc) => vec![loc],
        GotoDefinitionResponse::Array(locs) => locs,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|link: LocationLink| Location {
                uri: link.target_uri,
                range: link.target_range,
            })
            .collect(),
    };

    assert!(
        locations
            .iter()
            .any(|loc| loc.uri == test_uri() && loc.range.start.line == 0),
        "expected definition to point to line 0, got: {locations:?}"
    );
}

#[test]
fn document_symbol_shows_structure() {
    let client = start_server();
    initialize(&client);

    let source = "fn add(a: Int, b: Int) -> Int { a + b }\nfn main() -> Int { add(1, 2) }";
    open_document(&client, source);

    let resp = document_symbols(&client);
    assert!(
        resp.error.is_none(),
        "documentSymbol returned error: {:?}",
        resp.error
    );
    assert!(
        resp.result.is_some() && !resp.result.as_ref().unwrap().is_null(),
        "expected documentSymbol result, got null"
    );

    let result: DocumentSymbolResponse = serde_json::from_value(resp.result.unwrap())
        .expect("documentSymbol result should deserialize to DocumentSymbolResponse");
    let symbols: Vec<SymbolInformation> = match result {
        DocumentSymbolResponse::Flat(symbols) => symbols,
        DocumentSymbolResponse::Nested(nested) => nested
            .into_iter()
            .map(|s| SymbolInformation {
                name: s.name,
                kind: s.kind,
                location: Location {
                    uri: test_uri(),
                    range: s.range,
                },
                container_name: s.detail,
                #[allow(deprecated)]
                deprecated: None,
                tags: None,
            })
            .collect(),
    };

    let function_names: Vec<&str> = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::FUNCTION)
        .map(|s| s.name.as_str())
        .collect();

    assert!(
        function_names.len() >= 2,
        "expected at least 2 function symbols, got: {symbols:?}"
    );
    assert!(
        function_names.contains(&"add"),
        "expected symbol 'add', got: {function_names:?}"
    );
    assert!(
        function_names.contains(&"main"),
        "expected symbol 'main', got: {function_names:?}"
    );
}

#[test]
fn formatting_formats_document() {
    let client = start_server();
    initialize(&client);

    let source = "fn   add(a:Int,b:Int)->Int{a+b}";
    open_document(&client, source);
    // Consume the diagnostics notification published on didOpen.
    let _ = expect_diagnostics(&client);

    let resp = format_document(&client);
    assert!(
        resp.error.is_none(),
        "formatting returned error: {:?}",
        resp.error
    );
    assert!(
        resp.result.is_some() && !resp.result.as_ref().unwrap().is_null(),
        "expected formatting result with text edits, got null"
    );

    let edits: Vec<TextEdit> = serde_json::from_value(resp.result.unwrap())
        .expect("formatting result should deserialize to a list of text edits");
    assert!(
        !edits.is_empty(),
        "expected at least one formatting text edit, got none"
    );

    // The first edit should rewrite the entire document, and the new text
    // should have cleaner spacing around punctuation.
    let first_edit = &edits[0];
    let new_text = &first_edit.new_text;
    assert!(
        new_text.contains("fn add") || new_text.contains("a: Int") || new_text.contains(", "),
        "expected formatted text to add spacing, got: {new_text}"
    );
}

#[test]
fn did_save_triggers_diagnostics() {
    let client = start_server();
    initialize(&client);

    let source = "fn answer() -> Int { 40 + 2 }";
    open_document(&client, source);

    // The document is valid, so the initial diagnostics should be clean.
    let diagnostics = expect_diagnostics(&client);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for clean document, got: {diagnostics:?}"
    );

    // Replace with a document that has a type error.
    change_document(&client, "fn answer() -> Int { 40 + true }");

    // didChange already publishes diagnostics; consume them.
    let change_diagnostics = expect_diagnostics(&client);
    assert!(
        change_diagnostics
            .iter()
            .any(|d| d.message.contains("type")),
        "expected change diagnostics to include a type error, got: {change_diagnostics:?}"
    );

    // Save the document. A real implementation should re-publish diagnostics.
    save_document(&client);

    let save_diagnostics = expect_diagnostics(&client);
    assert!(
        save_diagnostics.iter().any(|d| d.message.contains("type")),
        "expected save to re-publish diagnostics including the type error, got: {save_diagnostics:?}"
    );
}

/// Convert hover contents to a plain string for assertions.
fn hover_contents_to_string(hover: &Hover) -> String {
    match &hover.contents {
        lsp_types::HoverContents::Scalar(text) => marked_string_to_string(text),
        lsp_types::HoverContents::Array(parts) => parts
            .iter()
            .map(marked_string_to_string)
            .collect::<Vec<_>>()
            .join("\n"),
        lsp_types::HoverContents::Markup(markup) => markup.value.clone(),
    }
}

fn marked_string_to_string(ms: &lsp_types::MarkedString) -> String {
    match ms {
        lsp_types::MarkedString::String(s) => s.clone(),
        lsp_types::MarkedString::LanguageString(ls) => ls.value.clone(),
    }
}
