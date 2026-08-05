//! Test coverage pass — verifies that functions have adequate test coverage.
//!
//! DWARF-117: This pass checks public functions for test coverage based on
//! decorators (@test, @tested, @skip_test) and naming conventions (test_add → add).

use dwarf_syntax::hir::{Decl, Decorator};
use std::collections::HashSet;

/// How strictly test coverage is enforced.
#[derive(Clone, Debug, PartialEq)]
pub enum CoverageMode {
    /// No coverage checks.
    Off,
    /// Emit warnings but build continues.
    Warning,
    /// Hard error, build fails.
    Required,
}

/// Which functions require test coverage.
#[derive(Clone, Debug, PartialEq)]
pub enum CoverageScope {
    /// Only public functions need coverage.
    AllPub,
    /// All functions except those with @skip_test.
    All,
    /// Only functions explicitly annotated with @tested.
    AnnotatedOnly,
}

/// Configuration for the test coverage pass.
#[derive(Clone, Debug)]
pub struct CoverageCheckConfig {
    pub mode: CoverageMode,
    pub scope: CoverageScope,
}

impl Default for CoverageCheckConfig {
    fn default() -> Self {
        Self {
            mode: CoverageMode::Required,
            scope: CoverageScope::AllPub,
        }
    }
}

/// A diagnostic produced by the test coverage pass.
#[derive(Debug, Clone)]
pub struct CoverageDiagnostic {
    /// Error code (e.g., "DWARF-E-COVER-0001").
    pub code: &'static str,
    /// Human-readable message.
    pub message: String,
    /// Name of the uncovered function.
    pub function_name: String,
}

/// The test coverage compilation pass.
///
/// Analyzes HIR declarations to verify that public functions have
/// corresponding test coverage via @test, @tested, @skip_test decorators,
/// or naming convention inference (test_add → add).
pub struct TestCoveragePass;

impl Default for TestCoveragePass {
    fn default() -> Self {
        Self
    }
}

impl TestCoveragePass {
    pub fn new() -> Self {
        Self
    }

    /// Run the test coverage check on a set of HIR declarations.
    ///
    /// Returns a list of coverage diagnostics for functions that lack
    /// adequate test coverage according to the provided config.
    pub fn check(&self, decls: &[Decl], config: &CoverageCheckConfig) -> Vec<CoverageDiagnostic> {
        // If coverage checking is off, nothing to do
        if config.mode == CoverageMode::Off {
            return Vec::new();
        }

        // Step 1: Collect the set of function names that are covered by tests.
        // A function is covered if:
        //   - It has @test decorator itself (it IS a test)
        //   - Another function has @tested(fn_name) pointing to it
        //   - Another @test function's name infers coverage (test_X → X)
        let mut covered_names: HashSet<String> = HashSet::new();

        // Collect names explicitly targeted by @tested decorators
        for decl in decls {
            if let Some(target) = get_tested_target(decl) {
                covered_names.insert(target);
            }
        }

        // Collect names inferred from @test function naming conventions
        for decl in decls {
            if has_test(decl) {
                if let Some(name) = get_fn_name(decl) {
                    // A @test function covers itself
                    covered_names.insert(name.clone());
                    // Also infer coverage via naming: test_X → X
                    for other_decl in decls {
                        if let Some(other_name) = get_fn_name(other_decl) {
                            if infer_coverage_by_naming(&name, &other_name) {
                                covered_names.insert(other_name);
                            }
                        }
                    }
                }
            }
        }

        // Step 2: Determine which functions need coverage based on scope
        let mut diagnostics = Vec::new();

        // Collect the set of function names explicitly named by @tested
        // decorators. In AnnotatedOnly scope, only these functions require
        // coverage — not the test functions that *carry* the @tested
        // decorator. For example `@tested(add) fn test_add() { ... }` means
        // `add` needs coverage, while `test_add` is the test providing it
        // (see DWARF-117: "Inference matches test_add to fn add without
        // @tested"). `@tested(add)` on any declaration counts, including on
        // `add` itself.
        let tested_targets: HashSet<String> = decls.iter().filter_map(get_tested_target).collect();

        for decl in decls {
            let Some(fn_name) = get_fn_name(decl) else {
                continue;
            };

            let needs_coverage = match config.scope {
                CoverageScope::AllPub => is_pub_fn(decl),
                CoverageScope::All => is_fn(decl),
                CoverageScope::AnnotatedOnly => is_fn(decl) && tested_targets.contains(&fn_name),
            };

            if !needs_coverage {
                continue;
            }

            // Functions with @skip_test are exempt
            if has_skip_test(decl) {
                continue;
            }

            // Check if this function is covered
            if !covered_names.contains(&fn_name) {
                diagnostics.push(CoverageDiagnostic {
                    code: "DWARF-E-COVER-0001",
                    message: format!(
                        "Function '{}' has no test coverage. \
                         Add a @test function, use @tested({}) on a test, \
                         or name a test function test_{}.",
                        fn_name, fn_name, fn_name
                    ),
                    function_name: fn_name,
                });
            }
        }

        diagnostics
    }
}

/// Check if a function name follows the test naming convention for a target.
/// e.g., "test_add" covers "add", "test_subtract" covers "subtract".
pub fn infer_coverage_by_naming(test_fn_name: &str, target_fn_name: &str) -> bool {
    match test_fn_name.strip_prefix("test_") {
        Some(stripped) => stripped == target_fn_name,
        None => false,
    }
}

/// Check if a declaration has a specific decorator.
pub fn has_decorator(decl: &Decl, decorator_check: impl Fn(&Decorator) -> bool) -> bool {
    match decl {
        Decl::Function { decorators, .. } => decorators.iter().any(decorator_check),
        _ => false,
    }
}

