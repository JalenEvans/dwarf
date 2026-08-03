//! RED-phase tests for the `dwarf init` subcommand (DWARF-60-T3 Chunk A).
//!
//! These tests MUST FAIL because the `init` subcommand does not exist yet.
//! They define the expected behavior for the implementation phase:
//!
//!   - CLI parsing: `init` appears in help, takes a required project name
//!   - Filesystem: creates project directory, `dwarf.conf.json`, `src/main.kzd`
//!   - Config: generated config is valid JSON with correct structure
//!   - Safety: refuses to overwrite an existing directory
//!
//! Do NOT implement anything — these tests are the specification.

use std::path::PathBuf;
use std::process::Command;

/// Helper to run the dwarf binary with given args.
fn dwarf(args: &[&str]) -> std::process::Output {
    let bin_path = get_binary_path();
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to run dwarf-cli binary")
}

/// Helper to run the dwarf binary in a specific working directory.
fn dwarf_in_dir(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let bin_path = get_binary_path();
    Command::new(bin_path)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("Failed to run dwarf-cli binary")
}

/// Find the dwarf binary in cargo's build output.
fn get_binary_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Workspace root is parent of crate directory
    manifest.parent().unwrap().join("target/debug/dwarf-cli")
}

// =========================================================================
// 1. CLI Parsing Tests
// =========================================================================

#[test]
fn test_init_help_shows_subcommand() {
    // `dwarf --help` should list `init` as an available subcommand.
    let output = dwarf(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "dwarf --help should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        stdout.contains("init"),
        "Top-level help should mention the 'init' subcommand.\nGot:\n{}",
        stdout,
    );
}

#[test]
fn test_init_help_shows_args() {
    // `dwarf init --help` should succeed and show the required project name arg.
    let output = dwarf(&["init", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dwarf init --help should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );
    assert!(
        stdout.contains("init"),
        "init --help should mention 'init'.\nGot:\n{}",
        stdout,
    );
}

#[test]
fn test_init_parses_with_project_name() {
    // `dwarf init my-project` should parse successfully (exit 0 or at least
    // not fail with a clap "unrecognized subcommand" error).
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "'init' should be a recognized subcommand, got stderr: {}",
        stderr,
    );
}

#[test]
fn test_init_without_args_shows_error() {
    // `dwarf init` (no project name) should fail with a useful error about
    // the missing required argument — NOT with "unrecognized subcommand".
    let output = dwarf(&["init"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "dwarf init without a project name should exit non-zero",
    );
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "'init' should be recognized; the error should be about the missing argument, got: {}",
        stderr,
    );
}

// =========================================================================
// 2. Behavior Tests (integration)
// =========================================================================

#[test]
fn test_init_creates_project_directory() {
    // `dwarf init my-project` should create a directory named `my-project`.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dwarf init my-project should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );
    let project_dir = dir.path().join("my-project");
    assert!(
        project_dir.is_dir(),
        "Expected directory 'my-project' to be created at {:?}",
        project_dir,
    );
}

#[test]
fn test_init_creates_config_file() {
    // `dwarf init my-project` should create `my-project/dwarf.conf.json`.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    assert!(output.status.success(), "dwarf init should succeed",);
    let config_path = dir.path().join("my-project").join("dwarf.conf.json");
    assert!(
        config_path.is_file(),
        "Expected dwarf.conf.json to be created at {:?}",
        config_path,
    );
}

#[test]
fn test_init_creates_src_directory() {
    // `dwarf init my-project` should create `my-project/src/`.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    assert!(output.status.success(), "dwarf init should succeed");
    let src_dir = dir.path().join("my-project").join("src");
    assert!(
        src_dir.is_dir(),
        "Expected src/ directory to be created at {:?}",
        src_dir,
    );
}

#[test]
fn test_init_creates_main_kzd_template() {
    // `dwarf init my-project` should create `my-project/src/main.kzd`
    // with a basic hello-world Dwarf source file.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    assert!(output.status.success(), "dwarf init should succeed");
    let main_kzd = dir.path().join("my-project").join("src").join("main.kzd");
    assert!(
        main_kzd.is_file(),
        "Expected src/main.kzd to be created at {:?}",
        main_kzd,
    );
    let contents = std::fs::read_to_string(&main_kzd).expect("Should be able to read main.kzd");
    assert!(!contents.is_empty(), "src/main.kzd should not be empty",);
}

