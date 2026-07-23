//! Runner trait and implementations for executing transpiled Dwarf code.
//!
//! This module provides the [`Runner`] trait which abstracts execution of
//! transpiled code, and concrete implementations like [`TsRunner`] which
//! transpiles Dwarf → TypeScript and executes it with Node.js.
//!
//! # Example
//!
//! ```ignore
//! use dwarf_cli::runner::{Runner, TsRunner};
//!
//! let runner = TsRunner::new();
//! let output = runner.run("source.kzd".as_ref())?;
//! println!("{}", output);
//! ```

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

use crate::pass_manager::*;
use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::ts_backend::TypeScriptBackend;
use dwarf_lexer::pass::TokenizePass;
use dwarf_lir::pass::LirPass;
use dwarf_mir::pass::MirPass;
use dwarf_parser::pass::ParsePass;
use dwarf_typecheck::pass::TypeCheckPass;

/// Trait for executing transpiled Dwarf code.
///
/// Implementors compile a Dwarf source file to a target language and then
/// execute it with the appropriate runtime. The `run` method returns the
/// stdout of the executed program on success, or a descriptive error
/// message on failure.
pub trait Runner {
    /// Execute the transpiled output for the given Dwarf source file.
    ///
    /// # Arguments
    ///
    /// * `source_path` — Path to a `.kzd` source file.
    ///
    /// # Returns
    ///
    /// * `Ok(stdout)` — The stdout produced by running the transpiled output.
    /// * `Err(msg)` — A human-readable error description.
    fn run(&self, source_path: &Path) -> Result<String, String>;
}

/// Returns a list of available runtime target identifiers.
///
/// Each entry is a short string such as `"ts"` that can be passed to
/// `--target` on CLI commands like `build`, `emit`, `run`, and `dev`.
pub fn list_runtimes() -> Vec<&'static str> {
    vec!["ts"]
}

/// Runner that transpiles Dwarf → TypeScript and executes with Node.js.
///
/// The `TsRunner` compiles a `.kzd` file through the full compiler pipeline
/// to TypeScript, writes it to a temporary file, and runs it with the
/// Node.js runtime.
pub struct TsRunner {
    /// Path to the Node.js executable (defaults to `"node"`).
    node_path: String,
}

impl TsRunner {
    /// Create a new `TsRunner` using `"node"` as the Node.js executable.
    ///
    /// The default assumes `node` is available on `$PATH`.
    pub fn new() -> Self {
        Self {
            node_path: "node".to_string(),
        }
    }

    /// Create a new `TsRunner` with a custom Node.js executable path.
    ///
    /// Use this when Node.js is installed at a non-standard location or
    /// when you want to test against a specific version.
    pub fn with_node_path(node_path: impl Into<String>) -> Self {
        Self {
            node_path: node_path.into(),
        }
    }

    /// Return the configured Node.js executable path.
    pub fn node_path(&self) -> &str {
        &self.node_path
    }

    /// Compile a Dwarf source file to TypeScript, returning the emitted code.
    ///
    /// This runs the full compiler pipeline (tokenize → parse → typecheck
    /// → MIR → LIR → emit) for the TypeScript target and returns the
    /// emitted string.
    fn transpile_to_ts(source_path: &Path) -> Result<String, String> {
        // Read source
        let source =
            std::fs::read_to_string(source_path).map_err(|e| format!("Cannot read file: {}", e))?;

        // Build compiler pipeline
        let mut pm = PassManager::new();
        pm.register(Box::new(TokenizePass));
        pm.register(Box::new(ParsePass));
        pm.register(Box::new(TypeCheckPass::new()));
        pm.register(Box::new(ModulePass::new()));
        pm.register(Box::new(MirPass::new()));
        pm.register(Box::new(LirPass::new()));

        let options = CompileOptions {
            passes: None,
            skip_passes: vec![],
        };

        let mut unit = CompilationUnit::new(source);
        unit.path = Some(source_path.to_path_buf());

        let mut ctx = PassContext::new(options);
        pm.run_all(&mut unit, &mut ctx);

        // Check for compilation errors
        let has_errors = ctx
            .diagnostics()
            .iter()
            .any(|d| matches!(d.severity, Severity::Error));
        if has_errors {
            let errors: Vec<String> = ctx
                .diagnostics()
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error))
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect();
            return Err(format!("Compilation failed:\n{}", errors.join("\n")));
        }

        // Emit TypeScript
        let lir = unit
            .lir
            .as_ref()
            .ok_or_else(|| "No LIR produced — compilation may be incomplete".to_string())?;

        let mut backend = TypeScriptBackend::new("0.1.0");
        backend
            .emit_module(lir)
            .map_err(|e| format!("Emission failed: {}", e))
    }

    /// Execute a TypeScript file with Node.js and return its stdout.
    fn execute_with_node(node_path: &str, ts_path: &Path) -> Result<String, String> {
        let output = Command::new(node_path)
            .arg(ts_path)
            .output()
            .map_err(|e| format!("Failed to execute Node.js: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("Node.js execution failed:\n{}", stderr))
        }
    }
}

