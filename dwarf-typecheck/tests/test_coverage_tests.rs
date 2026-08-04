//! Tests for the test coverage pass (DWARF-117).
//!
//! RED PHASE: These tests verify that the TestCoveragePass correctly detects
//! untested public functions and respects decorators (@test, @tested, @skip_test)
//! and naming convention inference (test_add → add).
//!
//! All tests WILL FAIL because the stub `check()` method returns an empty Vec.

use dwarf_syntax::hir::*;
use dwarf_syntax::span::Span;
use dwarf_typecheck::passes::test_coverage::{
    CoverageCheckConfig, CoverageDiagnostic, CoverageMode, CoverageScope, TestCoveragePass,
};

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn dummy_span() -> Span {
    Span::new(0, 0, 0)
}

/// Build a minimal public function with the given name and decorators.
fn make_pub_fn(name: &str, decorators: Vec<Decorator>) -> Decl {
    Decl::Function {
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body: Expr::Literal {
            value: LiteralValue::Int(42),
            span: dummy_span(),
        },
        is_pub: true,
        decorators,
        span: dummy_span(),
    }
}

/// Build a minimal private function with the given name and decorators.
fn make_priv_fn(name: &str, decorators: Vec<Decorator>) -> Decl {
    Decl::Function {
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body: Expr::Literal {
            value: LiteralValue::Int(0),
            span: dummy_span(),
        },
        is_pub: false,
        decorators,
        span: dummy_span(),
    }
}

fn required_config() -> CoverageCheckConfig {
    CoverageCheckConfig {
        mode: CoverageMode::Required,
        scope: CoverageScope::AllPub,
    }
}

/// Check if any diagnostic references a specific function name.
fn has_diagnostic_for(diagnostics: &[CoverageDiagnostic], fn_name: &str) -> bool {
    diagnostics.iter().any(|d| d.function_name == fn_name)
}

// ==================================================================
// Test 3: Untested public function → error
//
// A public function without @test or @tested should produce a
// DWARF-E-COVER error when mode=Required and scope=AllPub.
//
// Failure mode: The stub check() returns an empty Vec, so
// diagnostics.is_empty() is true but we assert !is_empty().
// ==================================================================

#[test]
fn test_untested_public_function_produces_error() {
    let pass = TestCoveragePass::new();
    let config = required_config();

    // A public function with no test-related decorators
    let decls = vec![make_pub_fn("calculate_total", vec![])];

    let diagnostics = pass.check(&decls, &config);

    assert!(
        !diagnostics.is_empty(),
        "Untested public function 'calculate_total' should produce a coverage diagnostic, \
         but got none — the coverage pass is likely a no-op stub"
    );

    assert!(
        has_diagnostic_for(&diagnostics, "calculate_total"),
        "Coverage diagnostic should reference 'calculate_total', got: {:?}",
        diagnostics.iter().map(|d| &d.function_name).collect::<Vec<_>>()
    );

    // The error code should be DWARF-E-COVER-*
    let has_cover_code = diagnostics
        .iter()
        .any(|d| d.code.starts_with("DWARF-E-COVER"));
    assert!(
        has_cover_code,
        "Expected a DWARF-E-COVER error code, got: {:?}",
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
    );
}

// ==================================================================
// Test 4: Tested function with @test → passes
//
// A function with @test decorator should pass coverage check
// (no coverage error for that function).
//
// Failure mode: The stub returns empty diagnostics, so this test
// technically passes vacuously. But we also include an untested
// function to verify the pass distinguishes tested from untested.
// ==================================================================

#[test]
fn test_function_with_test_decorator_passes_coverage() {
    let pass = TestCoveragePass::new();
    let config = required_config();

    let decls = vec![
        // This function has @test — should NOT produce a coverage error
        make_pub_fn("test_addition", vec![Decorator::Test]),
        // This function has no test — SHOULD produce a coverage error
        make_pub_fn("untested_fn", vec![]),
    ];

    let diagnostics = pass.check(&decls, &config);

    // test_addition should NOT be in diagnostics (it has @test)
    assert!(
        !has_diagnostic_for(&diagnostics, "test_addition"),
        "Function with @test decorator should NOT produce coverage error, \
         but found diagnostic for 'test_addition': {:?}",
        diagnostics
    );

    // untested_fn SHOULD be in diagnostics
    assert!(
        has_diagnostic_for(&diagnostics, "untested_fn"),
        "Untested public function 'untested_fn' should produce coverage error, \
         but got none — the pass is likely a no-op stub"
    );
}

// ==================================================================
// Test 5: @skip_test → no error
//
// A function annotated with @skip_test("reason") should not trigger
// a coverage error, even though it has no @test.
//
// Failure mode: The stub returns empty diagnostics, so the @skip_test
// function has no error (vacuously passes). But the untested function
// also has no error, which makes the second assertion fail.
// ==================================================================

