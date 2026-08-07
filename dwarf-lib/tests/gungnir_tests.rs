//! Integration tests for DWARF-120: Gungnir — Z3 formal verification.
//!
//! These tests define the expected contract for the SMT-LIB2 translator and
//! the verification-query builder, following the same structure as
//! `resolver_tests.rs` and `draupnir_module_tests.rs`.
//!
//! Now GREEN (DWARF-120): the `dwarf_lib::gungnir` module exists and these
//! tests pin the public API and SMT-LIB2 output shape. The Z3 subprocess bridge
//! intentionally lives in `forge` (see `forge/tests/gungnir_cli_tests.rs`), so
//! no test in this file spawns a subprocess.
//!
//! # Pinned API (the Green contract)
//!
//! ```ignore
//! // dwarf-lib/src/gungnir.rs, declared `pub mod gungnir` in src/lib.rs
//! use dwarf_syntax::hir::{Decl, Expr, Param, Type};
//!
//! pub struct GungnirContract {
//!     pub pre: Option<Expr>,        // parsed `@requires(cond)`
//!     pub post: Option<Expr>,       // parsed `@ensures(cond)`
//!     pub invariant: Option<Expr>,  // parsed `@invariant(cond)`
//! }
//!
//! pub struct GungnirFunction {
//!     pub name: String,
//!     pub params: Vec<Param>,
//!     pub return_type: Option<Type>,
//!     pub body: Expr,
//!     pub contract: GungnirContract,
//! }
//!
//! pub fn discover_gungnir(decls: &[Decl]) -> Vec<GungnirFunction>;
//! pub fn translate_smt2(expr: &Expr) -> String;                 // pure expr → SMT-LIB2
//! pub fn build_verification_query(f: &GungnirFunction) -> String; // full SMT script
//! pub fn parse_smt_output(stdout: &str) -> Verdict;
//!
//! pub enum Verdict {
//!     Proved,
//!     Counterexample { model: String },
//!     Unproven { reason: String },   // z3 `unknown` (timeout) — reported as "unproven"
//!     Error { reason: String },
//! }
//! impl Verdict { pub fn label(&self) -> &'static str; } // proved|counterexample|unproven|error
//! ```
//!
//! # Translation rules pinned here
//!
//! | Dwarf expr            | SMT-LIB2               |
//! |-----------------------|------------------------|
//! | `a + b`               | `(+ a b)`              |
//! | `a - b`               | `(- a b)`              |
//! | `a * b`               | `(* a b)`              |
//! | `a / b` (Int)         | `(div a b)`            |
//! | `a == b`              | `(= a b)`              |
//! | `a != b`              | `(not (= a b))`        |
//! | `a < b`               | `(< a b)`              |
//! | `a > b`               | `(> a b)`              |
//! | `a <= b`              | `(<= a b)`             |
//! | `a >= b`              | `(>= a b)`             |
//! | `a && b`              | `(and a b)`            |
//! | `a \|\| b`            | `(or a b)`             |
//! | `!a`                  | `(not a)`              |
//! | `-a` (unary)          | `(- a)`                |
//! | `old(e)`              | `e@pre` (pre-state)    |
//! | `if c { t } else { f }`| `(ite c t f)`          |
//! | `obj.field`           | `obj.field` (SMT symbol)|
//!
//! # Query shape pinned here
//!
//! For `fn f(params...) -> T { body }` with `@requires(PRE)`,
//! `@ensures(POST)`, `@invariant(INV)` the emitted script is, in order:
//!
//! ```text
//! (declare-const <param> <sort>)...          // Int for Int params
//! (declare-const <param>@pre <sort>)...      // ONLY for params referenced by old()
//! (declare-const result <sort>)
//! (assert (= <param> <param>@pre))...        // ONLY for params referenced by old()
//! (assert <INV resolved to record param>)    // when @invariant present (entry invariant)
//! (assert <PRE>)                             // when @requires present
//! (assert (= result <body>))
//! (assert (not <POST>))
//! (check-sat)
//! (get-model)
//! ```
//!
//! NOTE: the emitted assertion order is INV **before** PRE. The goldens below
//! and the builder in `dwarf_lib/src/gungnir.rs` both use this order.
//!
//! `result` in `@ensures` is the magic return-value variable (the decorator
//! grammar in `dwarf-syntax/src/hir.rs` already documents `@ensures(result > 0)`
//! style conditions). `old(e)` renders to the pre-state symbol namespace; for a
//! parameter `p`, `p@pre` is declared and constrained `(= p p@pre)` so `old()`
//! is sound for expression bodies (parameters are immutable inputs) while
//! remaining a distinct, observable symbol.
//!
//! # @invariant v1 semantics (pinned)
//!
//! For v1 the `@invariant(C)` on a `@gungnir` method is verified as an ENTRY
//! data-consistency invariant: `C` (with bare field names resolved against the
//! function's record-typed parameter, e.g. `balance` → `w.balance` for
//! `w: Wallet`) is asserted on entry and the method's `@ensures` must be
//! provable under it, so mutations of the record's data are checked to preserve
//! the invariant (disproved when the ensures fails → counterexample). The
//! stronger "check the invariant after every method of the record" semantics is
//! explicitly OUT OF SCOPE for DWARF-120 v1 and is noted in the report.

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::Parser;
use dwarf_syntax::hir::{Decl, Expr};

