//! Implementation of the `dwarf emit` subcommand.

use crate::output::{
    format_output, EmitPayload, FileEmitResult, OutputEnvelope, OutputFormat, StructuredDiagnostic,
};
use dwarf_lib::{CompileOptions, DwarfCompiler};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

/// Map a target name to its file extension.
fn target_ext(target: &str) -> &str {
    match target {
        "ts" => "ts",
        "py" => "py",
        "java" => "java",
        _ => "txt",
    }
}

/// Parse a comma-separated target string (or "all") into a list of targets.
fn parse_targets(target: &str) -> Vec<String> {
    if target == "all" {
        vec!["ts".into(), "py".into(), "java".into()]
    } else {
        target.split(',').map(|s| s.trim().to_string()).collect()
    }
}

pub fn run_emit(
    files: Vec<PathBuf>,
    target: String,
    json: bool,
    passes: Option<String>,
    skip_passes: Option<String>,
    stdlib_path: Option<String>,
) {
    // Parse the target string into a list of targets
    let targets: Vec<String> = if target.is_empty() {
        vec!["ts".into()]
    } else {
        parse_targets(&target)
    };

    let compiler = DwarfCompiler::new();
    let mut has_errors = false;
    let mut all_results: Vec<FileEmitResult> = Vec::new();
    let start = Instant::now();

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

        for tgt in &targets {
            let cli_options = CompileOptions {
                target: tgt.clone(),
                pretty: false,
                passes: passes
                    .clone()
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
                skip_passes: skip_passes
                    .clone()
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect(),
                source_map: false,
                stdlib_path: stdlib_path.clone(),
            };

            let options = crate::config::merge_config_with_cli(cli_options);

            match compiler.compile(&source, &path_str, options.clone()) {
                Ok(result) => {
                    if json {
                        all_results.push(FileEmitResult {
                            file: path_str.clone(),
                            target: tgt.clone(),
                            success: true,
                            output: result.output.clone(),
                            extension: target_ext(tgt).to_string(),
                            errors: result
                                .diagnostics
                                .iter()
                                .map(|d| StructuredDiagnostic {
                                    code: d.code.clone(),
                                    severity: format!("{}", d.severity),
                                    message: d.message.clone(),
                                    file: d.file.clone().unwrap_or_default(),
                                    line: d.line.unwrap_or(0),
                                    col: d.col.unwrap_or(0),
                                    related: vec![],
                                    fix: None,
                                })
                                .collect(),
                        });
                    } else {
                        println!("// {} [{}]:\n{}", path_str, tgt, result.output);
                        for diag in &result.diagnostics {
                            eprintln!("[{}] {}: {}", tgt, diag.code, diag.message);
                        }
                    }
                }
                Err(errors) => {
                    has_errors = true;
                    if json {
                        all_results.push(FileEmitResult {
                            file: path_str.clone(),
                            target: tgt.clone(),
                            success: false,
                            output: String::new(),
                            extension: String::new(),
                            errors: errors
                                .iter()
                                .map(|e| StructuredDiagnostic {
                                    code: "COMPILE_ERR".to_string(),
                                    severity: "error".to_string(),
                                    message: e.to_string(),
                                    file: path_str.clone(),
                                    line: 0,
                                    col: 0,
                                    related: vec![],
                                    fix: None,
                                })
                                .collect(),
                        });
                    } else {
                        eprintln!("Error compiling {} for target '{}':", path_str, tgt);
                        for err in &errors {
                            eprintln!("  {}", err);
                        }
                    }
                }
            }
        }
    }

    if json {
        let payload = EmitPayload { files: all_results };
        let envelope = OutputEnvelope::from_start("emit", payload, start);
        let output = format_output(OutputFormat::Json, &envelope);
        println!("{}", output);
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
    fn test_emit_cli_parse_target_all() {
        let cmd = crate::Cli::command();
        let matches =
            cmd.try_get_matches_from(["dwarf-emitter", "emit", "file.kzd", "--target", "all"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("all")
        );
    }

    #[test]
    fn test_emit_cli_parse_comma_targets() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-emitter",
            "emit",
            "file.kzd",
            "--target",
            "ts,py,java",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts,py,java")
        );
    }

    #[test]
    fn test_emit_parse_targets_all() {
        let targets = parse_targets("all");
        assert_eq!(targets, vec!["ts", "py", "java"]);
    }

    #[test]
    fn test_emit_parse_targets_comma() {
        let targets = parse_targets("ts,py,java");
        assert_eq!(targets, vec!["ts", "py", "java"]);
    }

    #[test]
    fn test_emit_parse_targets_single() {
        let targets = parse_targets("ts");
        assert_eq!(targets, vec!["ts"]);
    }

    #[test]
    fn test_emit_parse_targets_whitespace() {
        let targets = parse_targets(" ts , py ");
        assert_eq!(targets, vec!["ts", "py"]);
    }

    #[test]
    fn test_emit_target_ext() {
        assert_eq!(target_ext("ts"), "ts");
        assert_eq!(target_ext("py"), "py");
        assert_eq!(target_ext("java"), "java");
        assert_eq!(target_ext("debug"), "txt");
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

    // WILL FAIL — RED PHASE: --stdlib-path is not yet defined in clap args
    #[test]
    fn test_emit_cli_parse_with_stdlib_path() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-emitter",
            "emit",
            "file.kzd",
            "--target",
            "ts",
            "--stdlib-path",
            "/custom/stdlib",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());
        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("stdlib-path").map(String::as_str),
            Some("/custom/stdlib")
        );
    }
}