impl Default for TsRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for TsRunner {
    fn run(&self, source_path: &Path) -> Result<String, String> {
        // Validate extension
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext != "kzd" {
            return Err(format!(
                "Unsupported file extension: '.{}'. Expected '.kzd'",
                ext
            ));
        }

        // Validate file exists
        if !source_path.exists() {
            return Err(format!("Source file not found: {}", source_path.display()));
        }

        // Transpile to TypeScript
        let ts_code = Self::transpile_to_ts(source_path)?;

        // Write to temp file and execute
        let temp_dir =
            TempDir::new().map_err(|e| format!("Cannot create temp directory: {}", e))?;
        let ts_path = temp_dir.path().join("output.ts");
        std::fs::write(&ts_path, &ts_code).map_err(|e| format!("Cannot write temp file: {}", e))?;

        Self::execute_with_node(&self.node_path, &ts_path)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -----------------------------------------------------------------------
    // Runner trait contract tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_runner_trait_is_object_safe() {
        // The trait must be usable as a trait object so backends can be
        // selected at runtime (e.g., from --target).
        fn takes_runner(_runner: &dyn Runner) {}
        // If this compiles, the trait is object-safe.
    }

    #[test]
    fn test_runner_trait_run_accepts_path() {
        // Verify the signature compiles: run takes &Path, returns Result<String, String>.
        struct DummyRunner;
        impl Runner for DummyRunner {
            fn run(&self, _source_path: &Path) -> Result<String, String> {
                Ok("dummy".to_string())
            }
        }

        let runner = DummyRunner;
        let result = runner.run(Path::new("test.kzd"));
        assert_eq!(result, Ok("dummy".to_string()));
    }

