//! Implementation of the `dwarf build` subcommand.
//!
//! This module runs the same compiler pipeline as `emit` but writes each
//! file's output to disk under `dist/{target}/` (or a custom `--out-dir`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use dwarf_cli::pass_manager::*;
use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::debug_backend::DebugBackend;
use dwarf_emitter::ts_backend::TypeScriptBackend;
use dwarf_lexer::pass::TokenizePass;
use dwarf_lir::pass::LirPass;
use dwarf_mir::pass::MirPass;
use dwarf_parser::pass::ParsePass;
use dwarf_typecheck::pass::TypeCheckPass;

/// Run the build subcommand.
pub fn run_build(
    files: Vec<PathBuf>,
    target: String,
    out_dir: Option<PathBuf>,
    pretty: bool,
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

    // Resolve output directory
    let resolved_out_dir = out_dir.unwrap_or_else(|| {
        let mut default = PathBuf::from("dist");
        default.push(&target);
        default
    });

    // Ensure the output directory exists
    if let Err(e) = fs::create_dir_all(&resolved_out_dir) {
        eprintln!(
            "Error: Cannot create output directory '{}': {}",
            resolved_out_dir.display(),
            e
        );
        process::exit(1);
    }

    // Process each file
    let mut has_errors = false;
    let mut success_count = 0u32;
    let mut error_count = 0u32;

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();
        let result = process_file(file_path, &pm, &options, &target, &resolved_out_dir, pretty);

        if let Some(output_path) = result.output_path {
            println!("  Built  {} -> {}", path_str, output_path.display());
            success_count += 1;
        } else {
            eprintln!("  Error  {}", path_str);
            error_count += 1;
            has_errors = true;
        }

        for diag in &result.diagnostics {
            eprintln!("  {}: {}", diag.code, diag.message);
        }
    }

    // Summary
    let total = files.len();
    println!(
        "\nBuild summary: {} file(s) built, {} error(s), {} total",
        success_count, error_count, total
    );

    if has_errors {
        process::exit(1);
    }
}

/// Result of processing a single file for a build.
struct FileResult {
    diagnostics: Vec<Diagnostic>,
    output_path: Option<PathBuf>,
}

/// Select a backend implementation for the given target name.
fn select_backend(target: &str) -> Result<Box<dyn EmitterBackend<Output = String>>, String> {
    match target {
        "debug" => Ok(Box::new(DebugBackend::new())),
        "ts" => Ok(Box::new(TypeScriptBackend::new("0.1.0"))),
        other => Err(format!(
            "Unsupported target: '{}'. Supported targets: debug, ts",
            other
        )),
    }
}

/// Map a target name to the corresponding file extension.
fn target_extension(target: &str) -> &str {
    match target {
        "ts" => "ts",
        "py" => "py",
        "java" => "java",
        "debug" => "txt",
        _ => target,
    }
}

/// Compute the output path for a source file.
///
/// Replaces the `.kzd` extension with the target extension and places
/// the file under the output directory.
fn compute_output_path(file_path: &Path, out_dir: &Path, target: &str) -> PathBuf {
    let ext = target_extension(target);
    let stem = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());
    let filename = format!("{}.{}", stem, ext);
    out_dir.join(filename)
}

