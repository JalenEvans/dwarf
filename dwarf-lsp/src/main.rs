//! Entry point for the dwarf-lsp LSP server binary.
//!
//! This binary starts a Language Server Protocol server that provides
//! IDE features for the Dwarf programming language. Communication
//! happens over stdio using JSON-RPC.
//!
//! # Usage
//!
//! ```bash
//! # Start LSP server in stdio mode
//! dwarf-lsp --stdio
//!
//! # Show help
//! dwarf-lsp --help
//! ```

use clap::Parser;
use dwarf_lsp::handler::DwarfLspHandler;
use lsp_server::{Connection, Message, Response};
use lsp_types::*;
use std::error::Error;

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "dwarf-lsp", version, about = "Dwarf Language Server")]
struct Cli {
    /// Use stdio transport (required for LSP client integration)
    #[arg(long)]
    stdio: bool,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Initialize the tracing/logging subscriber.
    // The fmt subscriber writes to stderr by default, keeping stdout clean
    // for JSON-RPC messages.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    if cli.stdio {
        run_stdio_server().await?;
    } else {
        // No mode specified — print startup info and exit cleanly.
        println!("dwarf-lsp v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: dwarf-lsp --stdio");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// LSP server setup
// ---------------------------------------------------------------------------

/// Create and run the LSP server with stdio transport.
///
/// This function:
/// 1. Creates a stdio `Connection` (JSON-RPC over stdin/stdout).
/// 2. Initializes the LSP handshake (Initialize request/response).
/// 3. Creates the handler and enters the main message loop.
async fn run_stdio_server() -> Result<(), Box<dyn Error>> {
    let (connection, io_threads) = Connection::stdio();

    // Server capabilities: declare what features this LSP server supports.
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        ..Default::default()
    };

    let init_params = connection.initialize(serde_json::to_value(&server_capabilities)?)?;

    // Create the LSP handler with the initialized client capabilities.
    let client_capabilities: ClientCapabilities =
        serde_json::from_value(init_params).unwrap_or_default();
    let mut handler = DwarfLspHandler::new(client_capabilities);

    // Main message loop: process incoming requests, notifications, and
    // shutdown requests.
    main_loop(&connection, &mut handler)?;

    io_threads.join()?;
    Ok(())
}

/// Process incoming LSP messages until a shutdown request is received.
fn main_loop(
    connection: &Connection,
    handler: &mut DwarfLspHandler,
) -> Result<(), Box<dyn Error>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                // Dispatch request to the handler.
                match handler.handle_request(&req) {
                    Ok(Some(resp)) => connection.sender.send(resp.into())?,
                    Ok(None) => {}
                    Err(e) => {
                        let resp = Response::new_err(
                            req.id.clone(),
                            lsp_server::ErrorCode::InternalError as i32,
                            e.to_string(),
                        );
                        connection.sender.send(resp.into())?;
                    }
                }
            }
            Message::Notification(notif) => {
                handler.handle_notification(&notif);
            }
            Message::Response(_) => {
                // We don't send requests to the client (yet), so ignore responses.
            }
        }
    }
    Ok(())
}
