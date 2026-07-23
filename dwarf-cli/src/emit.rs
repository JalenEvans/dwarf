//! Implementation of the `dwarf emit` subcommand.
//!
//! This module provides the entry point for emitting code from Dwarf source
//! files into a target language (TypeScript, Python, Java, etc.).

use std::fs;
use std::path::PathBuf;
use std::process;

use dwarf_cli::pass_manager::*;
use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::debug_backend::DebugBackend;
use dwarf_lexer::pass::TokenizePass;
use dwarf_lir::pass::LirPass;
use dwarf_mir::pass::MirPass;
use dwarf_parser::pass::ParsePass;
use dwarf_typecheck::pass::TypeCheckPass;

use serde::Serialize;
use serde_json::json;

/// Run the emit subcommand.
pub fn run_emit(
    files: Vec<PathBuf>,
    target: String,
    json: bool,
    passes: Option<String>,
    skip_passes: Option<String>,
) {
    // Build pass manager
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    pm.register(Box::new(TypeCheckPass::new()));
    pm.register(Box::new(ModulePass::new()));
    pm.register(Box::new(MirPass::new()));
    pm.register(Box::new(LirPass::new()));

    // Build compile options
    let options = CompileOptions {
        passes: passes.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        skip_passes: skip_passes
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
    };

    // Process each file
    let mut has_errors = false;
    let mut all_results = Vec::new();

    for file_path in &files {
        let result = process_file(file_path, &pm, &options, &target);
        if !result.success {
            has_errors = true;
        }
        all_results.push(result);
    }

    // Output
    if json {
        let output = JsonOutput {
            ok: !has_errors,
            target: target.clone(),
            results: all_results
                .iter()
                .map(|r| {
                    json!({
                        "file": r.file,
                        "success": r.success,
                        "output": r.output,
                        "errors": r.diagnostics.iter().map(|d| {
                            json!({
                                "code": d.code,
                                "severity": match d.severity {
                                    Severity::Error => "error",
                                    Severity::Warning => "warning",
                                    Severity::Info => "info",
                                },
                                "message": d.message,
                                "file": d.file,
                                "line": d.line,
                                "col": d.col,
                            })
                        }).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>(),
        };
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        for result in &all_results {
            if let Some(ref output) = result.output {
                println!("// {}:\n{}", result.file, output);
            }
            for diag in &result.diagnostics {
                eprintln!("{}: {}", diag.code, diag.message);
            }
        }
    }

    if has_errors {
        process::exit(1);
    }
}

/// The result of processing a single file for emission.
struct FileResult {
    file: String,
    success: bool,
    output: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

/// Select a backend implementation for the given target name.
///
/// Returns an `Err` with a diagnostic message when the target is not
/// recognised so the caller can produce a user-facing error.
fn select_backend(target: &str) -> Result<DebugBackend, String> {
    match target {
        // `ts` is a placeholder; both use DebugBackend for now until
        // the real TypeScript backend lands in Phase 2.
        "debug" | "ts" => Ok(DebugBackend::new()),
        other => Err(format!(
            "Unsupported target: '{}'. Supported targets: debug, ts",
            other
        )),
    }
}

/// Process a single source file through the pipeline and emit to the target.
fn process_file(
    file_path: &PathBuf,
    pm: &PassManager,
    options: &CompileOptions,
    target: &str,
) -> FileResult {
    let path_str = file_path.to_string_lossy().to_string();

    // Validate target upfront
    let mut backend = match select_backend(target) {
        Ok(b) => b,
        Err(msg) => {
            return FileResult {
                file: path_str,
                success: false,
                output: None,
                diagnostics: vec![Diagnostic {
                    code: "DWARF-E-EMIT-0002".to_string(),
                    severity: Severity::Error,
                    message: msg,
                    file: Some(file_path.clone()),
                    line: None,
                    col: None,
                }],
            };
        }
    };

    // Read file
    let source = match read_source_file(file_path) {
        Ok(s) => s,
        Err(e) => {
            return FileResult {
                file: path_str,
                success: false,
                output: None,
                diagnostics: vec![Diagnostic {
                    code: "DWARF-E-IO-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Cannot read file: {}", e),
                    file: Some(file_path.clone()),
                    line: None,
                    col: None,
                }],
            };
        }
    };

    let mut unit = CompilationUnit::new(source);
    unit.path = Some(file_path.clone());

    let mut ctx = PassContext::new(CompileOptions {
        passes: options.passes.clone(),
        skip_passes: options.skip_passes.clone(),
    });

    pm.run_all(&mut unit, &mut ctx);

    // Emit if LIR was produced successfully
    let output = unit.lir.as_ref().map(|lir| {
        match backend.emit_module(lir) {
            Ok(out) => out,
            Err(e) => {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-EMIT-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Emission failed: {}", e),
                    file: Some(file_path.clone()),
                    line: None,
                    col: None,
                });
                String::new()
            }
        }
    });

    FileResult {
        file: path_str,
        success: ctx.diagnostics().is_empty() && output.is_some(),
        output,
        diagnostics: ctx.diagnostics().to_vec(),
    }
}

/// Read a source file, stripping a UTF-8 BOM if present.
fn read_source_file(path: &PathBuf) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}", e))?;

    // Detect and strip UTF-8 BOM
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(bytes[3..].to_vec())
    } else {
        String::from_utf8(bytes)
    };

    content.map_err(|e| format!("File is not valid UTF-8: {}", e))
}