#[test]
fn test_init_refuses_to_overwrite_existing_directory() {
    // If the target directory already exists, `dwarf init` should error
    // gracefully instead of overwriting its contents.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = dir.path().join("existing-project");
    std::fs::create_dir(&project_dir).expect("Should create dir");
    // Put a sentinel file inside to detect overwrite
    std::fs::write(project_dir.join("sentinel.txt"), "do not delete")
        .expect("Should write sentinel");

    let output = dwarf_in_dir(dir.path(), &["init", "existing-project"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // First: init must be a recognized subcommand (otherwise this test is
    // passing for the wrong reason).
    assert!(
        !stderr.contains("unrecognized subcommand"),
        "'init' must be a recognized subcommand for this test to be meaningful.\n\
         Got stderr: {}",
        stderr,
    );

    // The command should fail because the directory already exists.
    assert!(
        !output.status.success(),
        "dwarf init on an existing directory should exit non-zero.\nstdout: {}\nstderr: {}",
        stdout,
        stderr,
    );

    // The sentinel file should still exist — nothing was overwritten.
    let sentinel = project_dir.join("sentinel.txt");
    assert!(
        sentinel.is_file(),
        "Sentinel file should still exist — init must not overwrite",
    );
    assert_eq!(
        std::fs::read_to_string(&sentinel).unwrap(),
        "do not delete",
        "Sentinel file contents should be unchanged",
    );
}

#[test]
fn test_init_outputs_success_message() {
    // `dwarf init my-project` should print a helpful message telling the
    // user what was created.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "dwarf init should succeed",);
    assert!(
        !stdout.is_empty(),
        "dwarf init should print a success message to stdout",
    );
    // The message should mention the project name so the user knows what happened.
    assert!(
        stdout.contains("my-project"),
        "Success message should mention the project name.\nGot: {}",
        stdout,
    );
}

// =========================================================================
// 3. Config File Content Tests (integration)
// =========================================================================

#[test]
fn test_init_config_has_correct_structure() {
    // The generated dwarf.conf.json should contain the expected keys:
    // name, version, targets, out_dir, pretty.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    assert!(output.status.success(), "dwarf init should succeed");

    let config_path = dir.path().join("my-project").join("dwarf.conf.json");
    let contents =
        std::fs::read_to_string(&config_path).expect("Should be able to read dwarf.conf.json");
    let json: serde_json::Value =
        serde_json::from_str(&contents).expect("dwarf.conf.json should be valid JSON");

    assert!(
        json.get("name").is_some(),
        "Config should have a 'name' field.\nGot: {}",
        contents,
    );
    assert!(
        json.get("version").is_some(),
        "Config should have a 'version' field.\nGot: {}",
        contents,
    );
    assert!(
        json.get("targets").is_some(),
        "Config should have a 'targets' field.\nGot: {}",
        contents,
    );
    assert!(
        json.get("out_dir").is_some(),
        "Config should have an 'out_dir' field.\nGot: {}",
        contents,
    );
    assert!(
        json.get("pretty").is_some(),
        "Config should have a 'pretty' field.\nGot: {}",
        contents,
    );
}

#[test]
fn test_init_config_name_matches_argument() {
    // `dwarf init my-project` should produce a config with `"name": "my-project"`.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "my-project"]);
    assert!(output.status.success(), "dwarf init should succeed");

    let config_path = dir.path().join("my-project").join("dwarf.conf.json");
    let contents =
        std::fs::read_to_string(&config_path).expect("Should be able to read dwarf.conf.json");
    let json: serde_json::Value =
        serde_json::from_str(&contents).expect("dwarf.conf.json should be valid JSON");

    assert_eq!(
        json["name"].as_str().expect("'name' should be a string"),
        "my-project",
        "Config 'name' should match the project name argument",
    );
}

#[test]
fn test_init_config_is_valid_json() {
    // The generated config file must be parseable by serde_json.
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output = dwarf_in_dir(dir.path(), &["init", "json-test-project"]);
    assert!(output.status.success(), "dwarf init should succeed");

    let config_path = dir.path().join("json-test-project").join("dwarf.conf.json");
    let contents =
        std::fs::read_to_string(&config_path).expect("Should be able to read dwarf.conf.json");

    // This will panic with a descriptive error if the JSON is malformed.
    let _json: serde_json::Value = serde_json::from_str(&contents).unwrap_or_else(|e| {
        panic!(
            "dwarf.conf.json should be valid JSON.\nContents:\n{}\nError: {}",
            contents, e
        )
    });
}
