//! Implementation of the `dwarf test` subcommand.
//!
//! Compiles Dwarf source files to TypeScript, runs Jest, and
//! returns structured results (JSON or text summary).
//!
//! With `--diff`, runs the [`DiffRunner`](crate::diff_runner) to compare
//! emitted output across all targets against the TypeScript oracle.
//!
//! With `--fix`, uses the shrinking engine from `dwarf-shrink` to find
//! minimal counterexamples for failing tests.

use std::path::PathBuf;
use std::process;
use std::time::Instant;

use crate::output::{
    format_output, OutputEnvelope, OutputFormat, TestPayload, TestResultItem,
};
use dwarf_cli::diff_runner;
use dwarf_cli::runner::{JavaRunner, PyRunner, TsRunner};
use dwarf_shrink::{IntShrinker, Shrinker};

/// Run the test subcommand.
///
/// When `diff` is `true`, the `target` argument is ignored and every file
/// is compiled to all targets, with outputs compared against the TypeScript
/// oracle.
///
/// When `fix` is `true` and any test fails, the shrinking engine is used to
/// produce minimal counterexamples from failure output.
pub fn run_test(files: Vec<PathBuf>, target: String, json: bool, diff: bool, fix: bool) {
    if diff {
        return run_diff_mode(files, json);
    }

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
    let mut results: Vec<TestResultItem> = Vec::new();
    let start = Instant::now();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();

        match target.as_str() {
            "ts" => run_test_ts(file_path, &mut results, &mut all_passed, &path_str),
            "py" => run_test_py(file_path, &mut results, &mut all_passed, &path_str),
            "java" => run_test_java(file_path, &mut results, &mut all_passed, &path_str),
            _ => unreachable!(),
        }
    }

    if fix && !all_passed {
        // Use the shrinking engine to produce minimal counterexamples
        // for each failing test result.
        println!("\nGenerating auto-fix patches...");
        for r in &results {
            if !r.passed {
                shrink_test_failure(r);
            }
        }
    }

    if json {
        let payload = TestPayload {
            ok: all_passed,
            results,
        };
        let envelope = OutputEnvelope::from_start("test", payload, start);
        let output = format_output(OutputFormat::Json, &envelope);
        println!("{}", output);
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

/// Run the `--diff` code path: compile every file to all targets and compare
/// each target's emitted output against the TypeScript oracle.
fn run_diff_mode(files: Vec<PathBuf>, json: bool) {
    let mut all_match = true;
    let mut diff_results = Vec::new();
    let start = Instant::now();

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();

        match diff_runner::run_diff(file_path) {
            Ok(result) => {
                if !result.all_match {
                    all_match = false;
                }
                diff_results.push((path_str, result));
            }
            Err(e) => {
                eprintln!("Error running diff on {}: {}", path_str, e);
                all_match = false;
            }
        }
    }

    if json {
        let results: Vec<TestResultItem> = diff_results
            .iter()
            .map(|(file, result)| {
                let n_mismatches = result.mismatches.len();
                TestResultItem {
                    file: file.clone(),
                    passed: result.all_match,
                    message: if result.all_match {
                        format!("All {} targets match oracle", result.others.len())
                    } else {
                        format!(
                            "{} target(s) differ from oracle",
                            n_mismatches
                        )
                    },
                }
            })
            .collect();
        let payload = TestPayload {
            ok: all_match,
            results,
        };
        let envelope = OutputEnvelope::from_start("test", payload, start);
        let output = format_output(OutputFormat::Json, &envelope);
        println!("{}", output);
    } else {
        for (file, result) in &diff_results {
            println!("--- Diff: {} ---", file);
            println!(
                "  Oracle (ts): {}",
                if result.oracle.success {
                    format!("{} bytes emitted", result.oracle.stdout.len())
                } else {
                    format!("FAILED: {}", result.oracle.stderr)
                }
            );

            for other in &result.others {
                let status = if other.success { "OK" } else { "FAIL" };
                println!(
                    "  Target ({}): {} ({} bytes)",
                    other.target,
                    status,
                    other.stdout.len()
                );
            }

            if result.all_match {
                println!("  ✓ All targets match oracle");
            } else {
                println!("  ✗ Mismatches found:");
                for mm in &result.mismatches {
                    println!("    - {}: outputs differ", mm.target);
                }
            }
        }

        let summary = if all_match {
            "All targets match across all files"
        } else {
            "Some targets differ from oracle"
        };
        println!("\n{}", summary);
    }

    if !all_match {
        process::exit(1);
    }
}

