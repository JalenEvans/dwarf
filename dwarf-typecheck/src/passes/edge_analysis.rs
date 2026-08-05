//! Edge analysis pass — verifies test coverage of edge cases.
//!
//! This pass analyzes functions with `@covers` decorators and checks that
//! all expected edge cases for each parameter type are covered by tests.
//!
//! ## Edge Categories by Type
//!
//! For `Int` parameters: zero, positive, negative, max, min
//! For `Float` parameters: zero, positive, negative, max, min, nan, infinity
//! For `Bool` parameters: true, false
//! For `Str` parameters: empty, non_empty, max_length
//!
//! ## Warnings
//!
//! - `DWARF-W-EDGE`: Missing edge coverage (warning, not error)
//! - `DWARF-W-GUNGNIR`: Function marked `@gungnir` but not verified (warning)
//!
//! ## Configuration
//!
//! The pass respects `CompileOptions::edge_check` to enable/disable analysis.

use dwarf_syntax::hir::{Decl, Decorator, Type};
use dwarf_syntax::span::Span;

/// Edge category for a parameter type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeCategory {
    Zero,
    Positive,
    Negative,
    Max,
    Min,
    Nan,
    Infinity,
    True,
    False,
    Empty,
    NonEmpty,
    MaxLength,
}

impl EdgeCategory {
    /// Generate expected edge categories for a given type.
    pub fn for_type(ty: &Type) -> Vec<EdgeCategory> {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" | "int" => vec![
                    EdgeCategory::Zero,
                    EdgeCategory::Positive,
                    EdgeCategory::Negative,
                    EdgeCategory::Max,
                    EdgeCategory::Min,
                ],
                "Float" | "float" => vec![
                    EdgeCategory::Zero,
                    EdgeCategory::Positive,
                    EdgeCategory::Negative,
                    EdgeCategory::Max,
                    EdgeCategory::Min,
                    EdgeCategory::Nan,
                    EdgeCategory::Infinity,
                ],
                "Bool" | "bool" => vec![EdgeCategory::True, EdgeCategory::False],
                "Str" | "str" | "String" | "string" => {
                    vec![
                        EdgeCategory::Empty,
                        EdgeCategory::NonEmpty,
                        EdgeCategory::MaxLength,
                    ]
                }
                _ => vec![],
            },
            _ => vec![],
        }
    }

    /// Convert edge value string to EdgeCategory.
    pub fn parse(s: &str) -> Option<EdgeCategory> {
        match s.to_lowercase().as_str() {
            "zero" => Some(EdgeCategory::Zero),
            "positive" => Some(EdgeCategory::Positive),
            "negative" => Some(EdgeCategory::Negative),
            "max" => Some(EdgeCategory::Max),
            "min" => Some(EdgeCategory::Min),
            "nan" => Some(EdgeCategory::Nan),
            "infinity" => Some(EdgeCategory::Infinity),
            "true" => Some(EdgeCategory::True),
            "false" => Some(EdgeCategory::False),
            "empty" => Some(EdgeCategory::Empty),
            "non_empty" | "nonempty" => Some(EdgeCategory::NonEmpty),
            "max_length" | "maxlength" => Some(EdgeCategory::MaxLength),
            _ => None,
        }
    }
}

/// An edge coverage warning.
#[derive(Debug, Clone)]
pub struct EdgeWarning {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

/// Configuration for edge analysis.
#[derive(Debug, Clone, Default)]
pub struct EdgeAnalysisConfig {
    /// Whether edge checking is enabled.
    pub enabled: bool,
}

/// The edge analysis pass.
pub struct EdgeAnalysisPass {
    config: EdgeAnalysisConfig,
}

impl Default for EdgeAnalysisPass {
    fn default() -> Self {
        Self {
            config: EdgeAnalysisConfig { enabled: true },
        }
    }
}

impl EdgeAnalysisPass {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: EdgeAnalysisConfig) -> Self {
        Self { config }
    }

