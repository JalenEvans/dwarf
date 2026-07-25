//! MCP ServerHandler implementation for the Dwarf compiler.
//!
//! `DwarfMcpHandler` receives JSON-RPC method calls from an MCP client
//! (e.g. an LLM-powered IDE) and translates them into compiler operations
//! via `dwarf_lib` and `dwarf_gen`.

use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::*;
use std::sync::Arc;

use dwarf_lib::{CompileOptions, DwarfCompiler};
use dwarf_parser::pass::ParsePass;
use dwarf_syntax::hir::{Decl, LiteralValue, Type};
use dwarf_gen::generate_edge_cases;

/// The MCP server handler for the Dwarf compiler.
///
/// This struct implements `rust_mcp_sdk::mcp_server::ServerHandler`.
///
/// # Green phase
///
/// The implementation of `ServerHandler` provides:
/// - `handle_initialize_request` — returns server metadata (name, version, capabilities)
/// - `handle_list_resources_request` — returns all language resources
/// - `handle_read_resource_request` — returns content for a specific resource URI
/// - `handle_list_tools_request` — returns all compiler tools
/// - `handle_call_tool_request` — dispatches tool calls to compiler logic
pub struct DwarfMcpHandler;

impl DwarfMcpHandler {
    /// Create a new handler with default configuration.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DwarfMcpHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Resource definitions
// ---------------------------------------------------------------------------

/// Build the list of all known Dwarf language resources.
fn all_resources() -> Vec<Resource> {
    vec![
        Resource {
            uri: "dwarf://syntax/overview".to_string(),
            name: "Dwarf Syntax Overview".to_string(),
            title: Some("Dwarf Syntax Overview".to_string()),
            description: Some("Language philosophy, key design decisions, and syntax fundamentals.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/functions".to_string(),
            name: "Functions".to_string(),
            title: Some("Functions".to_string()),
            description: Some("Function declarations, parameters, return types, and the effects system (pure/io/async).".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/types".to_string(),
            name: "Types".to_string(),
            title: Some("Types".to_string()),
            description: Some("Records, unions, generics, type aliases, and refinement types like Int(0..100).".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/modules".to_string(),
            name: "Modules".to_string(),
            title: Some("Modules".to_string()),
            description: Some("Module declarations, imports via `import \"x\"`, and path resolution.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/expressions".to_string(),
            name: "Expressions".to_string(),
            title: Some("Expressions".to_string()),
            description: Some("If/match/block/loop — everything is an expression. Pipe operator `|>`.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://syntax/testing".to_string(),
            name: "Testing".to_string(),
            title: Some("Testing".to_string()),
            description: Some("@test decorator, assertions, property-based testing with `forAll`.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://stdlib/reference".to_string(),
            name: "Standard Library".to_string(),
            title: Some("Standard Library".to_string()),
            description: Some("Built-in types and common functions available in every Dwarf program.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/basic".to_string(),
            name: "Basic Examples".to_string(),
            title: Some("Basic Examples".to_string()),
            description: Some("Hello world, arithmetic, string manipulation, and I/O examples.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/types".to_string(),
            name: "Type Examples".to_string(),
            title: Some("Type Examples".to_string()),
            description: Some("Record, union, and generic type patterns with real Dwarf code.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
        Resource {
            uri: "dwarf://examples/testing".to_string(),
            name: "Testing Examples".to_string(),
            title: Some("Testing Examples".to_string()),
            description: Some("Unit tests, property-based tests, and edge-case patterns.".to_string()),
            mime_type: Some("text/markdown".to_string()),
            annotations: None,
            icons: vec![],
            meta: None,
            size: None,
        },
    ]
}

/// Return the markdown content for a known resource URI, or `None` if unknown.
fn resource_content(uri: &str) -> Option<&'static str> {
    match uri {
        "dwarf://syntax/overview" => Some(include_str!("../resources/syntax/overview.md")),
        "dwarf://syntax/functions" => Some(include_str!("../resources/syntax/functions.md")),
        "dwarf://syntax/types" => Some(include_str!("../resources/syntax/types.md")),
        "dwarf://syntax/modules" => Some(include_str!("../resources/syntax/modules.md")),
        "dwarf://syntax/expressions" => Some(include_str!("../resources/syntax/expressions.md")),
        "dwarf://syntax/testing" => Some(include_str!("../resources/syntax/testing.md")),
        "dwarf://stdlib/reference" => Some(include_str!("../resources/stdlib/reference.md")),
        "dwarf://examples/basic" => Some(include_str!("../resources/examples/basic.md")),
        "dwarf://examples/types" => Some(include_str!("../resources/examples/types.md")),
        "dwarf://examples/testing" => Some(include_str!("../resources/examples/testing.md")),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

/// Build the list of all known Dwarf compiler tools.
fn all_tools() -> Vec<Tool> {
    vec![
        Tool {
            annotations: None,
            description: Some(
                "Validate Dwarf source code and return structured diagnostics".to_string(),
            ),
            execution: None,
            icons: vec![],
            input_schema: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Dwarf source code to check"
                    },
                    "filename": {
                        "type": "string",
                        "description": "Source filename (optional, default: input.kzd)"
                    }
                },
                "required": ["source"]
            }))
            .unwrap(),
            meta: None,
            name: "dwarf_check".to_string(),
            output_schema: None,
            title: None,
        },
        Tool {
            annotations: None,
            description: Some(
                "Compile Dwarf source to a target language".to_string(),
            ),
            execution: None,
            icons: vec![],
            input_schema: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Dwarf source code to compile"
                    },
                    "target": {
                        "type": "string",
                        "description": "Target language: ts, py, java, or debug",
                        "enum": ["ts", "py", "java", "debug"]
                    },
                    "filename": {
                        "type": "string",
                        "description": "Source filename (optional)"
                    }
                },
                "required": ["source", "target"]
            }))
            .unwrap(),
            meta: None,
            name: "dwarf_compile".to_string(),
            output_schema: None,
            title: None,
        },
        Tool {
            annotations: None,
            description: Some(
                "Format Dwarf source code".to_string(),
            ),
            execution: None,
            icons: vec![],
            input_schema: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Dwarf source code to format"
                    }
                },
                "required": ["source"]
            }))
            .unwrap(),
            meta: None,
            name: "dwarf_format".to_string(),
            output_schema: None,
            title: None,
        },
        Tool {
            annotations: None,
            description: Some(
                "Generate edge case test values from type definitions".to_string(),
            ),
            execution: None,
            icons: vec![],
            input_schema: serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "Dwarf source code containing type definitions"
                    }
                },
                "required": ["source"]
            }))
            .unwrap(),
            meta: None,
            name: "dwarf_generate_tests".to_string(),
            output_schema: None,
            title: None,
        },
    ]
}

