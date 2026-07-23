//! Implementation of the `dwarf check` subcommand.

use std::fs;
use std::path::PathBuf;
use std::process;

use dwarf_cli::pass_manager::*;
use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::pass::ParsePass;
use dwarf_lir::pass::LirPass;
use dwarf_mir::pass::MirPass;
use dwarf_syntax::diagnostic::format_diagnostic;

use serde::Serialize;
use serde_json::json;

/// Run the check subcommand.
pub fn run_check(
    files: Vec<PathBuf>,
    json: bool,
    passes: Option<String>,
    skip_passes: Option<String>,
    list_passes: bool,
) {
    // Build pass manager
    let mut pm = PassManager::new();
    pm.register(Box::new(TokenizePass));
    pm.register(Box::new(ParsePass));
    pm.register(Box::new(ModulePass::new()));
    pm.register(Box::new(MirPass::new()));
    pm.register(Box::new(LirPass::new()));

    // Handle --list-passes
    if list_passes {
        for (name, desc) in pm.list_passes() {
            println!("  {:<20} {}", name, desc);
        }
        return;
    }

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
        let result = process_file(file_path, &pm, &options);
        if !result.success {
            has_errors = true;
        }
        all_results.push(result);
    }

    // Output
    if json {
        let output = JsonOutput {
            ok: !has_errors,
            results: all_results
                .iter()
                .map(|r| {
                    json!({
                        "file": r.file,
                        "success": r.success,
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
            for diag in &result.diagnostics {
                let file_str = diag.file.as_ref().map(|f| f.to_string_lossy().to_string());
                let formatted = format_diagnostic(
                    file_str.as_deref(),
                    &result.source,
                    &diag.code,
                    &diag.message,
                    diag.line.unwrap_or(0),
                    diag.col.unwrap_or(0),
                );
                eprint!("{}", formatted);
            }
        }
    }

    if has_errors {
        process::exit(1);
    }
}

struct FileResult {
    file: String,
    success: bool,
    diagnostics: Vec<Diagnostic>,
    source: String,
}

fn process_file(file_path: &PathBuf, pm: &PassManager, options: &CompileOptions) -> FileResult {
    let path_str = file_path.to_string_lossy().to_string();

    // Read file with BOM detection
    let source = match read_source_file(file_path) {
        Ok(s) => s,
        Err(e) => {
            return FileResult {
                file: path_str,
                success: false,
                diagnostics: vec![Diagnostic {
                    code: "DWARF-E-IO-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Cannot read file: {}", e),
                    file: Some(file_path.clone()),
                    line: None,
                    col: None,
                }],
                source: String::new(),
            };
        }
    };

    let source_for_result = source.clone();
    let mut unit = CompilationUnit::new(source);
    unit.path = Some(file_path.clone());

    let mut ctx = PassContext::new(CompileOptions {
        passes: options.passes.clone(),
        skip_passes: options.skip_passes.clone(),
    });

    pm.run_all(&mut unit, &mut ctx);

    FileResult {
        file: path_str,
        success: ctx.diagnostics().is_empty(),
        diagnostics: ctx.diagnostics().to_vec(),
        source: source_for_result,
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
    results: Vec<serde_json::Value>,
}
