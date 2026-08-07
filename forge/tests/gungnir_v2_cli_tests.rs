//! RED-phase end-to-end tests for DWARF-131: Gungnir v2 hardening (forge CLI).
//!
//! These tests drive the compiled `forge gungnir` binary against real `.kzd`
//! files and real Z3 — the subprocess-level counterpart to
//! `dwarf-lib/tests/gungnir_v2_unit_tests.rs`. Helper style mirrors
//! `forge/tests/gungnir_cli_tests.rs`.
//!
//! # Contracts pinned here (per acceptance criterion)
//!
//! - **AC-1 (honest invariant semantics / relabel)**: when a function carries
//!   an `@invariant`, the human report must surface the honest ENTRY-invariant
//!   semantics in the report wording (e.g. an "entry" note). Today the invariant
//!   function reports only `proved` with no acknowledgment of the semantics —
//!   that oversells it. The relabel is the RED.
//! - **AC-2 (old(member) e2e)**: a `@ensures(old(w.balance) <= result)` deposit
//!   that is in fact verifiable must be reported `proved`. Today it is rejected
//!   as `unproven` (the v1 `old(param)`-only guard) — the RED.
//! - **AC-4 (real subprocess timeout)**: forcing z3 to exceed a tiny
//!   `--timeout-ms` budget must be reported `unproven` (no hang, no crash), and
//!   the human report must surface the timeout reason so a real timeout is
//!   distinguishable from other `unproven` causes. The `unproven` mapping is
//!   implemented (DWARF-120); surfacing the timeout note is the RED.
//! - **AC-5 (multi-statement body, CLI)**: a body with multiple statements must
//!   be reported `unproven` with an "unsupported" diagnostic, NOT a bare `error`.
//!   Today the multi-statement body slips through the soundness gate and is
//!   reported `error` — the RED.
//!
//! # Z3 on this machine
//!
//! Z3 4.8.12 is found at `/usr/bin/z3` and on `$PATH`, so the solver-requiring
//! tests run (not ignored). [`has_z3`] mirrors the bridge's
//! `DWARF_Z3`-then-`$PATH` resolution; on a machine without z3 the real-timeout
//! test self-skips (a documented, non-flaky fallback) rather than failing.

use serde_json::Value;
use std::fs;
use std::process::Command;

/// Whether a Z3 solver is resolvable (mirror of the bridge's `DWARF_Z3` then
/// `$PATH` resolution).
fn has_z3() -> bool {
    if let Some(path) = std::env::var_os("DWARF_Z3") {
        return std::path::PathBuf::from(path).is_file();
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|dir| dir.join("z3").is_file())
}

/// Run the forge binary with the given args + optional env, returning Output.
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

/// Write a `.kzd` file into `dir` and return its full path.
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

// Verifiable money-flow contract carrying an @invariant. Z3 answers `unsat` for
// this script, so today it reports `proved`.
const WALLET_SRC: &str = "record Wallet { balance: Int }\n\
                          @gungnir\n\
                          @invariant(balance >= 0)\n\
                          @requires(amount >= 0)\n\
                          @ensures(result >= 0)\n\
                          fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }";

// Same money-flow, but the ensures references the pre-state member the proper
// (first-class) way. Under AC-2 support semantics this is verifiable → `proved`.
const OLD_MEMBER_SRC: &str = "record Wallet { balance: Int }\n\
                              @gungnir\n\
                              @invariant(balance >= 0)\n\
                              @requires(amount >= 0)\n\
                              @ensures(old(w.balance) <= result)\n\
                              fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }";

// A query meant to exceed a tiny solver budget: several multiplications.
const HARD_SRC: &str = "@gungnir\n\
                        @ensures(result >= 0)\n\
                        fn hard(a: Int, b: Int, c: Int, d: Int) -> Int { a * b * c * d + a - b }";

// A body with multiple statements (both referencing only params). Today it
// slips past the soundness gate and is reported `error`; it must be `unproven`.
const MULTI_STMT_SRC: &str = "@gungnir\n\
                              @ensures(result >= 0)\n\
                              fn multi(a: Int) -> Int { a\n a + 1 }";

// JSON-contract-pin fixtures (see `test_forge_gungnir_json_contract_stable`):
// `abs` is a proven function with no extras; `no_contract` deliberately omits
// a post-condition so it is reported `unproven` with a "no post-condition"
// reason (see `verify_file`'s post.is_none() guard).
const JSON_PIN_PROVEN_SRC: &str = "@gungnir\n\
                                   @ensures(result >= 0)\n\
                                   fn abs(a: Int) -> Int { if a < 0 { -a } else { a } }";

