//! Library entry point for the Dwarf CLI.
//! Exposes the pass manager, runner, diff runner, and config discovery.

use clap::{Parser, Subcommand};
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
pub mod install;
pub mod output;
pub mod run;
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

#[derive(Subcommand)]
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
    },

    /// Install a package and generate an extern declaration stub
    Install {
        /// Package identifier in the form '<prefix>:<name>' (e.g. 'npm:express', 'py:json', 'java:java.util.ArrayList')
        #[arg(required = true)]
        package: String,
    },
}

// ---------------------------------------------------------------------------
// Library surface tests — RED phase
//
// These tests verify that each subcommand's entry-point function is reachable
// through the library crate (dwarf_cli). They MUST FAIL right now because the
// subcommand modules (build, check, dev, emit, fmt, install, run, test) are
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
        // list_passes=false, no stdlib_path.
        crate::check::run_check(
            Vec::<PathBuf>::new(),
            false,
            None,
            None,
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
        crate::run::run_run(
            Vec::<PathBuf>::new(),
            String::from("ts"),
            None,
            None,
            None,
        );
    }

    // 5. dev::run_dev
    #[test]
    fn test_run_dev_is_accessible_via_library() {
        crate::dev::run_dev(
            Vec::<PathBuf>::new(),
            String::from("ts"),
            None,
            None,
            None,
        );
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
        );
    }

    // 8. install::run_install
    #[test]
    fn test_run_install_is_accessible_via_library() {
        crate::install::run_install("npm:example-pkg");
    }
}
