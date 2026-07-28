//! Implementation of the `dwarf dev` subcommand.
//!
//! Watches Dwarf source files for changes, debounces, and automatically
//! re-transpiles and re-executes them.

use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use dwarf_cli::runner::{Runner, TsRunner};

/// Run the dev (watch) subcommand.
pub fn run_dev(
    files: Vec<PathBuf>,
    target: String,
    _passes: Option<String>,
    _skip_passes: Option<String>,
    _stdlib_path: Option<String>,
) {
    // Validate target
    if target != "ts" {
        eprintln!(
            "Error: Unsupported target '{}'. Supported targets: ts",
            target
        );
        process::exit(1);
    }

    // Run once first
    println!("[dwarf dev] Initial run...");
    run_files(&files);

    // Set up file watcher
    println!("[dwarf dev] Watching for changes...");
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error: Cannot create file watcher: {}", e);
            process::exit(1);
        }
    };

    // Watch all source files and their parent directories
    for file in &files {
        if let Some(parent) = file.parent() {
            if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                eprintln!("Warning: Cannot watch '{}': {}", parent.display(), e);
            }
        }
    }

    // Debounce: track last event time to avoid rapid restarts
    let debounce = Duration::from_millis(500);
    let mut last_event = std::time::Instant::now();

    for res in rx {
        match res {
            Ok(event) => {
                // Check if any watched file was modified
                let relevant = event.paths.iter().any(|p| files.iter().any(|f| f == p));
                if !relevant {
                    continue;
                }

                let is_modify = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));

                if !is_modify {
                    continue;
                }

                let now = std::time::Instant::now();
                if now.duration_since(last_event) >= debounce {
                    last_event = now;
                    println!("\n[dwarf dev] Change detected, re-running...");
                    run_files(&files);
                    println!("[dwarf dev] Watching for changes...");
                }
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
            }
        }
    }
}

/// Run all specified files through the TsRunner.
fn run_files(files: &[PathBuf]) {
    for file_path in files {
        let path_str = file_path.to_string_lossy().to_string();
        let runner = TsRunner::new();
        match runner.run(file_path) {
            Ok(output) => {
                println!("--- {} ---\n{}", path_str, output);
            }
            Err(e) => {
                eprintln!("--- {} ---\nError: {}", path_str, e);
            }
        }
    }
}