const JSON_PIN_NO_CONTRACT_SRC: &str = "@gungnir\n\
                                        fn no_contract(a: Int) -> Int { a }";

// ---------------------------------------------------------------------------
// AC-1: @invariant semantics honesty — the report surfaces the ENTRY semantics
// ---------------------------------------------------------------------------

/// When an `@invariant` is verified, the human report must not just say
/// `proved`; it must acknowledge that `@invariant` is an ENTRY invariant (the
/// honest relabel), so it is not oversold as a check across all reachable states.
#[test]
fn test_forge_gungnir_invariant_reports_entry_semantics() {
    if !has_z3() {
        eprintln!("SKIP: z3 not resolvable; cannot exercise the solver");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "wallet.kzd", WALLET_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        output.status.success(),
        "invariant verification should run without tool failure; got:\n{out}"
    );
    assert!(
        out.to_lowercase().contains("deposit") && out.to_lowercase().contains("proved"),
        "the invariant-bearing deposit should still be verified as proved; got:\n{out}"
    );
    // The honest relabel: the report must surface that @invariant is an ENTRY
    // invariant (entry-only semantics), not a catch-all inductive preservation.
    assert!(
        out.to_lowercase().contains("entry"),
        "the report must acknowledge @invariant is verified as an ENTRY \
         (data-consistency) invariant, not oversold as checked across all \
         reachable states; got:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// AC-2: old(member) verified end-to-end
// ---------------------------------------------------------------------------

/// `old(w.balance) <= result` is a valid, verifiable contract once
/// old(member) is first-class. It must be reported `proved`, not rejected as
/// `unproven` by the v1 bare-param-only guard.
#[test]
fn test_forge_gungnir_old_member_verifies() {
    if !has_z3() {
        eprintln!("SKIP: z3 not available");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "oldmem.kzd", OLD_MEMBER_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        out.to_lowercase().contains("deposit — proved"),
        "an old(member) contract that holds must be verified as proved; got:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("deposit — unproven"),
        "old(member) must not be rejected as unsupported; got:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// AC-4: deterministic real-subprocess timeout
// ---------------------------------------------------------------------------

/// Force z3 to exceed a 1 ms budget via `--timeout-ms`. The kill path
/// (`recv_timeout` → kill → reap) must report `unproven` (not hang, not crash),
/// and the human report must surface the timeout reason so a real timeout is
/// distinguishable from other `unproven` causes. Self-skips if Z3 is not
/// resolvable (see [has_z3]).
#[test]
fn test_forge_gungnir_real_timeout_is_unproven_with_note() {
    if !has_z3() {
        eprintln!("SKIP: z3 not available; real-timeout cannot run");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "hard.kzd", HARD_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap(), "--timeout-ms", "1"]);
    let out = combined(&output);

    // The run must terminate quickly and without a crash.
    assert!(
        output.status.success(),
        "forge should exit successfully after a forced timeout (no crash); got:\n{out}"
    );
    assert!(
        out.to_lowercase().contains("hard — unproven"),
        "a solver that exceeds the budget must be reported unproven (not hung, \
         not proved), got:\n{out}"
    );
    // The honest note: distinguish a real timeout from other unproven causes.
    assert!(
        out.to_lowercase().contains("timeout") || out.to_lowercase().contains("exceeded"),
        "the human report must surface the timeout reason so a real timeout is \
         distinguishable from other unproven causes; got:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// AC-5: multi-statement body → clean unsupported/unproven, not a bare error
// ---------------------------------------------------------------------------

/// A body with multiple statements currently falls through the soundness gate
/// and is reported `error` (the translator emits `(unsupported-form)`). The
/// hardened contract: it must be rejected up front and reported `unproven`
/// with a clean "unsupported" diagnostic — never a bare `error`, never a
/// wrong `proved`/`counterexample`.
#[test]
fn test_forge_gungnir_multi_statement_body_is_unsupported() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = write_kzd(dir.path(), "multi.kzd", MULTI_STMT_SRC);

    let output = forge(&["gungnir", file_path.to_str().unwrap()]);
    let out = combined(&output);

    assert!(
        out.to_lowercase().contains("multi — unproven"),
        "a multi-statement body must be reported unproven with a clean \
         unsupported diagnostic; got:\n{out}"
    );
    assert!(
        out.to_lowercase().contains("unsupported"),
        "the diagnostic must state the body is unsupported; got:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("multi — error"),
        "a multi-statement body must not surface as a bare error; got:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// JSON output contract stability pin (Center review gap on DWARF-131)
// ---------------------------------------------------------------------------

/// Pins the `forge gungnir --json` output contract, so the format is a
/// regression-tested contract rather than an unasserted side effect.
///
/// `GungnirResult` carries an internal `has_invariant: bool` (added in
/// DWARF-131) that drives the honest ENTRY-invariant relabel in the human
/// report. `to_json` must NOT leak that field into the serialized form. This
/// test drives the real binary end-to-end over a proven function (`abs`) and a
/// no-contract function (`no_contract` — `unproven`, reason "no post-condition")
/// and asserts:
///
///   * every result object has exactly the contract keys for its status
///     (`file`, `function`, `status`, plus `reason` for `unproven`), and
///   * `has_invariant` never appears anywhere in the JSON document.
///
/// This is a stability PIN: it should PASS for the current (correct) contract.
/// A failure here means the JSON contract is currently violated — that is a
/// finding to report, not something to make green by loosening assertions.
#[test]
fn test_forge_gungnir_json_contract_stable() {
    if !has_z3() {
        eprintln!("SKIP: z3 not resolvable; cannot run the solver-backed JSON pin");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let proven = write_kzd(dir.path(), "abs.kzd", JSON_PIN_PROVEN_SRC);
    let nocontract = write_kzd(dir.path(), "nocontract.kzd", JSON_PIN_NO_CONTRACT_SRC);

    let output = forge(&[
        "gungnir",
        proven.to_str().unwrap(),
        nocontract.to_str().unwrap(),
        "--json",
    ]);
    assert!(
        output.status.success(),
        "forge gungnir --json should exit successfully; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The forge binary writes `{"gungnir": [...]}` to stdout with no extra
    // human text, so the whole stdout must parse as one JSON document.
    let root: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\n{stdout}"));

    // has_invariant must never leak — even at a substring level (defensive,
    // independent of how the object is structured).
    assert!(
        !stdout.contains("\"has_invariant\""),
        "JSON must not leak the internal has_invariant field; got:\n{stdout}"
    );
    assert!(
        !stdout.contains("has_invariant"),
        "JSON must not contain has_invariant anywhere; got:\n{stdout}"
    );

    let gungnir = root
        .get("gungnir")
        .and_then(Value::as_array)
        .expect("root must have a \"gungnir\" array");

    let judged: Vec<(&str, Value)> = gungnir
        .iter()
        .filter_map(|v| {
            let fn_name = v.get("function")?.as_str()?;
            Some((fn_name, v.clone()))
        })
        .collect();

    let find = |name: &str| {
        judged
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v)
            .expect("expected a result for {name}")
    };

    // --- proven contract: keys are exactly {file, function, status} ---
    let proven_obj = find("abs");
    assert_eq!(
        proven_obj.get("status").and_then(Value::as_str),
        Some("proved"),
        "abs must be reported proved; got:\n{stdout}"
    );
    let proven_keys: Vec<&str> = {
        let obj = proven_obj.as_object().expect("proven result is an object");
        let mut k: Vec<&str> = obj.keys().map(String::as_str).collect();
        k.sort_unstable();
        k
    };
    assert_eq!(
        proven_keys,
        vec!["file", "function", "status"],
        "a proved result must have EXACTLY file/function/status (no model, no \
         reason, no has_invariant); got:\n{stdout}"
    );

    // --- no-contract: keys are exactly {file, function, status, reason} ---
    let unproven_obj = find("no_contract");
    assert_eq!(
        unproven_obj.get("status").and_then(Value::as_str),
        Some("unproven"),
        "a function with no post-condition must be reported unproven; got:\n{stdout}"
    );
    let unproven_keys: Vec<&str> = {
        let obj = unproven_obj
            .as_object()
            .expect("unproven result is an object");
        let mut k: Vec<&str> = obj.keys().map(String::as_str).collect();
        k.sort_unstable();
        k
    };
    assert_eq!(
        unproven_keys,
        vec!["file", "function", "reason", "status"],
        "an unproven result must carry EXACTLY file/function/status/reason (no \
         model, no has_invariant); got:\n{stdout}"
    );
    assert_eq!(
        unproven_obj.get("reason").and_then(Value::as_str),
        Some("no post-condition"),
        "the no-contract function must carry the expected reason; got:\n{stdout}"
    );

    // Structural backstop: walk the whole parsed tree and reject has_invariant
    // at any key depth (independent of the raw-string check above).
    fn walk(prefix: &str, v: &Value) {
        match v {
            Value::Object(map) => {
                for (k, child) in map {
                    assert!(
                        !k.contains("has_invariant"),
                        "found has_invariant leak at {prefix}.{k}"
                    );
                    walk(&format!("{prefix}.{k}"), child);
                }
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    walk(&format!("{prefix}[{i}]"), child);
                }
            }
            _ => {}
        }
    }
    walk("root", &root);
}
