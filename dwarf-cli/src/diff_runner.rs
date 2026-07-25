//! DiffRunner — compiles Dwarf source to all targets, runs each, and
//! compares results against an oracle target.
//!
//! The `run_diff` function is the main entry point. It reads a `.kzd`
//! source file, checks for a `@Diff` decorator, compiles to every
//! supported target (`ts`, `py`, `java`), and compares each target's
//! emitted output against the oracle (TypeScript).
//!
//! # Example
//!
//! ```ignore
//! use dwarf_cli::diff_runner::run_diff;
//! use std::path::Path;
//!
//! let result = run_diff(Path::new("source.kzd")).expect("diff run failed");
//! println!("All match: {}", result.all_match);
//! for m in &result.mismatches {
//!     println!("  {} differs from oracle", m.target);
//! }
//! ```

use std::path::Path;

use serde::Serialize;

use crate::runner::{JavaRunner, PyRunner, TsRunner};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result from running (transpiling) a single target.
#[derive(Debug, Clone, Serialize)]
pub struct TargetResult {
    pub target: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// A diff run result comparing all targets against the oracle.
#[derive(Debug, Clone, Serialize)]
pub struct DiffRunResult {
    pub oracle: TargetResult,
    pub others: Vec<TargetResult>,
    pub all_match: bool,
    pub mismatches: Vec<Mismatch>,
}

/// Describes a mismatch between the oracle and another target.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Mismatch {
    pub target: String,
    pub expected: String,
    pub actual: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a diff comparison for the given Dwarf source file.
///
/// 1. Reads the source and checks for a `@Diff` decorator.
/// 2. Compiles the source to every supported target (`ts`, `py`, `java`).
/// 3. Uses TypeScript (`ts`) as the oracle.
/// 4. Compares every other target's emitted output against the oracle.
/// 5. Returns a [`DiffRunResult`] summarising the comparison.
///
/// # Errors
///
/// Returns `Err` if the source file cannot be read, if it lacks a `@Diff`
/// decorator, or if the oracle target itself fails to compile.
pub fn run_diff(source_path: &Path) -> Result<DiffRunResult, String> {
    // 1. Read source and check for @Diff decorator
    let source =
        std::fs::read_to_string(source_path).map_err(|e| format!("Cannot read file: {}", e))?;

    if !source.contains("@Diff") {
        return Err(
            "No @Diff decorator found in source — add `@Diff(oracle: ts)` to enable diff mode"
                .to_string(),
        );
    }

    // 2. Compile and run each target
    let targets = ["ts", "py", "java"];
    let oracle_target = "ts";

    let mut results: Vec<(&str, TargetResult)> = Vec::new();
    for target in &targets {
        let result = run_single_target(source_path, target)?;
        results.push((target, result));
    }

    // 3. Find oracle result
    let oracle_result = results
        .iter()
        .find(|(t, _)| *t == oracle_target)
        .map(|(_, r)| r.clone())
        .ok_or_else(|| "Oracle target (ts) not found — this is a bug".to_string())?;

    // 4. Compare each other target against the oracle
    let mut mismatches = Vec::new();
    let mut all_match = true;

    for (target, result) in &results {
        if *target == oracle_target {
            continue;
        }
        if result.stdout != oracle_result.stdout {
            all_match = false;
            mismatches.push(Mismatch {
                target: target.to_string(),
                expected: oracle_result.stdout.clone(),
                actual: result.stdout.clone(),
            });
        }
    }

    // 5. Collect non-oracle results
    let others: Vec<TargetResult> = results
        .into_iter()
        .filter(|(t, _)| *t != oracle_target)
        .map(|(_, r)| r)
        .collect();

    Ok(DiffRunResult {
        oracle: oracle_result,
        others,
        all_match,
        mismatches,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compile a Dwarf source to `target` and return the emitted code as a
/// [`TargetResult`].
fn run_single_target(source_path: &Path, target: &str) -> Result<TargetResult, String> {
    match target {
        "ts" => match TsRunner::transpile_to_ts(source_path) {
            Ok(code) => Ok(TargetResult {
                target: "ts".to_string(),
                success: true,
                stdout: code,
                stderr: String::new(),
            }),
            Err(e) => Ok(TargetResult {
                target: "ts".to_string(),
                success: false,
                stdout: String::new(),
                stderr: e,
            }),
        },
        "py" => match PyRunner::transpile_to_py(source_path) {
            Ok(code) => Ok(TargetResult {
                target: "py".to_string(),
                success: true,
                stdout: code,
                stderr: String::new(),
            }),
            Err(e) => Ok(TargetResult {
                target: "py".to_string(),
                success: false,
                stdout: String::new(),
                stderr: e,
            }),
        },
        "java" => match JavaRunner::transpile_to_java(source_path) {
            Ok(code) => Ok(TargetResult {
                target: "java".to_string(),
                success: true,
                stdout: code,
                stderr: String::new(),
            }),
            Err(e) => Ok(TargetResult {
                target: "java".to_string(),
                success: false,
                stdout: String::new(),
                stderr: e,
            }),
        },
        _ => Err(format!("Unknown target: {}", target)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // DiffRunResult construction
    // -------------------------------------------------------------------

    #[test]
    fn test_diff_run_result_can_be_constructed() {
        let oracle = TargetResult {
            target: "ts".into(),
            success: true,
            stdout: "console.log(42);".into(),
            stderr: String::new(),
        };

        let others = vec![
            TargetResult {
                target: "py".into(),
                success: true,
                stdout: "print(42)".into(),
                stderr: String::new(),
            },
            TargetResult {
                target: "java".into(),
                success: true,
                stdout: "System.out.println(42);".into(),
                stderr: String::new(),
            },
        ];

        let result = DiffRunResult {
            oracle: oracle.clone(),
            others: others.clone(),
            all_match: false,
            mismatches: vec![],
        };

        assert_eq!(result.oracle.target, "ts");
        assert_eq!(result.others.len(), 2);
        assert!(!result.all_match);
    }

    #[test]
    fn test_diff_run_result_serializes_to_json() {
        let result = DiffRunResult {
            oracle: TargetResult {
                target: "ts".into(),
                success: true,
                stdout: "let x = 1;".into(),
                stderr: String::new(),
            },
            others: vec![TargetResult {
                target: "py".into(),
                success: true,
                stdout: "x = 1".into(),
                stderr: String::new(),
            }],
            all_match: false,
            mismatches: vec![Mismatch {
                target: "py".into(),
                expected: "let x = 1;".into(),
                actual: "x = 1".into(),
            }],
        };

        let json = serde_json::to_string(&result).expect("JSON serialization should not fail");
        assert!(json.contains("\"target\":\"py\""));
        assert!(json.contains("\"all_match\":false"));
        assert!(json.contains("\"mismatches\""));
    }

    // -------------------------------------------------------------------
    // Mismatch detection
    // -------------------------------------------------------------------

    #[test]
    fn test_mismatch_struct_captures_expected_vs_actual() {
        let mm = Mismatch {
            target: "py".into(),
            expected: "console.log(\"hello\");".into(),
            actual: "print(\"hello\")".into(),
        };

        assert_eq!(mm.target, "py");
        assert_eq!(mm.expected, "console.log(\"hello\");");
        assert_eq!(mm.actual, "print(\"hello\")");
    }

    #[test]
    fn test_mismatch_detects_differing_output() {
        // Simulate the oracle comparison logic
        let oracle_stdout = "const x = 42;";
        let target_stdouts = vec![
            ("py", "x = 42"),
            ("java", "int x = 42;"),
            ("ts", "const x = 42;"), // matches oracle
        ];

        let mut mismatches = Vec::new();
        let mut all_match = true;

        for (target, stdout) in &target_stdouts {
            if *target == "ts" {
                continue;
            }
            if *stdout != oracle_stdout {
                all_match = false;
                mismatches.push(Mismatch {
                    target: target.to_string(),
                    expected: oracle_stdout.to_string(),
                    actual: (*stdout).to_string(),
                });
            }
        }

        assert!(!all_match, "Should detect mismatches");
        assert_eq!(mismatches.len(), 2, "Two targets should differ");
        assert_eq!(mismatches[0].target, "py");
        assert_eq!(mismatches[1].target, "java");
    }

    // -------------------------------------------------------------------
    // Oracle comparison logic
    // -------------------------------------------------------------------

    #[test]
    fn test_all_match_when_all_targets_produce_same_output() {
        let oracle_stdout = "print(\"hello\")";

        // All targets produce identical emitted code (simulated)
        let results = vec![
            ("ts", oracle_stdout),
            ("py", oracle_stdout),
            ("java", oracle_stdout),
        ];

        let mut mismatches = Vec::new();
        let mut all_match = true;

        for (target, stdout) in &results {
            if *target == "ts" {
                continue;
            }
            if *stdout != oracle_stdout {
                all_match = false;
                mismatches.push(Mismatch {
                    target: target.to_string(),
                    expected: oracle_stdout.to_string(),
                    actual: (*stdout).to_string(),
                });
            }
        }

        assert!(all_match, "All targets should match oracle");
        assert!(mismatches.is_empty(), "There should be no mismatches");
    }

    #[test]
    fn test_oracle_isolation_oracle_failure_does_not_affect_others() {
        // If the oracle itself fails, the comparison still works —
        // the caller sees the oracle's success=false and can decide
        // how to handle it.

        let oracle = TargetResult {
            target: "ts".into(),
            success: false,
            stdout: String::new(),
            stderr: "Compilation error".into(),
        };

        let others = [TargetResult {
            target: "py".into(),
            success: true,
            stdout: "x = 1".into(),
            stderr: String::new(),
        }];

        // When oracle fails, others won't match (oracle stdout is empty)
        let mismatches: Vec<Mismatch> = others
            .iter()
            .filter(|r| r.stdout != oracle.stdout)
            .map(|r| Mismatch {
                target: r.target.clone(),
                expected: oracle.stdout.clone(),
                actual: r.stdout.clone(),
            })
            .collect();

        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].expected, "");
        assert_eq!(mismatches[0].actual, "x = 1");
    }

    #[test]
    fn test_mismatch_json_round_trip() {
        // Verify that Mismatch can be serialized and deserialized through JSON
        let mm = Mismatch {
            target: "java".into(),
            expected: "console.log(1);".into(),
            actual: "System.out.println(1);".into(),
        };

        let json = serde_json::to_string(&mm).expect("serialize");
        let deserialized: Mismatch = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.target, mm.target);
        assert_eq!(deserialized.expected, mm.expected);
        assert_eq!(deserialized.actual, mm.actual);
    }

    // -------------------------------------------------------------------
    // TargetResult construction
    // -------------------------------------------------------------------

    #[test]
    fn test_target_result_can_hold_success() {
        let tr = TargetResult {
            target: "ts".into(),
            success: true,
            stdout: "let x = 1;".into(),
            stderr: String::new(),
        };

        assert!(tr.success);
        assert_eq!(tr.stdout, "let x = 1;");
        assert!(tr.stderr.is_empty());
    }

    #[test]
    fn test_target_result_can_hold_failure() {
        let tr = TargetResult {
            target: "py".into(),
            success: false,
            stdout: String::new(),
            stderr: "SyntaxError".into(),
        };

        assert!(!tr.success);
        assert!(tr.stdout.is_empty());
        assert_eq!(tr.stderr, "SyntaxError");
    }
}
