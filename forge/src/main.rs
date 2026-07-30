use clap::{Parser, Subcommand};
use std::path::PathBuf;

use dwarf_cli::{build, check, dev, emit, fmt, run, test};

#[derive(Parser)]
#[command(name = "forge", version, about = "Dwarf platform CLI — build, manage dependencies, and more")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
            build::run_build(files, target, out_dir, pretty, source_map, json, passes, skip_passes, stdlib_path);
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
        None => {
            eprintln!("Error: No subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
    }
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
        let cli = Cli::try_parse_from([
            "forge", "check", "test.kzd", "--passes", "tokenize,parse",
        ]);
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
        let cli = Cli::try_parse_from([
            "forge", "check", "test.kzd", "--skip-passes", "lint",
        ]);
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
            "forge", "build", "test.kzd", "--target", "ts", "--out-dir", "./dist",
        ]);
        assert!(cli.is_ok(), "forge build with --target and --out-dir should parse");
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
        let cli = Cli::try_parse_from([
            "forge", "build", "test.kzd", "--target", "ts", "--pretty",
        ]);
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
            "forge", "build", "test.kzd", "--target", "ts", "--source-map",
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
            Some(Commands::Emit {
                files, target, ..
            }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "py");
            }
            other => panic!("Expected Commands::Emit, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_emit_with_json_flag() {
        let cli = Cli::try_parse_from([
            "forge", "emit", "test.kzd", "-t", "ts", "--json",
        ]);
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
            Some(Commands::Run {
                files, target, ..
            }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
            }
            other => panic!("Expected Commands::Run, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_run_with_passes() {
        let cli = Cli::try_parse_from([
            "forge", "run", "test.kzd", "-t", "ts", "--passes", "tokenize,parse",
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
            Some(Commands::Dev {
                files, target, ..
            }) => {
                assert_eq!(files, vec![PathBuf::from("test.kzd")]);
                assert_eq!(target, "ts");
            }
            other => panic!("Expected Commands::Dev, got {:?}", other),
        }
    }

    #[test]
    fn test_forge_dev_with_skip_passes() {
        let cli = Cli::try_parse_from([
            "forge", "dev", "test.kzd", "-t", "ts", "--skip-passes", "lint",
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
        let cli = Cli::try_parse_from([
            "forge", "test", "test.kzd", "-t", "ts", "--json",
        ]);
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
        let cli = Cli::try_parse_from([
            "forge", "test", "test.kzd", "-t", "ts", "--diff",
        ]);
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
        let cli = Cli::try_parse_from([
            "forge", "test", "test.kzd", "-t", "ts", "--fix",
        ]);
        assert!(cli.is_ok(), "forge test --fix should parse");
        match cli.unwrap().command {
            Some(Commands::Test { fix, .. }) => {
                assert!(fix, "--fix flag should be true");
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }
}
