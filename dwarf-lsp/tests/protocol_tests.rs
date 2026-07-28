//! Integration tests for the dwarf-lsp protocol handlers.
//!
//! These tests start an in-process LSP server over a memory transport and
//! exercise the JSON-RPC protocol end-to-end. They are expected to fail while
//! the handler implementations are still stubs.

use std::str::FromStr;
use std::time::Duration;

use dwarf_lsp::handler::DwarfLspHandler;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{Initialize, Request as _, Shutdown};
use lsp_types::{
    ClientCapabilities, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializeResult, ServerCapabilities,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri, VersionedTextDocumentIdentifier,
};

const TIMEOUT: Duration = Duration::from_secs(1);

/// Start an in-process LSP server on a memory transport and return the client
/// side of the connection.
fn start_server() -> Connection {
    let (server_conn, client_conn) = Connection::memory();
    std::thread::spawn(move || {
        // Handle the initialize handshake before creating the handler,
        // just like production code does in main.rs.
        let server_capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..Default::default()
        };
        let _init_params = server_conn
            .initialize(serde_json::to_value(&server_capabilities).unwrap())
            .unwrap();

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

fn expect_notification(conn: &Connection, method: &str) -> Notification {
    loop {
        match conn.receiver.recv_timeout(TIMEOUT) {
            Ok(Message::Notification(notif)) if notif.method == method => return notif,
            Ok(_) => continue,
            Err(_) => panic!("timed out waiting for notification {method}"),
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

    // Send the initialized notification to complete the handshake.
    send_notification(conn, Initialized::METHOD, ());
}

#[test]
fn initialize_handshake() {
    let client = start_server();

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
    send_request(&client, RequestId::from(1), Initialize::METHOD, params);

    let resp = expect_response(&client, RequestId::from(1));
    assert!(
        resp.error.is_none(),
        "initialize returned error: {:?}",
        resp.error
    );

    let result: InitializeResult =
        serde_json::from_value(resp.result.expect("initialize response missing result"))
            .expect("initialize result should deserialize");

    assert!(
        result.capabilities.text_document_sync.is_some(),
        "server capabilities missing text_document_sync"
    );

    match result.capabilities.text_document_sync {
        Some(TextDocumentSyncCapability::Kind(kind)) => {
            assert!(
                kind == TextDocumentSyncKind::FULL || kind == TextDocumentSyncKind::INCREMENTAL,
                "unexpected text_document_sync kind: {kind:?}"
            );
        }
        other => panic!("unexpected text_document_sync capability: {other:?}"),
    }

    // Send the initialized notification to complete the handshake.
    send_notification(&client, Initialized::METHOD, ());
}

#[test]
fn did_open_notification() {
    let client = start_server();
    initialize(&client);

    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: test_uri(),
            language_id: "dwarf".to_string(),
            version: 1,
            text: "func main() {}".to_string(),
        },
    };
    send_notification(&client, DidOpenTextDocument::METHOD, params);

    // A real implementation parses the document and publishes diagnostics.
    let notif = expect_notification(&client, PublishDiagnostics::METHOD);
    assert!(
        !notif.params.is_null(),
        "publishDiagnostics params were null"
    );
}

#[test]
fn did_change_notification() {
    let client = start_server();
    initialize(&client);

    let uri = test_uri();
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "dwarf".to_string(),
            version: 1,
            text: "func main() {}".to_string(),
        },
    };
    send_notification(&client, DidOpenTextDocument::METHOD, open_params);

    let change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier { uri, version: 2 },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "func main() { print(\"hi\") }".to_string(),
        }],
    };
    send_notification(&client, DidChangeTextDocument::METHOD, change_params);

    let notif = expect_notification(&client, PublishDiagnostics::METHOD);
    assert!(
        !notif.params.is_null(),
        "publishDiagnostics params were null"
    );
}

#[test]
fn did_close_notification() {
    let client = start_server();
    initialize(&client);

    let params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: test_uri() },
    };
    send_notification(&client, DidCloseTextDocument::METHOD, params);

    // After closing the document the server should clear diagnostics.
    let notif = expect_notification(&client, PublishDiagnostics::METHOD);
    assert!(
        !notif.params.is_null(),
        "publishDiagnostics params were null"
    );
}

#[test]
fn shutdown_request() {
    let client = start_server();
    initialize(&client);

    send_request(
        &client,
        RequestId::from(99),
        Shutdown::METHOD,
        serde_json::Value::Null,
    );

    let resp = expect_response(&client, RequestId::from(99));
    assert!(
        resp.error.is_none(),
        "shutdown returned error: {:?}",
        resp.error
    );
    assert_eq!(
        resp.result,
        Some(serde_json::Value::Null),
        "shutdown should return null"
    );

    // After a successful shutdown the server should accept the exit notification.
    send_notification(&client, Exit::METHOD, serde_json::Value::Null);
}
