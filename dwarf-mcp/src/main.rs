//! Entry point for the dwarf-mcp MCP server binary.
//!
//! This binary starts a standalone MCP server that exposes the Dwarf compiler
//! through the Model Context Protocol. It supports:
//! - `--transport stdio` — JSON-RPC over stdin/stdout (for local AI-agent integration)
//! - `--transport sse` — Server-Sent Events (not yet implemented)
//!
//! # Usage
//!
//! ```bash
//! # Print startup info and exit
//! dwarf-mcp
//!
//! # Start MCP server in stdio mode (the default for AI-agent integration)
//! dwarf-mcp --transport stdio
//!
//! # Show help
//! dwarf-mcp --help
//! ```

use clap::Parser;
use dwarf_mcp::handler::DwarfMcpHandler;
use rust_mcp_sdk::{
    mcp_server::{self, McpServerOptions},
    schema::{
        Implementation, InitializeResult, ServerCapabilities,
        ServerCapabilitiesPrompts, ServerCapabilitiesResources,
        ServerCapabilitiesTools,
    },
    McpServer, StdioTransport, ToMcpServerHandler, TransportOptions,
};

// ---------------------------------------------------------------------------
// CLI argument definitions
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "dwarf-mcp", version, about = "Dwarf MCP Server")]
struct Cli {
    /// Transport mode (stdio or sse)
    #[arg(long)]
    transport: Option<String>,

    /// Port for SSE transport (not yet used)
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize the tracing/logging subscriber.
    // The fmt subscriber writes to stderr by default, keeping stdout clean
    // for JSON-RPC messages.
    let level = match cli.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    match cli.transport.as_deref() {
        Some("stdio") => {
            // Start the MCP server in stdio mode — reads JSON-RPC from stdin,
            // writes responses to stdout.
            if let Err(e) = run_stdio_server().await {
                eprintln!("dwarf-mcp server error: {e}");
                std::process::exit(1);
            }
        }
        Some(other) => {
            eprintln!(
                "dwarf-mcp: unsupported transport '{other}'. Only 'stdio' is supported."
            );
            std::process::exit(1);
        }
        None => {
            // No transport specified — print startup message and exit cleanly.
            // This allows basic smoke tests (e.g. `binary_compiles_and_runs`)
            // to verify the binary works without starting a long-lived server.
            println!("dwarf-mcp MCP server v{}", env!("CARGO_PKG_VERSION"));
            println!("Usage: dwarf-mcp --transport stdio");
        }
    }
}

// ---------------------------------------------------------------------------
// MCP server setup
// ---------------------------------------------------------------------------

/// Create and run the MCP server with stdio transport.
///
/// This function:
/// 1. Creates a `StdioTransport` that reads JSON-RPC from stdin and writes
///    responses to stdout.
/// 2. Wraps `DwarfMcpHandler` into an MCP handler via `to_mcp_server_handler()`.
/// 3. Builds the server runtime with server metadata (name, version, capabilities).
/// 4. Starts the server event loop (blocks until stdin is closed).
async fn run_stdio_server() -> rust_mcp_sdk::error::SdkResult<()> {
    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = DwarfMcpHandler.to_mcp_server_handler();

    let server_details = InitializeResult {
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
    };

    let server = mcp_server::server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler,
        task_store: None,
        client_task_store: None,
        message_observer: None,
    });

    server.start().await
}
