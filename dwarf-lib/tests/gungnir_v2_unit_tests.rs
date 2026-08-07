//! RED-phase integration tests for DWARF-131: Gungnir v2 hardening.
//!
//! This file pins the DWARF-131 acceptance criteria that live at the
//! `dwarf-lib` level (pure logic). The Z3 subprocess bridge is exercised in
//! `forge/tests/gungnir_v2_cli_tests.rs`.
//!
//! # Contracts pinned here (per acceptance criterion)
//!
//! - **AC-1 (honest invariant semantics)**: the `@invariant` continues to be
//!   verified as an ENTRY data-consistency invariant (DWARF-120 behavior is
//!   preserved): it is asserted exactly once, on the input side, before the
//!   pre-condition. The relabeled wording ("entry invariant") is pinned at the
//!   CLI level in the companion file.
//! - **AC-2 (old(member)/old(compound) first-class)**: `old(w.balance)` and
//!   `old(a + b)` become SUPPORTED. `unsupported_reason` must return `None` for
//!   them and `build_verification_query` must declare + constrain the pre-state
//!   symbols (`w@pre.balance`, `a@pre`, `b@pre`). Today these are rejected by
//!   the `old_unsupported_arg` guard — that rejection is the RED.
//! - **AC-3 (model extraction robustness)**: a `sat` verdict with a
//!   missing/empty model must NOT be a bare, misleading counterexample. The
//!   chosen contract: missing-model `sat` output maps to a distinct
//!   `unproven`/`error` result whose reason names the missing model — never a
//!   `counterexample` that formats to nothing in the CLI.
//! - **AC-5 (multi-statement / unhandled body nodes)**: a body with multiple
//!   statements, or a body node outside the v1 subset (e.g. `match`), must be
//!   rejected by `unsupported_reason` with a clean, informative "unsupported"
//!   diagnostic instead of being translated into an invalid SMT script that
//!   surfaces as a bare `error`.

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::Parser;
use dwarf_syntax::hir::Decl;

// ---------------------------------------------------------------------------
// Helpers (mirror dwarf-lib/tests/gungnir_tests.rs)
// ---------------------------------------------------------------------------

