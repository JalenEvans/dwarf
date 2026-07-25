//! Integration tests for the dwarf-mcp MCP server binary.
//!
//! These tests verify that:
//! 1. The crate compiles and the binary runs
//! 2. The binary accepts `--help` and `--transport stdio` arguments
//! 3. The binary performs a correct MCP initialize/initialized handshake over stdio
//! 4. The binary exposes MCP resources via `resources/list` and `resources/read`
//!
//! # Red phase
//!
//! All tests except `binary_compiles_and_runs` are expected to **fail**
//! until the Green phase implements CLI argument parsing, the MCP
//! server transport, and resource handlers.

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

/// Spawn `dwarf-mcp --transport stdio`, perform the MCP initialize handshake,
/// send the given JSON-RPC request, read the response (with a 5-second
/// timeout), pass it to `validator`, and clean up the child process.
///
/// The initialize request uses `id:1`, so callers should use request ids
/// starting at 3 or higher to avoid collisions.
fn with_initialized_server<F>(request_json: &str, validator: F)
where
    F: FnOnce(serde_json::Value) + Send + 'static,
{
    // Use channels to communicate the reader thread's result back.
    let (response_tx, response_rx) = std::sync::mpsc::channel::<String>();

    // ------------------------------------------------------------------
    // Phase 1 — Initialise and write the test request
    // ------------------------------------------------------------------
    // We do this before spawning the reader thread so we can hold a
    // mutable borrow on `child.stdin` without conflicts.

    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#;

    let mut child = spawn_dwarf_mcp(&["--transport", "stdio"]);

    // Send the initialize request.
    {
        let writer = child
            .stdin
            .as_mut()
            .expect("Child should have a piped stdin");
        writeln!(writer, "{init_request}")
            .expect("Failed to write initialize request");
        writer.flush().expect("Failed to flush stdin");
    }

    // Read the initialize response (a blocking read that we KNOW works
    // because the server always responds to initialize).
    {
        let stdout = child
            .stdout
            .as_mut()
            .expect("Child should have a piped stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("Failed to read init response");
        let _init: serde_json::Value =
            serde_json::from_str(line.trim()).expect("Init response must be valid JSON");
    }
    // `reader` and `stdout` are dropped here, releasing the borrow on
    // `child.stdout`.

    // Send the `initialized` notification (no `"id"` field — correct
    // JSON-RPC notification format).  The SDK must receive this before
    // it considers the session fully initialized.
    {
        let writer = child
            .stdin
            .as_mut()
            .expect("Child should still have piped stdin");
        writeln!(
            writer,
            r#"{{"jsonrpc":"2.0","method":"initialized"}}"#
        )
        .expect("Failed to write initialized notification");
        writer.flush().expect("Failed to flush stdin");
    }

    // Brief pause so the server can process the notification before we send
    // the test request.
    std::thread::sleep(Duration::from_millis(200));

    // Send the test request.
    {
        let writer = child
            .stdin
            .as_mut()
            .expect("Child should still have piped stdin");
        writeln!(writer, "{request_json}")
            .expect("Failed to write test request");
        writer.flush().expect("Failed to flush stdin");
    }

    // ------------------------------------------------------------------
    // Phase 2 — Read the response on a background thread with timeout
    // ------------------------------------------------------------------

    // Take ownership of stdout so the background thread can use it.
    let mut child_stdout = child
        .stdout
        .take()
        .expect("Child should have a piped stdout");

    let reader_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(&mut child_stdout);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                let _ = response_tx.send(String::new());
            }
            Ok(_) => {
                let _ = response_tx.send(line.trim().to_string());
            }
            Err(e) => {
                let _ = response_tx.send(format!("__IO_ERR__:{e}"));
            }
        }
    });

    // Wait for the reader thread with a 10-second timeout.
    let timeout_msg = "Timed out waiting for a response from the MCP server. \
                       The server is running but not producing output.";
    let response_line = match response_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(line) => line,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{timeout_msg}");
        }
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Channel error: {e}");
        }
    };

    // Join the reader thread (it should have finished).
    let _ = reader_handle.join();

    // Check for I/O errors from the reader thread.
    if let Some(err_msg) = response_line.strip_prefix("__IO_ERR__:") {
        let _ = child.kill();
        let _ = child.wait();
        panic!("I/O error reading response: {err_msg}");
    }

    // Handle empty response (server closed stdout).
    if response_line.is_empty() {
        let exit_status = child
            .try_wait()
            .ok()
            .flatten()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "still running".to_string());
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "Empty response from server (exit status: {exit_status}).\n\
             The server may have crashed before sending a response."
        );
    }

    // Helpful debug: if the response is not valid JSON, show what we got.
    let response: serde_json::Value = match serde_json::from_str(&response_line) {
        Ok(v) => v,
        Err(e) => {
            panic!(
                "Response is not valid JSON.\nError: {e}\nReceived: {response_line:?}"
            );
        }
    };

    // -- Validate ------------------------------------------------------------

    validator(response);

    // -- Clean up ------------------------------------------------------------

    let _ = child.kill();
    let _ = child.wait();
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

