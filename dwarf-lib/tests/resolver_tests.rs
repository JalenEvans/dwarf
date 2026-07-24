//! Integration tests for `ModuleResolver` trait and its implementations.
//!
//! These tests define the expected API contract for resolving module import
//! specifiers to source content. All tests are expected to FAIL in the current
//! Red phase because the trait and implementations (`PureResolver`,
//! `FilesystemResolver`) do not exist yet. They serve as the specification.
//!
//! Once implemented:
//! - `PureResolver` always returns `Err(DwarfError::Config(...))` — it cannot
//!   resolve anything (used for WASM/browser environments).
//! - `FilesystemResolver` resolves specifiers relative to the containing file's
//!   directory by reading from the filesystem.

use dwarf_lib::{DwarfError, FilesystemResolver, ModuleResolver, PureResolver};
use std::io::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a temporary directory for filesystem-based tests.
fn test_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Write content to a file within a temp directory, returning the canonical path.
fn write_file(dir: &tempfile::TempDir, rel_path: &str, content: &str) -> std::path::PathBuf {
    let full_path = dir.path().join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directories");
    }
    let mut file = std::fs::File::create(&full_path).expect("Failed to create file");
    file.write_all(content.as_bytes())
        .expect("Failed to write content");
    full_path
}

/// Create a `FilesystemResolver` for use in tests.
fn filesystem_resolver() -> FilesystemResolver {
    FilesystemResolver::new()
}

// ===========================================================================
// PureResolver Tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 1: PureResolver always returns a Config error
// ---------------------------------------------------------------------------

#[test]
fn pure_resolver_returns_error() {
    let resolver = PureResolver;

    let result = resolver.resolve("./utils", "test.dwarf");

    match result {
        Err(DwarfError::Config(msg)) => {
            assert!(!msg.is_empty(), "Config error message should not be empty");
        }
        other => panic!("Expected Err(DwarfError::Config(...)), got: {:?}", other),
    }
}

// ===========================================================================
// FilesystemResolver Tests
// ===========================================================================

// ---------------------------------------------------------------------------
// Test 2: Resolve an existing file
// ---------------------------------------------------------------------------

#[test]
fn filesystem_resolver_resolves_existing_file() {
    let dir = test_dir();
    let file_path = write_file(&dir, "test_import.dwarf", "fn greet() = \"hello\"");
    let containing_file = dir.path().join("main.dwarf");
    let containing_file_str = containing_file.to_string_lossy().to_string();

    let resolver = filesystem_resolver();
    let result = resolver.resolve("./test_import.dwarf", &containing_file_str);

    match result {
        Ok(content) => {
            assert_eq!(
                content, "fn greet() = \"hello\"",
                "Resolved content should match what was written to the file"
            );
        }
        Err(e) => panic!("Expected Ok(content) for existing file, got Err({:?})", e),
    }
}

// ---------------------------------------------------------------------------
// Test 3: Resolve a missing file returns an Io error
// ---------------------------------------------------------------------------

#[test]
fn filesystem_resolver_returns_error_for_missing_file() {
    let resolver = filesystem_resolver();

    let result = resolver.resolve("./nonexistent.dwarf", "/tmp/some_file.dwarf");

    match result {
        Err(DwarfError::Io(msg)) => {
            assert!(!msg.is_empty(), "IO error message should not be empty");
        }
        other => panic!(
            "Expected Err(DwarfError::Io(...)) for missing file, got: {:?}",
            other
        ),
    }
}

// ---------------------------------------------------------------------------
// Test 4: Resolve relative to containing file's directory
// ---------------------------------------------------------------------------

#[test]
fn filesystem_resolver_resolves_relative_to_containing_file() {
    let dir = test_dir();

    // Create directory structure:
    //   <tmp>/src/main.dwarf          (the containing file — not actually read)
    //   <tmp>/src/utils/helper.dwarf  (the import target)
    write_file(&dir, "src/utils/helper.dwarf", "fn helper() = 99");

    let containing_file = dir.path().join("src/main.dwarf");
    let containing_file_str = containing_file.to_string_lossy().to_string();

    // Create the containing file too (we want it to exist, even though the
    // resolver only uses its directory).
    write_file(&dir, "src/main.dwarf", "fn main() = helper()");

    let resolver = filesystem_resolver();
    let result = resolver.resolve("./utils/helper.dwarf", &containing_file_str);

    match result {
        Ok(content) => {
            assert_eq!(
                content, "fn helper() = 99",
                "Should resolve ./utils/helper.dwarf relative to src/main.dwarf's directory"
            );
        }
        Err(e) => panic!("Expected Ok(content) for relative import, got Err({:?})", e),
    }
}
