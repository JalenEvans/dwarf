//! Implementation of the `dwarf init` subcommand.
//!
//! Scaffolds a new Dwarf project by creating a directory structure with a
//! `dwarf.conf.json` configuration file and a starter `src/main.kzd` source file.

use std::fs;
use std::path::PathBuf;

/// Initialize a new Dwarf project with the given name.
///
/// Creates the following structure:
/// ```text
/// <name>/
///   dwarf.conf.json   — project configuration
///   src/
///     main.kzd        — entry-point source file
/// ```
///
/// Returns an error if the target directory already exists or if any filesystem
/// operation fails.
pub fn run_init(name: &str) -> Result<(), String> {
    let project_dir = PathBuf::from(name);

    // Refuse to overwrite an existing directory.
    if project_dir.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    // Create project root.
    fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create directory '{}': {}", name, e))?;

    // Create src/ directory.
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create src/ directory: {}", e))?;

    // Write dwarf.conf.json.
    let config = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "targets": ["ts"],
  "out_dir": "dist",
  "pretty": true
}}
"#,
        name
    );
    let config_path = project_dir.join("dwarf.conf.json");
    fs::write(&config_path, config).map_err(|e| format!("Failed to write config: {}", e))?;

    // Write src/main.kzd — a minimal hello-world Dwarf program.
    let template = "fn main() {\n    println(\"Hello, Dwarf!\");\n}\n";
    let main_path = src_dir.join("main.kzd");
    fs::write(&main_path, template).map_err(|e| format!("Failed to write main.kzd: {}", e))?;

    // Print a helpful success message.
    println!("Created Dwarf project '{}'", name);
    println!("  {} (project config)", config_path.display());
    println!("  {} (entry point)", main_path.display());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  dwarf build src/main.kzd");

    Ok(())
}
