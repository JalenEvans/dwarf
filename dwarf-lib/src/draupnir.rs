//! Draupnir — Dwarf Property-Based Testing Runtime (Rust module)
//!
//! This module provides the Rust-side interface to the Draupnir PBT library.
//! It reads the Draupnir Dwarf source files and compiles them through the
//! standard pipeline.
//!
//! DWARF-119: Implementation complete — compiles via the standard pipeline.

use std::path::PathBuf;

use crate::{CompileOptions, DwarfCompiler};

/// The Draupnir runtime source files, in load order.
const DRAUPNIR_SOURCES: [&str; 3] = ["draupnir.dwarf", "combinators.dwarf", "shrink.dwarf"];

/// Compile a single Draupnir source file through the standard pipeline.
///
/// Mirrors `dunit::compile_dunit` for one file of the runtime library.
fn compile_source(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source_path = PathBuf::from(manifest_dir)
        .join("runtime")
        .join("draupnir")
        .join(name);

    // Read the source file so compilation happens through the same string-based
    // entry point the rest of the pipeline uses.
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("draupnir source file {:?} should exist: {}", source_path, e));

    // Compile through the standard pipeline.
    let compiler = DwarfCompiler::new();
    let filename = source_path
        .to_str()
        .map(str::to_string)
        .unwrap_or_else(|| name.to_string());
    let options = CompileOptions::default();

    let result = compiler
        .compile(&source, &filename, options)
        .unwrap_or_else(|errors| {
            let msgs = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            panic!("Failed to compile {}:\n{}", name, msgs)
        });

    result.output
}

/// Compile the Draupnir property-based testing library and return the combined
/// compiled output.
///
/// This function reads the Draupnir runtime source files
/// (`draupnir.dwarf`, `combinators.dwarf`, `shrink.dwarf`) from the runtime
/// directory, compiles each through the standard Dwarf compiler pipeline, and
/// concatenates the individual outputs into a single runtime module.
///
/// # Returns
///
/// A string containing the compiled output (target language determined by
/// default compiler options — typically TypeScript).
///
/// # Panics
///
/// This function will panic if:
/// - Any draupnir source file cannot be found
/// - Any source file cannot be read
/// - Compilation fails
///
/// # Examples
///
/// ```rust
/// let output = dwarf_lib::draupnir::compile_draupnir();
/// assert!(!output.is_empty());
/// ```
pub fn compile_draupnir() -> String {
    DRAUPNIR_SOURCES
        .iter()
        .map(|name| compile_source(name))
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_draupnir_returns_string() {
        // This test verifies the function signature is correct
        let _output: String = compile_draupnir();
    }
}
