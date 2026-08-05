//! Test manifest collection — scans declarations for @test-annotated functions
//! and produces a serializable manifest of test metadata.
//!
//! DWARF-116: TestManifest struct and JSON emission.

use crate::hir::{Decl, Decorator};
use serde::{Deserialize, Serialize};

/// A single entry in the test manifest, representing one @test-annotated function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestEntry {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    /// Names of all decorators on this function (e.g. ["test", "skip"]).
    pub decorators: Vec<String>,
    /// Extracted @covers metadata entries.
    pub covers: Vec<CoverEntry>,
}

/// A single @covers metadata entry extracted from a decorator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverEntry {
    pub fn_name: String,
    pub param: String,
    pub edge_value: String,
}

/// The full test manifest — a collection of test entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestManifest {
    pub tests: Vec<TestEntry>,
}

/// Collect all @test-annotated functions from a slice of declarations and
/// produce a [`TestManifest`] with their metadata.
///
/// Iterates over declarations, filters for `Decl::Function` nodes carrying
/// a `Decorator::Test`, extracts `@covers` metadata, and builds a
/// serializable [`TestManifest`].
pub fn collect_test_manifest(decls: &[Decl]) -> TestManifest {
    let mut tests = Vec::new();

    for decl in decls {
        if let Decl::Function {
            name, decorators, ..
        } = decl
        {
            // Only include functions with @test decorator
            let has_test = decorators.iter().any(|d| matches!(d, Decorator::Test));
            if !has_test {
                continue;
            }

            // Collect @covers metadata
            let covers: Vec<CoverEntry> = decorators
                .iter()
                .filter_map(|d| {
                    if let Decorator::Covers {
                        fn_name,
                        param,
                        edge_value,
                    } = d
                    {
                        Some(CoverEntry {
                            fn_name: fn_name.clone(),
                            param: param.clone(),
                            edge_value: edge_value.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Collect decorator names for reporting
            let decorator_names: Vec<String> = decorators.iter().map(decorator_name).collect();

            tests.push(TestEntry {
                function_name: name.clone(),
                file: "unknown".to_string(), // span info available but keeping simple
                line: 0,
                decorators: decorator_names,
                covers,
            });
        }
    }

    TestManifest { tests }
}

/// Convert a [`Decorator`] to its string name for the manifest.
fn decorator_name(d: &Decorator) -> String {
    match d {
        Decorator::Test => "test".to_string(),
        Decorator::BeforeEach => "before_each".to_string(),
        Decorator::AfterEach => "after_each".to_string(),
        Decorator::Skip => "skip".to_string(),
        Decorator::Gungnir => "gungnir".to_string(),
        Decorator::Covers { .. } => "covers".to_string(),
        Decorator::Tested { .. } => "tested".to_string(),
        Decorator::SkipTest { .. } => "skip_test".to_string(),
        Decorator::Requires { .. } => "requires".to_string(),
        Decorator::Ensures { .. } => "ensures".to_string(),
        Decorator::Invariant { .. } => "invariant".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Expr;
    use crate::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Build a minimal Decl::Function with the given name and decorators.
    fn make_test_fn(name: &str, decorators: Vec<Decorator>) -> Decl {
        Decl::Function {
            name: name.to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Block {
                stmts: vec![],
                span: Span::new(0, 0, 0),
            },
            is_pub: false,
            decorators,
            span: Span::new(0, 10, 50),
        }
    }

    /// Build a non-test function (no @test decorator).
    fn make_plain_fn(name: &str) -> Decl {
        make_test_fn(name, vec![])
    }

    // ==================================================================
    // Test 1: Empty manifest
    //
    // collect_test_manifest(&[]) should return a manifest with no entries.
    // ==================================================================

    #[test]
    fn test_empty_manifest() {
        let manifest = collect_test_manifest(&[]);
        assert!(
            manifest.tests.is_empty(),
            "empty input should produce empty manifest, got {} entries",
            manifest.tests.len()
        );
    }

    // ==================================================================
    // Test 2: Single @test function
    //
    // A function with decorators: [Decorator::Test] should produce a
    // single TestEntry with function_name and decorators: ["test"].
    // ==================================================================

    #[test]
    fn test_single_test_function() {
        let decls = vec![make_test_fn("test_addition", vec![Decorator::Test])];
        let manifest = collect_test_manifest(&decls);

        assert_eq!(
            manifest.tests.len(),
            1,
            "expected exactly 1 test entry, got {}",
            manifest.tests.len()
        );

        let entry = &manifest.tests[0];
        assert_eq!(entry.function_name, "test_addition");
        assert_eq!(entry.decorators, vec!["test".to_string()]);
    }

    // ==================================================================
    // Test 3: @test with @covers metadata
    //
    // A function with @test and @covers(divide, b, zero) should produce
    // a TestEntry with covers: [CoverEntry { fn_name: "divide", ... }].
    // ==================================================================

    #[test]
    fn test_with_covers_metadata() {
        let decls = vec![make_test_fn(
            "test_divide_by_zero",
            vec![
                Decorator::Test,
                Decorator::Covers {
                    fn_name: "divide".to_string(),
                    param: "b".to_string(),
                    edge_value: "zero".to_string(),
                },
            ],
        )];
        let manifest = collect_test_manifest(&decls);

        assert_eq!(manifest.tests.len(), 1);

        let entry = &manifest.tests[0];
        assert_eq!(entry.function_name, "test_divide_by_zero");
        assert_eq!(entry.decorators.len(), 2);
        assert!(entry.decorators.contains(&"test".to_string()));
        assert!(entry.decorators.contains(&"covers".to_string()));

        assert_eq!(
            entry.covers.len(),
            1,
            "expected 1 covers entry, got {}",
            entry.covers.len()
        );
        assert_eq!(entry.covers[0].fn_name, "divide");
        assert_eq!(entry.covers[0].param, "b");
        assert_eq!(entry.covers[0].edge_value, "zero");
    }

    // ==================================================================
    // Test 4: Multiple @test functions
    //
    // Multiple functions with @test should produce multiple entries.
    // ==================================================================

    #[test]
    fn test_multiple_test_functions() {
        let decls = vec![
            make_test_fn("test_add", vec![Decorator::Test]),
            make_plain_fn("helper"),
            make_test_fn("test_sub", vec![Decorator::Test]),
            make_test_fn("test_mul", vec![Decorator::Test]),
        ];
        let manifest = collect_test_manifest(&decls);

        assert_eq!(
            manifest.tests.len(),
            3,
            "expected 3 test entries (non-test 'helper' excluded), got {}",
            manifest.tests.len()
        );

        let names: Vec<&str> = manifest
            .tests
            .iter()
            .map(|e| e.function_name.as_str())
            .collect();
        assert_eq!(names, vec!["test_add", "test_sub", "test_mul"]);
    }

    // ==================================================================
    // Test 5: Non-test functions ignored
    //
    // A function without @test should not appear in the manifest.
    // ==================================================================

    #[test]
    fn test_non_test_functions_ignored() {
        let decls = vec![
            make_plain_fn("regular_function"),
            make_test_fn("also_not_a_test", vec![Decorator::BeforeEach]),
            make_test_fn("setup_fn", vec![Decorator::AfterEach]),
        ];
        let manifest = collect_test_manifest(&decls);

        assert!(
            manifest.tests.is_empty(),
            "no @test functions present, manifest should be empty, got {} entries",
            manifest.tests.len()
        );
    }

    // ==================================================================
    // Test 6: @skip function included but marked
    //
    // A function with @test and @skip should appear in the manifest.
    // Skipped tests are collected but skipped at runtime, not at
    // manifest collection time.
    // ==================================================================

    #[test]
    fn test_skip_function_included() {
        let decls = vec![make_test_fn(
            "test_pending_feature",
            vec![Decorator::Test, Decorator::Skip],
        )];
        let manifest = collect_test_manifest(&decls);

        assert_eq!(
            manifest.tests.len(),
            1,
            "@skip @test function should still appear in manifest"
        );

        let entry = &manifest.tests[0];
        assert_eq!(entry.function_name, "test_pending_feature");
        assert!(
            entry.decorators.contains(&"test".to_string()),
            "decorators should include 'test'"
        );
        assert!(
            entry.decorators.contains(&"skip".to_string()),
            "decorators should include 'skip'"
        );
    }

    // ==================================================================
    // Test 7: JSON serialization
    //
    // TestManifest should serialize to valid JSON with serde_json and
    // survive a roundtrip (serialize → deserialize → compare).
    // ==================================================================

    #[test]
    fn test_json_serialization_roundtrip() {
        let manifest = TestManifest {
            tests: vec![
                TestEntry {
                    function_name: "test_add".to_string(),
                    file: "math.dwarf".to_string(),
                    line: 10,
                    decorators: vec!["test".to_string()],
                    covers: vec![],
                },
                TestEntry {
                    function_name: "test_divide".to_string(),
                    file: "math.dwarf".to_string(),
                    line: 25,
                    decorators: vec!["test".to_string(), "covers".to_string()],
                    covers: vec![CoverEntry {
                        fn_name: "divide".to_string(),
                        param: "b".to_string(),
                        edge_value: "zero".to_string(),
                    }],
                },
            ],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&manifest).expect("serialize TestManifest to JSON");

        // Verify it's valid JSON by deserializing back
        let deserialized: TestManifest =
            serde_json::from_str(&json).expect("deserialize TestManifest from JSON");

        // Roundtrip should preserve all data
        assert_eq!(manifest, deserialized);

        // Verify JSON contains expected keys
        assert!(json.contains("\"function_name\""));
        assert!(json.contains("\"test_add\""));
        assert!(json.contains("\"covers\""));
        assert!(json.contains("\"edge_value\""));
    }
}