/// Process a single source file through the pipeline and write the output.
fn process_file(
    file_path: &Path,
    pm: &PassManager,
    options: &CompileOptions,
    target: &str,
    out_dir: &Path,
    _pretty: bool,
) -> FileResult {
    // Validate target upfront
    let mut backend = match select_backend(target) {
        Ok(b) => b,
        Err(msg) => {
            return FileResult {
                diagnostics: vec![Diagnostic {
                    code: "DWARF-E-EMIT-0002".to_string(),
                    severity: Severity::Error,
                    message: msg,
                    file: Some(file_path.to_path_buf()),
                    line: None,
                    col: None,
                }],
                output_path: None,
            };
        }
    };

    // Read file
    let source = match read_source_file(file_path) {
        Ok(s) => s,
        Err(e) => {
            return FileResult {
                diagnostics: vec![Diagnostic {
                    code: "DWARF-E-IO-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Cannot read file: {}", e),
                    file: Some(file_path.to_path_buf()),
                    line: None,
                    col: None,
                }],
                output_path: None,
            };
        }
    };

    let mut unit = CompilationUnit::new(source);
    unit.path = Some(file_path.to_path_buf());

    let mut ctx = PassContext::new(CompileOptions {
        passes: options.passes.clone(),
        skip_passes: options.skip_passes.clone(),
    });

    pm.run_all(&mut unit, &mut ctx);

    // Emit if LIR was produced successfully
    let output = unit
        .lir
        .as_ref()
        .and_then(|lir| match backend.emit_module(lir) {
            Ok(out) => Some(out),
            Err(e) => {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-EMIT-0001".to_string(),
                    severity: Severity::Error,
                    message: format!("Emission failed: {}", e),
                    file: Some(file_path.to_path_buf()),
                    line: None,
                    col: None,
                });
                None
            }
        });

    // Write output file if emission succeeded
    let output_path = output.as_ref().and_then(|out| {
        let out_path = compute_output_path(file_path, out_dir, target);
        match write_output(&out_path, out) {
            Ok(_) => Some(out_path),
            Err(e) => {
                ctx.push_diagnostic(Diagnostic {
                    code: "DWARF-E-IO-0002".to_string(),
                    severity: Severity::Error,
                    message: format!("Cannot write output: {}", e),
                    file: Some(file_path.to_path_buf()),
                    line: None,
                    col: None,
                });
                None
            }
        }
    });

    FileResult {
        diagnostics: ctx.diagnostics().to_vec(),
        output_path,
    }
}

/// Read a source file, stripping a UTF-8 BOM if present.
fn read_source_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}", e))?;

    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8(bytes[3..].to_vec())
    } else {
        String::from_utf8(bytes)
    };

    content.map_err(|e| format!("File is not valid UTF-8: {}", e))
}

/// Write the emitted output to a file.
fn write_output(path: &Path, content: &str) -> Result<(), String> {
    // Ensure parent directory exists
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

    #[test]
    fn test_compute_output_path_basic() {
        let input = PathBuf::from("src/hello.kzd");
        let out_dir = PathBuf::from("dist/ts");
        let path = compute_output_path(&input, &out_dir, "ts");
        assert_eq!(path, PathBuf::from("dist/ts/hello.ts"));
    }

    #[test]
    fn test_compute_output_path_custom_out_dir() {
        let input = PathBuf::from("foo.kzd");
        let out_dir = PathBuf::from("custom/path");
        let path = compute_output_path(&input, &out_dir, "ts");
        assert_eq!(path, PathBuf::from("custom/path/foo.ts"));
    }

    #[test]
    fn test_compute_output_path_python() {
        let input = PathBuf::from("mod.kzd");
        let out_dir = PathBuf::from("dist/py");
        let path = compute_output_path(&input, &out_dir, "py");
        assert_eq!(path, PathBuf::from("dist/py/mod.py"));
    }

    #[test]
    fn test_target_extension_ts() {
        assert_eq!(target_extension("ts"), "ts");
    }

    #[test]
    fn test_target_extension_py() {
        assert_eq!(target_extension("py"), "py");
    }

    #[test]
    fn test_target_extension_debug() {
        assert_eq!(target_extension("debug"), "txt");
    }

    #[test]
    fn test_target_extension_unknown() {
        assert_eq!(target_extension("java"), "java");
    }

    #[test]
    fn test_select_backend_ts() {
        let backend = select_backend("ts");
        assert!(backend.is_ok(), "ts backend should be selectable");
    }

    #[test]
    fn test_select_backend_debug() {
        let backend = select_backend("debug");
        assert!(backend.is_ok(), "debug backend should be selectable");
    }

    #[test]
    fn test_select_backend_unsupported() {
        let backend = select_backend("java");
        assert!(backend.is_err(), "java backend should not be selectable");
    }
}