// ---------------------------------------------------------------------------
// Helpers
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

/// Parse a bare expression body (e.g. `a + b`) by wrapping it in a function
/// and returning the parsed body `Expr`. The parser does no name resolution, so
/// undeclared variables (params, `result`, `old`) parse fine.
fn expr_from_source(expr_src: &str) -> Expr {
    let source = format!("fn __f__() -> Bool {{ {} }}", expr_src);
    let decls = parse_decls(&source);
    match &decls[0] {
        Decl::Function { body, .. } => body.clone(),
        other => panic!("expected function decl, got {other:?}"),
    }
}

/// Collapse all whitespace so golden SMT text can be compared robustly.
fn norm_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ===========================================================================
// Part 1: module wiring — dwarf-lib must declare `pub mod gungnir`
// ===========================================================================

/// dwarf-lib must declare a `gungnir` module in lib.rs and the module file must
/// exist on disk (mirror of the dunit/draupnir module wiring).
#[test]
fn test_gungnir_module_declared_in_lib() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let lib_rs_path = std::path::PathBuf::from(manifest_dir).join("src").join("lib.rs");

    assert!(
        lib_rs_path.exists(),
        "dwarf-lib/src/lib.rs should exist at {:?}",
        lib_rs_path
    );
    let lib_content = std::fs::read_to_string(&lib_rs_path).expect("should be able to read lib.rs");
    assert!(
        lib_content.contains("mod gungnir") || lib_content.contains("pub mod gungnir"),
        "lib.rs should declare a 'mod gungnir' module, got:\n{}",
        lib_content
    );

    let module_path = std::path::PathBuf::from(manifest_dir).join("src").join("gungnir.rs");
    assert!(
        module_path.exists(),
        "dwarf-lib/src/gungnir.rs should exist at {:?}, but file not found",
        module_path
    );
}

// ===========================================================================
// Part 2: @gungnir discovery — the module's entry point
// ===========================================================================

/// The flagship discovery fixture: a `@gungnir` function with `@ensures`. Both
/// the marker and the contract decorators must be surfaced.
#[test]
fn test_discover_gungnir_finds_annotated_function() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn abs(a: Int) -> Int { if a < 0 { -a } else { a } }",
    );

    let functions = dwarf_lib::gungnir::discover_gungnir(&decls);
    assert_eq!(
        functions.len(),
        1,
        "exactly one @gungnir function should be discovered"
    );
    let f = &functions[0];
    assert_eq!(f.name, "abs", "discovered function name should match the source");
    assert_eq!(f.params.len(), 1, "abs takes one parameter");
    assert_eq!(f.params[0].name, "a");

    // The ensures condition must be recovered as a parsed Expr, not a string.
    let post = f
        .contract
        .post
        .as_ref()
        .expect("@ensures(result >= 0) should be recovered as a parsed post-condition");
    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::translate_smt2(post)),
        "(>= result 0)",
        "the @ensures condition should translate to its SMT-LIB2 form"
    );
}