#[test]
fn test_skip_test_decorator_suppresses_coverage_error() {
    let pass = TestCoveragePass::new();
    let config = required_config();

    let decls = vec![
        // This function has @skip_test — should NOT produce a coverage error
        make_pub_fn(
            "pending_feature",
            vec![Decorator::SkipTest {
                reason: "not implemented yet".to_string(),
            }],
        ),
        // This function has nothing — SHOULD produce a coverage error
        make_pub_fn("needs_test", vec![]),
    ];

    let diagnostics = pass.check(&decls, &config);

    // pending_feature should NOT be in diagnostics (it has @skip_test)
    assert!(
        !has_diagnostic_for(&diagnostics, "pending_feature"),
        "Function with @skip_test should NOT produce coverage error, \
         but found diagnostic for 'pending_feature': {:?}",
        diagnostics
    );

    // needs_test SHOULD be in diagnostics
    assert!(
        has_diagnostic_for(&diagnostics, "needs_test"),
        "Public function without @test or @skip_test should produce coverage error, \
         but got none — the pass is likely a no-op stub"
    );
}

// ==================================================================
// Test 6: @tested(explicit) → implicit coverage satisfied
//
// A function with @tested(myFunc) decorator should satisfy coverage
// for myFunc. The coverage pass should recognize that myFunc is
// covered by this test function.
//
// Failure mode: The stub returns empty diagnostics, so myFunc has no
// error (vacuously passes). But the truly untested function also has
// no error, making the second assertion fail.
// ==================================================================

#[test]
fn test_tested_decorator_satisfies_coverage_for_target() {
    let pass = TestCoveragePass::new();
    let config = required_config();

    let decls = vec![
        // Production function
        make_pub_fn("multiply", vec![]),
        // Test function with @tested(multiply) — covers "multiply"
        make_priv_fn(
            "verify_multiply",
            vec![Decorator::Tested {
                fn_name: "multiply".to_string(),
            }],
        ),
        // Another production function with NO coverage
        make_pub_fn("divide", vec![]),
    ];

    let diagnostics = pass.check(&decls, &config);

    // multiply should NOT be in diagnostics (covered by @tested(multiply))
    assert!(
        !has_diagnostic_for(&diagnostics, "multiply"),
        "Function 'multiply' should be covered by @tested(multiply) decorator, \
         but found coverage diagnostic for it: {:?}",
        diagnostics
    );

    // divide SHOULD be in diagnostics (no test covers it)
    assert!(
        has_diagnostic_for(&diagnostics, "divide"),
        "Untested public function 'divide' should produce coverage error, \
         but got none — the pass does not check @tested targets"
    );
}

// ==================================================================
// Test 7: Inference — test_add covers fn add
//
// By convention, a function named `test_add` should be inferred as
// testing `add` without an explicit @tested decorator.
//
// Failure mode: The stub returns empty diagnostics (no inference logic).
// The untested function also has no error, making the assertion fail.
// Additionally, infer_coverage_by_naming is a stub that always returns false.
// ==================================================================

#[test]
fn test_naming_convention_inference() {
    let pass = TestCoveragePass::new();
    let config = required_config();

    let decls = vec![
        // Production function
        make_pub_fn("add", vec![]),
        // Test function following naming convention — should cover "add"
        make_priv_fn("test_add", vec![Decorator::Test]),
        // Another production function with NO coverage
        make_pub_fn("subtract", vec![]),
    ];

    let diagnostics = pass.check(&decls, &config);

    // "add" should NOT be in diagnostics (inferred coverage from test_add)
    assert!(
        !has_diagnostic_for(&diagnostics, "add"),
        "Function 'add' should be covered by naming convention from 'test_add', \
         but found coverage diagnostic for it: {:?}",
        diagnostics
    );

    // "subtract" SHOULD be in diagnostics (no test covers it)
    assert!(
        has_diagnostic_for(&diagnostics, "subtract"),
        "Untested public function 'subtract' should produce coverage error, \
         but got none — the pass does not implement naming convention inference"
    );
}

#[test]
fn test_infer_coverage_by_naming_helper() {
    // Direct test of the naming inference helper
    use dwarf_typecheck::passes::test_coverage::infer_coverage_by_naming;

    assert!(
        infer_coverage_by_naming("test_add", "add"),
        "test_add should be inferred as covering add"
    );
    assert!(
        infer_coverage_by_naming("test_subtract", "subtract"),
        "test_subtract should be inferred as covering subtract"
    );
    assert!(
        !infer_coverage_by_naming("test_add", "subtract"),
        "test_add should NOT be inferred as covering subtract"
    );
    assert!(
        !infer_coverage_by_naming("helper_fn", "add"),
        "helper_fn should NOT be inferred as covering add (no test_ prefix)"
    );
}
