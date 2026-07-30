//! LIR Walker Architecture
//!
//! This crate provides the `LirBackend` trait and a shared tree walker engine
//! for traversing LIR (Low-level Intermediate Representation) trees. Backends
//! implement the trait to process LIR nodes without writing traversal code.

use std::fmt::Debug;

use dwarf_lir::{
    Effect, LirBinaryOp, LirField, LirLiteral, LirParam, LirUnaryOp, LirVariant, TargetHint,
};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error type returned by all backend hooks.
#[derive(Debug, Clone)]
pub struct BackendError {
    message: String,
}

impl BackendError {
    /// Create a new `BackendError` from any string-like value.
    pub fn msg(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

// ---------------------------------------------------------------------------
// ReducedArm — simplified match arm (already reduced by the walker)
// ---------------------------------------------------------------------------

/// A match arm after the walker has reduced its sub-expressions.
pub struct ReducedArm<R> {
    pub pattern: R,
    pub guard: Option<R>,
    pub body: R,
}

// ---------------------------------------------------------------------------
// LirBackend trait
// ---------------------------------------------------------------------------

/// Backend trait for processing LIR trees.
///
/// Each method corresponds to a LIR node category. The generic parameter `R`
/// is the reduction type — backends choose what each node reduces to (e.g.
/// `()` for side-effect-only backends, `String` for pretty-printers, an AST
/// node for code generators, etc.).
pub trait LirBackend<R: Debug> {
    // ------ Expression hooks (20) ------

    fn visit_expr_literal(
        &mut self,
        value: &LirLiteral,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_variable(
        &mut self,
        name: &str,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_call(
        &mut self,
        func: R,
        args: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_member(
        &mut self,
        obj: R,
        field: &str,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_if(
        &mut self,
        cond: R,
        then: R,
        else_: Option<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_match(
        &mut self,
        expr: R,
        arms: Vec<ReducedArm<R>>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_block(
        &mut self,
        stmts: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_assign(
        &mut self,
        target: R,
        value: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_lambda(
        &mut self,
        params: &[LirParam],
        body: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_record(
        &mut self,
        fields: Vec<(String, R)>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_variant(
        &mut self,
        name: &str,
        arg: Option<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_array(
        &mut self,
        items: Vec<R>,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_binary(
        &mut self,
        op: LirBinaryOp,
        lhs: R,
        rhs: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_unary(
        &mut self,
        op: LirUnaryOp,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_wildcard(&mut self, hint: &TargetHint, span: Span) -> Result<R, BackendError>;

    fn visit_expr_for_all(
        &mut self,
        type_: &Type,
        binding: R,
        property: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_assert_consistent(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_try(
        &mut self,
        body: R,
        binding: R,
        guard: Option<R>,
        handler: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_throw(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_expr_propagate(
        &mut self,
        expr: R,
        hint: &TargetHint,
        span: Span,
    ) -> Result<R, BackendError>;

    // ------ Statement hooks (2) ------

    fn visit_stmt_let(&mut self, pat: R, value: R) -> Result<R, BackendError>;

    fn visit_stmt_expr(&mut self, expr: R) -> Result<R, BackendError>;

    // ------ Pattern hooks (5) ------

    fn visit_pat_wildcard(&mut self) -> Result<R, BackendError>;

    fn visit_pat_literal(&mut self, value: &LirLiteral) -> Result<R, BackendError>;

    fn visit_pat_variable(&mut self, name: &str) -> Result<R, BackendError>;

    fn visit_pat_variant(&mut self, name: &str, arg: Option<R>) -> Result<R, BackendError>;

    fn visit_pat_record(&mut self, fields: Vec<(String, R)>, rest: bool)
        -> Result<R, BackendError>;

    // ------ Declaration hooks (4) ------

    #[allow(clippy::too_many_arguments)]
    fn visit_decl_function(
        &mut self,
        name: &str,
        params: &[LirParam],
        return_type: &Option<Type>,
        body: R,
        effect: &Effect,
        hint: &TargetHint,
        is_pub: bool,
        is_generator: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_record_def(
        &mut self,
        name: &str,
        fields: &[LirField],
        is_pub: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_union_def(
        &mut self,
        name: &str,
        variants: &[LirVariant],
        is_pub: bool,
        span: Span,
    ) -> Result<R, BackendError>;

    fn visit_decl_extern(
        &mut self,
        source: &str,
        name: &str,
        params: &[LirParam],
        return_type: &Option<Type>,
        is_pub: bool,
    ) -> Result<R, BackendError>;

    // ------ Lifecycle hooks (4) ------

    fn enter_module(&mut self) -> Result<(), BackendError>;

    fn exit_module(&mut self, decls: Vec<R>) -> Result<R, BackendError>;

    fn enter_function(&mut self, name: &str) -> Result<(), BackendError>;

    fn exit_function(&mut self, name: &str, body: R) -> Result<R, BackendError>;
}

// ------ Tests (RED phase — these must fail to compile) ------

#[cfg(test)]
mod tests {
    use crate::{BackendError, LirBackend, ReducedArm};
    use dwarf_lir::{
        Effect, LirArm, LirBinaryOp, LirDecl, LirExpr, LirField, LirLiteral, LirParam, LirPat,
        LirStmt, LirUnaryOp, LirVariant, TargetHint,
    };
    use dwarf_syntax::hir::Type;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn s() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint() -> TargetHint {
        TargetHint::None
    }

    fn make_literal(val: i64) -> LirExpr {
        LirExpr::Literal {
            value: LirLiteral::Int(val),
            hint: hint(),
            span: s(),
        }
    }

    fn make_var(name: &str) -> LirExpr {
        LirExpr::Variable {
            name: name.into(),
            hint: hint(),
            span: s(),
        }
    }

    // ------------------------------------------------------------------
    // BackendError — error type must exist and implement std::error::Error
    // ------------------------------------------------------------------

    #[test]
    fn test_backend_error_is_std_error() {
        let err = BackendError::msg("something went wrong");
        // Must implement std::error::Error (which requires Debug + Display).
        let _: &dyn std::error::Error = &err;
        // Debug must work.
        let debug_str = format!("{err:?}");
        assert!(!debug_str.is_empty());
        // Display must work.
        let display_str = format!("{err}");
        assert!(!display_str.is_empty());
    }

    #[test]
    fn test_backend_error_construction() {
        let err1 = BackendError::msg("test error");
        assert!(format!("{err1}").contains("test error"));

        let err2 = BackendError::msg("another error");
        assert!(format!("{err2}").contains("another error"));

        // Two different messages should produce different Display output.
        assert_ne!(format!("{err1}"), format!("{err2}"));
    }

    // ------------------------------------------------------------------
    // MockBackend — implements LirBackend<()> to prove the trait exists
    // and has the expected shape.
    // ------------------------------------------------------------------

    struct MockBackend;

    impl LirBackend<()> for MockBackend {
        // ------ Expression hooks (20) ------

        fn visit_expr_literal(
            &mut self,
            _value: &LirLiteral,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_variable(
            &mut self,
            _name: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_call(
            &mut self,
            _func: (),
            _args: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_member(
            &mut self,
            _obj: (),
            _field: &str,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_if(
            &mut self,
            _cond: (),
            _then: (),
            _else_: Option<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_match(
            &mut self,
            _expr: (),
            _arms: Vec<ReducedArm<()>>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_block(
            &mut self,
            _stmts: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_assign(
            &mut self,
            _target: (),
            _value: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_lambda(
            &mut self,
            _params: &[LirParam],
            _body: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_record(
            &mut self,
            _fields: Vec<(String, ())>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_variant(
            &mut self,
            _name: &str,
            _arg: Option<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_array(
            &mut self,
            _items: Vec<()>,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_binary(
            &mut self,
            _op: LirBinaryOp,
            _lhs: (),
            _rhs: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_unary(
            &mut self,
            _op: LirUnaryOp,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_wildcard(
            &mut self,
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_for_all(
            &mut self,
            _type_: &Type,
            _binding: (),
            _property: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_assert_consistent(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_try(
            &mut self,
            _body: (),
            _binding: (),
            _guard: Option<()>,
            _handler: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_throw(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_expr_propagate(
            &mut self,
            _expr: (),
            _hint: &TargetHint,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Statement hooks (2) ------

        fn visit_stmt_let(&mut self, _pat: (), _value: ()) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_stmt_expr(&mut self, _expr: ()) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Pattern hooks (5) ------

        fn visit_pat_wildcard(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_literal(&mut self, _value: &LirLiteral) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_variable(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_variant(&mut self, _name: &str, _arg: Option<()>) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_pat_record(
            &mut self,
            _fields: Vec<(String, ())>,
            _rest: bool,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Declaration hooks (4) ------

        #[allow(clippy::too_many_arguments)]
        fn visit_decl_function(
            &mut self,
            _name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            _body: (),
            _effect: &Effect,
            _hint: &TargetHint,
            _is_pub: bool,
            _is_generator: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_record_def(
            &mut self,
            _name: &str,
            _fields: &[LirField],
            _is_pub: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_union_def(
            &mut self,
            _name: &str,
            _variants: &[LirVariant],
            _is_pub: bool,
            _span: Span,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        fn visit_decl_extern(
            &mut self,
            _source: &str,
            _name: &str,
            _params: &[LirParam],
            _return_type: &Option<Type>,
            _is_pub: bool,
        ) -> Result<(), BackendError> {
            Ok(())
        }

        // ------ Lifecycle hooks (4) ------

        fn enter_module(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn exit_module(&mut self, _decls: Vec<()>) -> Result<(), BackendError> {
            Ok(())
        }

        fn enter_function(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }

        fn exit_function(&mut self, _name: &str, _body: ()) -> Result<(), BackendError> {
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Trait can be implemented — proves LirBackend<R> exists
    // ------------------------------------------------------------------

    #[test]
    fn test_trait_impl_compiles() {
        let _backend = MockBackend;
        // If this compiles, the trait exists and can be implemented.
    }

    // ------------------------------------------------------------------
    // Expression hooks — every LirExpr variant has a corresponding hook
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_expr_literal_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_literal(&LirLiteral::Int(42), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_variable_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_variable("x", &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_call_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_call((), vec![(), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_member_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_member((), "field", &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_if_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_if((), (), Some(()), &hint(), s());
        assert!(result.is_ok());

        // Also test without else branch.
        let result2 = b.visit_expr_if((), (), None, &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_match_hook() {
        let mut b = MockBackend;
        let arms = vec![ReducedArm {
            pattern: (),
            guard: None,
            body: (),
        }];
        let result = b.visit_expr_match((), arms, &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_block_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_block(vec![(), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_assign_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_assign((), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_lambda_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "x".into(),
            type_: None,
        }];
        let result = b.visit_expr_lambda(&params, (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_record_hook() {
        let mut b = MockBackend;
        let fields = vec![("x".into(), ()), ("y".into(), ())];
        let result = b.visit_expr_record(fields, &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_variant_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_variant("Some", Some(()), &hint(), s());
        assert!(result.is_ok());

        let result2 = b.visit_expr_variant("None", None, &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_array_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_array(vec![(), (), ()], &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_binary_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_binary(LirBinaryOp::Add, (), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_unary_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_unary(LirUnaryOp::Neg, (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_wildcard_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_wildcard(&hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_for_all_hook() {
        let mut b = MockBackend;
        let ty = Type::Named("Int".into());
        let result = b.visit_expr_for_all(&ty, (), (), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_assert_consistent_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_assert_consistent((), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_try_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_try((), (), None, (), &hint(), s());
        assert!(result.is_ok());

        // Also test with guard.
        let result2 = b.visit_expr_try((), (), Some(()), (), &hint(), s());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_expr_throw_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_throw((), &hint(), s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_expr_propagate_hook() {
        let mut b = MockBackend;
        let result = b.visit_expr_propagate((), &hint(), s());
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Statement hooks — both LirStmt variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_stmt_let_hook() {
        let mut b = MockBackend;
        let result = b.visit_stmt_let((), ());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_stmt_expr_hook() {
        let mut b = MockBackend;
        let result = b.visit_stmt_expr(());
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Pattern hooks — all 5 LirPat variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_pat_wildcard_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_wildcard();
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_literal_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_literal(&LirLiteral::Int(7));
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_variable_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_variable("binding");
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_pat_variant_hook() {
        let mut b = MockBackend;
        let result = b.visit_pat_variant("Some", Some(()));
        assert!(result.is_ok());

        let result2 = b.visit_pat_variant("None", None);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_visit_pat_record_hook() {
        let mut b = MockBackend;
        let fields = vec![("x".into(), ()), ("y".into(), ())];
        let result = b.visit_pat_record(fields, false);
        assert!(result.is_ok());

        // Also test with rest.
        let result2 = b.visit_pat_record(vec![], true);
        assert!(result2.is_ok());
    }

    // ------------------------------------------------------------------
    // Declaration hooks — all 4 LirDecl variants
    // ------------------------------------------------------------------

    #[test]
    fn test_visit_decl_function_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "a".into(),
            type_: None,
        }];
        let result = b.visit_decl_function(
            "main",
            &params,
            &None,
            (),
            &Effect::Pure,
            &hint(),
            true,
            false,
            s(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_record_def_hook() {
        let mut b = MockBackend;
        let fields = vec![LirField {
            name: "x".into(),
            type_: Type::Named("Int".into()),
        }];
        let result = b.visit_decl_record_def("Point", &fields, true, s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_union_def_hook() {
        let mut b = MockBackend;
        let variants = vec![
            LirVariant {
                name: "Some".into(),
                arg: Some(Type::Named("Int".into())),
            },
            LirVariant {
                name: "None".into(),
                arg: None,
            },
        ];
        let result = b.visit_decl_union_def("Option", &variants, true, s());
        assert!(result.is_ok());
    }

    #[test]
    fn test_visit_decl_extern_hook() {
        let mut b = MockBackend;
        let params = vec![LirParam {
            name: "fd".into(),
            type_: None,
        }];
        let result = b.visit_decl_extern("libc", "read", &params, &None, true);
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // Lifecycle hooks — module and function enter/exit
    // ------------------------------------------------------------------

    #[test]
    fn test_lifecycle_hooks() {
        let mut b = MockBackend;

        // Module lifecycle.
        assert!(b.enter_module().is_ok());
        assert!(b.exit_module(vec![(), ()]).is_ok());

        // Function lifecycle.
        assert!(b.enter_function("main").is_ok());
        assert!(b.exit_function("main", ()).is_ok());
    }

    // ------------------------------------------------------------------
    // Generic return type — trait works with different R types
    // ------------------------------------------------------------------

    struct StringBackend;

    impl LirBackend<String> for StringBackend {
        fn visit_expr_literal(
            &mut self,
            v: &LirLiteral,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{v:?}"))
        }
        fn visit_expr_variable(
            &mut self,
            name: &str,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(name.to_string())
        }
        fn visit_expr_call(
            &mut self,
            func: String,
            args: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("call({func}, [{}])", args.join(", ")))
        }
        fn visit_expr_member(
            &mut self,
            obj: String,
            field: &str,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{obj}.{field}"))
        }
        fn visit_expr_if(
            &mut self,
            c: String,
            t: String,
            e: Option<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            match e {
                Some(el) => Ok(format!("if {c} then {t} else {el}")),
                None => Ok(format!("if {c} then {t}")),
            }
        }
        fn visit_expr_match(
            &mut self,
            expr: String,
            _arms: Vec<ReducedArm<String>>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("match {expr}"))
        }
        fn visit_expr_block(
            &mut self,
            stmts: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{{ {} }}", stmts.join("; ")))
        }
        fn visit_expr_assign(
            &mut self,
            target: String,
            value: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{target} = {value}"))
        }
        fn visit_expr_lambda(
            &mut self,
            params: &[LirParam],
            body: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            Ok(format!("|{}| {body}", names.join(", ")))
        }
        fn visit_expr_record(
            &mut self,
            fields: Vec<(String, String)>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            Ok(format!("{{ {} }}", pairs.join(", ")))
        }
        fn visit_expr_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.to_string()),
            }
        }
        fn visit_expr_array(
            &mut self,
            items: Vec<String>,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("[{}]", items.join(", ")))
        }
        fn visit_expr_binary(
            &mut self,
            op: LirBinaryOp,
            lhs: String,
            rhs: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("({lhs} {op:?} {rhs})"))
        }
        fn visit_expr_unary(
            &mut self,
            op: LirUnaryOp,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("({op:?} {expr})"))
        }
        fn visit_expr_wildcard(
            &mut self,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok("_".to_string())
        }
        fn visit_expr_for_all(
            &mut self,
            _ty: &Type,
            _binding: String,
            property: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("forAll {property}"))
        }
        fn visit_expr_assert_consistent(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("assertConsistent({expr})"))
        }
        fn visit_expr_try(
            &mut self,
            body: String,
            _binding: String,
            _guard: Option<String>,
            handler: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("try {body} catch {handler}"))
        }
        fn visit_expr_throw(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("throw {expr}"))
        }
        fn visit_expr_propagate(
            &mut self,
            expr: String,
            _h: &TargetHint,
            _s: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("{expr}?"))
        }

        fn visit_stmt_let(&mut self, pat: String, value: String) -> Result<String, BackendError> {
            Ok(format!("let {pat} = {value}"))
        }
        fn visit_stmt_expr(&mut self, expr: String) -> Result<String, BackendError> {
            Ok(expr)
        }

        fn visit_pat_wildcard(&mut self) -> Result<String, BackendError> {
            Ok("_".into())
        }
        fn visit_pat_literal(&mut self, v: &LirLiteral) -> Result<String, BackendError> {
            Ok(format!("{v:?}"))
        }
        fn visit_pat_variable(&mut self, name: &str) -> Result<String, BackendError> {
            Ok(name.into())
        }
        fn visit_pat_variant(
            &mut self,
            name: &str,
            arg: Option<String>,
        ) -> Result<String, BackendError> {
            match arg {
                Some(a) => Ok(format!("{name}({a})")),
                None => Ok(name.into()),
            }
        }
        fn visit_pat_record(
            &mut self,
            fields: Vec<(String, String)>,
            rest: bool,
        ) -> Result<String, BackendError> {
            let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            let mut s = format!("{{ {} }}", pairs.join(", "));
            if rest {
                s.push_str(" ..");
            }
            Ok(s)
        }

        #[allow(clippy::too_many_arguments)]
        fn visit_decl_function(
            &mut self,
            name: &str,
            _params: &[LirParam],
            _ret: &Option<Type>,
            body: String,
            _effect: &Effect,
            _hint: &TargetHint,
            _is_pub: bool,
            _is_gen: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("fn {name} = {body}"))
        }
        fn visit_decl_record_def(
            &mut self,
            name: &str,
            _fields: &[LirField],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("record {name}"))
        }
        fn visit_decl_union_def(
            &mut self,
            name: &str,
            _variants: &[LirVariant],
            _is_pub: bool,
            _span: Span,
        ) -> Result<String, BackendError> {
            Ok(format!("union {name}"))
        }
        fn visit_decl_extern(
            &mut self,
            source: &str,
            name: &str,
            _params: &[LirParam],
            _ret: &Option<Type>,
            _is_pub: bool,
        ) -> Result<String, BackendError> {
            Ok(format!("extern {source} {name}"))
        }

        fn enter_module(&mut self) -> Result<(), BackendError> {
            Ok(())
        }
        fn exit_module(&mut self, decls: Vec<String>) -> Result<String, BackendError> {
            Ok(decls.join("\n"))
        }
        fn enter_function(&mut self, _name: &str) -> Result<(), BackendError> {
            Ok(())
        }
        fn exit_function(&mut self, _name: &str, body: String) -> Result<String, BackendError> {
            Ok(body)
        }
    }

    #[test]
    fn test_generic_return_type_string() {
        let mut b = StringBackend;
        let lit = b
            .visit_expr_literal(&LirLiteral::Int(42), &hint(), s())
            .unwrap();
        assert!(
            !lit.is_empty(),
            "String backend should produce non-empty output"
        );

        let var = b.visit_expr_variable("x", &hint(), s()).unwrap();
        assert_eq!(var, "x");

        let call = b.visit_expr_call(var, vec![lit], &hint(), s()).unwrap();
        assert!(call.contains("call"), "should contain 'call'");
    }

    #[test]
    fn test_generic_return_type_i32() {
        // Prove the trait works with a numeric return type too.
        struct CountBackend;

        impl LirBackend<i32> for CountBackend {
            fn visit_expr_literal(
                &mut self,
                _v: &LirLiteral,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_variable(
                &mut self,
                _n: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_expr_call(
                &mut self,
                f: i32,
                args: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(f + args.into_iter().sum::<i32>())
            }
            fn visit_expr_member(
                &mut self,
                obj: i32,
                _f: &str,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(obj)
            }
            fn visit_expr_if(
                &mut self,
                c: i32,
                t: i32,
                e: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(c + t + e.unwrap_or(0))
            }
            fn visit_expr_match(
                &mut self,
                expr: i32,
                _arms: Vec<ReducedArm<i32>>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(expr)
            }
            fn visit_expr_block(
                &mut self,
                stmts: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(stmts.into_iter().sum())
            }
            fn visit_expr_assign(
                &mut self,
                t: i32,
                v: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(t + v)
            }
            fn visit_expr_lambda(
                &mut self,
                _p: &[LirParam],
                body: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_expr_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            fn visit_expr_variant(
                &mut self,
                _n: &str,
                arg: Option<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(arg.unwrap_or(0))
            }
            fn visit_expr_array(
                &mut self,
                items: Vec<i32>,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(items.into_iter().sum())
            }
            fn visit_expr_binary(
                &mut self,
                _op: LirBinaryOp,
                l: i32,
                r: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(l + r)
            }
            fn visit_expr_unary(
                &mut self,
                _op: LirUnaryOp,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_wildcard(
                &mut self,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_expr_for_all(
                &mut self,
                _t: &Type,
                _b: i32,
                p: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(p)
            }
            fn visit_expr_assert_consistent(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_try(
                &mut self,
                body: i32,
                _b: i32,
                _g: Option<i32>,
                h: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body + h)
            }
            fn visit_expr_throw(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_expr_propagate(
                &mut self,
                e: i32,
                _h: &TargetHint,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_stmt_let(&mut self, _p: i32, v: i32) -> Result<i32, BackendError> {
                Ok(v)
            }
            fn visit_stmt_expr(&mut self, e: i32) -> Result<i32, BackendError> {
                Ok(e)
            }
            fn visit_pat_wildcard(&mut self) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_pat_literal(&mut self, _v: &LirLiteral) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variable(&mut self, _n: &str) -> Result<i32, BackendError> {
                Ok(1)
            }
            fn visit_pat_variant(&mut self, _n: &str, a: Option<i32>) -> Result<i32, BackendError> {
                Ok(a.unwrap_or(0))
            }
            fn visit_pat_record(
                &mut self,
                fields: Vec<(String, i32)>,
                _rest: bool,
            ) -> Result<i32, BackendError> {
                Ok(fields.into_iter().map(|(_, v)| v).sum())
            }
            #[allow(clippy::too_many_arguments)]
            fn visit_decl_function(
                &mut self,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                body: i32,
                _e: &Effect,
                _h: &TargetHint,
                _pub: bool,
                _gen: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(body)
            }
            fn visit_decl_record_def(
                &mut self,
                _n: &str,
                _f: &[LirField],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_union_def(
                &mut self,
                _n: &str,
                _v: &[LirVariant],
                _pub: bool,
                _s: Span,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn visit_decl_extern(
                &mut self,
                _s: &str,
                _n: &str,
                _p: &[LirParam],
                _r: &Option<Type>,
                _pub: bool,
            ) -> Result<i32, BackendError> {
                Ok(0)
            }
            fn enter_module(&mut self) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_module(&mut self, decls: Vec<i32>) -> Result<i32, BackendError> {
                Ok(decls.into_iter().sum())
            }
            fn enter_function(&mut self, _n: &str) -> Result<(), BackendError> {
                Ok(())
            }
            fn exit_function(&mut self, _n: &str, body: i32) -> Result<i32, BackendError> {
                Ok(body)
            }
        }

        let mut b = CountBackend;
        let count = b
            .visit_expr_binary(LirBinaryOp::Add, 1, 2, &hint(), s())
            .unwrap();
        assert_eq!(count, 3, "i32 backend should sum children");
    }

    // ------------------------------------------------------------------
    // Full walk — exercise every hook category in sequence
    // ------------------------------------------------------------------

    #[test]
    fn test_full_walk_all_categories() {
        let mut b = MockBackend;

        // Module enter.
        b.enter_module().unwrap();

        // Declaration: function.
        b.enter_function("main").unwrap();
        b.visit_decl_function(
            "main",
            &[],
            &None,
            (),
            &Effect::Pure,
            &hint(),
            true,
            false,
            s(),
        )
        .unwrap();

        // Expression hooks.
        b.visit_expr_literal(&LirLiteral::Int(1), &hint(), s())
            .unwrap();
        b.visit_expr_variable("x", &hint(), s()).unwrap();
        b.visit_expr_call((), vec![], &hint(), s()).unwrap();
        b.visit_expr_member((), "f", &hint(), s()).unwrap();
        b.visit_expr_if((), (), None, &hint(), s()).unwrap();
        b.visit_expr_match((), vec![], &hint(), s()).unwrap();
        b.visit_expr_block(vec![], &hint(), s()).unwrap();
        b.visit_expr_assign((), (), &hint(), s()).unwrap();
        b.visit_expr_lambda(&[], (), &hint(), s()).unwrap();
        b.visit_expr_record(vec![], &hint(), s()).unwrap();
        b.visit_expr_variant("None", None, &hint(), s()).unwrap();
        b.visit_expr_array(vec![], &hint(), s()).unwrap();
        b.visit_expr_binary(LirBinaryOp::Add, (), (), &hint(), s())
            .unwrap();
        b.visit_expr_unary(LirUnaryOp::Not, (), &hint(), s())
            .unwrap();
        b.visit_expr_wildcard(&hint(), s()).unwrap();
        b.visit_expr_for_all(&Type::Named("Int".into()), (), (), &hint(), s())
            .unwrap();
        b.visit_expr_assert_consistent((), &hint(), s()).unwrap();
        b.visit_expr_try((), (), None, (), &hint(), s()).unwrap();
        b.visit_expr_throw((), &hint(), s()).unwrap();
        b.visit_expr_propagate((), &hint(), s()).unwrap();

        // Statement hooks.
        b.visit_stmt_let((), ()).unwrap();
        b.visit_stmt_expr(()).unwrap();

        // Pattern hooks.
        b.visit_pat_wildcard().unwrap();
        b.visit_pat_literal(&LirLiteral::Null).unwrap();
        b.visit_pat_variable("x").unwrap();
        b.visit_pat_variant("Some", None).unwrap();
        b.visit_pat_record(vec![], false).unwrap();

        // Declaration hooks (non-function).
        b.visit_decl_record_def("Pt", &[], true, s()).unwrap();
        b.visit_decl_union_def("Opt", &[], true, s()).unwrap();
        b.visit_decl_extern("libc", "write", &[], &None, false)
            .unwrap();

        // Function/module exit.
        b.exit_function("main", ()).unwrap();
        b.exit_module(vec![()]).unwrap();
    }
}
