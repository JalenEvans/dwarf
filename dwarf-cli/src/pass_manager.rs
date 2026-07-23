//! Pass manager infrastructure for the Dwarf compiler pipeline.

use std::path::PathBuf;
use std::time::Instant;

use dwarf_syntax::hir::Decl;
use dwarf_syntax::token::Token;

/// The result of running a single pass.
pub enum PassResult {
    /// Continue to the next pass.
    Continue,
    /// Halt the pipeline — do not run subsequent passes.
    Halt,
}

/// A single compilation stage in the pipeline.
pub trait Pass {
    /// Short name for CLI flags (e.g., "tokenize", "parse").
    fn name(&self) -> &str;
    /// Human-readable description.
    fn description(&self) -> &str;
    /// Execute this pass on the compilation unit.
    fn run(&self, ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult;
}

/// Holds the source and accumulated outputs as they flow through the pipeline.
pub struct CompilationUnit {
    pub source: String,
    pub path: Option<PathBuf>,
    pub tokens: Option<Vec<Token>>,
    pub decls: Option<Vec<Decl>>,
    pub mir: Option<Vec<dwarf_mir::MirDecl>>,
    pub lir: Option<Vec<dwarf_lir::LirDecl>>,
    pub module_graph: Option<ModuleGraph>,
}

impl CompilationUnit {
    pub fn new(source: String) -> Self {
        CompilationUnit {
            source,
            path: None,
            tokens: None,
            decls: None,
            mir: None,
            lir: None,
            module_graph: None,
        }
    }
}

/// A diagnostic message produced during compilation.
#[derive(Clone)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

/// Severity of a diagnostic.
#[derive(Clone)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Metrics collected for a single pass execution.
pub struct PassMetrics {
    pub name: String,
    pub duration_ms: u64,
    pub input_size: usize,
    pub output_size: usize,
}

/// Global context shared across all passes in a compilation.
pub struct PassContext {
    diagnostics: Vec<Diagnostic>,
    pub metrics: Vec<PassMetrics>,
    pub options: CompileOptions,
}

impl PassContext {
    pub fn new(options: CompileOptions) -> Self {
        PassContext {
            diagnostics: Vec::new(),
            metrics: Vec::new(),
            options,
        }
    }

    pub fn push_diagnostic(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Configuration for which passes to run.
#[derive(Default, Clone)]
pub struct CompileOptions {
    /// If `Some`, only run passes whose names appear in this list.
    pub passes: Option<Vec<String>>,
    /// Pass names to skip.
    pub skip_passes: Vec<String>,
}

/// Orchestrates a sequence of compilation passes.
pub struct PassManager {
    passes: Vec<Box<dyn Pass>>,
}

impl PassManager {
    pub fn new() -> Self {
        PassManager { passes: Vec::new() }
    }

    pub fn register(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    /// Run all registered passes (subject to filtering in `ctx.options`).
    /// Stops early if any pass returns `PassResult::Halt`.
    pub fn run_all(&self, unit: &mut CompilationUnit, ctx: &mut PassContext) {
        for pass in &self.passes {
            // Filter: if passes list is set, only run selected passes
            if let Some(ref passes) = ctx.options.passes {
                if !passes.iter().any(|n| n == pass.name()) {
                    continue;
                }
            }
            // Filter: skip passes in the skip list
            if ctx.options.skip_passes.iter().any(|n| n == pass.name()) {
                continue;
            }

            let start = Instant::now();
            let result = pass.run(ctx, unit);
            let duration = start.elapsed();

            let output_size = if pass.name() == "tokenize" {
                unit.tokens.as_ref().map_or(0, |t| t.len())
            } else if pass.name() == "parse" {
                unit.decls.as_ref().map_or(0, |d| d.len())
            } else if pass.name() == "mir" {
                unit.mir.as_ref().map_or(0, |m| m.len())
            } else if pass.name() == "lir" {
                unit.lir.as_ref().map_or(0, |l| l.len())
            } else {
                0
            };

            ctx.metrics.push(PassMetrics {
                name: pass.name().to_string(),
                duration_ms: duration.as_millis() as u64,
                input_size: unit.source.len(),
                output_size,
            });

            if matches!(result, PassResult::Halt) {
                break;
            }
        }
    }

    /// Return `(name, description)` for all registered passes.
    pub fn list_passes(&self) -> Vec<(&str, &str)> {
        self.passes
            .iter()
            .map(|p| (p.name(), p.description()))
            .collect()
    }
}

impl Default for PassManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Adapt existing passes to the `Pass` trait ----

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::pass::ParsePass;
use dwarf_typecheck::pass::TypeCheckPass;
use dwarf_lir::pass::LirPass;
use dwarf_mir::modules::ModuleGraph;
use dwarf_mir::pass::MirPass;

impl Pass for TokenizePass {
    fn name(&self) -> &str {
        "tokenize"
    }

    fn description(&self) -> &str {
        "Tokenize source text into a stream of tokens"
    }

    fn run(&self, ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        match self.tokenize(&unit.source) {
            Ok(tokens) => {
                unit.tokens = Some(tokens);
                PassResult::Continue
            }
            Err(e) => {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-LEX-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Lexer error: {}", e),
                    file: unit.path.clone(),
                    line: None,
                    col: None,
                });
                PassResult::Halt
            }
        }
    }
}

impl Pass for ParsePass {
    fn name(&self) -> &str {
        "parse"
    }