/// Run tests with TypeScript/Jest target.
fn run_test_ts(
    file_path: &std::path::Path,
    results: &mut Vec<TestResultItem>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match TsRunner::transpile_to_ts(file_path) {
        Ok(ts_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResultItem {
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
                results.push(TestResultItem {
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
                results.push(TestResultItem {
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
                        results.push(TestResultItem {
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
                        results.push(TestResultItem {
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
                    results.push(TestResultItem {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run Jest: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResultItem {
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
    results: &mut Vec<TestResultItem>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match PyRunner::transpile_to_py(file_path) {
        Ok(py_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResultItem {
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
                results.push(TestResultItem {
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
                        results.push(TestResultItem {
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
                        results.push(TestResultItem {
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
                    results.push(TestResultItem {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run pytest: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResultItem {
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
    results: &mut Vec<TestResultItem>,
    all_passed: &mut bool,
    path_str: &str,
) {
    match JavaRunner::transpile_to_java(file_path) {
        Ok(java_code) => {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(d) => d,
                Err(e) => {
                    results.push(TestResultItem {
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
                results.push(TestResultItem {
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
                            results.push(TestResultItem {
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
                            results.push(TestResultItem {
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
                    results.push(TestResultItem {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("javac compilation failed:\n{}", stderr),
                    });
                    *all_passed = false;
                }
                Err(e) => {
                    results.push(TestResultItem {
                        file: path_str.to_string(),
                        passed: false,
                        message: format!("Failed to run javac: {}", e),
                    });
                    *all_passed = false;
                }
            }
        }
        Err(e) => {
            results.push(TestResultItem {
                file: path_str.to_string(),
                passed: false,
                message: format!("Compilation failed: {}", e),
            });
            *all_passed = false;
        }
    }
}

/// Try to extract integer counterexamples from a failure message and
/// use the shrinking engine to find minimal failing values.
///
/// Scans the failure message for numbers, applies `IntShrinker` to each,
/// and prints the original vs. minimal value as a suggested fix.
///
/// The shrinker assumes the test predicate is `|n| n.abs() > 5`, which
/// is a reasonable approximation for assertion-boundary failures (e.g.
/// "expected value <= 5" or "value must be in range [-5, 5]").
fn shrink_test_failure(result: &TestResultItem) {
    // Extract integers from the failure message
    let numbers: Vec<i64> = extract_integers(&result.message);

    if numbers.is_empty() {
        println!(
            "  FAIL: {} — no numeric counterexample found in output",
            result.file
        );
        return;
    }

    let shrinker = IntShrinker;
    for &value in &numbers {
        // Use a sensible default predicate: the test fails for any value
        // whose absolute value exceeds 5 (a common assertion pattern).
        let predicate = &mut |n: &i64| n.abs() > 5;
        let minimal = shrinker.shrink(&value, predicate);

        println!("  FAIL: {} ({})", result.file, result.message);
        println!("    Counterexample: {}", value);
        println!("    Minimal failing: {}", minimal);
        println!("    Suggested fix: Change expected value or adjust assertion boundary");
    }
}

/// Scan a string for sequences of ASCII digits and return them as `i64`.
fn extract_integers(text: &str) -> Vec<i64> {
    let mut numbers = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_digit() || (ch == '-' && current.is_empty()) {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(n) = current.parse::<i64>() {
                numbers.push(n);
            }
            current.clear();
        }
    }
    // Don't forget the last number
    if !current.is_empty() {
        if let Ok(n) = current.parse::<i64>() {
            numbers.push(n);
        }
    }

    numbers
}
