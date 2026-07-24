//! Module resolution abstractions for the Dwarf compiler.
//!
//! The [`ModuleResolver`] trait abstracts how import specifiers are resolved
//! to source content. Different implementations enable the compiler to work
//! in different environments (CLI filesystem, WASM sandbox, test mocks).

use crate::DwarfError;
use std::path::{Path, PathBuf};

/// A trait for resolving module import specifiers to source content.
pub trait ModuleResolver {
    /// Resolve an import specifier to source content.
    ///
    /// `specifier` is the import path from source (e.g., `"./utils"`)
    /// `containing_file` is the file that contains the import
    ///
    /// Returns the resolved source content or a [`DwarfError`].
    fn resolve(&self, specifier: &str, containing_file: &str) -> Result<String, DwarfError>;
}

/// A resolver that never resolves any imports.
///
/// Suitable for environments without filesystem access (e.g., WASM/browser).
/// Returns a `DwarfError::Config` error for any resolution attempt.
pub struct PureResolver;

impl PureResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PureResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver for PureResolver {
    fn resolve(&self, specifier: &str, containing_file: &str) -> Result<String, DwarfError> {
        Err(DwarfError::Config(format!(
            "Cannot resolve import '{}' from '{}': pure resolver has no filesystem access",
            specifier, containing_file
        )))
    }
}

/// A resolver that resolves imports relative to the containing file's directory.
///
/// This is the standard resolver for CLI usage.
pub struct FilesystemResolver;

impl FilesystemResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilesystemResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver for FilesystemResolver {
    fn resolve(&self, specifier: &str, containing_file: &str) -> Result<String, DwarfError> {
        // Resolve relative to the containing file's directory
        let containing_path = Path::new(containing_file);
        let parent_dir = containing_path.parent().unwrap_or_else(|| Path::new("."));

        let resolved_path = parent_dir.join(specifier);

        // Normalize the path (resolve . and ..)
        let normalized = normalize_path(&resolved_path);

        // Read the file
        std::fs::read_to_string(&normalized).map_err(|e| {
            DwarfError::Io(format!(
                "Cannot resolve import '{}' from '{}': {}",
                specifier, containing_file, e
            ))
        })
    }
}

/// Normalize a path by resolving `.` and `..` components.
/// This is similar to `std::fs::canonicalize` but doesn't require the path to exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {
                // Skip
            }
            other => {
                components.push(other);
            }
        }
    }
    components.iter().collect::<PathBuf>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_resolver_returns_error() {
        let resolver = PureResolver::new();
        let result = resolver.resolve("./utils", "test.dwarf");
        assert!(result.is_err());
        match result.unwrap_err() {
            DwarfError::Config(msg) => {
                assert!(msg.contains("pure resolver"));
            }
            other => panic!("Expected DwarfError::Config, got {:?}", other),
        }
    }

    #[test]
    fn test_filesystem_resolver_resolves_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_import.dwarf");
        std::fs::write(&file_path, "fn helper() = 42").unwrap();

        let containing_file = dir.path().join("main.dwarf");
        // Create the containing file too
        std::fs::write(&containing_file, "import \"./test_import.dwarf\"").unwrap();

        let resolver = FilesystemResolver::new();
        let result = resolver.resolve("./test_import.dwarf", containing_file.to_str().unwrap());
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
        let content = result.unwrap();
        assert_eq!(content, "fn helper() = 42");
    }

    #[test]
    fn test_filesystem_resolver_missing_file() {
        let resolver = FilesystemResolver::new();
        let result = resolver.resolve("./nonexistent.dwarf", "/tmp/some_file.dwarf");
        assert!(result.is_err());
        match result.unwrap_err() {
            DwarfError::Io(msg) => {
                assert!(msg.contains("Cannot resolve import"));
            }
            other => panic!("Expected DwarfError::Io, got {:?}", other),
        }
    }

    #[test]
    fn test_filesystem_resolver_relative_resolution() {
        let dir = tempfile::tempdir().unwrap();

        // Create src/main.dwarf
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("main.dwarf"),
            "import \"./utils/helper.dwarf\"",
        )
        .unwrap();

        // Create src/utils/helper.dwarf
        let utils_dir = src_dir.join("utils");
        std::fs::create_dir_all(&utils_dir).unwrap();
        std::fs::write(utils_dir.join("helper.dwarf"), "fn helper() = 99").unwrap();

        let resolver = FilesystemResolver::new();
        let result = resolver.resolve(
            "./utils/helper.dwarf",
            src_dir.join("main.dwarf").to_str().unwrap(),
        );
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());
        let content = result.unwrap();
        assert_eq!(content, "fn helper() = 99");
    }
}