    fn description(&self) -> &str {
        "Parse tokens into HIR declarations"
    }

    fn run(&self, ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        match self.parse(&unit.source) {
            Ok((decls, parse_errors)) => {
                for err in &parse_errors {
                    let (line, col) =
                        dwarf_syntax::diagnostic::byte_to_line_col(&unit.source, err.span.start)
                            .unwrap_or((0, 0));
                    ctx.push_diagnostic(Diagnostic {
                        code: err.code.to_string(),
                        severity: Severity::Error,
                        message: err.message.clone(),
                        file: unit.path.clone(),
                        line: Some(line),
                        col: Some(col),
                    });
                }
                unit.decls = Some(decls);
                PassResult::Continue
            }
            Err(e) => {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-LEX-0001".to_string(),
                    severity: Severity::Error,
                    message: e,
                    file: unit.path.clone(),
                    line: None,
                    col: None,
                });
                PassResult::Halt
            }
        }
    }
}

impl Pass for TypeCheckPass {
    fn name(&self) -> &str {
        "typecheck"
    }

    fn description(&self) -> &str {
        "Check types and infer expressions"
    }

    fn run(&self, ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        if let Some(decls) = &unit.decls {
            let (_registry, errors) = self.check(decls);
            for err in &errors {
                let (line, col) =
                    dwarf_syntax::diagnostic::byte_to_line_col(&unit.source, err.span.start)
                        .unwrap_or((0, 0));
                ctx.push_diagnostic(Diagnostic {
                    code: err.code.to_string(),
                    severity: Severity::Error,
                    message: err.message.clone(),
                    file: unit.path.clone(),
                    line: Some(line),
                    col: Some(col),
                });
            }
        }
        PassResult::Continue
    }
}

impl Pass for MirPass {
    fn name(&self) -> &str {
        "mir"
    }

    fn description(&self) -> &str {
        "Desugar HIR into MIR — pipes, propagation, for-loops, decorators, type aliases"
    }

    fn run(&self, _ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        if let Some(decls) = &unit.decls {
            let mir = MirPass::run(self, decls);
            unit.mir = Some(mir);
        }
        PassResult::Continue
    }
}

/// Pass that builds the module dependency graph from HIR declarations.
///
/// Runs after parsing and before type-checking to detect circular imports
/// and make the dependency graph available for downstream passes.
pub struct ModulePass;

impl ModulePass {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ModulePass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for ModulePass {
    fn name(&self) -> &str {
        "modules"
    }

    fn description(&self) -> &str {
        "Resolve module imports and build dependency graph"
    }

    fn run(&self, ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        if let Some(ref decls) = unit.decls {
            let graph = ModuleGraph::build(decls);
            if graph.has_cycle() {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-MOD-0001".to_string(),
                    severity: Severity::Error,
                    message: "Circular module dependency detected".to_string(),
                    file: unit.path.clone(),
                    line: None,
                    col: None,
                });
            }
            unit.module_graph = Some(graph);
        }
        PassResult::Continue
    }
}

impl Pass for LirPass {
    fn name(&self) -> &str {
        "lir"
    }

    fn description(&self) -> &str {
        "Lower MIR to LIR with target hints and resolve effects"
    }

    fn run(&self, _ctx: &mut PassContext, unit: &mut CompilationUnit) -> PassResult {
        if let Some(ref mir) = unit.mir {
            let lir = self.run(mir);
            unit.lir = Some(lir);
        }
        PassResult::Continue
    }
}