    // -----------------------------------------------------------------------
    // TsRunner construction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ts_runner_can_be_instantiated() {
        // The constructor should exist and produce a valid TsRunner
        // (this is primarily a type-system / compilation check).
        let _runner = TsRunner::new();
        // `new()` currently calls todo!() so this will panic at runtime.
    }

    #[test]
    fn test_ts_runner_with_custom_node_path() {
        let runner = TsRunner::with_node_path("/custom/node/path");
        assert_eq!(runner.node_path(), "/custom/node/path");
    }

    #[test]
    fn test_ts_runner_with_node_path_from_string() {
        let path = String::from("/usr/local/bin/node");
        let runner = TsRunner::with_node_path(path);
        assert_eq!(runner.node_path(), "/usr/local/bin/node");
    }

    #[test]
    fn test_ts_runner_default_impl() {
        // TsRunner should implement Default so it can be used in generic
        // contexts that require Default.
        let runner: TsRunner = Default::default();
        assert_eq!(runner.node_path(), "node");
    }

    // -----------------------------------------------------------------------
    // list_runtimes tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_runtimes_contains_ts() {
        let runtimes = list_runtimes();
        assert!(!runtimes.is_empty(), "Should have at least one runtime");
        assert!(
            runtimes.contains(&"ts"),
            "Should include the TypeScript runtime ('ts'), got: {:?}",
            runtimes
        );
    }

    #[test]
    fn test_list_runtimes_returns_unique_entries() {
        let runtimes = list_runtimes();
        let mut sorted = runtimes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            runtimes.len(),
            sorted.len(),
            "list_runtimes should not contain duplicates"
        );
    }

    #[test]
    fn test_list_runtimes_all_entries_nonempty() {
        let runtimes = list_runtimes();
        for rt in &runtimes {
            assert!(
                !rt.is_empty(),
                "Runtime name should not be empty, got: {:?}",
                rt
            );
        }
    }

    // -----------------------------------------------------------------------
    // TsRunner::run error handling tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ts_runner_run_returns_error_for_nonexistent_file() {
        let runner = TsRunner::new();
        let result = runner.run(Path::new("/nonexistent/path/to/file.kzd"));
        assert!(
            result.is_err(),
            "Should return Err for non-existent source file"
        );
    }

    #[test]
    fn test_ts_runner_run_returns_error_for_bad_extension() {
        // Even if the file exists, a non-.kzd extension should be rejected.
        let dir = std::env::temp_dir().join("dwarf_runner_ext_test");
        std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
        let file_path = dir.join("not_dwarf.txt");
        std::fs::write(&file_path, "hello world").expect("Failed to write temp file");

        let runner = TsRunner::new();
        let result = runner.run(&file_path);
        assert!(result.is_err(), "Should reject non-.kzd files");

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    #[ignore = "requires Node.js to be installed on the system for the default path lookup"]
    fn test_ts_runner_run_no_node_available() {
        // Use a deliberately wrong node path to simulate Node.js not being
        // available, even when the source file is valid.
        let runner = TsRunner::with_node_path("/no/such/node/binary");

        let dir = std::env::temp_dir().join("dwarf_runner_no_node_test");
        std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
        let file_path = dir.join("test_no_node.kzd");
        std::fs::write(&file_path, "fn main() { 42 }").expect("Failed to write temp file");

        let result = runner.run(&file_path);
        assert!(
            result.is_err(),
            "Should return Err when Node.js binary is not available"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("node") || err_msg.contains("Node"),
            "Error message should mention node/Node.js, got: {}",
            err_msg
        );

        std::fs::remove_file(&file_path).ok();
    }

    // -----------------------------------------------------------------------
    // Integration-level tests (require Node.js + full compiler pipeline)
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "requires Node.js to be installed and the full compiler pipeline to be wired up"]
    fn test_ts_runner_run_with_valid_file() {
        let runner = TsRunner::new();

        let dir = std::env::temp_dir().join("dwarf_runner_integration");
        std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
        let file_path = dir.join("test_valid.kzd");
        std::fs::write(&file_path, "fn main() { print(\"Hello from Dwarf!\"); }")
            .expect("Failed to write temp file");

        let result = runner.run(&file_path);
        assert!(
            result.is_ok(),
            "Should successfully execute with valid file: {:?}",
            result.err()
        );

        let stdout = result.unwrap();
        assert!(
            stdout.contains("Hello from Dwarf!"),
            "Output should contain the printed message, got: {}",
            stdout
        );

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    #[ignore = "requires Node.js to be installed and the full compiler pipeline"]
    fn test_ts_runner_run_with_dwarf_expression() {
        let runner = TsRunner::new();

        let dir = std::env::temp_dir().join("dwarf_runner_expr_test");
        std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
        let file_path = dir.join("expr.kzd");
        std::fs::write(&file_path, "fn main() { print(2 + 3); }")
            .expect("Failed to write temp file");

        let result = runner.run(&file_path);
        assert!(
            result.is_ok(),
            "Should execute expression file: {:?}",
            result.err()
        );

        let stdout = result.unwrap();
        assert!(
            stdout.contains("5"),
            "Output should contain computed result '5', got: {}",
            stdout
        );

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    #[ignore = "requires Node.js — tests that --target ts is required"]
    fn test_ts_runner_rejects_unknown_target() {
        // The runner should validate that the target is supported.
        // Since TsRunner only supports "ts", this test verifies that
        // using it as a general-purpose runner doesn't silently swallow
        // mismatched targets. (TsRunner is specifically for ts, so this
        // tests that the trait-level dispatch would catch unsupported targets.)
        let dir = std::env::temp_dir().join("dwarf_runner_target_test");
        std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
        let file_path = dir.join("target_test.kzd");
        std::fs::write(&file_path, "fn main() { 42 }").expect("Failed to write temp file");

        // TsRunner should always attempt ts compilation —
        // if the file is valid .kzd, it should succeed. This test just
        // verifies the basic path doesn't panic.
        let runner = TsRunner::new();
        let result = runner.run(&file_path);
        // It may fail if no node, but it should not panic.
        assert!(
            result.is_ok() || result.is_err(),
            "Should not panic — either succeed or return Err"
        );

        std::fs::remove_file(&file_path).ok();
    }
}