    /// Run edge analysis on declarations.
    ///
    /// Returns a list of warnings for missing edge coverage or unverified @gungnir.
    pub fn analyze(&self, decls: &[Decl]) -> Vec<EdgeWarning> {
        if !self.config.enabled {
            return vec![];
        }

        let mut warnings = Vec::new();

        // Collect all @covers decorators from both:
        // 1. Decl::Decorator wrapper nodes (legacy format)
        // 2. Function's own decorators list (new format)
        let mut covers: Vec<(String, String, String)> = Vec::new();

        // From Decl::Decorator wrappers
        for d in decls {
            if let Decl::Decorator { name, target, .. } = d {
                if name == "covers" {
                    if let Decl::Function { decorators, .. } = target.as_ref() {
                        for dec in decorators {
                            if let Decorator::Covers {
                                fn_name,
                                param,
                                edge_value,
                            } = dec
                            {
                                covers.push((fn_name.clone(), param.clone(), edge_value.clone()));
                            }
                        }
                    }
                }
            }
        }

        // From Function's own decorators list
        for d in decls {
            if let Decl::Function {
                name: _,
                decorators,
                ..
            } = d
            {
                for dec in decorators {
                    if let Decorator::Covers {
                        fn_name,
                        param,
                        edge_value,
                    } = dec
                    {
                        covers.push((fn_name.clone(), param.clone(), edge_value.clone()));
                    }
                }
            }
        }

        // Check each function for edge coverage
        for decl in decls {
            if let Decl::Function {
                name,
                params,
                decorators,
                span,
                ..
            } = decl
            {
                // Check for @gungnir
                let has_gungnir = decorators.iter().any(|d| matches!(d, Decorator::Gungnir));
                if has_gungnir {
                    // TODO: Check if function is verified
                    // For now, emit warning for any @gungnir function
                    warnings.push(EdgeWarning {
                        code: "DWARF-W-GUNGNIR",
                        message: format!(
                            "Function '{}' is marked @gungnir but verification status is unknown",
                            name
                        ),
                        span: *span,
                    });
                }

                // Only functions that declare edge expectations via @covers
                // are required to cover edges. Functions without any @covers
                // referencing them opt out of edge checking — otherwise every
                // function with a typed parameter would emit missing-edge
                // warnings even though the module never asked for edge coverage.
                let has_covers_for_fn = covers.iter().any(|(fn_name, _, _)| fn_name == name);
                if !has_covers_for_fn {
                    continue;
                }

                // Check edge coverage for each parameter
                for param in params {
                    if let Some(ref ty) = param.type_ {
                        let expected_edges = EdgeCategory::for_type(ty);

                        // Find which edges are covered by @covers decorators
                        let covered_edges: Vec<EdgeCategory> = covers
                            .iter()
                            .filter(|(fn_name, p, _)| fn_name == name && p == &param.name)
                            .filter_map(|(_, _, edge_val)| EdgeCategory::parse(edge_val))
                            .collect();

                        // Find missing edges
                        for expected in &expected_edges {
                            if !covered_edges.contains(expected) {
                                warnings.push(EdgeWarning {
                                    code: "DWARF-W-EDGE",
                                    message: format!(
                                        "Function '{}' parameter '{}' is missing edge coverage for {:?}",
                                        name, param.name, expected
                                    ),
                                    span: *span,
                                });
                            }
                        }
                    }
                }
            }
        }

        warnings
    }
}

