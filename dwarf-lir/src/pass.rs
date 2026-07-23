//! LirPass — the LIR lowering and effect resolution compilation pass.
//!
//! This pass takes MIR declarations, lowers them to LIR, builds a call
//! graph, resolves effects (pure/async/impure) through the graph, and
//! attaches the resolved effects to LIR function declarations.

use std::collections::HashMap;
use dwarf_mir::MirDecl;
use crate::effects::{build_call_graph, resolve_effects};
use crate::lower::lower_to_lir;

/// The LIR lowering and effect resolution pass.
///
/// Orchestrates MIR→LIR lowering, call graph construction, effect
/// propagation, and attaches resolved effects to LIR function decls.
pub struct LirPass;

impl LirPass {
    pub fn new() -> Self {
        Self
    }

    /// Run the full LIR pipeline on a slice of MIR declarations.
    ///
    /// Returns a `Vec<LirDecl>` with resolved effects attached to
    /// function declarations (the default `Effect::Pure` from lowering
    /// is overwritten when the call-graph analysis yields a stronger
    /// effect classification).
    pub fn run(&self, mir_decls: &[MirDecl]) -> Vec<crate::LirDecl> {
        // Step 1: Lower MIR to LIR
        let mut lir_decls = lower_to_lir(mir_decls);

        // Step 2: Build call graph from MIR
        let callgraph = build_call_graph(mir_decls);

        // Step 3: Resolve effects through the call graph
        let resolved = resolve_effects(mir_decls, &callgraph, &HashMap::new());

        // Step 4: Apply resolved effects to LIR function declarations
        for decl in &mut lir_decls {
            if let crate::LirDecl::Function {
                ref name,
                ref mut effect,
                ..
            } = decl
            {
                if let Some(resolved_effect) = resolved.get(name) {
                    *effect = resolved_effect.clone();
                }
            }
        }

        lir_decls
    }
}

impl Default for LirPass {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_mir::{MirDecl, MirExpr, MirLiteral};
    use dwarf_syntax::span::Span;
    use crate::{Effect, LirDecl, TargetHint};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn span1() -> Span {
        Span::new(0, 0, 0)
    }

    fn make_func(name: &str, body: MirExpr) -> MirDecl {
        MirDecl::Function {
            name: name.into(),
            params: vec![],
            return_type: None,
            body,
            is_pub: true,
            span: span1(),
        }
    }

    fn make_call(target: &str) -> MirExpr {
        MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: target.into(),
                span: span1(),
            }),
            args: vec![],
            span: span1(),
        }
    }

    fn make_literal(val: i64) -> MirExpr {
        MirExpr::Literal {
            value: MirLiteral::Int(val),
            span: span1(),
        }
    }

    // ------------------------------------------------------------------
    // Basic pass execution
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_pass_empty_input() {
        let pass = LirPass::new();
        let result = pass.run(&[]);
        assert!(result.is_empty(), "empty input should produce empty output");
    }

    #[test]
    fn test_lir_pass_single_function() {
        let pass = LirPass::new();
        let decls = vec![make_func("main", make_literal(42))];
        let result = pass.run(&decls);

        assert_eq!(result.len(), 1);
        match &result[0] {
            LirDecl::Function {
                name,
                effect,
                hint,
                ..
            } => {
                assert_eq!(name, "main");
                assert_eq!(*effect, Effect::Pure);
                assert_eq!(*hint, TargetHint::None);
            }
            other => panic!("expected Function variant, got {other:?}"),
        }
    }

    #[test]
    fn test_lir_pass_skips_type_defs() {
        let pass = LirPass::new();
        let decls = vec![
            MirDecl::TypeDef {
                name: "MyInt".into(),
                type_: dwarf_syntax::hir::Type::Named("Int".into()),
                is_pub: true,
                span: span1(),
            },
            make_func("main", make_literal(1)),
        ];
        let result = pass.run(&decls);

        assert_eq!(result.len(), 1, "TypeDef should be skipped");
        match &result[0] {
            LirDecl::Function { name, .. } => {
                assert_eq!(name, "main");
            }
            other => panic!("expected Function variant, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Effect resolution through call graph
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_pass_effect_propagation() {
        // a() calls b(), b() calls c().
        // Seed c as async via initial_effects replacement in resolve_effects.
        // Since resolve_effects starts from initial_effects (= Pure for all),
        // we need a way to mark c as async. The resolve_effects function
        // runs a fixpoint that propagates stronger effects — but without
        // seeds everything stays Pure.
        //
        // This test verifies that when all functions are Pure the pipeline
        // still runs without error and produces correct output shapes.
        let pass = LirPass::new();
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_call("c")),
            make_func("c", make_literal(1)),
        ];
        let result = pass.run(&decls);

        assert_eq!(result.len(), 3);
        for decl in &result {
            match decl {
                LirDecl::Function { name, effect, .. } => {
                    assert_eq!(
                        *effect,
                        Effect::Pure,
                        "all functions remain Pure without async seeds: {name}"
                    );
                }
                _ => panic!("expected Function variant"),
            }
        }
    }

    // ------------------------------------------------------------------
    // Record and union defs pass through unchanged
    // ------------------------------------------------------------------

    #[test]
    fn test_lir_pass_record_union_preserved() {
        use dwarf_mir::MirField;

        let pass = LirPass::new();
        let decls = vec![
            make_func("f", make_literal(0)),
            MirDecl::RecordDef {
                name: "Point".into(),
                fields: vec![
                    MirField {
                        name: "x".into(),
                        type_: dwarf_syntax::hir::Type::Named("Int".into()),
                    },
                    MirField {
                        name: "y".into(),
                        type_: dwarf_syntax::hir::Type::Named("Int".into()),
                    },
                ],
                is_pub: true,
                span: span1(),
            },
            MirDecl::UnionDef {
                name: "Option".into(),
                variants: vec![dwarf_mir::MirVariant {
                    name: "Some".into(),
                    arg: Some(dwarf_syntax::hir::Type::Named("Int".into())),
                }],
                is_pub: true,
                span: span1(),
            },
        ];
        let result = pass.run(&decls);

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_lir_pass_new_and_default() {
        let _a = LirPass::new();
        let _b = LirPass::default();
        // Both constructors produce the same unit struct; we just verify
        // they don't panic and are usable.
    }
}