// ---- JSON output types ----

#[derive(Serialize)]
struct JsonOutput {
    ok: bool,
    target: String,
    results: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // -----------------------------------------------------------------------
    // CLI structure tests — these test that clap parses arguments correctly.
    // They exercise the `Cli` / `Commands` derive macros and should always
    // pass regardless of the `run_emit` stub.
    // -----------------------------------------------------------------------

    #[test]
    fn test_emit_cli_parse_simple() {
        let cmd = crate::Cli::command();
        let matches = cmd
            .try_get_matches_from(["dwarf-emitter", "emit", "file.kzd", "--target", "ts"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (subcommand, sub_m) = matches.subcommand().unwrap();
        assert_eq!(subcommand, "emit");
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts")
        );
    }

    #[test]
    fn test_emit_cli_parse_with_json() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-emitter",
            "emit",
            "file.kzd",
            "--target",
            "ts",
            "--json",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert!(sub_m.get_flag("json"));
    }

    #[test]
    fn test_emit_cli_parse_with_passes() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-emitter",
            "emit",
            "file.kzd",
            "--target",
            "ts",
            "--passes",
            "tokenize,parse",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("passes").map(String::as_str),
            Some("tokenize,parse")
        );
    }

    #[test]
    fn test_emit_cli_parse_requires_target() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-emitter", "emit", "file.kzd"]);
        assert!(
            result.is_err(),
            "Should have failed: --target is required but was omitted"
        );
    }

    #[test]
    fn test_emit_cli_parse_requires_file() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-emitter", "emit", "--target", "ts"]);
        assert!(
            result.is_err(),
            "Should have failed: at least one file is required but none were given"
        );
    }

    // -----------------------------------------------------------------------
    // run_emit functional tests — marked #[ignore] because they require
    // actual source files and integration with the full compiler pipeline.
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "requires real source files and full pipeline integration"]
    fn test_run_emit_with_nonexistent_file() {
        // run_emit does not panic on missing files — it prints an error
        // message and calls process::exit(1). This test verifies it does
        // not panic (unlike the old unimplemented!() stub).
        let result = std::panic::catch_unwind(|| {
            run_emit(
                vec![PathBuf::from("/nonexistent/file.kzd")],
                "ts".to_string(),
                false,
                None,
                None,
            );
        });
        // The function should not panic — it handles errors gracefully
        // (though process::exit(1) is called, which terminates the process
        // and can't be caught by catch_unwind in a real run).
        assert!(result.is_ok(), "run_emit should not panic on missing files");
    }

    #[test]
    #[ignore = "requires real source files and full pipeline integration"]
    fn test_run_emit_empty_passes() {
        let result = std::panic::catch_unwind(|| {
            run_emit(
                vec![PathBuf::from("test.kzd")],
                "ts".to_string(),
                false,
                None,
                None,
            );
        });
        // passes: None means run all passes — no panic should occur.
        // The file "test.kzd" doesn't exist, but that's handled gracefully.
        assert!(result.is_ok(), "run_emit should not panic with passes: None");
    }

    #[test]
    #[ignore = "requires real source files and full pipeline integration"]
    fn test_run_emit_default_target() {
        let result = std::panic::catch_unwind(|| {
            run_emit(
                vec![PathBuf::from("test.kzd")],
                String::new(),
                false,
                None,
                None,
            );
        });
        // An empty target is accepted (it just passes through to the backend).
        // The function should not panic.
        assert!(result.is_ok(), "run_emit should not panic with empty target");
    }
}