// ---------------------------------------------------------------------------
// Tool implementation helpers
// ---------------------------------------------------------------------------

/// Extract a string argument by key from the tool call arguments map.
fn get_arg(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Option<String> {
    args.as_ref()
        .and_then(|m| m.get(key))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Create a `CallToolResult` wrapping an error message.
fn make_error_result(message: &str) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::TextContent(TextContent::new(
            serde_json::json!({ "error": message }).to_string(),
            None,
            None,
        ))],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

/// Create a `CallToolResult` wrapping a JSON-serializable value.
fn make_ok_result(value: impl serde::Serialize) -> CallToolResult {
    let json = serde_json::to_string_pretty(&value).unwrap_or_default();
    CallToolResult {
        content: vec![ContentBlock::TextContent(TextContent::new(json, None, None))],
        is_error: Some(false),
        meta: None,
        structured_content: None,
    }
}

// ---------------------------------------------------------------------------
// Tool handlers
// ---------------------------------------------------------------------------

/// Handle `dwarf_check` — validate Dwarf source and return structured diagnostics.
fn handle_dwarf_check(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
) -> CallToolResult {
    let source = match get_arg(args, "source") {
        Some(s) => s,
        None => return make_error_result("Missing required argument: source"),
    };
    let filename = get_arg(args, "filename").unwrap_or_else(|| "input.kzd".to_string());

    let compiler = DwarfCompiler::new();
    let options = CompileOptions {
        target: "debug".to_string(),
        ..Default::default()
    };

    // Collect diagnostics from either Ok or Err path
    let diagnostics = match compiler.compile(&source, &filename, options) {
        Ok(result) => result.diagnostics,
        Err(errs) => errs
            .into_iter()
            .flat_map(|e| match e {
                dwarf_lib::DwarfError::Compilation(diags) => diags,
                dwarf_lib::DwarfError::Config(msg) => vec![dwarf_lib::Diagnostic {
                    code: "DWARF-E-CONFIG".to_string(),
                    severity: dwarf_lib::Severity::Error,
                    message: msg,
                    file: Some(filename.clone()),
                    line: None,
                    col: None,
                }],
                dwarf_lib::DwarfError::Io(msg) => vec![dwarf_lib::Diagnostic {
                    code: "DWARF-E-IO".to_string(),
                    severity: dwarf_lib::Severity::Error,
                    message: msg,
                    file: Some(filename.clone()),
                    line: None,
                    col: None,
                }],
            })
            .collect(),
    };

    let diags: Vec<serde_json::Value> = diagnostics
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "code": d.code,
                "severity": d.severity.to_string(),
                "message": d.message,
                "file": d.file,
                "line": d.line,
                "col": d.col,
            })
        })
        .collect();

    make_ok_result(serde_json::json!({ "diagnostics": diags }))
}