/// Functions WITHOUT the `@gungnir` marker must not be discovered.
#[test]
fn test_discover_gungnir_ignores_plain_functions() {
    let decls = parse_decls(
        "fn add(a: Int, b: Int) -> Int { a + b }\n\
         @gungnir\n\
         @ensures(result > a)\n\
         fn inc(a: Int) -> Int { a + 1 }",
    );

    let functions = dwarf_lib::gungnir::discover_gungnir(&decls);
    assert_eq!(functions.len(), 1, "only the @gungnir function is discovered");
    assert_eq!(functions[0].name, "inc");
}

/// A `@gungnir` function with no contract decorators must still be discovered
/// (with an empty contract) so the CLI can report it as unverifiable.
#[test]
fn test_discover_gungnir_function_without_contract() {
    let decls = parse_decls("@gungnir\nfn mystery(a: Int) -> Int { a }");

    let functions = dwarf_lib::gungnir::discover_gungnir(&decls);
    assert_eq!(functions.len(), 1, "@gungnir without contract is still discovered");
    assert!(functions[0].contract.pre.is_none());
    assert!(functions[0].contract.post.is_none());
    assert!(functions[0].contract.invariant.is_none());
}

// ===========================================================================
// Part 3: SMT-LIB2 expression translator
// ===========================================================================

#[test]
fn test_translate_smt2_arithmetic() {
    let cases = [
        ("a + b", "(+ a b)"),
        ("a - b", "(- a b)"),
        ("a * b", "(* a b)"),
        ("a / b", "(div a b)"),
    ];
    for (src, expected) in cases {
        let expr = expr_from_source(src);
        assert_eq!(
            norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
            expected,
            "translating `{src}` should produce `{expected}`"
        );
    }
}

#[test]
fn test_translate_smt2_comparisons() {
    let cases = [
        ("a == b", "(= a b)"),
        ("a != b", "(not (= a b))"),
        ("a < b", "(< a b)"),
        ("a > b", "(> a b)"),
        ("a <= b", "(<= a b)"),
        ("a >= b", "(>= a b)"),
    ];
    for (src, expected) in cases {
        let expr = expr_from_source(src);
        assert_eq!(
            norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
            expected,
            "translating `{src}` should produce `{expected}`"
        );
    }
}

#[test]
fn test_translate_smt2_logical_and_unary() {
    let cases = [
        ("a && b", "(and a b)"),
        ("a || b", "(or a b)"),
        ("!a", "(not a)"),
        ("-a", "(- a)"),
    ];
    for (src, expected) in cases {
        let expr = expr_from_source(src);
        assert_eq!(
            norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
            expected,
            "translating `{src}` should produce `{expected}`"
        );
    }
}

/// `old(e)` must translate to the pre-state symbol `e@pre` — NOT to a generic
/// call and NOT to the bare post-state value. This is what makes `old()`
/// observable in the emitted query.
#[test]
fn test_translate_smt2_old_pre_state() {
    let expr = expr_from_source("old(a)");
    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
        "a@pre",
        "old(a) must render to the pre-state symbol a@pre"
    );
}

/// Nested old() translates recursively (old of a compound expression).
#[test]
fn test_translate_smt2_old_nested_expression() {
    let expr = expr_from_source("old(a + b)");
    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
        "(+ a@pre b@pre)",
        "old(a + b) must translate its argument against the pre-state namespace"
    );
}

