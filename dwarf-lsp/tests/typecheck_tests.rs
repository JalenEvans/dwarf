//! Integration tests for type-check diagnostics in dwarf-lsp.
//!
//! These tests verify that the LSP server publishes type errors (not just
//! parse errors) as diagnostics. They are expected to fail while the handler
//! only runs `ParsePass` and does not yet run `TypeCheckPass`.

use std::str::FromStr;
use std::time::Duration;

use dwarf_lsp::handler::DwarfLspHandler;
use lsp_server::{Connection, ErrorCode, Message, Notification, Response};
use lsp_types::notification::{DidOpenTextDocument, Notification as _, PublishDiagnostics};
use lsp_types::{
    ClientCapabilities, Diagnostic, DiagnosticSeverity, DidOpenTextDocumentParams, NumberOrString,
    PublishDiagnosticsParams, TextDocumentItem, Uri,
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

fn send_notification<P: serde::Serialize>(conn: &Connection, method: &str, params: P) {
    let notif = Notification::new(method.to_string(), params);
    conn.sender
        .send(notif.into())
        .expect("failed to send notification");
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

/// Open a document on the server and return the published diagnostics.
fn open_document(conn: &Connection, text: &str) -> Vec<Diagnostic> {
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: test_uri(),
            language_id: "dwarf".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    send_notification(conn, DidOpenTextDocument::METHOD, params);

    let notif = expect_notification(conn, PublishDiagnostics::METHOD);
    let params: PublishDiagnosticsParams =
        serde_json::from_value(notif.params).expect("publishDiagnostics params should deserialize");
    params.diagnostics
}

fn diagnostic_code(diag: &Diagnostic) -> Option<String> {
    diag.code.as_ref().map(|c| match c {
        NumberOrString::Number(n) => n.to_string(),
        NumberOrString::String(s) => s.clone(),
    })
}

fn has_error_with_code_prefix(diagnostics: &[Diagnostic], prefix: &str) -> bool {
    diagnostics.iter().any(|d| {
        diagnostic_code(d)
            .map(|code| code.starts_with(prefix))
            .unwrap_or(false)
    })
}

#[test]
fn type_error_diagnostics() {
    let client = start_server();

    // Valid parse, invalid types: Int + Bool is a type error.
    let diagnostics = open_document(&client, "fn add(a: Int, b: Int) -> Int { a + true }");

    assert!(
        has_error_with_code_prefix(&diagnostics, "DWARF-E-TYPE-"),
        "expected a DWARF-E-TYPE- diagnostic for a type error, got: {diagnostics:?}"
    );
}

#[test]
fn parse_and_type_errors_combined() {
    let client = start_server();

    // First function has a parse error (missing ')'); second has a type error.
    let source = r#"fn broken( { 1 }
fn type_error() -> Int { 1 + true }"#;
    let diagnostics = open_document(&client, source);

    assert!(
        has_error_with_code_prefix(&diagnostics, "DWARF-E-PARSE-"),
        "expected a DWARF-E-PARSE- diagnostic for the parse error, got: {diagnostics:?}"
    );
    assert!(
        has_error_with_code_prefix(&diagnostics, "DWARF-E-TYPE-"),
        "expected a DWARF-E-TYPE- diagnostic for the type error, got: {diagnostics:?}"
    );
}

#[test]
fn severity_mapping() {
    let client = start_server();

    let diagnostics = open_document(&client, "fn foo() -> Int { 1 + true }");

    let type_errors: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|d| {
            diagnostic_code(d)
                .map(|code| code.starts_with("DWARF-E-TYPE-"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !type_errors.is_empty(),
        "expected at least one type error diagnostic, got: {diagnostics:?}"
    );

    for diag in &type_errors {
        assert_eq!(
            diag.severity,
            Some(DiagnosticSeverity::ERROR),
            "type error diagnostic should have ERROR severity, got: {diag:?}"
        );
    }
}

#[test]
fn no_errors_clean_document() {
    let client = start_server();

    let diagnostics = open_document(&client, "fn answer() -> Int { 40 + 2 }");

    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for a clean document, got: {diagnostics:?}"
    );
}