/// Sanitize a Dwarf source snippet so it can be wrapped in a function body.
///
/// Removes type annotations from `let` bindings (e.g. `let x: Int = 42`
/// becomes `let x = 42`) since Dwarf's parser doesn't support type
/// annotations in let-patterns.
fn sanitize_let_annotations(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("let ") && trimmed.contains(':') && trimmed.contains('=') {
                // Strip `: TypeName` from `let name: TypeName = value`
                let colon_pos = trimmed.find(':').unwrap();
                let eq_pos = trimmed.find('=').unwrap();
                if eq_pos > colon_pos {
                    // Build without `: TypeName`
                    let mut result = String::with_capacity(trimmed.len());
                    result.push_str(&trimmed[..colon_pos]);
                    result.push_str(&trimmed[eq_pos..]);
                    return result;
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Try to compile `source` with the given `compiler` and `options`.
///
/// If the result has empty output but the source is non-trivial, falls back
/// to wrapping the source in a synthetic function body (`fn __expr__() { … }`)
/// so that bare top-level expressions and `let` bindings compile correctly.
fn compile_or_wrap(
    compiler: &DwarfCompiler,
    source: &str,
    filename: &str,
    options: CompileOptions,
) -> std::result::Result<dwarf_lib::CompileResult, Vec<dwarf_lib::DwarfError>> {
    // First attempt: compile the source as-is.
    let result = compiler.compile(source, filename, options.clone());

    // If compilation succeeds and produces output, return it directly.
    if let Ok(ref r) = result {
        if !r.output.is_empty() {
            return result;
        }
    }

    // Fallback 1: wrap source in a function body and re-compile.
    // This handles bare `let` bindings and expressions that aren't valid
    // top-level declarations but are valid inside a function body.
    let wrapped = format!("fn __expr__() = {{\n    {}\n}}", source);
    let fallback_options = CompileOptions {
        target: options.target.clone(),
        ..Default::default()
    };
    let fallback = compiler.compile(&wrapped, filename, fallback_options);

    if let Ok(ref r) = fallback {
        if !r.output.is_empty() {
            return fallback;
        }
    }

    // Fallback 2: strip type annotations from let bindings and wrap again.
    // e.g. `let x: Int = 42` → `let x = 42` (valid inside a function body)
    let sanitized = sanitize_let_annotations(source);
    let wrapped2 = format!("fn __expr__() = {{\n    {}\n}}", sanitized);
    let fallback2_options = CompileOptions {
        target: options.target.clone(),
        ..Default::default()
    };
    let fallback2 = compiler.compile(&wrapped2, filename, fallback2_options);

    match fallback2 {
        Ok(ref r) if !r.output.is_empty() => fallback2,
        _ => result,
    }
}

/// Handle `dwarf_compile` — compile Dwarf source to a target language.
fn handle_dwarf_compile(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
) -> std::result::Result<CallToolResult, CallToolError> {
    let source = match get_arg(args, "source") {
        Some(s) => s,
        None => {
            return Ok(make_error_result(
                "Missing required argument: source",
            ))
        }
    };
    let target = match get_arg(args, "target") {
        Some(t) => t,
        None => {
            return Ok(make_error_result(
                "Missing required argument: target",
            ))
        }
    };
    let filename = get_arg(args, "filename").unwrap_or_else(|| "input.kzd".to_string());

    let compiler = DwarfCompiler::new();
    let options = CompileOptions {
        target,
        ..Default::default()
    };

    match compile_or_wrap(&compiler, &source, &filename, options) {
        Ok(result) => {
            let diags: Vec<serde_json::Value> = result
                .diagnostics
                .into_iter()
                .map(|d| {
                    serde_json::json!({
                        "code": d.code,
                        "severity": d.severity.to_string(),
                        "message": d.message,
                        "file": d.file,
                        "line": d.line,
                        "col": d.col,
                    })
                })
                .collect();

            Ok(make_ok_result(serde_json::json!({
                "output": result.output,
                "diagnostics": diags,
            })))
        }
        Err(errs) => {
            let error_msg = errs
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            Ok(make_error_result(&error_msg))
        }
    }
}

/// Handle `dwarf_format` — format Dwarf source code.
fn handle_dwarf_format(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
) -> CallToolResult {
    let source = match get_arg(args, "source") {
        Some(s) => s,
        None => return make_error_result("Missing required argument: source"),
    };

    // Basic formatting: trim lines, normalize whitespace, collapse runs of spaces.
    let formatted = basic_format(&source);

    make_ok_result(serde_json::json!({ "formatted": formatted }))
}

/// Basic formatting pass: trim each line and collapse multiple whitespace
/// characters to a single space while preserving non-whitespace content.
fn basic_format(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                String::new()
            } else {
                let mut result = String::with_capacity(trimmed.len());
                let mut prev_was_space = false;
                for ch in trimmed.chars() {
                    if ch.is_whitespace() && ch != '\n' {
                        if !prev_was_space {
                            result.push(' ');
                            prev_was_space = true;
                        }
                    } else {
                        result.push(ch);
                        prev_was_space = false;
                    }
                }
                result
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Handle `dwarf_generate_tests` — generate edge case test values from type definitions.
fn handle_dwarf_generate_tests(
    args: &Option<serde_json::Map<String, serde_json::Value>>,
) -> CallToolResult {
    let source = match get_arg(args, "source") {
        Some(s) => s,
        None => return make_error_result("Missing required argument: source"),
    };

    // Parse the source to extract type definitions
    let parser = ParsePass;
    let (decls, _errors) = match parser.parse(&source) {
        Ok(d) => d,
        Err(e) => {
            return make_error_result(&format!("Parse error: {}", e));
        }
    };

    // Extract types from declarations
    let types = extract_types_from_decls(&decls);

    // Generate edge cases for each type
    let mut all_tests: Vec<serde_json::Value> = Vec::new();
    for (name, ty) in &types {
        let cases = generate_edge_cases(ty);
        for tc in cases {
            all_tests.push(serde_json::json!({
                "type_name": name,
                "description": tc.description,
                "value": literal_value_to_json(&tc.value),
            }));
        }
    }

    make_ok_result(serde_json::json!({ "tests": all_tests }))
}

/// Convert a `LiteralValue` to a `serde_json::Value` for serialization.
fn literal_value_to_json(value: &LiteralValue) -> serde_json::Value {
    match value {
        LiteralValue::Int(n) => serde_json::json!({ "Int": n }),
        LiteralValue::Float(f) => serde_json::json!({ "Float": f }),
        LiteralValue::Str(s) => serde_json::json!({ "String": s }),
        LiteralValue::RawStr(s) => serde_json::json!({ "RawString": s }),
        LiteralValue::Bool(b) => serde_json::json!({ "Bool": b }),
        LiteralValue::Null => serde_json::Value::Null,
    }
}

/// Extract type definitions from parsed HIR declarations.
///
/// Converts `RecordDef` and `UnionDef` declarations into their corresponding
/// `Type` representations so they can be passed to `generate_edge_cases`.
fn extract_types_from_decls(decls: &[Decl]) -> Vec<(String, Type)> {
    let mut types = Vec::new();
    for decl in decls {
        match decl {
            Decl::TypeDef { name, type_, .. } => {
                types.push((name.clone(), type_.clone()));
            }
            Decl::RecordDef { name, fields, .. } => {
                let record_type = Type::Record(
                    fields
                        .iter()
                        .map(|f| (f.name.clone(), Box::new(f.type_.clone())))
                        .collect(),
                );
                types.push((name.clone(), record_type));
            }
            Decl::UnionDef { name, variants, .. } => {
                let union_type = Type::Union(
                    variants
                        .iter()
                        .map(|v| {
                            if let Some(arg) = &v.arg {
                                Type::Generic {
                                    base: v.name.clone(),
                                    args: vec![arg.clone()],
                                }
                            } else {
                                Type::Named(v.name.clone())
                            }
                        })
                        .collect(),
                );
                types.push((name.clone(), union_type));
            }
            _ => {}
        }
    }
    types
}

// ---------------------------------------------------------------------------
// ServerHandler trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ServerHandler for DwarfMcpHandler {
    /// Handle the MCP initialize handshake.
    ///
    /// Returns server metadata including protocol version, capabilities,
    /// and server info (name + version).  Stores the client details on the
    /// runtime so that `is_initialized()` returns `true` after the handshake.
    async fn handle_initialize_request(
        &self,
        params: InitializeRequestParams,
        runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<InitializeResult, RpcError> {
        // Persist client details so the runtime knows the session is initialised.
        // Without this, the SDK's error-handling path (which checks
        // `is_initialized()`) will drop `RpcError` responses instead of
        // forwarding them to the client.
        runtime
            .set_client_details(params)
            .await
            .map_err(|e| RpcError::internal_error().with_message(e.to_string()))?;

        Ok(InitializeResult {
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
        })
    }

    /// Handle `resources/list` — return all known Dwarf language resources.
    async fn handle_list_resources_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        Ok(ListResourcesResult {
            resources: all_resources(),
            next_cursor: None,
            meta: None,
        })
    }

    /// Handle `resources/read` — return the markdown content for a resource URI.
    ///
    /// Returns a JSON-RPC error with code `-32602` when the URI is unknown.
    async fn handle_read_resource_request(
        &self,
        params: ReadResourceRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        match resource_content(&params.uri) {
            Some(text) => Ok(ReadResourceResult {
                contents: vec![ReadResourceContent::TextResourceContents(TextResourceContents {
                    uri: params.uri,
                    mime_type: Some("text/markdown".to_string()),
                    text: text.to_string(),
                    meta: None,
                })],
                meta: None,
            }),
            None => Err(RpcError {
                code: -32602,
                message: format!("Resource not found: {}", params.uri),
                data: None,
            }),
        }
    }

    /// Handle `tools/list` — return all known Dwarf compiler tools.
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: all_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    /// Handle `tools/call` — dispatch tool calls to the appropriate compiler logic.
    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn rust_mcp_sdk::McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        match params.name.as_str() {
            "dwarf_check" => Ok(handle_dwarf_check(&params.arguments)),
            "dwarf_compile" => handle_dwarf_compile(&params.arguments),
            "dwarf_format" => Ok(handle_dwarf_format(&params.arguments)),
            "dwarf_generate_tests" => Ok(handle_dwarf_generate_tests(&params.arguments)),
            _ => Err(CallToolError::unknown_tool(format!(
                "Unknown tool: {}",
                params.name
            ))),
        }
    }
}
