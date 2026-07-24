//! Integration tests for source map generation.
//!
//! These tests define the expected API for source map support in the
//! `dwarf-emitter` crate. They should fail at compile time until the
//! corresponding types and methods are implemented.
//!
//! # Expected compile errors
//!
//! - `unresolved import dwarf_emitter::sourcemap` — module doesn't exist yet
//! - `no function or associated item named from_json found` — builder method doesn't exist yet
//! - `no method named emit_module_with_sourcemap` — backend method doesn't exist yet
//! - `no method named add_mapping found` — `SourceMapBuilder::add_mapping` doesn't exist yet

use dwarf_emitter::sourcemap::SourceMapBuilder;
use dwarf_emitter::ts::backend::TypeScriptBackend;
use dwarf_lir::{Effect, LirDecl, LirExpr, LirLiteral, TargetHint};
use dwarf_syntax::span::Span;

// ==================================================================
// Test 1: SourceMapBuilder produces a valid source map v3 document
// ==================================================================

#[test]
fn test_sourcemap_builder_creates_valid_map() {
    let mut builder = SourceMapBuilder::new("test.kzd", "fn main() = 42\n");

    // Record a mapping: generated line 0, col 0 → source line 0, col 0
    builder.add_mapping(0, 0, 0, 0, None);
    // Record: generated line 0, col 10 → source line 0, col 11, with name "main"
    builder.add_mapping(0, 10, 0, 11, Some("main"));

    let output = builder.into_json();

    // Source map v3 must have 'version' field set to 3
    assert_eq!(output["version"], 3);
    assert_eq!(output["file"], "test.kzd");
    assert_eq!(output["sources"], serde_json::json!(["test.kzd"]));
    assert_eq!(
        output["sourcesContent"],
        serde_json::json!(["fn main() = 42\n"])
    );
}

// ==================================================================
// Test 2: Source map can be serialized through serde_json
// ==================================================================

#[test]
fn test_source_map_in_compile_result() {
    let mut builder = SourceMapBuilder::new("test.kzd", "source content");
    builder.add_mapping(0, 0, 0, 0, None);
    let map_json = builder.into_json();

    // Verify it survives a serialization round-trip
    let source_map_json = serde_json::to_string(&map_json).unwrap();
    assert!(!source_map_json.is_empty());

    // Parse it back and verify fields
    let parsed: serde_json::Value = serde_json::from_str(&source_map_json).unwrap();
    assert_eq!(parsed["version"], 3);
    assert_eq!(parsed["file"], "test.kzd");
    assert_eq!(parsed["sources"], serde_json::json!(["test.kzd"]));
    assert!(parsed["mappings"].is_string());
}

// ==================================================================
// Test 3: VLQ-encoded mappings string
// ==================================================================

#[test]
fn test_source_map_mappings_vlq() {
    let mut builder = SourceMapBuilder::new("input.kzd", "line1\nline2\n");
    // Mapping for first line
    builder.add_mapping(0, 0, 0, 0, None);
    // Mapping for second line
    builder.add_mapping(1, 0, 1, 0, None);

    let output = builder.into_json();
    let mappings = output["mappings"].as_str().unwrap();
    assert!(!mappings.is_empty(), "mappings should not be empty");
    // Mappings string should follow source map v3 VLQ format:
    // semicolons separate lines, no spaces
    assert!(
        !mappings.contains(' '),
        "mappings should not contain spaces"
    );
    // With two lines mapped, there should be at least one semicolon
    assert!(
        mappings.contains(';') || mappings.len() > 2,
        "mappings for multi-line should contain a semicolon or substantial VLQ data"
    );
}

// ==================================================================
// Test 4: TypeScript emitter produces a source map
// ==================================================================

#[test]
fn test_emitter_produces_source_map() {
    let mut backend = TypeScriptBackend::new("0.1.0");

    // Create a simple function declaration with known source positions
    let decls = vec![LirDecl::Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: TargetHint::None,
            span: Span::new(0, 11, 13), // The "42" literal
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        is_generator: false,
        span: Span::new(0, 0, 16), // The full function
    }];

    // Emit with source map enabled
    let result = backend
        .emit_module_with_sourcemap(&decls, "test.kzd", "fn main() = 42")
        .unwrap();

    // Result should contain both output and source map
    assert!(!result.output.is_empty(), "output should not be empty");
    assert!(result.source_map.is_some(), "source map should be present");

    let source_map = result.source_map.unwrap();
    assert_eq!(source_map["version"], 3);
    assert_eq!(source_map["file"], "test.kzd");
    assert_eq!(source_map["sources"], serde_json::json!(["test.kzd"]));
}
