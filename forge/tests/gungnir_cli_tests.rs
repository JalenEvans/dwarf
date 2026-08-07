//! End-to-end integration tests for the `forge gungnir` subcommand (DWARF-120).
//!
//! `forge gungnir` discovers `@gungnir`-annotated functions, verifies their
//! contracts with the Z3 SMT solver, and reports a per-function status of
//! `proved` / `counterexample` / `unproven` / `error`.
//!
//! RED PHASE (DWARF-120): the `Commands::Gungnir` variant does not exist yet,
//! so every invocation fails with clap's `error: unrecognized subcommand
//! 'gungnir'` (exit code 2). That failure IS the Red: the `gungnir` subcommand
//! and the forge Z3 subprocess bridge (the "Z3 subprocess bridge in forge"
//! requirement) must be implemented for these tests to pass.
//!
//! Now GREEN (DWARF-120): `Commands::Gungnir` exists and the forge Z3 bridge is
//! implemented. These tests now pin the CLI contract: discovery, per-function
//! status reporting, the missing-z3 path, and `--timeout-ms` handling.
//!
//! These tests invoke the compiled forge binary. They require `z3` only when a
//! test actually exercises the solver; the missing-z3 test overrides the
//! solver path (via the `DWARF_Z3` env hook) so the install-instructions path
//! is checked WITHOUT depending on the developer's PATH.
//!
//! # CLI contract pinned here
//!
//! ```text
//! forge gungnir <files...> [--json] [--timeout-ms <ms>]
//! ```
//!
//! - Discovery: only functions carrying the `@gungnir` decorator are verified;
//!   plain functions are never reported.
//! - Reporting (human mode): one line per function, e.g.
//!   `gungnir: abs — proved` and `gungnir: identity — counterexample (a = 5)`.
//! - Counterexamples carry concrete values from the z3 model.
//! - Z3 availability: the solver binary is resolved from the `DWARF_Z3` env var
//!   when set — it is AUTHORITATIVE: a set-but-nonexistent `DWARF_Z3` counts as
//!   "solver not found" (no `$PATH` fallback). When unset, `z3` is looked up on
//!   `$PATH`. When unavailable, install instructions are printed and forge
//!   exits non-zero. This makes the missing-z3 test deterministic whether or
//!   not z3 is installed on the machine.
//! - `--timeout-ms <n>` is accepted; a solver that returns `unknown` or exceeds
//!   the budget is reported as `unproven`.

use std::fs;
use std::process::Command;

/// Helper: run the forge binary with the given args + optional env, returning Output.
fn forge_with_env(args: &[&str], env: Option<(&str, &str)>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_forge"));
    cmd.args(args);
    if let Some((k, v)) = env {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute forge binary")
}

fn forge(args: &[&str]) -> std::process::Output {
    forge_with_env(args, None)
}

/// Helper: write a .kzd file into `dir` and return its full path.
fn write_kzd(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(name);
    fs::write(&file_path, content).expect("Failed to write .kzd test file");
    file_path
}

/// Combine stdout+stderr into one lowercase string for substring checks.
fn combined(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// Correct (provable) contract: `abs` always returns a non-negative result.
const ABS_SRC: &str = "@gungnir\n\
                       @ensures(result >= 0)\n\
                       fn abs(a: Int) -> Int { if a < 0 { -a } else { a } }";

// Incorrect contract: `identity` cannot guarantee `result < 5` for all inputs.
const IDENTITY_SRC: &str = "@gungnir\n\
                            @ensures(result < 5)\n\
                            fn identity(a: Int) -> Int { a }";

// ---------------------------------------------------------------------------
// 1. The `gungnir` subcommand exists
// ---------------------------------------------------------------------------

#[test]
fn test_forge_gungnir_subcommand_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "abs.kzd", ABS_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        !out.to_lowercase().contains("unrecognized subcommand")
            && !out.to_lowercase().contains("invalid subcommand"),
        "gungnir should be a recognized subcommand; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 2. Per-function status — a correct contract is reported as `proved`
// ---------------------------------------------------------------------------

#[test]
fn test_forge_gungnir_reports_proved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "abs.kzd", ABS_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        out.to_lowercase().contains("abs"),
        "the report should name the verified function; got:\n{}",
        out
    );
    assert!(
        out.to_lowercase().contains("proved"),
        "a correct contract should be reported as proved; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 3. An incorrect contract is reported as `counterexample` with concrete values
// ---------------------------------------------------------------------------

#[test]
fn test_forge_gungnir_reports_counterexample_with_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "identity.kzd", IDENTITY_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        out.to_lowercase().contains("counterexample"),
        "a failing contract should be reported as a counterexample; got:\n{}",
        out
    );
    // The concrete model witness must surface (z3 picks a = 5 for this query).
    assert!(
        out.contains('5') || out.to_lowercase().contains("a ="),
        "the counterexample report should include concrete model values; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 4. Only @gungnir functions are reported
// ---------------------------------------------------------------------------

#[test]
fn test_forge_gungnir_only_reports_annotated_functions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(
        dir.path(),
        "mixed.kzd",
        "fn helper(a: Int, b: Int) -> Int { a + b }\n\
         @gungnir\n\
         @ensures(result >= 0)\n\
         fn double(x: Int) -> Int { x * 2 }",
    );

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        out.to_lowercase().contains("double"),
        "the @gungnir function must be reported; got:\n{}",
        out
    );
    assert!(
        !out.to_lowercase().contains("helper"),
        "plain functions must NOT be reported as verification targets; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 5. Missing z3 → install instructions printed, non-zero exit
// ---------------------------------------------------------------------------

#[test]
fn test_forge_gungnir_missing_z3_prints_install_hints() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "abs.kzd", ABS_SRC);

    // Force z3 absence by pointing DWARF_Z3 at a nonexistent binary. The bridge
    // must treat that as "solver not found" rather than crashing.
    let output = forge_with_env(
        &["gungnir", file_path.to_str().unwrap()],
        Some(("DWARF_Z3", "/nonexistent/z3-binary")),
    );
    let out = combined(&output);

    assert!(
        !output.status.success(),
        "the gungnir command should exit non-zero when z3 is unavailable"
    );
    assert!(
        out.to_lowercase().contains("z3") && out.to_lowercase().contains("install"),
        "missing z3 should print install instructions mentioning z3 + install; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 6. --timeout-ms is accepted, and unproven is a valid reported status
// ---------------------------------------------------------------------------

/// The timeout flag must be recognized (no clap "unknown argument" error).
#[test]
fn test_forge_gungnir_timeout_flag_recognized() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "abs.kzd", ABS_SRC);

    let output = forge(&[
        "gungnir",
        file_path.to_str().unwrap(),
        "--timeout-ms",
        "5000",
    ]);
    let out = combined(&output);

    // The subcommand itself must be recognized, and --timeout-ms must be an
    // accepted flag of the subcommand.
    assert!(
        !out.to_lowercase().contains("unrecognized subcommand")
            && !out.to_lowercase().contains("invalid subcommand"),
        "gungnir should be a recognized subcommand; got:\n{}",
        out
    );
    assert!(
        !out.to_lowercase().contains("unknown argument")
            && !out.to_lowercase().contains("unexpected argument"),
        "--timeout-ms should be a recognized flag; got:\n{}",
        out
    );
}