/// Tokenize + parse a Dwarf source string into its declarations.
fn parse_decls(source: &str) -> Vec<Decl> {
    let tokens = TokenizePass
        .tokenize(source)
        .expect("fixture should tokenize without lexer errors");
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(
        errors.is_empty(),
        "fixture should parse without errors, got:\n  {}",
        errors
            .iter()
            .map(|e| format!("{} (code {})", e.message, e.code))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    decls
}

/// Collapse all whitespace so SMT text can be compared robustly.
fn norm_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text a verdict carries to the user (model for a counterexample, reason
/// for unproven/error), so assertions can be written against the carried text
/// without over-constraining which verdict variant the implementer picks.
fn carried_text(v: &dwarf_lib::gungnir::Verdict) -> String {
    match v {
        dwarf_lib::gungnir::Verdict::Proved => String::new(),
        dwarf_lib::gungnir::Verdict::Counterexample { model } => model.clone(),
        dwarf_lib::gungnir::Verdict::Unproven { reason }
        | dwarf_lib::gungnir::Verdict::Error { reason } => reason.clone(),
    }
}

// ===========================================================================
// AC-1: @invariant semantics honesty (preservation side of the relabel)
// ===========================================================================

/// The DWARF-120 @invariant behavior must be PRESERVED: the invariant is
/// verified as an ENTRY data-consistency invariant — asserted exactly once, on
/// the input side, before the pre-condition. It is NOT re-checked after the
/// mutation (there is no "after every reachable state" induction in v1; the
/// relabeled wording surfaces that honesty at the CLI level).
#[test]
fn test_invariant_entry_semantics_preserved() {
    let decls = parse_decls(
        "record Wallet { balance: Int }\n\
         @gungnir\n\
         @invariant(balance >= 0)\n\
         @requires(amount >= 0)\n\
         @ensures(result >= 0)\n\
         fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let query = dwarf_lib::gungnir::build_verification_query(f);
    let norm = norm_ws(&query);

    // Exactly one occurrence: the invariant is an entry assertion, never
    // duplicated as an after-mutation re-check.
    assert_eq!(
        norm.matches("(>= w.balance 0)").count(),
        1,
        "the invariant must be asserted exactly once (entry semantics), got:\n{query}"
    );
    // Order: invariant before pre-condition.
    let inv_idx = norm.find("(assert (>= w.balance 0))").expect("invariant assert");
    let pre_idx = norm.find("(assert (>= amount 0))").expect("pre assert");
    assert!(
        inv_idx < pre_idx,
        "entry invariant must be asserted before the pre-condition, got:\n{query}"
    );
}

// ===========================================================================
// AC-2: old(member) / old(compound) first-class support
// ===========================================================================

/// DECISION: support (not clean-reject). `prestate()` already renders
/// `old(w.balance)` → `w@pre.balance` and `old(a + b)` → `(+ a@pre b@pre`);
/// the missing pieces are (a) removing the `old_unsupported_arg` rejection and
/// (b) declaring + constraining the pre-state symbols. `old(w.balance)` in a
/// contract condition must NOT be rejected.
#[test]
fn test_old_member_argument_is_supported() {
    let decls = parse_decls(
        "record Wallet { balance: Int }\n\
         @gungnir\n\
         @invariant(balance >= 0)\n\
         @requires(amount >= 0)\n\
         @ensures(old(w.balance) <= result)\n\
         fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    assert_eq!(
        dwarf_lib::gungnir::unsupported_reason(f),
        None,
        "old(w.balance) must be a supported first-class old() argument \
         (v1 currently rejects non-param old() args)"
    );
}

/// `old(w.balance)` must emit a pre-state declaration for the record member and
/// an equality binding to the current-state member, so the `w@pre.balance`
/// symbol referenced by the negated ensures is declared.
#[test]
fn test_old_member_query_declares_prestate_bindings() {
    let decls = parse_decls(
        "record Wallet { balance: Int }\n\
         @gungnir\n\
         @invariant(balance >= 0)\n\
         @requires(amount >= 0)\n\
         @ensures(old(w.balance) <= result)\n\
         fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let query = dwarf_lib::gungnir::build_verification_query(f);
    let norm = norm_ws(&query);

    assert!(
        norm.contains("(declare-const w@pre.balance Int)"),
        "old(w.balance) must declare the pre-state member symbol; got:\n{query}"
    );
    assert!(
        norm.contains("(assert (= w.balance w@pre.balance))"),
        "old(w.balance) must bind the pre-state member to the current-state member; got:\n{query}"
    );
    assert!(
        norm.contains("(assert (not (<= w@pre.balance result)))"),
        "the ensures must be checked against the pre-state member symbol; got:\n{query}"
    );
}

/// A compound `old(a + b)` argument must not be rejected either.
#[test]
fn test_old_compound_argument_is_supported() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(old(a + b) > 0)\n\
         fn sum_positive(a: Int, b: Int) -> Int { a + b }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    assert_eq!(
        dwarf_lib::gungnir::unsupported_reason(f),
        None,
        "old(a + b) must be a supported first-class old() argument"
    );
}

/// `old(a + b)` must declare + bind every parameter referenced under `old()`
/// (`a@pre`, `b@pre`), so the compound pre-state expression is well-formed.
#[test]
fn test_old_compound_query_declares_prestate_bindings() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(old(a + b) > 0)\n\
         fn sum_positive(a: Int, b: Int) -> Int { a + b }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let query = dwarf_lib::gungnir::build_verification_query(f);
    let norm = norm_ws(&query);

    assert!(
        norm.contains("(declare-const a@pre Int)") && norm.contains("(declare-const b@pre Int)"),
        "old(a + b) must declare every pre-state param symbol; got:\n{query}"
    );
    assert!(
        norm.contains("(assert (= a a@pre))") && norm.contains("(assert (= b b@pre))"),
        "old(a + b) must bind every pre-state param to its current state; got:\n{query}"
    );
}

// ===========================================================================
// AC-3: sat + missing/empty model must not be a misleading counterexample
// ===========================================================================

/// A bare `sat` verdict line with NO model following it is today parsed as
/// `Counterexample { model: "" }` — an empty, misleading counterexample that
/// formats to nothing in the CLI. The hardened contract: it must map to a
/// distinct `unproven`/`error` result whose reason names the missing model.
#[test]
fn test_parse_smt_output_bare_sat_is_not_bare_counterexample() {
    let verdict = dwarf_lib::gungnir::parse_smt_output("sat");
    assert!(
        matches!(
            verdict,
            dwarf_lib::gungnir::Verdict::Unproven { .. }
                | dwarf_lib::gungnir::Verdict::Error { .. }
        ),
        "bare `sat` with no model must be reported as unproven/error, not as a \
         bare empty counterexample; got {verdict:?}"
    );
    let text = carried_text(&verdict).to_lowercase();
    assert!(
        text.contains("model"),
        "the reason must name the missing model; got: {:?}",
        carried_text(&verdict)
    );
}

/// `sat` followed by z3's `(error "model is not available")` must NOT be
/// reported as a counterexample either: without a witness there is nothing to
/// show, so the verdict must be a distinct unproven/error that explicitly notes
/// the model is unavailable.
#[test]
fn test_parse_smt_output_sat_missing_model_is_distinct() {
    let verdict = dwarf_lib::gungnir::parse_smt_output("sat\n(error \"model is not available\")");
    assert!(
        matches!(
            verdict,
            dwarf_lib::gungnir::Verdict::Unproven { .. }
                | dwarf_lib::gungnir::Verdict::Error { .. }
        ),
        "sat + missing-model must be reported as unproven/error, not as a \
         counterexample; got {verdict:?}"
    );
    let text = carried_text(&verdict).to_lowercase();
    assert!(
        text.contains("model") && text.contains("not available"),
        "the reason must explicitly note the missing model; got: {:?}",
        carried_text(&verdict)
    );
}

// ===========================================================================
// AC-5: multi-statement / unhandled body nodes → clean unsupported diagnostic
// ===========================================================================

/// A body with multiple expression statements (all referencing only params)
/// currently slips past `unsupported_reason` (None), so the builder emits
/// `(unsupported-form)` and the CLI reports a bare `error`. The hardened
/// contract: it must be rejected up front with a clean "unsupported" reason so
/// the CLI can report `unproven` with a meaningful diagnostic.
#[test]
fn test_unsupported_reason_multiple_body_statements() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn multi_stmt(a: Int) -> Int { a\n a + 1 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let reason = dwarf_lib::gungnir::unsupported_reason(f);
    assert!(
        reason.is_some(),
        "a multi-statement body must be rejected with a clean unsupported reason"
    );
    let msg = reason.unwrap().to_lowercase();
    assert!(
        msg.contains("unsupported") || msg.contains("statement") || msg.contains("multiple"),
        "the diagnostic must clearly say the body is unsupported; got: {msg}"
    );
}

/// A body node outside the v1 subset (here a `match` expression) currently
/// slips past `unsupported_reason` (None) and becomes invalid SMT. It must be
/// rejected with a clean "unsupported" diagnostic instead of producing a wrong
/// verdict downstream.
#[test]
fn test_unsupported_reason_unsupported_body_node_match() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn matcher(a: Int) -> Int { match a { 1 => 10 } }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let reason = dwarf_lib::gungnir::unsupported_reason(f);
    assert!(
        reason.is_some(),
        "a `match` body node (outside the v1 subset) must be rejected with a \
         clean unsupported reason"
    );
    let msg = reason.unwrap().to_lowercase();
    assert!(
        msg.contains("unsupported"),
        "the diagnostic must clearly say the body node is unsupported; got: {msg}"
    );
}
