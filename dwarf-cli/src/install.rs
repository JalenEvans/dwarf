//! Implementation of the `dwarf install` subcommand.
//!
//! Generates extern declaration stubs for supported package sources.
//! Supported prefixes: npm, py, java.
//!
//! For npm/py: the full `prefix:name` becomes the source string, and `name`
//! becomes the function name.
//!
//! For java: the package path (everything before the last dot segment) becomes
//! the source string, and the class name (last segment) becomes the function
//! name. E.g. `java:java.util.ArrayList` → `extern "java:java.util" fn ArrayList()`.

use std::process;

pub fn run_install(package: &str) {
    // Split into prefix and name on the first ':'
    let (prefix, name) = match package.split_once(':') {
        Some((p, n)) if !p.is_empty() && !n.is_empty() => (p, n),
        _ => {
            eprintln!(
                "Error: invalid package format '{}'. Expected '<prefix>:<name>' (e.g. 'npm:express')",
                package
            );
            process::exit(1);
        }
    };

    // Validate the source prefix
    match prefix {
        "npm" | "py" => {
            // For npm/py, the full prefix:name is the source, name is the fn
            let source = package;
            let fn_name = name;
            println!(r#"extern "{source}" fn {fn_name}() -> ()"#);

            // Run the package manager (npm or pip) to install the package.
            // This is best-effort: the extern stub is already printed above,
            // so package manager failures only produce warnings.
            let pm = if prefix == "npm" { "npm" } else { "pip" };
            let result = process::Command::new(pm).arg("install").arg(name).status();

            match result {
                Ok(status) if status.success() => {
                    eprintln!("Installed {name}");
                }
                Ok(status) => {
                    eprintln!(
                        "Warning: {pm} install {name} failed with exit code {:?}",
                        status.code()
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("Warning: {pm} not found. Please install the package manually.");
                }
                Err(e) => {
                    eprintln!("Warning: failed to run {pm}: {e}");
                }
            }
        }
        "java" => {
            // For java, split name by dots: package path → source, last segment → fn name
            let parts: Vec<&str> = name.rsplitn(2, '.').collect();
            if parts.len() < 2 {
                eprintln!(
                    "Error: invalid java package '{}'. Expected dotted path like 'java.util.ArrayList'",
                    name
                );
                process::exit(1);
            }
            let class_name = parts[0];
            let package_path = parts[1];
            let source = format!("java:{package_path}");
            println!(r#"extern "{source}" fn {class_name}() -> ()"#);
        }
        _ => {
            eprintln!("Error: unknown source prefix '{prefix}'. Supported prefixes: npm, py, java");
            process::exit(1);
        }
    }
}
