//! Implementation of the `dwarf test` subcommand.
//!
//! Compiles Dwarf source files to TypeScript, runs Jest, and
//! returns structured results (JSON or text summary).

use std::path::PathBuf;
use std::process;

use dwarf_cli::runner::TsRunner;

/// Run the test subcommand.
pub fn run_test(files: Vec<PathBuf>, target: String, json: bool) {
    // Validate target
    if target != "ts" {
        eprintln!(
            "Error: Unsupported target '{}'. Supported targets: ts",
            target
        );
        process::exit(1);
    }

    let mut all_passed = true;
    let mut results = Vec::new();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();

        // Compile to TypeScript
        match TsRunner::transpile_to_ts(file_path) {
            Ok(ts_code) => {
                // Write to temp file for Jest
                let temp_dir = match tempfile::TempDir::new() {
                    Ok(d) => d,
                    Err(e) => {
                        results.push(TestResult {
                            file: path_str.clone(),
                            passed: false,
                            message: format!("Cannot create temp dir: {}", e),
                        });
                        all_passed = false;
                        continue;
                    }
                };

                let ts_path = temp_dir.path().join("output.test.ts");
                if let Err(e) = std::fs::write(&ts_path, &ts_code) {
                    results.push(TestResult {
                        file: path_str.clone(),
                        passed: false,
                        message: format!("Cannot write temp file: {}", e),
                    });
                    all_passed = false;
                    continue;
                }

                // Create a minimal package.json for Jest
                let pkg_path = temp_dir.path().join("package.json");
                let pkg = r#"{"scripts":{"test":"jest"},"jest":{"testMatch":["**/*.test.ts"]}}"#;
                let _ = std::fs::write(&pkg_path, pkg);

                // Run Jest
                let jest_output = std::process::Command::new("npx")
                    .arg("jest")
                    .arg("--json")
                    .arg(ts_path.to_str().unwrap())
                    .current_dir(temp_dir.path())
                    .output();

                match jest_output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);

                        // Try to parse Jest JSON output
                        if let Ok(jest_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                            let success = jest_json
                                .get("success")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            results.push(TestResult {
                                file: path_str.clone(),
                                passed: success,
                                message: if success {
                                    "All tests passed".into()
                                } else {
                                    "Some tests failed".into()
                                },
                            });
                            if !success {
                                all_passed = false;
                            }
                        } else {
                            // Fallback: no JSON output
                            let passed = output.status.success();
                            results.push(TestResult {
                                file: path_str,
                                passed,
                                message: if passed {
                                    "Tests passed".into()
                                } else {
                                    format!("Jest stderr: {}", stderr)
                                },
                            });
                            if !passed {
                                all_passed = false;
                            }
                        }
                    }
                    Err(e) => {
                        results.push(TestResult {
                            file: path_str,
                            passed: false,
                            message: format!("Failed to run Jest: {}", e),
                        });
                        all_passed = false;
                    }
                }
            }
            Err(e) => {
                results.push(TestResult {
                    file: path_str,
                    passed: false,
                    message: format!("Compilation failed: {}", e),
                });
                all_passed = false;
            }
        }
    }

    // Output results
    if json {
        let output = serde_json::json!({
            "ok": all_passed,
            "results": results.iter().map(|r| serde_json::json!({
                "file": r.file,
                "passed": r.passed,
                "message": r.message,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        for r in &results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            println!("{}: {} ({})", status, r.file, r.message);
        }
        let summary = if all_passed {
            "All tests passed"
        } else {
            "Some tests failed"
        };
        println!("\n{}", summary);
    }

    if !all_passed {
        process::exit(1);
    }
}

#[derive(Debug)]
struct TestResult {
    file: String,
    passed: bool,
    message: String,
}
