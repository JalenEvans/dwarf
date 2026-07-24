//! CLI entry point for the Dwarf compiler.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod build;
mod check;
mod dev;
mod emit;
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

        /// Comma-separated list of passes to run
        #[arg(long)]
        passes: Option<String>,

        /// Comma-separated list of passes to skip
        #[arg(long)]
        skip_passes: Option<String>,
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
        }) => {
            run::run_run(files, target, passes, skip_passes);
        }
        Some(Commands::Check {
            files,
            json,
            passes,
            skip_passes,
            list_passes,
        }) => {
            check::run_check(files, json, passes, skip_passes, list_passes);
        }
        Some(Commands::Emit {
            files,
            target,
            json,
            passes,
            skip_passes,
        }) => {
            emit::run_emit(files, target, json, passes, skip_passes);
        }
        Some(Commands::Dev {
            files,
            target,
            passes,
            skip_passes,
        }) => {
            dev::run_dev(files, target, passes, skip_passes);
        }
        Some(Commands::Build {
            files,
            target,
            out_dir,
            pretty,
            source_map,
            passes,
            skip_passes,
        }) => {
            build::run_build(
                files,
                target,
                out_dir,
                pretty,
                source_map,
                passes,
                skip_passes,
            );
        }
        Some(Commands::Test {
            files,
            target,
            json,
            diff,
        }) => {
            test::run_test(files, target, json, diff);
        }
        None => {
            eprintln!("Error: No subcommand provided. Use --help for usage.");
            std::process::exit(1);
        }
    }
}