#[test]
fn test_translate_smt2_ite() {
    let expr = expr_from_source("if a < 0 { -a } else { a }");
    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
        "(ite (< a 0) (- a) a)",
        "if/else must translate to SMT-LIB2 ite"
    );
}

/// Record member access translates to a dotted SMT symbol (`w.balance`) so the
/// record data is directly addressable in queries.
#[test]
fn test_translate_smt2_member_access() {
    let expr = expr_from_source("w.balance");
    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
        "w.balance",
        "member access must translate to a dotted SMT symbol"
    );
}

/// Literals and the magic `result` variable keep their names.
#[test]
fn test_translate_smt2_literals_and_result() {
    let cases = [
        ("5", "5"),
        ("true", "true"),
        ("false", "false"),
        ("result", "result"),
    ];
    for (src, expected) in cases {
        let expr = expr_from_source(src);
        assert_eq!(
            norm_ws(&dwarf_lib::gungnir::translate_smt2(&expr)),
            expected,
            "translating `{src}` should produce `{expected}`"
        );
    }
}

// ===========================================================================
// Part 4: verification-query builder
// ===========================================================================

/// Golden query for the flagship ABS contract. Z3 answers `unsat` for this
/// script (verified against z3 4.8.12), which the bridge reports as "proved".
#[test]
fn test_build_verification_query_abs_proved_shape() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn abs(a: Int) -> Int { if a < 0 { -a } else { a } }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let expected = "\
        (declare-const a Int)\n\
        (declare-const result Int)\n\
        (assert (= result (ite (< a 0) (- a) a)))\n\
        (assert (not (>= result 0)))\n\
        (check-sat)\n\
        (get-model)";

    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::build_verification_query(f)),
        norm_ws(expected),
        "the abs verification query must match the pinned SMT-LIB2 shape"
    );
}

/// Golden query for an INCORRECT contract (`result < 5` for `identity`). Z3
/// answers `sat` (a = 5), which the bridge reports as a counterexample.
#[test]
fn test_build_verification_query_counterexample_shape() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result < 5)\n\
         fn identity(a: Int) -> Int { a }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let expected = "\
        (declare-const a Int)\n\
        (declare-const result Int)\n\
        (assert (= result a))\n\
        (assert (not (< result 5)))\n\
        (check-sat)\n\
        (get-model)";

    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::build_verification_query(f)),
        norm_ws(expected),
        "the identity verification query must match the pinned SMT-LIB2 shape"
    );
}

/// Golden query pinning @requires and old(): the pre-condition is asserted,
/// `old(a)` resolves to `a@pre`, and `a@pre` is declared + tied to `a` so the
/// pre-state alias is sound. Z3 answers `unsat` (proved).
#[test]
fn test_build_verification_query_requires_and_old_shape() {
    let decls = parse_decls(
        "@gungnir\n\
         @requires(a > 0)\n\
         @ensures(old(a) < result)\n\
         fn successor(a: Int) -> Int { a + 1 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let expected = "\
        (declare-const a Int)\n\
        (declare-const a@pre Int)\n\
        (declare-const result Int)\n\
        (assert (= a a@pre))\n\
        (assert (> a 0))\n\
        (assert (= result (+ a 1)))\n\
        (assert (not (< a@pre result)))\n\
        (check-sat)\n\
        (get-model)";

    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::build_verification_query(f)),
        norm_ws(expected),
        "requires + old() must produce the pinned pre-state query shape"
    );
}