/// Check if a function has @skip_test decorator.
pub fn has_skip_test(decl: &Decl) -> bool {
    has_decorator(decl, |d| matches!(d, Decorator::SkipTest { .. }))
}

/// Check if a function has @test decorator.
pub fn has_test(decl: &Decl) -> bool {
    has_decorator(decl, |d| matches!(d, Decorator::Test))
}

/// Get the target function name from a @tested decorator, if present.
pub fn get_tested_target(decl: &Decl) -> Option<String> {
    match decl {
        Decl::Function { decorators, .. } => decorators.iter().find_map(|d| {
            if let Decorator::Tested { fn_name } = d {
                Some(fn_name.clone())
            } else {
                None
            }
        }),
        _ => None,
    }
}

/// Get the name of a function declaration, if it is a function.
fn get_fn_name(decl: &Decl) -> Option<String> {
    match decl {
        Decl::Function { name, .. } => Some(name.clone()),
        _ => None,
    }
}

/// Check if a declaration is a public function.
fn is_pub_fn(decl: &Decl) -> bool {
    matches!(decl, Decl::Function { is_pub: true, .. })
}

/// Check if a declaration is a function (public or private).
fn is_fn(decl: &Decl) -> bool {
    matches!(decl, Decl::Function { .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::*;
    use dwarf_syntax::span::Span;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    fn func(name: &str, is_pub: bool, decorators: Vec<Decorator>) -> Decl {
        Decl::Function {
            name: name.to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub,
            decorators,
            span: dummy_span(),
        }
    }

    // ------------------------------------------------------------------
    // B4: CoverageScope::AnnotatedOnly must use the set of @tested targets,
    // not the declarations that carry the @tested decorator.
    // ------------------------------------------------------------------

    #[test]
    /// The function that carries `@tested(add)` is the *test*, not the target.
    /// It must NOT be flagged as needing coverage. Before the fix the carrier
    /// was flagged, producing a false positive DWARF-E-COVER for the test.
    fn annotated_only_does_not_flag_tested_carrier() {
        let config = CoverageCheckConfig {
            mode: CoverageMode::Required,
            scope: CoverageScope::AnnotatedOnly,
        };
        let pass = TestCoveragePass::new();

        let decls = vec![
            // Production function named by @tested.
            func("add", true, vec![]),
            // Test function that carries @tested(add).
            func(
                "tst",
                false,
                vec![Decorator::Tested {
                    fn_name: "add".to_string(),
                }],
            ),
        ];

        let diagnostics = pass.check(&decls, &config);

        // `add` is a @tested target so it is covered by `tst`; `tst` is the
        // carrier and must not require coverage. No false positives allowed.
        assert!(
            diagnostics.is_empty(),
            "AnnotatedOnly should not flag the @tested carrier or a covered target, got: {:?}",
            diagnostics
        );
    }

    #[test]
    /// A function with NO @tested annotation pointing at it must NOT require
    /// coverage under AnnotatedOnly, even if it is public and uncovered.
    fn annotated_only_ignores_unannotated_public_functions() {
        let config = CoverageCheckConfig {
            mode: CoverageMode::Required,
            scope: CoverageScope::AnnotatedOnly,
        };
        let pass = TestCoveragePass::new();

        let decls = vec![
            // Public, uncovered, but never named by @tested → exempt.
            func("helper", true, vec![]),
            // Test for a different function.
            func(
                "tst",
                false,
                vec![Decorator::Tested {
                    fn_name: "add".to_string(),
                }],
            ),
            // The named target (covered by the annotation).
            func("add", true, vec![]),
        ];

        let diagnostics = pass.check(&decls, &config);

        assert!(
            diagnostics.is_empty(),
            "AnnotatedOnly must only require coverage for @tested targets, got: {:?}",
            diagnostics
        );
    }

    #[test]
    /// A function named by someone's `@tested` annotation — even when the
    /// naming test doesn't follow the `test_X` convention — needs coverage.
    fn annotated_only_requires_annotation_based_coverage() {
        let config = CoverageCheckConfig {
            mode: CoverageMode::Required,
            scope: CoverageScope::AnnotatedOnly,
        };
        let pass = TestCoveragePass::new();

        // `validate` is named by @tested but the carrier is `checker`, so
        // naming-convention inference does not apply; without an explicit
        // @tested pointing at `validate`, it must still be considered covered
        // because @tested is the annotation that grants coverage.
        let decls = vec![
            func("validate", true, vec![]),
            func(
                "checker",
                false,
                vec![Decorator::Tested {
                    fn_name: "validate".to_string(),
                }],
            ),
        ];

        let diagnostics = pass.check(&decls, &config);

        assert!(
            diagnostics.is_empty(),
            "@tested(validate) should grant coverage to 'validate', got: {:?}",
            diagnostics
        );
    }

    // ------------------------------------------------------------------
    // AllPub scope still flags uncovered public functions.
    // ------------------------------------------------------------------

    #[test]
    fn all_pub_flags_uncovered_public_function() {
        let config = CoverageCheckConfig {
            mode: CoverageMode::Required,
            scope: CoverageScope::AllPub,
        };
        let pass = TestCoveragePass::new();

        let decls = vec![func("uncovered", true, vec![])];

        let diagnostics = pass.check(&decls, &config);
        assert_eq!(
            diagnostics.len(),
            1,
            "one uncovered pub fn should be flagged"
        );
        assert_eq!(diagnostics[0].function_name, "uncovered");
    }
}
