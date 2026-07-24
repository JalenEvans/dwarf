//! Implementation of the `dwarf test` subcommand.
//!
//! Compiles Dwarf source files to TypeScript, runs Jest, and
//! returns structured results (JSON or text summary).

use std::path::PathBuf;
use std::process;

use dwarf_cli::runner::{JavaRunner, PyRunner, TsRunner};

/// Run the test subcommand.
pub fn run_test(files: Vec<PathBuf>, target: String, json: bool) {
    let supported = ["ts", "py", "java"];
    if !supported.contains(&target.as_str()) {
        eprintln!(
            "Error: Unsupported target '{}'. Supported targets: {}",
            target,
            supported.join(", ")
        );
        process::exit(1);
    }

    let mut all_passed = true;
    let mut results = Vec::new();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();

        match target.as_str() {
            "ts" => run_test_ts(file_path, &mut results, &mut all_passed, &path_str),
            "py" => run_test_py(file_path, &mut results, &mut all_passed, &path_str),
            "java" => run_test_java(file_path, &mut results, &mut all_passed, &path_str),
            _ => unreachable!(),
        }
    }

    if json {
        let output = serde_json::json!({
            "ok": all_passed,
            "results": results.iter().map(|r| serde_json::json!({
                "file": r.file,
                "passed": r.passed,
                "message": r.message,
            })).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("JSON serialization should not fail")
        );
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

/// Run tests with TypeScript/Jest target.
fn run_test_ts(
    file_path: &std::path::Path,
    results: &mut Vec<TestResult>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match TsRunner::transpile_to_ts(file_path) {
        Ok(ts_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Cannot create temp dir: {}", e),
                    });
                    *all_passed = false;
                    return;
                }
            };

            let ts_path = temp_dir.path().join("output.test.ts");
            if let Err(e) = std::fs::write(&ts_path, &ts_code) {
                results.push(TestResult {
                    file: path_str.to_string(),
                    passed: false,
                    message: format!("Cannot write temp file: {}", e),
                });
                *all_passed = false;
                return;
            }

            let pkg_path = temp_dir.path().join("package.json");
            let pkg = r#"{"scripts":{"test":"jest"},"jest":{"testMatch":["**/*.test.ts"]}}"#;
            if let Err(e) = std::fs::write(&pkg_path, pkg) {
                results.push(TestResult {
                    file: path_str.to_string(),
                    passed: false,
                    message: format!("Cannot write package.json: {}", e),
                });
                *all_passed = false;
                return;
            }

            let jest_output = std::process::Command::new("npx")
                .arg("jest")
                .arg("--json")
                .arg(ts_path.to_str().expect("temp path is valid UTF-8"))
                .current_dir(temp_dir.path())
                .output();

            match jest_output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    if let Ok(jest_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let success = jest_json
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        results.push(TestResult {
                            file: path_str.to_string(),
                            passed: success,
                            message: if success {
                                "All tests passed".into()
                            } else {
                                "Some tests failed".into()
                            },
                        });
                        if !success {
                            *all_passed = false;
                        }
                    } else {
                        let passed = output.status.success();
                        results.push(TestResult {
                            file: path_str.to_string(),
                            passed,
                            message: if passed {
                                "Tests passed".into()
                            } else {
                                format!("Jest stderr: {}", stderr)
                            },
                        });
                        if !passed {
                            *all_passed = false;
                        }
                    }
                }
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run Jest: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResult {
                file: path_str.to_string(),
                passed: false,
                message: format!("Compilation failed: {}", e),
            });
            *all_passed = false;
        }
    }
}

/// Run tests with Python/pytest target.
fn run_test_py(
    file_path: &std::path::Path,
    results: &mut Vec<TestResult>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match PyRunner::transpile_to_py(file_path) {
        Ok(py_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Cannot create temp dir: {}", e),
                    });
                    *all_passed = false;
                    return;
                }
            };

            let py_path = temp_dir.path().join("test_output.py");
            if let Err(e) = std::fs::write(&py_path, &py_code) {
                results.push(TestResult {
                    file: path_str.to_string(),
                    passed: false,
                    message: format!("Cannot write temp file: {}", e),
                });
                *all_passed = false;
                return;
            }

            // Run pytest with JSON output
            let pytest_output = std::process::Command::new("python3")
                .arg("-m")
                .arg("pytest")
                .arg("--json")
                .arg(py_path.to_str().expect("temp path is valid UTF-8"))
                .current_dir(temp_dir.path())
                .output();

            match pytest_output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let passed = output.status.success();

                    if let Ok(pytest_json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                        let success = pytest_json
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(passed);
                        results.push(TestResult {
                            file: path_str.to_string(),
                            passed: success,
                            message: if success {
                                "All tests passed".into()
                            } else {
                                "Some tests failed".into()
                            },
                        });
                        if !success {
                            *all_passed = false;
                        }
                    } else {
                        results.push(TestResult {
                            file: path_str.to_string(),
                            passed,
                            message: if passed {
                                "Tests passed".into()
                            } else {
                                format!("pytest stderr: {}", stderr)
                            },
                        });
                        if !passed {
                            *all_passed = false;
                        }
                    }
                }
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run pytest: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResult {
                file: path_str.to_string(),
                passed: false,
                message: format!("Compilation failed: {}", e),
            });
            *all_passed = false;
        }
    }
}

/// Run tests with Java/JUnit target.
fn run_test_java(
    file_path: &std::path::Path,
    results: &mut Vec<TestResult>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match JavaRunner::transpile_to_java(file_path) {
        Ok(java_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Cannot create temp dir: {}", e),
                    });
                    *all_passed = false;
                    return;
                }
            };

            let java_path = temp_dir.path().join("DwarfTestGen.java");
            if let Err(e) = std::fs::write(&java_path, &java_code) {
                results.push(TestResult {
                    file: path_str.to_string(),
                    passed: false,
                    message: format!("Cannot write temp file: {}", e),
                });
                *all_passed = false;
                return;
            }

            // Step 1: Compile with javac
            let javac_output = std::process::Command::new("javac")
                .arg("--release")
                .arg("17")
                .arg(&java_path)
                .current_dir(temp_dir.path())
                .output();

            match javac_output {
                Ok(output) if output.status.success() => {
                    // Step 2: Run JUnit via ConsoleLauncher
                    let class_name = "DwarfTestGen";
                    let junit_output = std::process::Command::new("java")
                        .arg("org.junit.platform.console.ConsoleLauncher")
                        .arg("--select-class")
                        .arg(class_name)
                        .arg("--disable-banner")
                        .current_dir(temp_dir.path())
                        .output();

                    match junit_output {
                        Ok(output) => {
                            let _stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let passed = output.status.success();
                            results.push(TestResult {
                                file: path_str.to_string(),
                                passed,
                                message: if passed {
                                    "All tests passed".into()
                                } else {
                                    format!("JUnit failures:\n{}", stderr)
                                },
                            });
                            if !passed {
                                *all_passed = false;
                            }
                        }
                        Err(e) => {
                            results.push(TestResult {
                                file: path_str.to_string(),
                                passed: false,
                                message: format!("Failed to run JUnit: {}", e),
                            });
                            *all_passed = false;
                        }
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("javac compilation failed:\n{}", stderr),
                    });
                    *all_passed = false;
                }
                Err(e) => {
                    results.push(TestResult {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run javac: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResult {
                file: path_str.to_string(),
                passed: false,
                message: format!("Compilation failed: {}", e),
            });
            *all_passed = false;
        }
    }
}

#[derive(Debug)]
struct TestResult {
    file: String,
    passed: bool,
    message: String,
}
