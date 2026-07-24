//! Implementation of the `dwarf emit` subcommand.

use dwarf_lib::{CompileOptions, DwarfCompiler};
use std::fs;
use std::path::PathBuf;
use std::process;

pub fn run_emit(
    files: Vec<PathBuf>,
    target: String,
    json: bool,
    passes: Option<String>,
    skip_passes: Option<String>,
) {
    let cli_options = CompileOptions {
        target: if target.is_empty() {
            "ts".to_string()
        } else {
            target.clone()
        },
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
                if json {
                    all_results.push(serde_json::json!({
                        "file": path_str,
                        "success": true,
                        "output": result.output,
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
                    println!("// {}:\n{}", path_str, result.output);
                    for diag in &result.diagnostics {
                        eprintln!("{}: {}", diag.code, diag.message);
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
                    eprintln!("Error compiling {}:", path_str);
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
                "target": target,
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
        let matches =
            cmd.try_get_matches_from(["dwarf-emitter", "emit", "file.kzd", "--target", "ts"]);
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
}
