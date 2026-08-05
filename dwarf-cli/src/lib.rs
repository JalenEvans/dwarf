//! Library entry point for the Dwarf CLI.
//! Exposes the pass manager, runner, diff runner, and config discovery.

use clap::{Parser, Subcommand};
#[allow(unused_imports)]
use dwarf_lib::{CompileOptions, CoverageMode};
use std::path::PathBuf;

pub mod config;
pub mod diff_runner;
pub mod pass_manager;
pub mod runner;

pub mod build;
pub mod check;
pub mod dev;
pub mod emit;
pub mod fmt;
pub mod init;
pub mod output;
pub mod run;
pub mod splash;
pub mod test;

#[derive(Parser)]
#[command(
    name = "dwarf-cli",
    version,
    about = "Dwarf compiler toolchain",
    subcommand_required = false
)]
pub struct Cli {
    /// List available runtime targets and exit
    #[arg(long, global = true, id = "list-runtimes")]
    pub list_runtimes: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

        /// Bypass all coverage checks
        #[arg(long)]
        quick: bool,

        /// Bypass edge-case analysis only
        #[arg(long = "skip-edge-check")]
        skip_edge_check: bool,

        /// Coverage enforcement mode (on, off, warning, required)
        #[arg(long = "test-coverage")]
        test_coverage: Option<CoverageMode>,
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

        /// Bypass all coverage checks
        #[arg(long)]
        quick: bool,

        /// Bypass edge-case analysis only
        #[arg(long = "skip-edge-check")]
        skip_edge_check: bool,

        /// Coverage enforcement mode (on, off, warning, required)
        #[arg(long = "test-coverage")]
        test_coverage: Option<CoverageMode>,
    },

    /// Initialize a new Dwarf project
    Init {
        /// Project name (directory will be created)
        name: String,
    },
}

// ---------------------------------------------------------------------------
// Library surface tests — RED phase
//
// These tests verify that each subcommand's entry-point function is reachable
// through the library crate (dwarf_cli). They MUST FAIL right now because the
// subcommand modules (build, check, dev, emit, fmt, run, test) are
// declared as private modules inside main.rs and are not part of the library.
//
// Once the refactoring moves those modules into lib.rs as `pub mod`, these
// tests will compile and pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod library_surface_tests {
    use std::path::PathBuf;

    // 1. check::run_check
    #[test]
    fn test_run_check_is_accessible_via_library() {
        // Minimal valid args: no files, no json, no passes, no skip_passes,
        // list_passes=false, no stdlib_path, no coverage flags.
        crate::check::run_check(
            Vec::<PathBuf>::new(),
            false,
            None,
            None,
            false,
            None,
            false,
            false,
            None,
        );
    }

    // 2. build::run_build
    #[test]
    fn test_run_build_is_accessible_via_library() {
        crate::build::run_build(
            Vec::<PathBuf>::new(),
            String::from("ts"),
            None,
            false,
            false,
            false,
            None,
            None,
            None,
        );
    }

    // 3. emit::run_emit
    #[test]
    fn test_run_emit_is_accessible_via_library() {
        crate::emit::run_emit(
            Vec::<PathBuf>::new(),
            String::from("ts"),
            false,
            None,
            None,
            None,
        );
    }

    // 4. run::run_run
    #[test]
    fn test_run_run_is_accessible_via_library() {
        crate::run::run_run(Vec::<PathBuf>::new(), String::from("ts"), None, None, None);
    }

    // 5. dev::run_dev
    #[test]
    fn test_run_dev_is_accessible_via_library() {
        crate::dev::run_dev(Vec::<PathBuf>::new(), String::from("ts"), None, None, None);
    }

    // 6. fmt::run_fmt
    #[test]
    fn test_run_fmt_is_accessible_via_library() {
        crate::fmt::run_fmt(Vec::<PathBuf>::new(), false, false);
    }

    // 7. test::run_test
    #[test]
    fn test_run_test_is_accessible_via_library() {
        crate::test::run_test(
            Vec::<PathBuf>::new(),
            String::from("ts"),
            false,
            false,
            false,
            false,
            false,
            None,
        );
    }
}

