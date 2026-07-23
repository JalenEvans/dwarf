//! CLI entry point for the Dwarf compiler.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod check;
mod emit;

#[derive(Parser)]
#[command(name = "dwarf-cli", version, about = "Dwarf compiler toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            files,
            json,
            passes,
            skip_passes,
            list_passes,
        } => {
            check::run_check(files, json, passes, skip_passes, list_passes);
        }
        Commands::Emit {
            files,
            target,
            json,
            passes,
            skip_passes,
        } => {
            emit::run_emit(files, target, json, passes, skip_passes);
        }
    }
}
