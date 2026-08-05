//! Core pipeline orchestration and backend selection.
//!
//! This module contains the pipeline runner that drives the full compilation
//! pipeline (lexer → parser → typecheck → MIR → LIR → emitter) and a
//! backend selector that maps target names to emitter backends.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::debug_backend::DebugBackend;
use dwarf_emitter::java::backend::JavaBackend;
use dwarf_emitter::py::backend::PythonBackend;
use dwarf_emitter::ts::backend::TypeScriptBackend;
use dwarf_lexer::pass::TokenizePass;
use dwarf_lir::pass::LirPass;
use dwarf_lir::LirDecl;
use dwarf_mir::modules::ModuleGraph;
use dwarf_mir::pass::MirPass;
use dwarf_parser::pass::ParsePass;
use dwarf_typecheck::pass::TypeCheckPass;
use dwarf_typecheck::passes::edge_analysis::EdgeAnalysisPass;
use dwarf_typecheck::passes::test_coverage::{
    CoverageCheckConfig, CoverageMode as TcCoverageMode, CoverageScope as TcCoverageScope,
    TestCoveragePass,
};

use crate::{CompileOptions, CoverageMode, Diagnostic, Severity};

/// Run the full compiler pipeline on a source string, collecting diagnostics.
/// Returns the LIR declarations and any diagnostics, or an error string.
pub(crate) fn run_pipeline(
    source: &str,
    filename: &str,
    options: &CompileOptions,
) -> Result<(Vec<LirDecl>, Vec<Diagnostic>), String> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // 1. Tokenize
    let tokenizer = TokenizePass;
    let _tokens = match tokenizer.tokenize(source) {
        Ok(t) => t,
        Err(e) => return Err(format!("Lexer error: {}", e)),
    };

    // 2. Parse
    let parser = ParsePass;
    let (decls, parse_errors) = parser
        .parse(source)
        .map_err(|e| format!("Parse error: {}", e))?;

    // Collect parse errors as diagnostics but continue
    for err in &parse_errors {
        let (line, col) =
            dwarf_syntax::diagnostic::byte_to_line_col(source, err.span.start).unwrap_or((0, 0));
        diagnostics.push(Diagnostic {
            code: err.code.to_string(),
            severity: Severity::Error,
            message: err.message.clone(),
            file: Some(filename.to_string()),
            line: Some(line),
            col: Some(col),
        });
    }

    // 3. Typecheck
    let typechecker = TypeCheckPass::new();
    let (_registry, type_errors) = typechecker.check(&decls);

    for err in &type_errors {
        let (line, col) =
            dwarf_syntax::diagnostic::byte_to_line_col(source, err.span.start).unwrap_or((0, 0));
        diagnostics.push(Diagnostic {
            code: err.code.to_string(),
            severity: Severity::Error,
            message: err.message.clone(),
            file: Some(filename.to_string()),
            line: Some(line),
            col: Some(col),
        });
    }

    // 3.5 Test coverage enforcement + edge-case analysis (DWARF-117).
    //
    // After typechecking, verify that public functions have test coverage and
    // that edge cases are covered. Both passes are skipped when `--quick`
    // bypasses the checks or when coverage is set to `Off`. Edge analysis is
    // additionally gated behind `--skip-edge-check`. Diagnostics are surfaced
    // as hard errors in `Required` mode and as warnings in `Warning` mode.
    if !options.quick && options.test_coverage != CoverageMode::Off {
        let tc_mode = match options.test_coverage {
            CoverageMode::Off => TcCoverageMode::Off,
            CoverageMode::Warning => TcCoverageMode::Warning,
            CoverageMode::Required | CoverageMode::On => TcCoverageMode::Required,
        };

        let coverage_pass = TestCoveragePass::new();
        let cover_config = CoverageCheckConfig {
            mode: tc_mode.clone(),
            scope: TcCoverageScope::AllPub,
        };

        for cov in coverage_pass.check(&decls, &cover_config) {
            let severity = match tc_mode {
                TcCoverageMode::Off => continue,
                TcCoverageMode::Warning => Severity::Warning,
                TcCoverageMode::Required => Severity::Error,
            };
            diagnostics.push(Diagnostic {
                code: cov.code.to_string(),
                severity,
                message: cov.message,
                file: Some(filename.to_string()),
                line: None,
                col: None,
            });
        }

        if !options.skip_edge_check {
            let edge_pass = EdgeAnalysisPass::new();
            for warn in edge_pass.analyze(&decls) {
                let (line, col) =
                    dwarf_syntax::diagnostic::byte_to_line_col(source, warn.span.start)
                        .unwrap_or((0, 0));
                let severity = match tc_mode {
                    TcCoverageMode::Off => continue,
                    TcCoverageMode::Warning => Severity::Warning,
                    TcCoverageMode::Required => Severity::Error,
                };
                diagnostics.push(Diagnostic {
                    code: warn.code.to_string(),
                    severity,
                    message: warn.message,
                    file: Some(filename.to_string()),
                    line: Some(line),
                    col: Some(col),
                });
            }
        }
    }

    // 4. Modules — build dependency graph (optional, used for import resolution)
    let _module_graph = ModuleGraph::build(&decls);

    // 5. MIR desugaring
    let mir_pass = MirPass::new();
    let mir = mir_pass.run(&decls);

    // 6. LIR lowering
    let lir_pass = LirPass::new();
    let lir = lir_pass.run(&mir);

    Ok((lir, diagnostics))
}

/// Select an emitter backend for the given target string.
pub(crate) fn select_backend(
    target: &str,
) -> Result<Box<dyn EmitterBackend<Output = String>>, String> {
    match target {
        "debug" => Ok(Box::new(DebugBackend::new())),
        "ts" => Ok(Box::new(TypeScriptBackend::new("0.1.0"))),
        "py" => Ok(Box::new(PythonBackend::new())),
        "java" => Ok(Box::new(JavaBackend::new("dwarf.gen", "0.1.0"))),
        other => Err(format!(
            "Unsupported target: '{}'. Supported targets: debug, ts, py, java",
            other
        )),
    }
}
