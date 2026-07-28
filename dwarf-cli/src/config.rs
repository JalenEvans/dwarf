//! Configuration file discovery for the CLI.
//!
//! Looks for `dwarf.conf.json` in the current directory and parent directories.

use dwarf_lib::{CompilerConfig, DwarfError};
use std::path::Path;

/// Try to find and load a `dwarf.conf.json` configuration file.
///
/// Searches the current directory and parent directories up to the filesystem root.
/// Returns `Ok(None)` if no config file is found (not an error).
pub fn find_and_load_config() -> Result<Option<CompilerConfig>, DwarfError> {
    let cwd = std::env::current_dir()
        .map_err(|e| DwarfError::Io(format!("Cannot get current directory: {}", e)))?;

    find_config_in(&cwd)
}

/// Search for dwarf.conf.json starting from `dir` and moving up.
fn find_config_in(dir: &Path) -> Result<Option<CompilerConfig>, DwarfError> {
    let mut current = Some(dir.to_path_buf());

    while let Some(ref path) = current {
        let config_path = path.join("dwarf.conf.json");
        if config_path.exists() {
            return CompilerConfig::from_file(config_path.to_str().unwrap()).map(Some);
        }
        current = path.parent().map(|p| p.to_path_buf());
    }

    Ok(None)
}

/// Merge CLI options with a discovered config file.
///
/// If a config file is found, CLI options override config values.
/// If no config file, use CLI options directly.
pub fn merge_config_with_cli(cli_options: dwarf_lib::CompileOptions) -> dwarf_lib::CompileOptions {
    match find_and_load_config() {
        Ok(Some(config)) => config.merge_with_cli(&cli_options),
        _ => cli_options, // No config found or error — use CLI as-is
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_config_in_current_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("dwarf.conf.json");
        std::fs::write(
            &config_path,
            r#"{"targets": ["debug"], "out_dir": "build"}"#,
        )
        .unwrap();

        let result = find_config_in(dir.path()).unwrap();
        assert!(result.is_some());
        let config = result.unwrap();
        assert_eq!(config.targets, vec!["debug"]);
        assert_eq!(config.out_dir, "build");
    }

    #[test]
    fn test_find_config_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_config_in(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_config_in_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let child_dir = dir.path().join("src").join("components");
        std::fs::create_dir_all(&child_dir).unwrap();

        // Put config in parent dir
        std::fs::write(dir.path().join("dwarf.conf.json"), r#"{"targets": ["ts"]}"#).unwrap();

        let result = find_config_in(&child_dir).unwrap();
        assert!(result.is_some(), "Should find config in parent");
    }

    // WILL FAIL — RED PHASE: CompilerConfig does not yet have a stdlib_path field
    #[test]
    fn test_config_with_stdlib_path() {
        let json = r#"{"targets": ["ts"], "stdlib_path": "/project/stdlib"}"#;
        let config = CompilerConfig::from_json(json).unwrap();
        assert_eq!(
            config.stdlib_path,
            Some("/project/stdlib".to_string())
        );
    }
}