// ---------------------------------------------------------------------------
// Tests — RED PHASE
//
// These tests verify the edge analysis pass behavior. They will FAIL until
// the pass is fully implemented and integrated with the typechecker.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::*;
    use dwarf_syntax::span::Span;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    // ------------------------------------------------------------------
    // Test 1: Function with edge categories generated from param types
    // ------------------------------------------------------------------

    #[test]
    /// Given `fn divide(a: Int, b: Int) -> Int`, the edge analyser should
    /// generate expected edges for Int params: zero, positive, negative, max, min.
    ///
    /// This test verifies that edge categories are derived from parameter types.
    fn test_edge_categories_generated_from_int_params() {
        let int_type = Type::Named("Int".to_string());
        let edges = EdgeCategory::for_type(&int_type);

        assert!(
            edges.contains(&EdgeCategory::Zero),
            "Int should have Zero edge"
        );
        assert!(
            edges.contains(&EdgeCategory::Positive),
            "Int should have Positive edge"
        );
        assert!(
            edges.contains(&EdgeCategory::Negative),
            "Int should have Negative edge"
        );
        assert!(
            edges.contains(&EdgeCategory::Max),
            "Int should have Max edge"
        );
        assert!(
            edges.contains(&EdgeCategory::Min),
            "Int should have Min edge"
        );
        assert_eq!(edges.len(), 5, "Int should have exactly 5 edge categories");
    }

    #[test]
    /// Float params should have additional edges: nan, infinity.
    fn test_edge_categories_generated_from_float_params() {
        let float_type = Type::Named("Float".to_string());
        let edges = EdgeCategory::for_type(&float_type);

        assert!(edges.contains(&EdgeCategory::Nan));
        assert!(edges.contains(&EdgeCategory::Infinity));
        assert_eq!(edges.len(), 7, "Float should have 7 edge categories");
    }

    #[test]
    /// Bool params should have true/false edges.
    fn test_edge_categories_generated_from_bool_params() {
        let bool_type = Type::Named("Bool".to_string());
        let edges = EdgeCategory::for_type(&bool_type);

        assert!(edges.contains(&EdgeCategory::True));
        assert!(edges.contains(&EdgeCategory::False));
        assert_eq!(edges.len(), 2, "Bool should have 2 edge categories");
    }

    // ------------------------------------------------------------------
    // Test 2: Missing edge coverage → DWARF-W-EDGE warning
    // ------------------------------------------------------------------

    #[test]
    /// If @covers covers "positive" but not "zero" for a required edge,
    /// emit DWARF-W-EDGE warning.
    fn test_missing_edge_coverage_emits_warning() {
        let pass = EdgeAnalysisPass::new();

        // Function: fn divide(a: Int, b: Int) -> Int
        let func = Decl::Function {
            name: "divide".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    type_: Some(Type::Named("Int".to_string())),
                },
                Param {
                    name: "b".to_string(),
                    type_: Some(Type::Named("Int".to_string())),
                },
            ],
            return_type: Some(Type::Named("Int".to_string())),
            body: Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            },
            is_pub: true,
            decorators: vec![
                // Only covers "positive" for param "a", missing other edges
                Decorator::Covers {
                    fn_name: "divide".to_string(),
                    param: "a".to_string(),
                    edge_value: "positive".to_string(),
                },
            ],
            span: dummy_span(),
        };

        let warnings = pass.analyze(&[func]);

        // Should have warnings for missing edges
        let edge_warnings: Vec<_> = warnings
            .iter()
            .filter(|w| w.code == "DWARF-W-EDGE")
            .collect();

        assert!(
            !edge_warnings.is_empty(),
            "Should emit DWARF-W-EDGE for missing edge coverage"
        );

        // Should mention the missing edges
        let has_zero_warning = edge_warnings
            .iter()
            .any(|w| w.message.contains("Zero") || w.message.contains("zero"));
        assert!(
            has_zero_warning,
            "Should warn about missing Zero edge coverage"
        );
    }

    // ------------------------------------------------------------------
    // Test 3: All edges covered → no warning
    // ------------------------------------------------------------------

    #[test]
    /// When all generated edge cases are covered by @covers, no warning.
    fn test_all_edges_covered_no_warning() {
        let pass = EdgeAnalysisPass::new();

        // Function with all Int edges covered
        let func = Decl::Function {
            name: "safe_divide".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                type_: Some(Type::Named("Int".to_string())),
            }],
            return_type: Some(Type::Named("Int".to_string())),
            body: Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            },
            is_pub: true,
            decorators: vec![
                Decorator::Covers {
                    fn_name: "safe_divide".to_string(),
                    param: "x".to_string(),
                    edge_value: "zero".to_string(),
                },
                Decorator::Covers {
                    fn_name: "safe_divide".to_string(),
                    param: "x".to_string(),
                    edge_value: "positive".to_string(),
                },
                Decorator::Covers {
                    fn_name: "safe_divide".to_string(),
                    param: "x".to_string(),
                    edge_value: "negative".to_string(),
                },
                Decorator::Covers {
                    fn_name: "safe_divide".to_string(),
                    param: "x".to_string(),
                    edge_value: "max".to_string(),
                },
                Decorator::Covers {
                    fn_name: "safe_divide".to_string(),
                    param: "x".to_string(),
                    edge_value: "min".to_string(),
                },
            ],
            span: dummy_span(),
        };

        let warnings = pass.analyze(&[func]);

        let edge_warnings: Vec<_> = warnings
            .iter()
            .filter(|w| w.code == "DWARF-W-EDGE")
            .collect();

        assert!(
            edge_warnings.is_empty(),
            "Should not emit DWARF-W-EDGE when all edges are covered, got: {:?}",
            edge_warnings
        );
    }

    // ------------------------------------------------------------------
    // Test 4: edge_check=off → no warnings
    // ------------------------------------------------------------------

    #[test]
    /// When edge_check is Off, no edge warnings are emitted.
    fn test_edge_check_off_no_warnings() {
        let config = EdgeAnalysisConfig { enabled: false };
        let pass = EdgeAnalysisPass::with_config(config);

        // Function with missing edges but analysis disabled
        let func = Decl::Function {
            name: "unchecked".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                type_: Some(Type::Named("Int".to_string())),
            }],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub: true,
            decorators: vec![], // No @covers at all
            span: dummy_span(),
        };

        let warnings = pass.analyze(&[func]);

        assert!(
            warnings.is_empty(),
            "Should emit no warnings when edge_check is disabled, got: {:?}",
            warnings
        );
    }

    // ------------------------------------------------------------------
    // Test 5: @gungnir unverified → DWARF-W-GUNGNIR
    // ------------------------------------------------------------------

    #[test]
    /// A function with @gungnir that is not verified should emit
    /// DWARF-W-GUNGNIR warning (not error).
    fn test_gungnir_unverified_emits_warning() {
        let pass = EdgeAnalysisPass::new();

        let func = Decl::Function {
            name: "critical_fn".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub: true,
            decorators: vec![Decorator::Gungnir],
            span: dummy_span(),
        };

        let warnings = pass.analyze(&[func]);

        let gungnir_warnings: Vec<_> = warnings
            .iter()
            .filter(|w| w.code == "DWARF-W-GUNGNIR")
            .collect();

        assert!(
            !gungnir_warnings.is_empty(),
            "Should emit DWARF-W-GUNGNIR for unverified @gungnir function"
        );

        // Verify it's a warning, not an error (code starts with W, not E)
        assert!(
            gungnir_warnings[0].code.contains("-W-"),
            "DWARF-W-GUNGNIR should be a warning, not an error"
        );
    }
}
