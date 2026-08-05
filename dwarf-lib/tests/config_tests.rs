//! Integration tests for `CompilerConfig` — project config loading and CLI merging.
//!
//! These tests define the expected API contract for the config subsystem:
//!
//! - `CompilerConfig::from_json(json)` — parse config from a JSON string
//! - `CompilerConfig::from_file(path)` — load config from a `dwarf.conf.json` file
//! - `CompilerConfig::merge_with_cli(options)` — produce `CompileOptions` with CLI
//!   values overriding config defaults
//!
//! All three methods do **not exist yet**. These tests will fail to compile
//! in the current Red phase and serve as the specification for implementation.

use dwarf_lib::{CompileOptions, CompilerConfig, DwarfError};
use std::io::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary directory for filesystem-based tests.
fn test_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Write content to a file within a temp directory, returning the canonical path.
fn write_file(dir: &tempfile::TempDir, rel_path: &str, content: &str) -> std::path::PathBuf {
    let full_path = dir.path().join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directories");
    }
    let mut file = std::fs::File::create(&full_path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write content");
    full_path
}

// ===========================================================================
// Config from JSON
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 1: Parse a full config from JSON with all fields
// ---------------------------------------------------------------------------

#[test]
fn config_from_json_full() {
    let json = r#"{
        "name": "test",
        "version": "1.0",
        "targets": ["ts", "debug"],
        "out_dir": "build",
        "pretty": true,
        "skip_passes": ["typecheck"]
    }"#;

    let config = CompilerConfig::from_json(json).expect("Failed to parse valid config JSON");

    assert_eq!(
        config.name,
        Some("test".to_string()),
        "name should be Some(\"test\")"
    );
    assert_eq!(
        config.version,
        Some("1.0".to_string()),
        "version should be Some(\"1.0\")"
    );
    assert_eq!(
        config.targets,
        vec!["ts".to_string(), "debug".to_string()],
        "targets should be [\"ts\", \"debug\"]"
    );
    assert_eq!(config.out_dir, "build", "out_dir should be \"build\"");
    assert!(config.pretty, "pretty should be true");
    assert_eq!(
        config.skip_passes,
        vec!["typecheck".to_string()],
        "skip_passes should be [\"typecheck\"]"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Parse a partial config — missing fields use serde defaults
// ---------------------------------------------------------------------------

#[test]
fn config_from_json_partial() {
    let json = r#"{"name": "minimal"}"#;

    let config = CompilerConfig::from_json(json).expect("Failed to parse partial config JSON");

    assert_eq!(
        config.name,
        Some("minimal".to_string()),
        "name should be Some(\"minimal\")"
    );
    assert_eq!(config.version, None, "version should be None when absent");
    assert_eq!(
        config.targets,
        vec!["ts".to_string()],
        "targets should default to [\"ts\"]"
    );
    assert_eq!(config.out_dir, "dist", "out_dir should default to \"dist\"");
    assert!(!config.pretty, "pretty should default to false");
    assert!(
        config.skip_passes.is_empty(),
        "skip_passes should default to empty vec"
    );
}

// ===========================================================================
// Config from file
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 3: Load config from a valid JSON file on disk
// ---------------------------------------------------------------------------

#[test]
fn config_from_file() {
    let dir = test_dir();
    let path = write_file(
        &dir,
        "dwarf.conf.json",
        r#"{"name": "file-test", "targets": ["debug"], "pretty": true}"#,
    );

    let config = CompilerConfig::from_file(path.to_str().expect("valid UTF-8 path"))
        .expect("Failed to load config from file");

    assert_eq!(
        config.name,
        Some("file-test".to_string()),
        "name should be Some(\"file-test\")"
    );
    assert_eq!(
        config.targets,
        vec!["debug".to_string()],
        "targets should be [\"debug\"]"
    );
    assert!(config.pretty, "pretty should be true");
    // Unset fields should use defaults.
    assert_eq!(config.out_dir, "dist", "out_dir should default to \"dist\"");
}

// ---------------------------------------------------------------------------
// Test 4: Load config from a non-existent file returns an Io error
// ---------------------------------------------------------------------------

#[test]
fn config_from_file_missing() {
    let result = CompilerConfig::from_file("/nonexistent/path/dwarf.conf.json");

    match result {
        Err(DwarfError::Io(msg)) => {
            assert!(!msg.is_empty(), "IO error message should not be empty");
        }
        other => panic!(
            "Expected Err(DwarfError::Io(...)) for missing file, got: {:?}",
            other
        ),
    }
}

// ===========================================================================
// Error handling
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 5: Invalid JSON produces a Config error
// ---------------------------------------------------------------------------

#[test]
fn config_invalid_json() {
    let result = CompilerConfig::from_json("{invalid}");

    match result {
        Err(DwarfError::Config(msg)) => {
            assert!(!msg.is_empty(), "Config error message should not be empty");
        }
        other => panic!(
            "Expected Err(DwarfError::Config(...)) for invalid JSON, got: {:?}",
            other
        ),
    }
}

// ===========================================================================
// CLI merge
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 6: CLI options override config values when merging
// ---------------------------------------------------------------------------

#[test]
fn config_merge_with_cli() {
    // Config specifies multiple targets, pretty off.
    let config = CompilerConfig {
        name: Some("merge-test".to_string()),
        version: None,
        targets: vec!["ts".to_string(), "debug".to_string()],
        out_dir: "dist".to_string(),
        pretty: false,
        skip_passes: vec![],
        stdlib_path: None,
        test_coverage: Default::default(),
    };

    // CLI overrides: pick one target and enable pretty.
    let cli_options = CompileOptions {
        target: "debug".to_string(),
        pretty: true,
        passes: None,
        skip_passes: vec![],
        source_map: false,
        stdlib_path: None,
        ..Default::default()
    };

    let merged = config.merge_with_cli(&cli_options);

    // CLI target should win over config's target list.
    assert_eq!(
        merged.target, "debug",
        "CLI target 'debug' should override config defaults"
    );
    // CLI pretty should win over config's pretty=false.
    assert!(
        merged.pretty,
        "CLI pretty=true should override config pretty=false"
    );
    // CLI skip_passes should be preserved (empty vec in this test).
    assert!(
        merged.skip_passes.is_empty(),
        "CLI skip_passes should be preserved when set"
    );
    // CLI passes should be preserved (None in this test).
    assert!(
        merged.passes.is_none(),
        "CLI passes should be preserved when set"
    );
}
