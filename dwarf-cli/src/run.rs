//! Implementation of the `dwarf run` subcommand.
//!
//! Transpiles Dwarf source files to a target language and executes them
//! using the appropriate runtime.

use std::path::PathBuf;
use std::process;

use dwarf_cli::runner::{Runner, TsRunner};

/// Run the run subcommand.
pub fn run_run(
    files: Vec<PathBuf>,
    target: String,
    _passes: Option<String>,
    _skip_passes: Option<String>,
) {
    // Validate target
    if target != "ts" {
        eprintln!(
            "Error: Unsupported target '{}'. Supported targets: ts",
            target
        );
        process::exit(1);
    }

    // For each file, compile and run
    let mut has_errors = false;
    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();
        let runner = TsRunner::new();
        match runner.run(file_path) {
            Ok(output) => {
                print!("{}", output);
            }
            Err(e) => {
                eprintln!("Error running {}: {}", path_str, e);
                has_errors = true;
            }
        }
    }

    if has_errors {
        process::exit(1);
    }
}