/// A reported per-function status of `unproven` is a valid, accepted outcome
/// (mirrors the DWARF-120 timeout acceptance criterion; the deterministic
/// `unknown → unproven` mapping is pinned at unit level in
/// `dwarf-lib/tests/gungnir_tests.rs::test_parse_smt_output_unknown_is_unproven`).
#[test]
fn test_forge_gungnir_accepts_unproven_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    // No verifiable contract — the function is discovered but cannot be proven.
    let file_path = write_kzd(
        dir.path(),
        "opaque.kzd",
        "@gungnir\nfn opaque(x: Int) -> Int { x }",
    );

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    // The command must not crash, and must emit a report naming the function.
    let out = combined(&output);
    assert!(
        out.to_lowercase().contains("opaque"),
        "the report should name the @gungnir function; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 7. Soundness hardening — no post-condition → unproven, not counterexample
// ---------------------------------------------------------------------------

/// A function with NO @ensures has nothing to disprove. It must be reported as
/// `unproven` (with a "no post-condition" reason), NOT as a false
/// `counterexample` that conflates "no contract" with "violated" (Fix #4).
#[test]
fn test_forge_gungnir_no_contract_is_unproven_not_counterexample() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(
        dir.path(),
        "nopost.kzd",
        "@gungnir\nfn opaque(x: Int) -> Int { x }",
    );

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);
    assert!(
        out.to_lowercase().contains("opaque — unproven"),
        "a function without a post-condition must be reported as unproven; got:\n{}",
        out
    );
    assert!(
        !out.to_lowercase().contains("opaque — counterexample"),
        "a function without a post-condition must NOT be reported as a counterexample; got:\n{}",
        out
    );
}

// ---------------------------------------------------------------------------
// 8. Soundness hardening — a body referencing `result` must not be proved
// ---------------------------------------------------------------------------

/// `fn f(a: Int) -> Int { result + 1 }` would build `(= result (+ result 1))`,
/// trivially unsat → a vacuous `Proved`. The engine must now reject it (Fix #2).
#[test]
fn test_forge_gungnir_result_in_body_is_not_proved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(
        dir.path(),
        "vacuous.kzd",
        "@gungnir\n\
         @ensures(result < 0)\n\
         fn vacuous(a: Int) -> Int { result + 1 }",
    );

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);
    assert!(
        out.to_lowercase().contains("vacuous — unproven"),
        "a vacuous result-in-body function must be reported as unproven; got:\n{}",
        out
    );
    assert!(
        !out.to_lowercase().contains("vacuous — proved"),
        "a vacuous result-in-body function must NOT be reported as proved; got:\n{}",
        out
    );
}
