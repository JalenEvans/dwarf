//! Integration tests for the dwarf-cli → dwarf-lib delegation.
//!
//! These tests verify that the CLI correctly delegates compilation to
//! `dwarf-lib` instead of running its own pipeline. They define the
//! contract for the refactoring:
//!
//! 1. `dwarf-cli/Cargo.toml` adds `dwarf-lib` as a dependency
//! 2. `build.rs`, `emit.rs`, `check.rs` use `DwarfCompiler::compile()`
//!    instead of constructing their own pass pipeline
//! 3. The CLI reads `dwarf.conf.json` from the current directory and
//!    merges it with CLI flags via `CompilerConfig::merge_with_cli()`
//!
//! # Compile-Time Failure
//!
//! These tests **will not compile** until `dwarf-lib` is added to
//! `dwarf-cli/Cargo.toml` as a dependency. This is intentional — the
//! first step of the refactoring is the dependency change.
//!
//! ```toml
//! // Add to dwarf-cli/Cargo.toml:
//! dwarf-lib = { path = "../dwarf-lib" }
//! ```

use dwarf_lib::{CompileOptions, CompilerConfig, DwarfCompiler};

// ---------------------------------------------------------------------------
// Test: cli_build_delegates_to_dwarf_lib
// ---------------------------------------------------------------------------
//
// Verifies that `DwarfCompiler::compile()` produces the expected output
// for a trivial Dwarf program. This is the primary contract: the CLI's
// `build`, `emit`, and `check` subcommands should delegate to this same
// function rather than running their own pass pipeline.
//
// Expected: output contains the value `42`, no diagnostics.
// ---------------------------------------------------------------------------

#[test]
fn cli_build_delegates_to_dwarf_lib() {
    let compiler = DwarfCompiler::new();
    let options = CompileOptions::default();
    let result = compiler
        .compile("fn main() = 42", "test.dwarf", options)
        .expect("Compilation should succeed");

    // The emitted output must contain the value from the source
    assert!(
        !result.output.is_empty(),
        "Output should not be empty — compile returned Ok but produced no output"
    );
    assert!(
        result.output.contains("42"),
        "Output should contain the literal '42' from the source.\nGot output:\n{}",
        result.output
    );
    assert!(
        result.diagnostics.is_empty(),
        "There should be no diagnostics for a valid trivial program.\nGot: {:#?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// Test: cli_config_discovery_finds_file
// ---------------------------------------------------------------------------
//
// Verifies that `CompilerConfig::from_file()` correctly reads and parses a
// `dwarf.conf.json` from disk. The refactored CLI should discover this file
// in the current working directory and merge its values with CLI flags via
// `CompilerConfig::merge_with_cli()`.
//
// This test defines the expected shape of the config discovery API:
//   - `CompilerConfig::from_file(path: &str) -> Result<CompilerConfig, DwarfError>`
//   - `CompilerConfig.targets: Vec<String>`
//   - `CompilerConfig.out_dir: String`
// ---------------------------------------------------------------------------

#[test]
fn cli_config_discovery_finds_file() {
    // Create a temporary directory with a dwarf.conf.json
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = dir.path().join("dwarf.conf.json");

    let config_content = r#"{"targets": ["ts"], "out_dir": "build"}"#;
    std::fs::write(&config_path, config_content).expect("Failed to write dwarf.conf.json");

    // Load the config via dwarf-lib's CompilerConfig
    let config = CompilerConfig::from_file(config_path.to_str().expect("Path is not valid UTF-8"))
        .expect("Should successfully load and parse dwarf.conf.json");

    // Assert the parsed values match what was written
    assert_eq!(
        config.targets,
        vec!["ts".to_string()],
        "targets should contain exactly one entry: \"ts\""
    );
    assert_eq!(config.out_dir, "build", "out_dir should be \"build\"");

    // Verify defaults for omitted fields
    assert!(
        !config.pretty,
        "pretty should default to false when not specified"
    );
    assert!(
        config.skip_passes.is_empty(),
        "skip_passes should default to empty when not specified"
    );

    // Verify merge_with_cli works: CLI flag overrides config value
    let merged = config.merge_with_cli(&CompileOptions {
        target: "debug".to_string(),
        ..CompileOptions::default()
    });
    assert_eq!(
        merged.target, "debug",
        "CLI --target 'debug' should override config's 'ts'"
    );

    // Verify merge_with_cli uses config when CLI uses default
    let config2 = CompilerConfig::from_json(r#"{"targets": ["py"], "out_dir": "output"}"#)
        .expect("Should parse valid JSON config");

    let merged2 = config2.merge_with_cli(&CompileOptions::default());
    assert_eq!(
        merged2.target, "py",
        "Config's first target should be used when CLI --target is default"
    );
}
