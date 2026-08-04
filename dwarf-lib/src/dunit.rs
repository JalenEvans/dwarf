//! dUnit — Dwarf Unit Testing Assertion Library (Rust module)
//!
//! This module provides the Rust-side interface to the dUnit assertion library.
//! It reads the Dwarf source file and compiles it through the standard pipeline.
//!
//! DWARF-118: Implementation complete — compiles via the standard pipeline.

use std::path::PathBuf;

use crate::{CompileOptions, DwarfCompiler};

/// Compile the dUnit assertion library source and return the compiled output.
///
/// This function reads the dunit.dwarf source file from the runtime directory
/// and compiles it through the standard Dwarf compiler pipeline.
///
/// # Returns
///
/// A string containing the compiled output (target language determined by
/// default compiler options — typically TypeScript).
///
/// # Panics
///
/// This function will panic if:
/// - The dunit.dwarf source file cannot be found
/// - The source file cannot be read
/// - Compilation fails
///
/// # Examples
///
/// ```rust
/// let output = dwarf_lib::dunit::compile_dunit();
/// assert!(!output.is_empty());
/// ```
pub fn compile_dunit() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dunit_path = PathBuf::from(manifest_dir)
        .join("runtime")
        .join("dunit")
        .join("dunit.dwarf");

    // Read the source file so compilation happens through the same string-based
    // entry point the rest of the pipeline uses.
    let source =
        std::fs::read_to_string(&dunit_path).expect("dunit.dwarf source file should exist");

    // Compile through the standard pipeline.
    let compiler = DwarfCompiler::new();
    let filename = dunit_path
        .to_str()
        .map(str::to_string)
        .unwrap_or_else(|| "dunit.dwarf".to_string());
    let options = CompileOptions::default();

    let result = compiler
        .compile(&source, &filename, options)
        .unwrap_or_else(|errors| {
            let msgs = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            panic!("Failed to compile dunit.dwarf:\n{}", msgs)
        });

    result.output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_dunit_returns_string() {
        // This test verifies the function signature is correct
        let _output: String = compile_dunit();
    }
}
