//! Implementation of the `dwarf check` subcommand.

use dwarf_lib::{CompileOptions, DwarfCompiler};
use dwarf_syntax::diagnostic::format_diagnostic;
use std::fs;
use std::path::PathBuf;
use std::process;

pub fn run_check(
    files: Vec<PathBuf>,
    json: bool,
    passes: Option<String>,
    skip_passes: Option<String>,
    list_passes: bool,
) {
    if list_passes {
        println!(
            "  {:<20} {}",
            "tokenize", "Tokenize source text into a stream of tokens"
        );
        println!("  {:<20} {}", "parse", "Parse tokens into HIR declarations");
        println!(
            "  {:<20} {}",
            "typecheck", "Check types and infer expressions"
        );
        println!(
            "  {:<20} {}",
            "modules", "Resolve module imports and build dependency graph"
        );
        println!("  {:<20} {}", "mir", "Desugar HIR into MIR");
        println!("  {:<20} {}", "lir", "Lower MIR to LIR with target hints");
        return;
    }

    let cli_options = CompileOptions {
        target: "debug".to_string(),
        pretty: false,
        passes: passes.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        skip_passes: skip_passes
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
    };

    let options = dwarf_cli::config::merge_config_with_cli(cli_options);
    let compiler = DwarfCompiler::new();
    let mut has_errors = false;
    let mut all_results = Vec::new();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();
        let source = match read_source_file(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {}: {}", path_str, e);
                has_errors = true;
                continue;
            }
        };

        match compiler.compile(&source, &path_str, options.clone()) {
            Ok(result) => {
                let has_errs = result
                    .diagnostics
                    .iter()
                    .any(|d| matches!(d.severity, dwarf_lib::Severity::Error));
                if has_errs {
                    has_errors = true;
                }

                if json {
                    all_results.push(serde_json::json!({
                        "file": path_str,
                        "success": !has_errs,
                        "errors": result.diagnostics.iter().map(|d| {
                            serde_json::json!({
                                "code": d.code,
                                "severity": format!("{}", d.severity),
                                "message": d.message,
                                "file": d.file,
                                "line": d.line,
                                "col": d.col,
                            })
                        }).collect::<Vec<_>>(),
                    }));
                } else {
                    for diag in &result.diagnostics {
                        let formatted = format_diagnostic(
                            diag.file.as_deref(),
                            &source,
                            &diag.code,
                            &diag.message,
                            diag.line.unwrap_or(0),
                            diag.col.unwrap_or(0),
                        );
                        eprint!("{}", formatted);
                    }
                }
            }
            Err(errors) => {
                has_errors = true;
                if json {
                    all_results.push(serde_json::json!({
                        "file": path_str,
                        "success": false,
                        "errors": errors.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                    }));
                } else {
                    eprintln!("Error checking {}:", path_str);
                    for err in &errors {
                        eprintln!("  {}", err);
                    }
                }
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": !has_errors,
                "results": all_results,
            }))
            .unwrap()
        );
    }

    if has_errors {
        process::exit(1);
    }
}

fn read_source_file(file_path: &PathBuf) -> Result<String, String> {
    let bytes = fs::read(file_path).map_err(|e| format!("{}", e))?;
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(bytes[3..].to_vec())
    } else {
        String::from_utf8(bytes)
    };
    content.map_err(|e| format!("File is not valid UTF-8: {}", e))
}
