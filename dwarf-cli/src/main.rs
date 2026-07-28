//! CLI entry point for the Dwarf compiler.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod build;
mod check;
mod dev;
mod emit;
mod fmt;
mod output;
mod run;
mod test;

#[derive(Parser)]
#[command(
    name = "dwarf-cli",
    version,
    about = "Dwarf compiler toolchain",
    subcommand_required = false
)]
struct Cli {
    /// List available runtime targets and exit
    #[arg(long, global = true, id = "list-runtimes")]
    list_runtimes: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
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
}

fn main() {
    let cli = Cli::parse();

    if cli.list_runtimes {
        for rt in dwarf_cli::runner::list_runtimes() {
            println!("{}", rt);
        }
        return;
    }

    match cli.command {
        Some(Commands::Run {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path: _,
        }) => {
            run::run_run(files, target, passes, skip_passes);
        }
        Some(Commands::Check {
            files,
            json,
            passes,
            skip_passes,
            list_passes,
            stdlib_path: _,
        }) => {
            check::run_check(files, json, passes, skip_passes, list_passes);
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
        Some(Commands::Dev {
            files,
            target,
            passes,
            skip_passes,
            stdlib_path: _,
        }) => {
            dev::run_dev(files, target, passes, skip_passes);
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