/// Verify the `resources/list` method returns resource entries after
/// the initialize handshake.
///
/// A compliant MCP server must return a non-empty array of `Resource` objects,
/// each with `uri`, `name`, and `description` fields.  At least 9 resources
/// are expected when all language references are registered.
///
/// ## Expected (Green phase)
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 3,
///   "result": {
///     "resources": [
///       { "uri": "dwarf://...", "name": "...", "description": "..." },
///       ...
///     ]
///   }
/// }
/// ```
///
/// ## Red phase
/// **Should fail** — `handle_list_resources_request` is not implemented, so
/// the default handler returns `-32601` (Method not found).
#[test]
fn resources_list_returns_entries() {
    let request = r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#;
    with_initialized_server(request, |response| {
        let resources = response["result"]["resources"]
            .as_array()
            .expect("result.resources should be a non-empty array");

        assert!(
            !resources.is_empty(),
            "result.resources should contain at least one resource"
        );

        for (i, resource) in resources.iter().enumerate() {
            let obj = resource
                .as_object()
                .unwrap_or_else(|| panic!("resources[{i}] should be an object, got {resource}"));
            assert!(
                obj.contains_key("uri"),
                "resources[{i}] must have a 'uri' field.\nGot: {resource}"
            );
            assert!(
                obj.contains_key("name"),
                "resources[{i}] must have a 'name' field.\nGot: {resource}"
            );
            assert!(
                obj.contains_key("description"),
                "resources[{i}] must have a 'description' field.\nGot: {resource}"
            );
        }

        let first_uri = resources[0]["uri"]
            .as_str()
            .expect("first resource uri should be a string");
        assert!(
            first_uri.starts_with("dwarf://"),
            "first resource URI should start with 'dwarf://'.\nGot: {first_uri}"
        );

        assert!(
            resources.len() >= 9,
            "expected at least 9 resources, got {}",
            resources.len()
        );
    });
}

/// Verify that `resources/read` returns syntax overview content for the
/// `dwarf://syntax/overview` resource.
///
/// ## Expected (Green phase)
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 4,
///   "result": {
///     "contents": [
///       { "uri": "dwarf://syntax/overview", "mimeType": "text/markdown", "text": "..." }
///     ]
///   }
/// }
/// ```
///
/// ## Red phase
/// **Should fail** — `handle_read_resource_request` is not implemented, so
/// the default handler returns `-32601` (Method not found).
#[test]
fn resources_read_syntax_overview() {
    let request = r#"{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"dwarf://syntax/overview"}}"#;
    with_initialized_server(request, |response| {
        let contents = response["result"]["contents"]
            .as_array()
            .expect("result.contents should be a non-empty array");

        assert!(
            !contents.is_empty(),
            "result.contents should contain at least one content item"
        );

        for (i, item) in contents.iter().enumerate() {
            let obj = item
                .as_object()
                .unwrap_or_else(|| panic!("contents[{i}] should be an object, got {item}"));
            assert!(
                obj.contains_key("uri"),
                "contents[{i}] must have a 'uri' field.\nGot: {item}"
            );
            assert!(
                obj.contains_key("mimeType"),
                "contents[{i}] must have a 'mimeType' field.\nGot: {item}"
            );
            assert!(
                obj.contains_key("text"),
                "contents[{i}] must have a 'text' field.\nGot: {item}"
            );
        }

        assert_eq!(
            contents[0]["mimeType"], "text/markdown",
            "mimeType should be 'text/markdown'.\nGot: {}",
            contents[0]["mimeType"]
        );

        let text = contents[0]["text"]
            .as_str()
            .expect("contents[0].text should be a string");
        assert!(
            !text.is_empty(),
            "contents[0].text should be a non-empty string"
        );
    });
}

/// Verify that `resources/read` returns an error for an unknown URI.
///
/// When the resource URI does not match any known resource, the server must
/// return a JSON-RPC error response rather than a success response.
///
/// ## Expected (Green phase)
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "id": 5,
///   "error": { "code": -32602, "message": "..." }
/// }
/// ```
///
/// ## Red phase
/// **Should fail** — the default handler returns `-32601` (Method not found),
/// but we assert the code is NOT `-32601`, ensuring we detect the difference
/// between "method not recognised" and "method recognised but URI unknown".
#[test]
fn resources_read_returns_error_for_unknown_uri() {
    let request = r#"{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"dwarf://nonexistent"}}"#;
    with_initialized_server(request, |response| {
        let error = response
            .get("error")
            .expect("response should contain an 'error' field for an unknown resource URI");

        let code = error["code"]
            .as_i64()
            .expect("error.code should be an integer");
        assert_ne!(code, 0, "error code should be non-zero");

        // In Red phase the default handler returns -32601 (Method not found).
        // We explicitly reject that code so this test stays Red until the
        // proper handler is implemented.
        assert_ne!(
            code, -32601,
            "error code must not be -32601 (Method not found) — the handler must \
             recognise the resources/read method and return a resource-specific error.\n\
             Got response: {response}"
        );
    });
}

/// Verify that `resources/read` returns testing-related content for the
/// `dwarf://examples/testing` resource.
///
/// ## Expected (Green phase)
/// The response contains `result.contents` with at least one item whose
/// `text` field includes testing-related keywords.
///
/// ## Red phase
/// **Should fail** — `handle_read_resource_request` is not implemented, so
/// the default handler returns `-32601` (Method not found).
#[test]
fn resources_read_examples_testing() {
    let request = r#"{"jsonrpc":"2.0","id":6,"method":"resources/read","params":{"uri":"dwarf://examples/testing"}}"#;
    with_initialized_server(request, |response| {
        let contents = response["result"]["contents"]
            .as_array()
            .expect("result.contents should be a non-empty array");

        assert!(
            !contents.is_empty(),
            "result.contents should contain at least one content item"
        );

        let text = contents[0]["text"]
            .as_str()
            .expect("contents[0].text should be a string");

        assert!(
            text.to_lowercase().contains("test"),
            "contents[0].text should contain testing-related content.\nGot (first 300 chars): {}",
            &text[..text.len().min(300)]
        );
    });
}
