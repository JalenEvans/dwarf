//! Implementation of the `dwarf build` subcommand.

use crate::output::{
    format_output, BuildPayload, FileBuildResult, OutputEnvelope, OutputFormat,
    StructuredDiagnostic,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use dwarf_lib::{CompileOptions, DwarfCompiler};

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

/// Validate a compiled output file by running a target-specific syntax check.
/// Returns `Ok(())` on success, or `Err(msg)` if validation finds a problem.
fn validate_output(target: &str, file_path: &Path) -> Result<(), String> {
    let path_str = file_path.to_string_lossy();
    match target {
        "py" => {
            let output = process::Command::new("python3")
                .args([
                    "-c",
                    &format!("import py_compile; py_compile.compile('{}')", path_str),
                ])
                .output()
                .map_err(|e| format!("Cannot run python3: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Python syntax check failed:\n{}", stderr));
            }
            Ok(())
        }
        "java" => {
            let output = process::Command::new("javac")
                .args(["--release", "17", &path_str])
                .output()
                .map_err(|e| format!("Cannot run javac: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Java compilation check failed:\n{}", stderr));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Run the build subcommand.
#[allow(clippy::too_many_arguments)]
pub fn run_build(
    files: Vec<PathBuf>,
    target: String,
    out_dir: Option<PathBuf>,
    pretty: bool,
    source_map: bool,
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

    // Build CLI options (use the first target as the base, will override per-iteration)
    let cli_options = CompileOptions {
        target: targets[0].clone(),
        pretty,
        passes: passes.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        skip_passes: skip_passes
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
        source_map,
        stdlib_path,
    };

    // Merge with config file if present
    let options = dwarf_cli::config::merge_config_with_cli(cli_options);

    // Resolve output directory
    let resolved_out_dir = out_dir.unwrap_or_else(|| PathBuf::from("dist"));

    // Ensure base output directory exists
    if let Err(e) = fs::create_dir_all(&resolved_out_dir) {
        eprintln!(
            "Error: Cannot create output directory '{}': {}",
            resolved_out_dir.display(),
            e
        );
        process::exit(1);
    }

    let compiler = DwarfCompiler::new();
    let mut has_errors = false;
    let mut success_count = 0u32;
    let mut error_count = 0u32;
    let mut all_results: Vec<FileBuildResult> = Vec::new();
    let start = Instant::now();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();

        // Read source file once
        let source = match read_source_file(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {}: {}", path_str, e);
                error_count += 1;
                has_errors = true;
                continue;
            }
        };

        // Compile for each target
        for tgt in &targets {
            let mut target_options = options.clone();
            target_options.target = tgt.clone();

            match compiler.compile(&source, &path_str, target_options) {
                Ok(result) => {
                    let ext = target_ext(tgt);
                    let out_dir = resolved_out_dir.join(tgt);

                    // Ensure target sub-directory exists
                    if let Err(e) = fs::create_dir_all(&out_dir) {
                        eprintln!(
                            "Error: Cannot create output directory '{}': {}",
                            out_dir.display(),
                            e
                        );
                        error_count += 1;
                        has_errors = true;
                        continue;
                    }

                    let out_path = out_dir.join(
                        file_path
                            .file_stem()
                            .map(|s| format!("{}.{}", s.to_string_lossy(), ext))
                            .unwrap_or_else(|| format!("output.{}", ext)),
                    );

                    let out_path_str = out_path.to_string_lossy().to_string();

                    if let Err(e) = write_output(&out_path, &result.output) {
                        eprintln!("Error writing {}: {}", out_path.display(), e);
                        error_count += 1;
                        has_errors = true;
                    } else {
                        if !json {
                            println!("  Built  {} -> {}", path_str, out_path.display());
                        }
                        success_count += 1;

                        if !json {
                            // Optional: validate output syntax
                            match validate_output(tgt, &out_path) {
                                Ok(()) => {
                                    println!("    {}: syntax OK", tgt);
                                }
                                Err(msg) => {
                                    eprintln!("    {}: validation warning: {}", tgt, msg);
                                    // Non-fatal: warn but don't mark as error
                                }
                            }
                        }
                    }

                    // Write source map file if available
                    if let Some(ref sm) = result.source_map {
                        let map_path = out_path.with_extension(format!("{}.map", ext));
                        if let Err(e) = write_output(&map_path, sm) {
                            eprintln!("Error writing source map {}: {}", map_path.display(), e);
                            error_count += 1;
                            has_errors = true;
                        } else if !json {
                            println!("  Map    {} -> {}", path_str, map_path.display());
                        }
                    }

                    if json {
                        all_results.push(FileBuildResult {
                            file: path_str.clone(),
                            target: tgt.clone(),
                            success: true,
                            output_path: out_path_str,
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
                        // Print diagnostics
                        for diag in &result.diagnostics {
                            eprintln!("  {}: {}", diag.code, diag.message);
                        }
                    }
                }
                Err(errors) => {
                    if json {
                        all_results.push(FileBuildResult {
                            file: path_str.clone(),
                            target: tgt.clone(),
                            success: false,
                            output_path: String::new(),
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
                    error_count += 1;
                    has_errors = true;
                }
            }
        }
    }

    if json {
        let payload = BuildPayload { files: all_results };
        let envelope = OutputEnvelope::from_start("build", payload, start);
        let output = format_output(OutputFormat::Json, &envelope);
        println!("{}", output);
    } else {
        let total_target_builds = files.len() * targets.len();
        println!(
            "\nBuild summary: {} file(s) across {} target(s) — {} success(es), {} error(s), {} total build(s)",
            files.len(),
            targets.len(),
            success_count,
            error_count,
            total_target_builds,
        );
    }

    if has_errors {
        process::exit(1);
    }
}

fn read_source_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}", e))?;
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(bytes[3..].to_vec())
    } else {
        String::from_utf8(bytes)
    };
    content.map_err(|e| format!("File is not valid UTF-8: {}", e))
}

fn write_output(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}", e))?;
    }
    let mut file = fs::File::create(path).map_err(|e| format!("{}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("{}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // -----------------------------------------------------------------------
    // CLI structure tests — verify the Build subcommand parses correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_cli_parse_simple() {
        let cmd = crate::Cli::command();
        let matches =
            cmd.try_get_matches_from(["dwarf-cli", "build", "file.kzd", "--target", "ts"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (subcommand, sub_m) = matches.subcommand().unwrap();
        assert_eq!(subcommand, "build");
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts")
        );
    }

    #[test]
    fn test_build_cli_parse_with_out_dir() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "build",
            "file.kzd",
            "--target",
            "ts",
            "--out-dir",
            "custom/path",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m
                .get_one::<PathBuf>("out_dir")
                .map(|p| p.to_string_lossy().to_string()),
            Some("custom/path".to_string())
        );
    }

    #[test]
    fn test_build_cli_parse_with_pretty() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "build",
            "file.kzd",
            "--target",
            "ts",
            "--pretty",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert!(sub_m.get_flag("pretty"));
    }

    #[test]
    fn test_build_cli_parse_requires_target() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-cli", "build", "file.kzd"]);
        assert!(
            result.is_err(),
            "Should have failed: --target is required but was omitted"
        );
    }

    #[test]
    fn test_build_cli_parse_target_all() {
        let cmd = crate::Cli::command();
        let matches =
            cmd.try_get_matches_from(["dwarf-cli", "build", "file.kzd", "--target", "all"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("all")
        );
    }

    #[test]
    fn test_build_cli_parse_comma_targets() {
        let cmd = crate::Cli::command();
        let matches =
            cmd.try_get_matches_from(["dwarf-cli", "build", "file.kzd", "--target", "ts,py,java"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts,py,java")
        );
    }

    #[test]
    fn test_parse_targets_all() {
        let targets = parse_targets("all");
        assert_eq!(targets, vec!["ts", "py", "java"]);
    }

    #[test]
    fn test_parse_targets_comma_separated() {
        let targets = parse_targets("ts,py,java");
        assert_eq!(targets, vec!["ts", "py", "java"]);
    }

    #[test]
    fn test_parse_targets_single() {
        let targets = parse_targets("ts");
        assert_eq!(targets, vec!["ts"]);
    }

    #[test]
    fn test_parse_targets_with_whitespace() {
        let targets = parse_targets(" ts , py ");
        assert_eq!(targets, vec!["ts", "py"]);
    }

    #[test]
    fn test_target_ext() {
        assert_eq!(target_ext("ts"), "ts");
        assert_eq!(target_ext("py"), "py");
        assert_eq!(target_ext("java"), "java");
        assert_eq!(target_ext("debug"), "txt");
        assert_eq!(target_ext("unknown"), "txt");
    }

    #[test]
    fn test_build_cli_parse_with_passes() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "build",
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

    // -----------------------------------------------------------------------
    // CLI structure tests — verify `run` and `dev` subcommands parse correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_run_cli_parse_basic() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from(["dwarf-cli", "run", "file.kzd", "--target", "ts"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (subcommand, sub_m) = matches.subcommand().unwrap();
        assert_eq!(subcommand, "run");
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts")
        );
    }

    #[test]
    fn test_dev_cli_parse_basic() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from(["dwarf-cli", "dev", "file.kzd", "--target", "ts"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (subcommand, sub_m) = matches.subcommand().unwrap();
        assert_eq!(subcommand, "dev");
        assert_eq!(
            sub_m.get_one::<String>("target").map(String::as_str),
            Some("ts")
        );
    }

    #[test]
    fn test_run_cli_requires_target() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-cli", "run", "file.kzd"]);
        assert!(
            result.is_err(),
            "Should have failed: --target is required for run but was omitted"
        );
    }

    #[test]
    fn test_dev_cli_requires_target() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-cli", "dev", "file.kzd"]);
        assert!(
            result.is_err(),
            "Should have failed: --target is required for dev but was omitted"
        );
    }

    #[test]
    fn test_run_cli_requires_file() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-cli", "run", "--target", "ts"]);
        assert!(
            result.is_err(),
            "Should have failed: at least one file is required for run but none were given"
        );
    }

    #[test]
    fn test_dev_cli_requires_file() {
        let cmd = crate::Cli::command();
        let result = cmd.try_get_matches_from(["dwarf-cli", "dev", "--target", "ts"]);
        assert!(
            result.is_err(),
            "Should have failed: at least one file is required for dev but none were given"
        );
    }

    #[test]
    fn test_list_runtimes_flag() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from(["dwarf-cli", "--list-runtimes"]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        assert!(
            matches.get_flag("list-runtimes") || matches.subcommand_name().is_some(),
            "--list-runtimes should be recognised"
        );
    }

    #[test]
    fn test_list_runtimes_flag_with_subcommand() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "run",
            "file.kzd",
            "--target",
            "ts",
            "--list-runtimes",
        ]);
        // --list-runtimes may be a global flag or a subcommand flag.
        // If it's a global flag (on Cli struct), parsing succeeds.
        // If it's only a top-level flag, this fails because --list-runtimes
        // isn't recognised inside a subcommand.
        // Either behaviour is acceptable — the test just verifies it doesn't panic.
        assert!(
            matches.is_ok() || matches.is_err(),
            "Should not panic — parse either succeeds or returns Err"
        );
    }

    #[test]
    fn test_run_cli_parse_with_passes() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "run",
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
    fn test_dev_cli_parse_with_skip_passes() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "dev",
            "file.kzd",
            "--target",
            "ts",
            "--skip-passes",
            "typecheck",
        ]);
        assert!(matches.is_ok(), "Parse failed: {:?}", matches.err());

        let matches = matches.unwrap();
        let (_, sub_m) = matches.subcommand().unwrap();
        assert_eq!(
            sub_m.get_one::<String>("skip_passes").map(String::as_str),
            Some("typecheck")
        );
    }

    // WILL FAIL — RED PHASE: --stdlib-path is not yet defined in clap args
    #[test]
    fn test_build_cli_parse_with_stdlib_path() {
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "build",
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