// ---------------------------------------------------------------------------
// CLI coverage flag tests — RED PHASE (DWARF-117)
//
// These tests verify that the CLI supports coverage-related flags:
// - --quick: bypass all coverage checks
// - --skip-edge-check: bypass edge analysis only
// - --test-coverage=off: disable all coverage enforcement
//
// They MUST FAIL right now because:
//   1. The `CompileOptions` struct does not have `quick`, `skip_edge_check`,
//      or `test_coverage` fields.
//   2. The CLI commands do not parse these flags.
//
// Once the fields and flags are added, these tests will compile and pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod coverage_flag_tests {
    use super::*;
    use clap::Parser;

    // -- Test 6: --quick flag -----------------------------------------------

    #[test]
    /// The --quick flag should bypass all coverage checks.
    /// When --quick is set, no coverage errors or warnings should be emitted.
    fn test_quick_flag_bypasses_coverage_checks() {
        // Test that CompileOptions has a `quick` field
        let opts = CompileOptions {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: Vec::new(),
            source_map: false,
            stdlib_path: None,
            quick: true,
            ..Default::default()
        };
        assert!(opts.quick, "--quick flag should be true");
    }

    #[test]
    /// The --quick flag should be parseable from the CLI for the test subcommand.
    fn test_quick_flag_parses_in_test_subcommand() {
        let cli = Cli::try_parse_from(["dwarf-cli", "test", "test.kzd", "-t", "ts", "--quick"]);
        assert!(
            cli.is_ok(),
            "dwarf-cli test --quick should parse: {:?}",
            cli.err()
        );
        match cli.unwrap().command {
            Some(Commands::Test { quick, .. }) => {
                assert!(quick, "--quick flag should be true");
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }

    #[test]
    /// The --quick flag should be parseable from the CLI for the check subcommand.
    fn test_quick_flag_parses_in_check_subcommand() {
        let cli = Cli::try_parse_from(["dwarf-cli", "check", "test.kzd", "--quick"]);
        assert!(
            cli.is_ok(),
            "dwarf-cli check --quick should parse: {:?}",
            cli.err()
        );
        match cli.unwrap().command {
            Some(Commands::Check { quick, .. }) => {
                assert!(quick, "--quick flag should be true");
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    // -- Test 7: --skip-edge-check flag -------------------------------------

    #[test]
    /// The --skip-edge-check flag should bypass edge analysis only,
    /// but coverage checks should still run.
    fn test_skip_edge_check_flag_bypasses_edge_analysis_only() {
        let opts = CompileOptions {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: Vec::new(),
            source_map: false,
            stdlib_path: None,
            quick: false,
            skip_edge_check: true,
            test_coverage: CoverageMode::On,
        };
        assert!(
            opts.skip_edge_check,
            "--skip-edge-check flag should be true"
        );
        assert!(
            !opts.quick,
            "--quick should be false when only --skip-edge-check is set"
        );
    }

    #[test]
    /// The --skip-edge-check flag should be parseable from the CLI.
    fn test_skip_edge_check_flag_parses() {
        let cli = Cli::try_parse_from(["dwarf-cli", "check", "test.kzd", "--skip-edge-check"]);
        assert!(
            cli.is_ok(),
            "dwarf-cli check --skip-edge-check should parse: {:?}",
            cli.err()
        );
        match cli.unwrap().command {
            Some(Commands::Check {
                skip_edge_check, ..
            }) => {
                assert!(skip_edge_check, "--skip-edge-check flag should be true");
            }
            other => panic!("Expected Commands::Check, got {:?}", other),
        }
    }

    // -- Test 8: --test-coverage=off flag -----------------------------------

    #[test]
    /// The --test-coverage=off flag should disable all coverage enforcement.
    fn test_coverage_off_flag_disables_enforcement() {
        let opts = CompileOptions {
            target: "ts".to_string(),
            pretty: false,
            passes: None,
            skip_passes: Vec::new(),
            source_map: false,
            stdlib_path: None,
            quick: false,
            skip_edge_check: false,
            test_coverage: CoverageMode::Off,
        };
        assert!(
            matches!(opts.test_coverage, CoverageMode::Off),
            "--test-coverage=off should set test_coverage to Off"
        );
    }

    #[test]
    /// The --test-coverage=off flag should be parseable from the CLI.
    fn test_coverage_off_flag_parses() {
        let cli = Cli::try_parse_from([
            "dwarf-cli",
            "test",
            "test.kzd",
            "-t",
            "ts",
            "--test-coverage=off",
        ]);
        assert!(
            cli.is_ok(),
            "dwarf-cli test --test-coverage=off should parse: {:?}",
            cli.err()
        );
        match cli.unwrap().command {
            Some(Commands::Test { test_coverage, .. }) => {
                assert!(
                    matches!(test_coverage, Some(CoverageMode::Off)),
                    "--test-coverage=off should set test_coverage to Off"
                );
            }
            other => panic!("Expected Commands::Test, got {:?}", other),
        }
    }

    #[test]
    /// The --test-coverage=on flag should be the default.
    fn test_coverage_on_is_default() {
        let opts = CompileOptions::default();
        assert!(
            matches!(opts.test_coverage, CoverageMode::On),
            "Default test_coverage should be On"
        );
    }

    // -- CoverageMode enum tests --------------------------------------------

    #[test]
    /// CoverageMode enum should have On and Off variants.
    fn test_coverage_mode_enum_variants() {
        let on = CoverageMode::On;
        let off = CoverageMode::Off;

        assert!(matches!(on, CoverageMode::On));
        assert!(matches!(off, CoverageMode::Off));
        assert_ne!(
            format!("{:?}", on),
            format!("{:?}", off),
            "On and Off should be distinct variants"
        );
    }
}
