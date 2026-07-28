//! Dwarf compiler library — programmatic API for compiling Dwarf source code.
//!
//! This crate wraps the full compiler pipeline (lexer → parser → typecheck →
//! MIR → LIR → emitter) into a reusable library API. It also handles
//! project configuration via `dwarf.conf.json`.

mod pipeline;
pub mod resolver;
pub use resolver::{FilesystemResolver, ModuleResolver, PureResolver};

/// The main entry point for the Dwarf compiler.
///
/// Orchestrates the full compilation pipeline and returns emitted output
/// along with diagnostics and metrics.
pub struct DwarfCompiler {
    // TODO: Add internal state in Phase 1 implementation
}

impl DwarfCompiler {
    /// Create a new compiler instance with default options.
    pub fn new() -> Self {
        Self {}
    }

    /// Create a compiler instance configured from a project config file.
    pub fn from_config(config: CompilerConfig) -> Self {
        let _ = config;
        Self {}
    }

    /// Compile a single source string, returning the result.
    ///
    /// This is the primary compilation entry point. It runs the full pipeline
    /// and returns emitted code, diagnostics, and metrics.
    pub fn compile(
        &self,
        source: &str,
        filename: &str,
        options: CompileOptions,
    ) -> Result<CompileResult, Vec<DwarfError>> {
        // Run the pipeline
        let (lir, mut diagnostics) = match pipeline::run_pipeline(source, filename, &options) {
            Ok(result) => result,
            Err(msg) => {
                return Err(vec![DwarfError::Compilation(vec![Diagnostic {
                    code: "DWARF-E-PIPE-0001".to_string(),
                    severity: Severity::Error,
                    message: msg,
                    file: Some(filename.to_string()),
                    line: None,
                    col: None,
                }])]);
            }
        };

        // Select backend
        let mut backend = match pipeline::select_backend(&options.target) {
            Ok(b) => b,
            Err(msg) => {
                diagnostics.push(Diagnostic {
                    code: "DWARF-E-EMIT-0002".to_string(),
                    severity: Severity::Error,
                    message: msg,
                    file: Some(filename.to_string()),
                    line: None,
                    col: None,
                });
                // Return what we have so far — diagnostics include the error
                return Ok(CompileResult {
                    output: String::new(),
                    diagnostics,
                    output_extension: "txt".to_string(),
                    source_map: None,
                });
            }
        };

        // Emit — with optional source map generation
        let (output, source_map_json) = if options.source_map {
            backend
                .emit_module_with_source_map(&lir, filename, source)
                .unwrap_or_else(|_| (String::new(), None))
        } else {
            backend
                .emit_module(&lir)
                .map(|o| (o, None))
                .unwrap_or_else(|_| (String::new(), None))
        };

        // Also handle emission errors that came through the non-source-map path
        // (if source_map was false and emit_module failed, we get empty string).

        let ext = match options.target.as_str() {
            "ts" => "ts",
            "py" => "py",
            "java" => "java",
            "debug" => "txt",
            _ => "txt",
        };

        let source_map = source_map_json.map(|sm| sm.to_string());

        Ok(CompileResult {
            output,
            diagnostics,
            output_extension: ext.to_string(),
            source_map,
        })
    }
}

impl Default for DwarfCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a single compilation.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct CompileOptions {
    /// Target language to emit (e.g., "ts", "debug").
    pub target: String,
    /// Whether to apply pretty formatting.
    pub pretty: bool,
    /// Optional list of passes to run. If None, run all.
    pub passes: Option<Vec<String>>,
    /// Pass names to skip.
    pub skip_passes: Vec<String>,
    /// Whether to generate a source map alongside the output.
    #[serde(default)]
    pub source_map: bool,
    /// Path to the standard library runtime directory.
    /// If None, the compiler searches default locations.
    #[serde(default)]
    pub stdlib_path: Option<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: Vec::new(),
            source_map: false,
            stdlib_path: None,
        }
    }
}

/// The result of a single compilation.
#[derive(Debug)]
pub struct CompileResult {
    /// The emitted code (per target).
    pub output: String,
    /// Diagnostics produced during compilation.
    pub diagnostics: Vec<Diagnostic>,
    /// File extension for the emitted output.
    pub output_extension: String,
    /// Optional JSON source map (if requested via options.source_map).
    pub source_map: Option<String>,
}

/// A diagnostic message produced during compilation.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

/// Severity of a diagnostic.
#[derive(Clone, Debug)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// Project configuration (`dwarf.conf.json`).
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CompilerConfig {
    /// Project name.
    pub name: Option<String>,
    /// Project version.
    pub version: Option<String>,
    /// Target languages to emit.
    #[serde(default = "default_targets")]
    pub targets: Vec<String>,
    /// Output directory.
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
    /// Whether to pretty-print output.
    #[serde(default)]
    pub pretty: bool,
    /// Pass names to skip.
    #[serde(default)]
    pub skip_passes: Vec<String>,
    /// Path to the standard library runtime directory.
    #[serde(default)]
    pub stdlib_path: Option<String>,
}

fn default_targets() -> Vec<String> {
    vec!["ts".to_string()]
}

fn default_out_dir() -> String {
    "dist".to_string()
}

impl CompilerConfig {
    /// Load configuration from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, DwarfError> {
        serde_json::from_str(json)
            .map_err(|e| DwarfError::Config(format!("Failed to parse config: {}", e)))
    }

    /// Load configuration from a JSON file.
    pub fn from_file(path: &str) -> Result<Self, DwarfError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| DwarfError::Io(format!("Cannot read config file '{}': {}", path, e)))?;
        Self::from_json(&content)
    }

    /// Merge this config with CLI-provided options.
    ///
    /// CLI options take precedence over config file values.
    pub fn merge_with_cli(self, options: &CompileOptions) -> CompileOptions {
        CompileOptions {
            // CLI target overrides config's first target
            target: if !options.target.is_empty() && options.target != "ts" {
                options.target.clone()
            } else if !self.targets.is_empty() {
                self.targets[0].clone()
            } else {
                options.target.clone()
            },
            pretty: options.pretty || self.pretty,
            passes: options.passes.clone(),
            skip_passes: if !options.skip_passes.is_empty() {
                options.skip_passes.clone()
            } else {
                self.skip_passes.clone()
            },
            source_map: options.source_map,
            stdlib_path: if options.stdlib_path.is_some() {
                options.stdlib_path.clone()
            } else {
                self.stdlib_path.clone()
            },
        }
    }
}

/// Top-level error type for the compiler library.
#[derive(Debug)]
pub enum DwarfError {
    /// A compilation error with diagnostics.
    Compilation(Vec<Diagnostic>),
    /// A configuration error.
    Config(String),
    /// An I/O error.
    Io(String),
}

impl std::fmt::Display for DwarfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DwarfError::Compilation(diags) => {
                for d in diags {
                    writeln!(f, "[{}] {}: {}", d.code, d.severity, d.message)?;
                }
                Ok(())
            }
            DwarfError::Config(msg) => write!(f, "Config error: {}", msg),
            DwarfError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for DwarfError {}

#[cfg(test)]
mod tests {
    use super::*;

    // WILL FAIL — RED PHASE: CompileOptions does not yet have a stdlib_path field
    #[test]
    fn test_compile_options_stdlib_path_default() {
        let opts = CompileOptions::default();
        // stdlib_path should default to None (system will search default paths)
        assert!(opts.stdlib_path.is_none());
    }
}
