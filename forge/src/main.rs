use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

use dwarf_cli::{build, check, dev, emit, fmt, run, test};

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "Dwarf platform CLI — build, manage dependencies, and more"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn validate_package_format(s: &str) -> Result<String, String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Ok(s.to_string())
    } else {
        Err("Package must be in format '<prefix>:<name>' (e.g., 'npm:express')".to_string())
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Check Dwarf source files for errors
    Check {
        /// Source files to check (.kzd)
        files: Vec<PathBuf>,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Comma-separated list of passes to run (e.g., "tokenize,parse")
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,

        /// List available passes and exit
        #[arg(long)]
        list_passes: bool,

        /// Path to standard library runtime files
        #[arg(long, id = "stdlib-path")]
        stdlib_path: Option<String>,
    },

    /// Build Dwarf source files into target language
    Build {
        /// Source files to compile (.kzd)
        files: Vec<PathBuf>,

        /// Target language (e.g., "ts", "py", "java")
        #[arg(long, short)]
        target: String,

        /// Output directory (default: dist/{target})
        #[arg(long)]
        out_dir: Option<PathBuf>,

        /// Apply pretty formatting to output
        #[arg(long)]
        pretty: bool,

        /// Generate source maps (.map files) alongside output
        #[arg(long)]
        source_map: bool,

        /// Output build results as JSON
        #[arg(long)]
        json: bool,

        /// Comma-separated list of passes to run
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,

        /// Path to standard library runtime files
        #[arg(long, id = "stdlib-path")]
        stdlib_path: Option<String>,
    },

    /// Emit code from Dwarf source files to a target language
    Emit {
        /// Source files to compile (.kzd)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Target language to emit (e.g., "ts", "py", "java")
        #[arg(long, short)]
        target: String,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Comma-separated list of passes to run (e.g., "tokenize,parse")
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,

        /// Path to standard library runtime files
        #[arg(long, id = "stdlib-path")]
        stdlib_path: Option<String>,
    },

    /// Transpile and run a Dwarf source file
    Run {
        /// Source files to run (.kzd)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Target language (e.g., "ts")
        #[arg(long, short)]
        target: String,

        /// Comma-separated list of passes to run
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,

        /// Path to standard library runtime files
        #[arg(long, id = "stdlib-path")]
        stdlib_path: Option<String>,
    },

    /// Watch source files and re-run on changes
    Dev {
        /// Source files to watch (.kzd)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Target language (e.g., "ts")
        #[arg(long, short)]
        target: String,

        /// Comma-separated list of passes to run
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,

        /// Path to standard library runtime files
        #[arg(long, id = "stdlib-path")]
        stdlib_path: Option<String>,
    },

    /// Format Dwarf source files
    Fmt {
        /// Source files to format (.kzd)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Check mode: exit with code 1 if files would be reformatted
        #[arg(long)]
        check: bool,

        /// Write formatted output to stdout
        #[arg(long)]
        stdout: bool,
    },

    /// Compile and run tests with Jest
    Test {
        /// Source files to test (.kzd)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Target language (e.g., "ts")
        #[arg(long, short)]
        target: String,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Diff mode: compile to all targets and compare against oracle
        #[arg(long)]
        diff: bool,

        /// Apply auto-fix patches for failing tests by shrinking counterexamples
        #[arg(long)]
        fix: bool,
    },

    /// Initialize a new Dwarf project
    Init {
        /// Project name (optional — defaults to directory name)
        name: Option<String>,
    },

    /// Add a dependency to the project
    Add {
        /// Package to install (e.g., "npm:express", "py:requests", "java:com.google.gson.Gson")
        #[arg(required = true, value_parser = validate_package_format)]
        package: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Check {
            files,
            json,
            passes,
            skip_passes,
            list_passes,
            stdlib_path,
        }) => {
            check::run_check(files, json, passes, skip_passes, list_passes, stdlib_path);
        }
        Some(Commands::Build {
            files,
            target,
            out_dir,
            pretty,
            source_map,
            json,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            build::run_build(
                files,
                target,
                out_dir,
                pretty,
                source_map,
                json,
                passes,
                skip_passes,
                stdlib_path,
            );
        }
        Some(Commands::Emit {
            files,
            target,
            json,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            emit::run_emit(files, target, json, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Run {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            run::run_run(files, target, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Dev {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path,
        }) => {
            dev::run_dev(files, target, passes, skip_passes, stdlib_path);
        }
        Some(Commands::Fmt {
            files,
            check,
            stdout,
        }) => {
            fmt::run_fmt(files, check, stdout);
        }
        Some(Commands::Test {
            files,
            target,
            json,
            diff,
            fix,
        }) => {
            test::run_test(files, target, json, diff, fix);
        }
        Some(Commands::Init { name }) => match run_init(None, name.as_deref()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Add { package }) => {
            let cwd = std::env::current_dir().unwrap_or_default();
            match run_add(&cwd, &package) {
                Ok(extern_stub) => println!("{extern_stub}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("Error: No subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// forge init — project scaffolding
// ---------------------------------------------------------------------------

/// Initialize a new Dwarf project by creating `forge.toml` and `dwarf.toml`.
///
/// - `project_dir`: target directory (created if absent). `None` = current dir.
/// - `name`: project name. `None` = derive from directory name.
///
/// Returns `Err` if `forge.toml` or `dwarf.toml` already exists in the target directory.
pub fn run_init(project_dir: Option<&std::path::Path>, name: Option<&str>) -> Result<(), String> {
    // Reject path traversal in user-provided project name before any directory
    // operations (W4). The broader character validation runs after name resolution.
    if let Some(n) = name {
        if n.contains("..") || n.contains('/') || n.contains('\\') {
            return Err(format!(
                "Invalid project name: {}. Project name must not contain path separators or '..'.",
                n
            ));
        }
    }

    let target_dir = match project_dir {
        Some(dir) => {
            if !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    format!("Failed to create directory '{}': {}", dir.display(), e)
                })?;
            }
            dir.to_path_buf()
        }
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?;

            // If name is provided, create a directory with that name in current dir
            if let Some(project_name) = name {
                let dir = cwd.join(project_name);
                if !dir.exists() {
                    std::fs::create_dir_all(&dir).map_err(|e| {
                        format!("Failed to create directory '{}': {}", dir.display(), e)
                    })?;
                }
                dir
            } else {
                // No name, no dir - use current directory
                cwd
            }
        }
    };

    let forge_toml_path = target_dir.join("forge.toml");
    let dwarf_toml_path = target_dir.join("dwarf.toml");
    if forge_toml_path.exists() {
        return Err(format!(
            "forge.toml already exists in '{}'. Refusing to overwrite an existing project.",
            target_dir.display()
        ));
    }
    if dwarf_toml_path.exists() {
        return Err(format!(
            "dwarf.toml already exists in '{}'. Refusing to overwrite an existing project.",
            target_dir.display()
        ));
    }

    // Determine project name: explicit > directory name > fallback
    let project_name = match name {
        Some(n) => n.to_string(),
        None => target_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string(),
    };

    // Validate the final project name (covers both user-provided and derived names)
    if project_name.is_empty()
        || !project_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Invalid project name: {}. Use only letters, numbers, hyphens, and underscores.",
            project_name
        ));
    }

    // Generate forge.toml
    let forge_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
description = "A Dwarf project"

[dependencies]
# npm, py, java dependencies go here
"#,
        name = project_name
    );

    // Generate dwarf.toml
    let dwarf_toml = r#"[target]
# Target languages: ts, py, java
typescript = true
python = true
java = true

[compiler]
# Standard library path (optional)
# stdlib_path = "./stdlib"
"#;

    std::fs::write(&forge_toml_path, forge_toml)
        .map_err(|e| format!("Failed to write forge.toml: {}", e))?;
    std::fs::write(&dwarf_toml_path, dwarf_toml)
        .map_err(|e| format!("Failed to write dwarf.toml: {}", e))?;

    // Create externs/ directory for extern declaration files
    let externs_dir = target_dir.join("externs");
    std::fs::create_dir_all(&externs_dir).map_err(|e| {
        format!(
            "Failed to create externs directory '{}': {}",
            externs_dir.display(),
            e
        )
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// forge add — dependency management
// ---------------------------------------------------------------------------

/// Add a dependency to the project.
///
/// - `project_dir`: directory containing `forge.toml`.
/// - `package`: package string in `prefix:name` format (e.g., `npm:express`).
///
/// Returns the generated extern declaration stub on success.
pub fn run_add(project_dir: &std::path::Path, package: &str) -> Result<String, String> {
    // Parse prefix:name
    let (prefix, name) = package
        .split_once(':')
        .filter(|(p, n)| !p.is_empty() && !n.is_empty())
        .ok_or_else(|| {
            format!(
                "Invalid package format '{}'. Expected '<prefix>:<name>' (e.g., 'npm:express')",
                package
            )
        })?;

    // Generate extern stub based on prefix
    let extern_stub = match prefix {
        "npm" | "py" => {
            format!(r#"extern "{}" fn {}() -> ()"#, package, name)
        }
        "java" => {
            let parts: Vec<&str> = name.rsplitn(2, '.').collect();
            if parts.len() < 2 {
                return Err(format!(
                    "Invalid java package '{}'. Expected dotted path like 'java.util.ArrayList'",
                    name
                ));
            }
            let class_name = parts[0];
            let package_path = parts[1];
            format!(
                r#"extern "java:{}" fn {}() -> ()"#,
                package_path, class_name
            )
        }
        _ => {
            return Err(format!(
                "Unknown source prefix '{}'. Supported prefixes: npm, py, java",
                prefix
            ));
        }
    };

    // Write the extern declaration to externs/{prefix}/{name}.kzd
    let extern_dir = project_dir.join("externs").join(prefix);
    std::fs::create_dir_all(&extern_dir).map_err(|e| {
        format!(
            "Failed to create extern directory '{}': {}",
            extern_dir.display(),
            e
        )
    })?;

    let extern_file = extern_dir.join(format!("{}.kzd", name));
    std::fs::write(&extern_file, &extern_stub).map_err(|e| {
        format!(
            "Failed to write extern file '{}': {}",
            extern_file.display(),
            e
        )
    })?;

    // Read forge.toml
    let forge_toml_path = project_dir.join("forge.toml");
    let mut contents = std::fs::read_to_string(&forge_toml_path)
        .map_err(|e| format!("Failed to read forge.toml: {}", e))?;

    // Add dependency to [dependencies] section
    let dep_line = format!("\"{}\" = \"*\"", package);

    if let Some(pos) = contents.find("[dependencies]") {
        let after_header = pos + "[dependencies]".len();
        // Find end of the [dependencies] line to insert after it
        let insert_pos = contents[after_header..]
            .find('\n')
            .map(|p| after_header + p + 1)
            .unwrap_or(contents.len());
        contents.insert_str(insert_pos, &format!("{}\n", dep_line));
    } else {
        return Err("forge.toml does not have a [dependencies] section".to_string());
    }

    // Write updated forge.toml
    std::fs::write(&forge_toml_path, &contents)
        .map_err(|e| format!("Failed to write forge.toml: {}", e))?;

    // Best-effort install via package manager (npm/pip only)
    match prefix {
        "npm" | "py" => {
            let pm = if prefix == "npm" { "npm" } else { "pip" };
            match process::Command::new(pm).arg("install").arg(name).status() {
                Ok(status) if status.success() => {
                    eprintln!("Installed {}", name);
                }
                Ok(status) => {
                    eprintln!(
                        "Warning: {} install {} failed with exit code {:?}",
                        pm,
                        name,
                        status.code()
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!(
                        "Warning: {} not found. Please install the package manually.",
                        pm
                    );
                }
                Err(e) => {
                    eprintln!("Warning: failed to run {}: {}", pm, e);
                }
            }
        }
        _ => {} // java has no CLI package manager
    }

    Ok(extern_stub)
}

// ---------------------------------------------------------------------------
// CLI passthrough tests — RED phase
//
// These tests verify that forge's CLI parses each passthrough subcommand
// correctly, mirroring the argument structure from dwarf-cli. They MUST FAIL
// right now because the Commands enum is empty and has no variants.
//
// Once the passthrough subcommands are added to Commands, these tests will
// compile and pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod cli_passthrough_tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    // ── check ──────────────────────────────────────────────────────────

    #[test]
    fn test_forge_check_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "check", "test.kzd"]);
        assert!(cli.is_ok(), "forge check test.kzd should parse");
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Check { files, .. }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_check_with_json_flag() {
        let cli = Cli::try_parse_from(["forge", "check", "test.kzd", "--json"]);
        assert!(cli.is_ok(), "forge check --json should parse");
        match cli.unwrap().command {
            Some(Commands::Check { json, .. }) => {
                assert!(json, "--json flag should be true");
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_check_with_passes() {
        let cli = Cli::try_parse_from(["forge", "check", "test.kzd", "--passes", "tokenize,parse"]);
        assert!(cli.is_ok(), "forge check --passes should parse");
        match cli.unwrap().command {
            Some(Commands::Check { passes, .. }) => {
                assert_eq!(passes.as_deref(), Some("tokenize,parse"));
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_check_with_skip_passes() {
        let cli = Cli::try_parse_from(["forge", "check", "test.kzd", "--skip-passes", "lint"]);
        assert!(cli.is_ok(), "forge check --skip-passes should parse");
        match cli.unwrap().command {
            Some(Commands::Check { skip_passes, .. }) => {
                assert_eq!(skip_passes.as_deref(), Some("lint"));
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    // ── build ──────────────────────────────────────────────────────────

    #[test]
    fn test_forge_build_parses_basic() {
        let cli = Cli::try_parse_from([
            "forge",
            "build",
            "test.kzd",
            "--target",
            "ts",
            "--out-dir",
            "./dist",
        ]);
        assert!(
            cli.is_ok(),
            "forge build with --target and --out-dir should parse"
        );
        match cli.unwrap().command {
            Some(Commands::Build {
                files,
                target,
                out_dir,
                ..
            }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
                assert_eq!(out_dir, Some(PathBuf::from("./dist")));
            }
            other => panic!("Expected Commands::Build, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_build_with_pretty_flag() {
        let cli = Cli::try_parse_from(["forge", "build", "test.kzd", "--target", "ts", "--pretty"]);
        assert!(cli.is_ok(), "forge build --pretty should parse");
        match cli.unwrap().command {
            Some(Commands::Build { pretty, .. }) => {
                assert!(pretty, "--pretty flag should be true");
            }
            other => panic!("Expected Commands::Build, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_build_with_source_map_flag() {
        let cli = Cli::try_parse_from([
            "forge",
            "build",
            "test.kzd",
            "--target",
            "ts",
            "--source-map",
        ]);
        assert!(cli.is_ok(), "forge build --source-map should parse");
        match cli.unwrap().command {
            Some(Commands::Build { source_map, .. }) => {
                assert!(source_map, "--source-map flag should be true");
            }
            other => panic!("Expected Commands::Build, got {:?}", other),
        }
    }

    // ── emit ───────────────────────────────────────────────────────────

    #[test]
    fn test_forge_emit_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "emit", "test.kzd", "-t", "py"]);
        assert!(cli.is_ok(), "forge emit -t py should parse");
        match cli.unwrap().command {
            Some(Commands::Emit { files, target, .. }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "py");
            }
            other => panic!("Expected Commands::Emit, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_emit_with_json_flag() {
        let cli = Cli::try_parse_from(["forge", "emit", "test.kzd", "-t", "ts", "--json"]);
        assert!(cli.is_ok(), "forge emit --json should parse");
        match cli.unwrap().command {
            Some(Commands::Emit { json, .. }) => {
                assert!(json, "--json flag should be true");
            }
            other => panic!("Expected Commands::Emit, got {:?}", other),
        }
    }

    // ── run ────────────────────────────────────────────────────────────

    #[test]
    fn test_forge_run_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "run", "test.kzd", "-t", "ts"]);
        assert!(cli.is_ok(), "forge run -t ts should parse");
        match cli.unwrap().command {
            Some(Commands::Run { files, target, .. }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
            }
            other => panic!("Expected Commands::Run, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_run_with_passes() {
        let cli = Cli::try_parse_from([
            "forge",
            "run",
            "test.kzd",
            "-t",
            "ts",
            "--passes",
            "tokenize,parse",
        ]);
        assert!(cli.is_ok(), "forge run --passes should parse");
        match cli.unwrap().command {
            Some(Commands::Run { passes, .. }) => {
                assert_eq!(passes.as_deref(), Some("tokenize,parse"));
            }
            other => panic!("Expected Commands::Run, got {:?}", other),
        }
    }

    // ── dev ────────────────────────────────────────────────────────────

    #[test]
    fn test_forge_dev_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "dev", "test.kzd", "-t", "ts"]);
        assert!(cli.is_ok(), "forge dev -t ts should parse");
        match cli.unwrap().command {
            Some(Commands::Dev { files, target, .. }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
            }
            other => panic!("Expected Commands::Dev, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_dev_with_skip_passes() {
        let cli = Cli::try_parse_from([
            "forge",
            "dev",
            "test.kzd",
            "-t",
            "ts",
            "--skip-passes",
            "lint",
        ]);
        assert!(cli.is_ok(), "forge dev --skip-passes should parse");
        match cli.unwrap().command {
            Some(Commands::Dev { skip_passes, .. }) => {
                assert_eq!(skip_passes.as_deref(), Some("lint"));
            }
            other => panic!("Expected Commands::Dev, got {:?}", other),
        }
    }

    // ── fmt ────────────────────────────────────────────────────────────

    #[test]
    fn test_forge_fmt_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "fmt", "test.kzd"]);
        assert!(cli.is_ok(), "forge fmt test.kzd should parse");
        match cli.unwrap().command {
            Some(Commands::Fmt { files, .. }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
            }
            other => panic!("Expected Commands::Fmt, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_fmt_with_check_flag() {
        let cli = Cli::try_parse_from(["forge", "fmt", "test.kzd", "--check"]);
        assert!(cli.is_ok(), "forge fmt --check should parse");
        match cli.unwrap().command {
            Some(Commands::Fmt { check, .. }) => {
                assert!(check, "--check flag should be true");
            }
            other => panic!("Expected Commands::Fmt, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_fmt_with_stdout_flag() {
        let cli = Cli::try_parse_from(["forge", "fmt", "test.kzd", "--stdout"]);
        assert!(cli.is_ok(), "forge fmt --stdout should parse");
        match cli.unwrap().command {
            Some(Commands::Fmt { stdout, .. }) => {
                assert!(stdout, "--stdout flag should be true");
            }
            other => panic!("Expected Commands::Fmt, got {:?}", other),
        }
    }

    // ── test ───────────────────────────────────────────────────────────

    #[test]
    fn test_forge_test_parses_basic() {
        let cli = Cli::try_parse_from(["forge", "test", "test.kzd", "-t", "ts", "--json"]);
        assert!(cli.is_ok(), "forge test -t ts --json should parse");
        match cli.unwrap().command {
            Some(Commands::Test {
                files,
                target,
                json,
                ..
            }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
                assert!(json, "--json flag should be true");
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_test_with_diff_flag() {
        let cli = Cli::try_parse_from(["forge", "test", "test.kzd", "-t", "ts", "--diff"]);
        assert!(cli.is_ok(), "forge test --diff should parse");
        match cli.unwrap().command {
            Some(Commands::Test { diff, .. }) => {
                assert!(diff, "--diff flag should be true");
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_test_with_fix_flag() {
        let cli = Cli::try_parse_from(["forge", "test", "test.kzd", "-t", "ts", "--fix"]);
        assert!(cli.is_ok(), "forge test --fix should parse");
        match cli.unwrap().command {
            Some(Commands::Test { fix, .. }) => {
                assert!(fix, "--fix flag should be true");
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }
}

// ---------------------------------------------------------------------------
// CLI init tests — RED phase
//
// These tests verify that `forge init` parses correctly and creates the
// expected project files (forge.toml, dwarf.toml). They MUST FAIL right now
// because:
//   1. The `Commands::Init` variant does not exist in the Commands enum.
//   2. The `run_init` function does not exist.
//   3. No project scaffolding logic has been implemented.
//
// Once `forge init` is implemented, these tests will compile and pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod cli_init_tests {
    use super::*;
    use clap::Parser;
    use std::fs;

    // ── CLI Parsing Tests ─────────────────────────────────────────────────
    // These test that clap recognizes `forge init` and its arguments.
    // They will fail to compile until Commands::Init { name: Option<String> }
    // is added to the Commands enum.

    #[test]
    fn test_forge_init_parses() {
        let cli = Cli::try_parse_from(["forge", "init"]);
        assert!(cli.is_ok(), "forge init with no args should parse");
        match cli.unwrap().command {
            Some(Commands::Init { name }) => {
                assert!(name.is_none(), "name should be None when not provided");
            }
            other => panic!("Expected Commands::Init, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_init_with_name_parses() {
        let cli = Cli::try_parse_from(["forge", "init", "my-project"]);
        assert!(cli.is_ok(), "forge init my-project should parse");
        match cli.unwrap().command {
            Some(Commands::Init { name }) => {
                assert_eq!(name.as_deref(), Some("my-project"));
            }
            other => panic!("Expected Commands::Init, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_init_help() {
        // --help causes clap to return a DisplayHelp error (controlled exit).
        // We verify the subcommand is recognized, not rejected as unknown.
        let result = Cli::try_parse_from(["forge", "init", "--help"]);
        let is_recognized = match &result {
            Ok(_) => true,
            Err(e) => matches!(e.kind(), clap::error::ErrorKind::DisplayHelp),
        };
        assert!(is_recognized, "forge init --help should be recognized");
    }

    // ── Integration Tests ─────────────────────────────────────────────────
    // These test the actual scaffolding behavior of `forge init`.
    // They will fail to compile until `run_init` is implemented and exported.
    //
    // Expected signature:
    //   fn run_init(project_dir: Option<&Path>, name: Option<&str>) -> Result<(), String>
    //
    // Behavior:
    //   - If project_dir is None, use current directory.
    //   - If project_dir is Some, create the directory (fail if it already
    //     contains a forge.toml).
    //   - Write forge.toml with [package] name, version, description.
    //   - Write dwarf.toml with compiler target configuration.

    #[test]
    fn test_forge_init_creates_forge_toml() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("new-proj");

        let result = run_init(Some(project_dir.as_path()), None);
        assert!(
            result.is_ok(),
            "forge init should succeed: {:?}",
            result.err()
        );
        assert!(
            project_dir.join("forge.toml").exists(),
            "forge.toml should be created in the project directory"
        );
    }

    #[test]
    fn test_forge_init_creates_dwarf_toml() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("new-proj");

        let result = run_init(Some(project_dir.as_path()), None);
        assert!(
            result.is_ok(),
            "forge init should succeed: {:?}",
            result.err()
        );
        assert!(
            project_dir.join("dwarf.toml").exists(),
            "dwarf.toml should be created in the project directory"
        );
    }

    #[test]
    fn test_forge_init_forge_toml_has_required_fields() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("my-app");

        run_init(Some(project_dir.as_path()), Some("my-app")).expect("forge init should succeed");

        let contents = fs::read_to_string(project_dir.join("forge.toml"))
            .expect("forge.toml should be readable");

        assert!(
            contents.contains("[package]"),
            "forge.toml must have a [package] section"
        );
        assert!(
            contents.contains("name"),
            "forge.toml [package] must have a name field"
        );
        assert!(
            contents.contains("version"),
            "forge.toml [package] must have a version field"
        );
    }

    #[test]
    fn test_forge_init_dwarf_toml_has_target_config() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("my-app");

        run_init(Some(project_dir.as_path()), Some("my-app")).expect("forge init should succeed");

        let contents = fs::read_to_string(project_dir.join("dwarf.toml"))
            .expect("dwarf.toml should be readable");

        assert!(
            contents.contains("target"),
            "dwarf.toml must have target configuration"
        );
    }

    #[test]
    fn test_forge_init_fails_if_directory_exists() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("existing");
        fs::create_dir(&project_dir).unwrap();
        // Pre-create forge.toml to simulate an existing project
        fs::write(
            project_dir.join("forge.toml"),
            "[package]\nname = \"existing\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let result = run_init(Some(project_dir.as_path()), None);
        assert!(
            result.is_err(),
            "forge init should fail when directory already contains a forge.toml"
        );
    }

    // ── Externs Directory Tests ───────────────────────────────────────────
    // Verify that `forge init` creates an empty externs/ directory for
    // extern declaration files. These MUST FAIL until run_init is updated
    // to create the externs/ directory during scaffolding.

    #[test]
    fn test_forge_init_creates_externs_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project_dir = dir.path().join("my-app");

        let result = run_init(Some(project_dir.as_path()), Some("my-app"));
        assert!(
            result.is_ok(),
            "forge init should succeed: {:?}",
            result.err()
        );

        let externs_dir = project_dir.join("externs");
        assert!(
            externs_dir.exists(),
            "forge init should create an externs/ directory"
        );
        assert!(externs_dir.is_dir(), "externs/ should be a directory");
    }
}

// ---------------------------------------------------------------------------
// CLI add tests — RED phase
//
// These tests verify that `forge add` parses correctly and manages
// dependencies in forge.toml. They MUST FAIL right now because:
//   1. The `Commands::Add` variant does not exist in the Commands enum.
//   2. The `run_add` function does not exist.
//   3. No package management logic has been implemented.
//
// Once `forge add` is implemented, these tests will compile and pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod cli_add_tests {
    use super::*;
    use clap::Parser;
    use std::fs;

    // ── CLI Parsing Tests ─────────────────────────────────────────────────
    // These test that clap recognizes `forge add` and its arguments.
    // They will fail to compile until Commands::Add { package: String }
    // is added to the Commands enum.

    #[test]
    fn test_forge_add_npm_parses() {
        let cli = Cli::try_parse_from(["forge", "add", "npm:express"]);
        assert!(cli.is_ok(), "forge add npm:express should parse");
        match cli.unwrap().command {
            Some(Commands::Add { package }) => {
                assert_eq!(package, "npm:express");
            }
            other => panic!("Expected Commands::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_add_py_parses() {
        let cli = Cli::try_parse_from(["forge", "add", "py:requests"]);
        assert!(cli.is_ok(), "forge add py:requests should parse");
        match cli.unwrap().command {
            Some(Commands::Add { package }) => {
                assert_eq!(package, "py:requests");
            }
            other => panic!("Expected Commands::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_add_java_parses() {
        let cli = Cli::try_parse_from(["forge", "add", "java:com.google.gson.Gson"]);
        assert!(
            cli.is_ok(),
            "forge add java:com.google.gson.Gson should parse"
        );
        match cli.unwrap().command {
            Some(Commands::Add { package }) => {
                assert_eq!(package, "java:com.google.gson.Gson");
            }
            other => panic!("Expected Commands::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_add_requires_package() {
        let result = Cli::try_parse_from(["forge", "add"]);
        match result {
            Err(e) => {
                assert_ne!(
                    e.kind(),
                    clap::error::ErrorKind::InvalidSubcommand,
                    "error should be about missing required argument, not unknown subcommand"
                );
            }
            Ok(_) => panic!("Expected parse error for missing package argument"),
        }
    }

    #[test]
    fn test_forge_add_invalid_format() {
        // "invalid" has no colon — should be rejected at parse time
        let result = Cli::try_parse_from(["forge", "add", "invalid"]);
        match result {
            Err(e) => {
                assert_ne!(
                    e.kind(),
                    clap::error::ErrorKind::InvalidSubcommand,
                    "error should be about invalid package format, not unknown subcommand"
                );
            }
            Ok(_) => panic!("Expected parse error for invalid package format"),
        }
    }

    // ── Integration Tests ─────────────────────────────────────────────────
    // These test the actual package management behavior of `forge add`.
    // They will fail to compile until `run_add` is implemented and exported.
    //
    // Expected signature:
    //   fn run_add(project_dir: &Path, package: &str) -> Result<String, String>
    //
    // Behavior:
    //   - Reads forge.toml from project_dir.
    //   - Parses the package string (prefix:name format).
    //   - Adds the dependency to the [dependencies] section.
    //   - Returns the generated extern declaration stub.
    //   - Returns Err if forge.toml is missing or package format is invalid.

    #[test]
    fn test_forge_add_npm_adds_to_forge_toml() {
        let dir = tempfile::tempdir().unwrap();
        let forge_toml = dir.path().join("forge.toml");
        fs::write(
            &forge_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        let result = run_add(dir.path(), "npm:express");
        assert!(result.is_ok(), "run_add should succeed: {:?}", result.err());

        let contents = fs::read_to_string(&forge_toml).unwrap();
        assert!(
            contents.contains("express"),
            "forge.toml should contain the added npm dependency"
        );
    }

    #[test]
    fn test_forge_add_extern_stub_printed() {
        let dir = tempfile::tempdir().unwrap();
        let forge_toml = dir.path().join("forge.toml");
        fs::write(
            &forge_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        let result = run_add(dir.path(), "npm:express");
        assert!(result.is_ok(), "run_add should succeed");
        let stub = result.unwrap();
        assert!(
            stub.contains("extern") && stub.contains("npm:express"),
            "run_add should return an extern stub containing the package source, got: {}",
            stub
        );
    }

    // ── Extern File Generation Tests ──────────────────────────────────────
    // These verify that `forge add` writes the extern declaration to a .kzd
    // file under externs/{prefix}/{name}.kzd. They MUST FAIL until the
    // extern auto-generation feature is implemented in run_add().

    #[test]
    fn test_forge_add_generates_extern_file() {
        let dir = tempfile::tempdir().unwrap();
        let forge_toml = dir.path().join("forge.toml");
        fs::write(
            &forge_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        let result = run_add(dir.path(), "npm:express");
        assert!(result.is_ok(), "run_add should succeed: {:?}", result.err());

        let extern_file = dir.path().join("externs").join("npm").join("express.kzd");
        assert!(
            extern_file.exists(),
            "forge add should create externs/npm/express.kzd, but it does not exist"
        );
    }

    #[test]
    fn test_forge_add_extern_file_has_correct_content() {
        let dir = tempfile::tempdir().unwrap();
        let forge_toml = dir.path().join("forge.toml");
        fs::write(
            &forge_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        let result = run_add(dir.path(), "npm:express");
        assert!(result.is_ok(), "run_add should succeed: {:?}", result.err());

        let extern_file = dir.path().join("externs").join("npm").join("express.kzd");
        let contents =
            fs::read_to_string(&extern_file).expect("externs/npm/express.kzd should be readable");

        assert!(
            contents.contains("extern"),
            "extern file should contain an extern declaration, got: {}",
            contents
        );
        assert!(
            contents.contains("npm:express"),
            "extern file should reference the package source 'npm:express', got: {}",
            contents
        );
    }

    #[test]
    fn test_forge_add_creates_extern_dir() {
        let dir = tempfile::tempdir().unwrap();
        let forge_toml = dir.path().join("forge.toml");
        fs::write(
            &forge_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();

        // Verify externs/ does not exist before forge add
        assert!(
            !dir.path().join("externs").exists(),
            "externs/ should not exist before forge add"
        );

        let result = run_add(dir.path(), "py:requests");
        assert!(result.is_ok(), "run_add should succeed: {:?}", result.err());

        let externs_dir = dir.path().join("externs");
        assert!(
            externs_dir.exists(),
            "forge add should create the externs/ directory"
        );
        assert!(externs_dir.is_dir(), "externs/ should be a directory");
    }
}
