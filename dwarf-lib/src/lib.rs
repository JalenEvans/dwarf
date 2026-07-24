//! Dwarf compiler library — programmatic API for compiling Dwarf source code.
//!
//! This crate wraps the full compiler pipeline (lexer → parser → typecheck →
//! MIR → LIR → emitter) into a reusable library API. It also handles
//! project configuration via `dwarf.conf.json`.

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
        let _ = (source, filename, options);
        todo!("implement compile() in Phase 1")
    }
}

impl Default for DwarfCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a single compilation.
#[derive(Clone, Debug)]
pub struct CompileOptions {
    /// Target language to emit (e.g., "ts", "debug").
    pub target: String,
    /// Whether to apply pretty formatting.
    pub pretty: bool,
    /// Optional list of passes to run. If None, run all.
    pub passes: Option<Vec<String>>,
    /// Pass names to skip.
    pub skip_passes: Vec<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: Vec::new(),
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
}

fn default_targets() -> Vec<String> {
    vec!["ts".to_string()]
}

fn default_out_dir() -> String {
    "dist".to_string()
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
