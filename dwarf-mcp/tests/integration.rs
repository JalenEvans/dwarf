//! Integration tests for the dwarf-mcp MCP server binary.
//!
//! These tests verify that:
//! 1. The crate compiles and the binary runs
//! 2. The binary accepts `--help` and `--transport stdio` arguments
//! 3. The binary performs a correct MCP initialize/initialized handshake over stdio
//!
//! # Red phase
//!
//! All tests except `binary_compiles_and_runs` are expected to **fail**
//! until the Green phase implements CLI argument parsing and the MCP
//! server transport.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Path to the compiled `dwarf-mcp` binary.
///
/// Cargo sets `CARGO_BIN_EXE_dwarf-mcp` when compiling integration tests
/// for a crate that defines a `[[bin]]` target with that name.
fn dwarf_mcp_binary() -> &'static str {
    env!("CARGO_BIN_EXE_dwarf-mcp")
}

/// Spawn `dwarf-mcp` with the given arguments and return a handle ready for
/// stdio communication.
fn spawn_dwarf_mcp(args: &[&str]) -> std::process::Child {
    Command::new(dwarf_mcp_binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dwarf-mcp process")
}

/// Read a single JSON-RPC message (one line) from the child's stdout.
fn read_json_rpc_line(reader: &mut BufReader<&mut std::process::ChildStdout>) -> String {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Failed to read line from child stdout");
    line.trim().to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Verify that the binary compiles and exits successfully with no arguments.
///
/// This is the most basic smoke test: the crate must compile and the binary
/// must start without crashing.
///
/// ## Expected (Green phase)
/// The binary prints its startup message and exits cleanly.
///
/// ## Red phase
/// **Should pass** — the stub in `main.rs` compiles and prints "dwarf-mcp stub".
#[test]
fn binary_compiles_and_runs() {
    let output = Command::new(dwarf_mcp_binary())
        .output()
        .expect("Failed to execute dwarf-mcp binary");

    assert!(
        output.status.success(),
        "dwarf-mcp should exit successfully.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "stdout should contain some output from the binary"
    );
}

/// Verify the binary prints usage information when `--help` is passed.
///
/// ## Expected (Green phase)
/// clap generates help text listing `--transport`, `--log-level`, etc.
///
/// ## Red phase
/// **Should fail** — the stub prints "dwarf-mcp stub" and ignores `--help`.
#[test]
fn binary_accepts_help_flag() {
    let output = Command::new(dwarf_mcp_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute dwarf-mcp --help");

    assert!(
        output.status.success(),
        "dwarf-mcp --help should exit successfully.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // clap help text must contain these markers
    assert!(
        stdout.contains("dwarf-mcp"),
        "Help text should mention the binary name.\nGot:\n{}",
        stdout
    );
    assert!(
        stdout.contains("--transport") || stdout.contains("transport"),
        "Help text should mention the --transport flag.\nGot:\n{}",
        stdout
    );
    assert!(
        stdout.contains("USAGE") || stdout.contains("Usage"),
        "Help text should have a USAGE section.\nGot:\n{}",
        stdout
    );
}

/// Verify the binary accepts `--transport stdio` and stays running in stdio mode.
///
/// When `--transport stdio` is passed, the binary should enter stdio mode,
/// listening for JSON-RPC messages on stdin and writing responses to stdout.
/// A binary in stdio mode **must not exit immediately** — it should block
/// waiting for input.
///
/// ## Expected (Green phase)
/// The binary starts and stays alive (the process does not exit).
///
/// ## Red phase
/// **Should fail** — the stub does not parse `--transport` and prints
/// "dwarf-mcp stub" then exits immediately.
#[test]
fn binary_accepts_transport_stdio() {
    let mut child = spawn_dwarf_mcp(&["--transport", "stdio"]);

    // Give the process a moment — if it's a stub it will exit immediately
    std::thread::sleep(Duration::from_millis(300));

    // Check whether the process is still running
    let status = child.try_wait().expect("Failed to check child status");
    assert!(
        status.is_none(),
        "dwarf-mcp --transport stdio should stay running (waiting for input), \
         but it exited prematurely.\n\
         A correct MCP server in stdio mode blocks on stdin read."
    );

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}

/// Verify the binary performs a correct MCP initialize handshake over stdio.
///
/// This is the most important integration test. A correct MCP server must:
/// 1. Accept a JSON-RPC `initialize` request and respond with server metadata
/// 2. Accept an `initialized` notification (no response expected)
///
/// The test sends these messages in sequence and validates the response shape.
///
/// ## Expected (Green phase)
/// The response contains:
/// - `jsonrpc: "2.0"`
/// - `id: 1`
/// - `result.protocolVersion: "2024-11-05"`
/// - `result.serverInfo.name: "dwarf-mcp"`
/// - `result.serverInfo.version` (semver string)
/// - `result.capabilities` (object with tools/resources/prompts)
///
/// ## Red phase
/// **Should fail** — the stub does not implement the MCP protocol, so the
/// process either exits immediately or produces no valid JSON-RPC response.
#[test]
fn stdio_initialize_handshake() {
    // --  Arrange  -----------------------------------------------------------

    let mut child = spawn_dwarf_mcp(&["--transport", "stdio"]);

    let stdout = child
        .stdout
        .as_mut()
        .expect("Child should have a piped stdout");
    let mut reader = BufReader::new(stdout);

    let initialize_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#;

    let initialized_notification =
        r#"{"jsonrpc":"2.0","id":2,"method":"initialized"}"#;

    // --  Act  ---------------------------------------------------------------

    // Send the initialize request
    {
        let writer = child.stdin.as_mut().expect("Child should have a piped stdin");
        writeln!(writer, "{initialize_request}").expect("Failed to write initialize request");
        writer.flush().expect("Failed to flush stdin");
    }

    // Read the initialize response
    let response_line = read_json_rpc_line(&mut reader);
    let response: serde_json::Value =
        serde_json::from_str(&response_line).expect("Initialize response must be valid JSON");

    // Send the initialized notification (no response expected)
    {
        let writer = child.stdin.as_mut().expect("Child should still have stdin");
        writeln!(writer, "{initialized_notification}")
            .expect("Failed to write initialized notification");
        writer.flush().expect("Failed to flush stdin");
    }

    // Give the server a moment to process before killing
    std::thread::sleep(Duration::from_millis(500));
    let _ = child.kill();
    let _ = child.wait();

    // --  Assert  ------------------------------------------------------------

    // JSON-RPC version
    assert_eq!(
        response["jsonrpc"], "2.0",
        "Response must declare JSON-RPC 2.0.\nGot: {}",
        response
    );

    // Must echo back the same id
    assert_eq!(
        response["id"], 1,
        "Response must echo request id.\nGot: {}",
        response
    );

    // Must not be an error response
    assert!(
        response.get("error").is_none(),
        "Initialize must not return an error.\nGot: {}",
        response
    );

    // Must have a result object
    let result = response
        .get("result")
        .expect("Response must contain a 'result' object");

    // Protocol version must be echoed back
    assert_eq!(
        result["protocolVersion"], "2024-11-05",
        "result.protocolVersion must be '2024-11-05'.\nGot: {}",
        result
    );

    // Server info must be present with name
    let server_info = result
        .get("serverInfo")
        .expect("result must contain 'serverInfo'");
    assert_eq!(
        server_info["name"], "dwarf-mcp",
        "serverInfo.name must be 'dwarf-mcp'.\nGot: {}",
        server_info
    );

    // Server version must be present
    let version = server_info
        .get("version")
        .expect("serverInfo must contain 'version'")
        .as_str()
        .expect("version must be a string");
    assert!(
        !version.is_empty(),
        "serverInfo.version must be a non-empty semver string.\nGot: {:?}",
        version
    );

    // Capabilities must be present
    assert!(
        result.get("capabilities").is_some(),
        "result must contain 'capabilities'.\nGot: {}",
        result
    );
}