/// @invariant v1: the invariant `balance >= 0` is resolved against the
/// record-typed parameter `w` and asserted on entry; the mutation (deposit)
/// must then prove the ensures. Z3 answers `unsat` (verified on mutation).
#[test]
fn test_build_verification_query_invariant_verified_shape() {
    let decls = parse_decls(
        "record Wallet { balance: Int }\n\
         @gungnir\n\
         @invariant(balance >= 0)\n\
         @requires(amount >= 0)\n\
         @ensures(result >= 0)\n\
         fn deposit(w: Wallet, amount: Int) -> Int { w.balance + amount }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let expected = "\
        (declare-const w.balance Int)\n\
        (declare-const amount Int)\n\
        (declare-const result Int)\n\
        (assert (>= w.balance 0))\n\
        (assert (>= amount 0))\n\
        (assert (= result (+ w.balance amount)))\n\
        (assert (not (>= result 0)))\n\
        (check-sat)\n\
        (get-model)";

    assert_eq!(
        norm_ws(&dwarf_lib::gungnir::build_verification_query(f)),
        norm_ws(expected),
        "the invariant query must assert the entry invariant and the mutation's ensures"
    );
}

/// @invariant disproved: without the @requires guard, overdrawing (amount >
/// balance) violates the invariant-derived ensures → the query is `sat`.
#[test]
fn test_build_verification_query_invariant_disproved_shape() {
    let decls = parse_decls(
        "record Wallet { balance: Int }\n\
         @gungnir\n\
         @invariant(balance >= 0)\n\
         @ensures(result >= 0)\n\
         fn withdraw(w: Wallet, amount: Int) -> Int { w.balance - amount }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    let query = dwarf_lib::gungnir::build_verification_query(f);
    assert!(
        norm_ws(&query).contains("(>= w.balance 0)"),
        "invariant must be asserted on entry; got query:\n{query}"
    );
    assert!(
        norm_ws(&query).contains("(assert (not (>= result 0)))"),
        "ensures negation must be asserted; got query:\n{query}"
    );
}

// ===========================================================================
// Part 4b: soundness hardening (DWARF-120) — unsupported_reason and holes
// ===========================================================================

/// A refined param (`a: Int(0..100)`) must have its range emitted as domain
/// assertions so z3 cannot produce a counterexample outside the type's domain
/// (a false counterexample). This pins Fix #1.
#[test]
fn test_build_verification_query_refined_domain_asserted() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn bounded(a: Int(0..100)) -> Int { a }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];

    // The refined param is declared with its base sort AND constrained.
    let query = dwarf_lib::gungnir::build_verification_query(f);
    let norm = norm_ws(&query);
    assert!(
        norm.contains("(declare-const a Int)"),
        "refined a Int(0..100) must declare with its base Int sort; got:\n{query}"
    );
    assert!(
        norm.contains("(assert (>= a 0))") && norm.contains("(assert (<= a 100))"),
        "refined range 0..100 must be asserted as the domain; got:\n{query}"
    );
}

/// A refined param must be SOUNDLY verifiable — `unsupported_reason` returns
/// None for it because the range is honored, not dropped.
#[test]
fn test_refined_type_is_supported_and_not_rejected() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn bounded(a: Int(0..100)) -> Int { a }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert_eq!(
        dwarf_lib::gungnir::unsupported_reason(f),
        None,
        "a refined Int param must not be rejected"
    );
}

/// A body that references the magic `result` would build `(= result (+ result 1))`
/// — trivially unsat → a **vacuous Proved**. The engine must now reject it (Fix #2).
#[test]
fn test_body_referencing_result_is_rejected() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result < 0)\n\
         fn broken(a: Int) -> Int { result + 1 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert!(
        dwarf_lib::gungnir::unsupported_reason(f).is_some(),
        "a body using `result` must be rejected, not vacuously proved"
    );
}

/// A body referencing `old(...)` is invalid in a body (old() is only for
/// contract conditions) — it must be rejected (Fix #2).
#[test]
fn test_body_referencing_old_is_rejected() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn broken(a: Int) -> Int { old(a) + 1 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert!(
        dwarf_lib::gungnir::unsupported_reason(f).is_some(),
        "a body using old(..) must be rejected"
    );
}

/// A body referencing a free variable that isn't a parameter is rejected (Fix #2).
#[test]
fn test_body_free_variable_is_rejected() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0)\n\
         fn bob(a: Int) -> Int { a + b }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert!(
        dwarf_lib::gungnir::unsupported_reason(f).is_some(),
        "a free variable in the body must be rejected"
    );
}

/// A Bool-returning function must declare `result` as Bool (not Int), so the
/// query does NOT emit `(= Int Bool)` — a sort error (Fix #3).
#[test]
fn test_build_verification_query_bool_return_declares_result_bool() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result == true)\n\
         fn pos(a: Int) -> Bool { a > 0 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert_eq!(dwarf_lib::gungnir::unsupported_reason(f), None);

    let query = dwarf_lib::gungnir::build_verification_query(f);
    assert!(
        norm_ws(&query).contains("(declare-const result Bool)"),
        "Bool-returning fn must declare result as Bool; got:\n{query}"
    );
    assert!(
        norm_ws(&query).contains("(assert (= result (> a 0)))"),
        "Bool body must equal the Bool translation of a > 0; got:\n{query}"
    );
}

/// A Float-returning function must map the bare Z3 `Float` to `Real` (Z3 has no
/// `Float` sort) (Fix #3).
#[test]
fn test_build_verification_query_float_param_maps_to_real() {
    let decls = parse_decls(
        "@gungnir\n\
         @ensures(result >= 0.0)\n\
         fn half(x: Float) -> Float { x / 2.0 }",
    );
    let f = &dwarf_lib::gungnir::discover_gungnir(&decls)[0];
    assert_eq!(dwarf_lib::gungnir::unsupported_reason(f), None);

    let query = dwarf_lib::gungnir::build_verification_query(f);
    let norm = norm_ws(&query);
    assert!(
        norm.contains("(declare-const x Real)"),
        "Float param must map to Real; got:\n{query}"
    );
    assert!(
        norm.contains("(declare-const result Real)"),
        "Float return must map to Real; got:\n{query}"
    );
}

// ===========================================================================
// Part 5: verdict parser (z3 stdout → Verdict)
// ===========================================================================

/// `unsat` from z3 means the contract holds → Proved.
#[test]
fn test_parse_smt_output_unsat_is_proved() {
    let verdict = dwarf_lib::gungnir::parse_smt_output("unsat");
    assert!(
        matches!(verdict, dwarf_lib::gungnir::Verdict::Proved),
        "unsat must map to Proved, got {verdict:?}"
    );
    assert_eq!(verdict.label(), "proved");
}

/// `sat` plus a model must surface a counterexample with concrete values.
#[test]
fn test_parse_smt_output_sat_is_counterexample_with_model() {
    let output = "sat\n(\n  (define-fun a () Int\n    5)\n  (define-fun result () Int\n    5)\n)";
    let verdict = dwarf_lib::gungnir::parse_smt_output(output);
    match &verdict {
        dwarf_lib::gungnir::Verdict::Counterexample { model } => {
            assert!(
                model.contains("a") && model.contains("5"),
                "counterexample model must contain the concrete binding a = 5, got: {model}"
            );
        }
        other => panic!("sat + model must map to Counterexample, got {other:?}"),
    }
    assert_eq!(verdict.label(), "counterexample");
}

/// `unknown` (z3 timeout / undecidable) must be reported as unproven.
#[test]
fn test_parse_smt_output_unknown_is_unproven() {
    let verdict = dwarf_lib::gungnir::parse_smt_output("unknown");
    assert!(
        matches!(
            verdict,
            dwarf_lib::gungnir::Verdict::Unproven { .. }
        ),
        "unknown must map to Unproven (timeout handled), got {verdict:?}"
    );
    assert_eq!(verdict.label(), "unproven");
}

/// Empty or unrecognizable output maps to Error, never a panic.
#[test]
fn test_parse_smt_output_garbage_is_error() {
    let verdict = dwarf_lib::gungnir::parse_smt_output("");
    assert!(
        matches!(verdict, dwarf_lib::gungnir::Verdict::Error { .. }),
        "empty output must map to Error, got {verdict:?}"
    );
    assert_eq!(verdict.label(), "error");
}
