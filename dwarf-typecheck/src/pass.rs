//! TypeCheckPass — the type-checking compilation pass.
//!
//! This pass runs the full type-check pipeline on parsed HIR declarations:
//! resolution of type declarations, then expression type inference.

use dwarf_syntax::hir::Decl;

use crate::error::{TypeCheckError, TYPE_ERROR_CODES};
use crate::infer::{infer_expr, TypeEnv};
use crate::registry::TypeRegistry;
use crate::resolve;
use crate::types::{TypeDef, ANY_TYPE_ID};

/// The type-checking compilation pass.
///
/// Register this pass in the pass manager after the parse pass
/// to perform type-checking on the parsed HIR.
pub struct TypeCheckPass;

impl Default for TypeCheckPass {
    fn default() -> Self {
        Self
    }
}

impl TypeCheckPass {
    pub fn new() -> Self {
        Self
    }

    /// Run the full type-check pipeline on a set of HIR declarations.
    ///
    /// Returns the populated TypeRegistry and any type errors found.
    pub fn check(&self, decls: &[Decl]) -> (TypeRegistry, Vec<TypeCheckError>) {
        let mut registry = TypeRegistry::new();
        let mut errors = Vec::new();

        // Phase 1: Register all type declarations (RecordDef, UnionDef, TypeDef, Extern)
        let result = resolve::register_decls(&mut registry, decls);
        let extern_map = result.extern_map;
        errors.extend(result.errors);
        // TODO: Thread name_map from resolve into Phase 2 so param type
        // annotations can resolve user-defined types (not just primitives).

        // Phase 2: Infer types for function declarations
        for decl in decls {
            if let Decl::Function {
                name,
                params,
                return_type: _,
                body,
                is_pub: _,
                span,
            } = decl
            {
                let mut env = TypeEnv::new();

                // Bind extern function names so they're available in function bodies
                for (extern_name, extern_type_id) in &extern_map {
                    env.bind(extern_name.clone(), *extern_type_id);
                }

                // Bind parameter types from type annotations
                for param in params {
                    if let Some(ref hir_type) = param.type_ {
                        // Resolve named types using simple name lookup
                        let type_id = match hir_type {
                            dwarf_syntax::hir::Type::Named(n) => match n.as_str() {
                                "int" | "Int" => 0,
                                "float" | "Float" => 1,
                                "str" | "Str" | "string" | "String" => 2,
                                "bool" | "Bool" => 3,
                                "null" | "Null" => 4,
                                "any" | "Any" => ANY_TYPE_ID,
                                unknown => {
                                    errors.push(TypeCheckError::new(
                                        "DWARF-E-TYPE-0002",
                                        format!("unknown type: {}", unknown),
                                        *span,
                                    ));
                                    continue;
                                }
                            },
                            dwarf_syntax::hir::Type::Generic { base, args: _ } => {
                                // Simple base name resolution for generic annotations
                                match base.as_str() {
                                    "int" | "Int" | "float" | "Float" | "str" | "Str"
                                    | "string" | "String" | "bool" | "Bool" | "null" | "Null" => {
                                        // Use the concrete registration for generic types
                                        errors.push(TypeCheckError::new(
                                            "DWARF-E-TYPE-0008",
                                            format!(
                                                "cannot fully infer generic type parameter in '{}'",
                                                base
                                            ),
                                            *span,
                                        ));
                                        continue;
                                    }
                                    // Built-in generic type constructors are valid annotations
                                    "Option" | "Result" | "List" | "Map" => {
                                        let builtin_id = match base.as_str() {
                                            "Option" => 5,
                                            "Result" => 6,
                                            "List" => 7,
                                            "Map" => 8,
                                            _ => unreachable!(),
                                        };
                                        builtin_id
                                    }
                                    _ => {
                                        errors.push(TypeCheckError::new(
                                            "DWARF-E-TYPE-0002",
                                            format!("unknown generic type: {}", base),
                                            *span,
                                        ));
                                        continue;
                                    }
                                }
                            }
                            dwarf_syntax::hir::Type::Refined { base, constraint } => {
                                // Register the refined type in the registry so
                                // infer_call can validate literal arguments against
                                // Range and NonEmpty constraints.
                                let base_type_id = match base.as_ref() {
                                    dwarf_syntax::hir::Type::Named(n) => match n.as_str() {
                                        "int" | "Int" => 0,
                                        "float" | "Float" => 1,
                                        "str" | "Str" | "string" | "String" => 2,
                                        "bool" | "Bool" => 3,
                                        "null" | "Null" => 4,
                                        "any" | "Any" => ANY_TYPE_ID,
                                        unknown => {
                                            errors.push(TypeCheckError::new(
                                                "DWARF-E-TYPE-0002",
                                                format!("unknown type: {}", unknown),
                                                *span,
                                            ));
                                            continue;
                                        }
                                    },
                                    _ => {
                                        errors.push(TypeCheckError::new(
                                            "DWARF-E-TYPE-0008",
                                            format!(
                                                "unsupported base type in refined annotation for parameter '{}'",
                                                param.name
                                            ),
                                            *span,
                                        ));
                                        continue;
                                    }
                                };
                                let tc_constraint = resolve::convert_ref_constraint(constraint);
                                registry.register(TypeDef::Refined {
                                    base: base_type_id,
                                    constraint: tc_constraint,
                                })
                            }
                            _ => {
                                errors.push(TypeCheckError::new(
                                    "DWARF-E-TYPE-0008",
                                    format!(
                                        "unsupported type annotation for parameter '{}'",
                                        param.name
                                    ),
                                    *span,
                                ));
                                continue;
                            }
                        };
                        env.bind(param.name.clone(), type_id);
                    }
                    // Parameters without type annotations get inferred from usage
                }

                // Infer the body type
                match infer_expr(body, &env, &mut registry) {
                    Ok(_) => {} // body type inferred successfully
                    Err(msg) => {
                        errors.push(TypeCheckError::new(
                            "DWARF-E-TYPE-0001",
                            format!("type error in function '{}': {}", name, msg),
                            *span,
                        ));
                    }
                }
            } else if let Decl::Decorator { target, .. } = decl {
                // Recursively typecheck the decorator's target.
                // This handles nested decorators (e.g. @A @B fn foo() { ... })
                // since each Decorator's target is unwrapped and fed back
                // into the same top-level loop via check().
                let (_, inner_errors) = self.check(std::slice::from_ref(target.as_ref()));
                errors.extend(inner_errors);
            }
        }

        (registry, errors)
    }
}

/// Ensure error codes are available from this crate.
pub fn type_error_codes() -> &'static [&'static str] {
    TYPE_ERROR_CODES
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::*;
    use dwarf_syntax::span::Span;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    #[test]
    fn test_typecheck_pass_can_be_created() {
        let pass = TypeCheckPass::new();
        let decls: Vec<Decl> = vec![];
        let (_registry, errors) = pass.check(&decls);
        assert!(errors.is_empty(), "Empty decls should have no errors");
    }

    #[test]
    fn test_typecheck_valid_function_no_params() {
        let pass = TypeCheckPass::new();
        let decls = vec![Decl::Function {
            name: "answer".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        }];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Valid function should have no errors: {:?}",
            errors
        );
    }

    // -------------------------------------------------------------------------
    // Decorator tests (DWARF-6: xUnit testing support)
    //
    // The parser produces Decl::Decorator to represent annotations like @Test.
    // The typechecker must unwrap decorators to reach the inner function and
    // typecheck its body. Currently the typechecker ignores Decl::Decorator
    // entirely (falls through to `_ => {}`), so tests 2 and 3 FAIL — they
    // demonstrate the bug.
    // -------------------------------------------------------------------------

    #[test]
    /// A decorator wrapping a valid function should pass typechecking (no errors).
    /// The typechecker must recursively unwrap Decl::Decorator to reach
    /// the inner Decl::Function and typecheck its body.
    fn test_decorator_valid_function() {
        let pass = TypeCheckPass::new();
        let decls = vec![Decl::Decorator {
            name: "Test".to_string(),
            args: vec![],
            target: Box::new(Decl::Function {
                name: "my_test".to_string(),
                params: vec![],
                return_type: None,
                body: Expr::Literal {
                    value: LiteralValue::Int(42),
                    span: dummy_span(),
                },
                is_pub: false,
                span: dummy_span(),
            }),
            is_pub: false,
            span: dummy_span(),
        }];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Decorator wrapping a valid function should produce no errors: {:?}",
            errors
        );
    }

    #[test]
    /// A decorator wrapping a function with type errors should report those errors.
    /// Currently the typechecker ignores Decl::Decorator, so this test FAILS:
    /// the inner function's body is never typechecked, the type error is missed,
    /// and errors.is_empty() is true — but we assert !errors.is_empty().
    fn test_decorator_type_error_in_body() {
        let pass = TypeCheckPass::new();

        // Function body with a type error: 1 + true (Int + Bool is invalid)
        let body = Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            }),
            rhs: Box::new(Expr::Literal {
                value: LiteralValue::Bool(true),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let decls = vec![Decl::Decorator {
            name: "Test".to_string(),
            args: vec![],
            target: Box::new(Decl::Function {
                name: "bad_test".to_string(),
                params: vec![],
                return_type: None,
                body,
                is_pub: false,
                span: dummy_span(),
            }),
            is_pub: false,
            span: dummy_span(),
        }];

        let (_registry, errors) = pass.check(&decls);
        assert!(
            !errors.is_empty(),
            "Decorator wrapping a function with type errors SHOULD report errors, \
             but got none — the typechecker likely ignored the Decorator"
        );
    }

    #[test]
    /// Stacked (nested) decorators should recursively unwrap to reach the
    /// innermost function and typecheck it. Currently fails because the
    /// typechecker ignores all Decorator nodes.
    fn test_decorator_nested() {
        let pass = TypeCheckPass::new();

        // Innermost function body with a type error: 1 + true
        let body = Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal {
                value: LiteralValue::Int(1),
                span: dummy_span(),
            }),
            rhs: Box::new(Expr::Literal {
                value: LiteralValue::Bool(true),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let inner_func = Decl::Function {
            name: "bad_test".to_string(),
            params: vec![],
            return_type: None,
            body,
            is_pub: false,
            span: dummy_span(),
        };

        // @B wraps the function
        let decorator_b = Decl::Decorator {
            name: "B".to_string(),
            args: vec![],
            target: Box::new(inner_func),
            is_pub: false,
            span: dummy_span(),
        };

        // @A wraps @B (stacked decorators: @A @B fn bad_test() { 1 + true })
        let decls = vec![Decl::Decorator {
            name: "A".to_string(),
            args: vec![],
            target: Box::new(decorator_b),
            is_pub: false,
            span: dummy_span(),
        }];

        let (_registry, errors) = pass.check(&decls);
        assert!(
            !errors.is_empty(),
            "Nested decorators wrapping a function with type errors SHOULD report \
             errors, but got none — the typechecker likely ignored all Decorator nodes"
        );
    }

    // ------------------------------------------------------------------
    // Built-in generic annotation tests (DWARF-GENERICS)
    //
    // WILL FAIL — RED PHASE
    //
    // This test verifies that a function parameter annotated with
    // `Option<Int>` is resolved as a valid type. It will fail until
    // the TypeCheckPass resolves built-in generic names (Option, Result,
    // List, Map) via the registry's built-in type constructors.
    // ------------------------------------------------------------------

    #[test]
    fn test_typecheck_option_int_annotation() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();
        let decls = vec![Decl::Function {
            name: "test".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                type_: Some(Type::Generic {
                    base: "Option".to_string(),
                    args: vec![Type::Named("Int".to_string())],
                }),
            }],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        }];
        let (_registry, errors) = pass.check(&decls);
        // This will currently fail because "Option" is not in the resolve name_map
        // The fix should make this pass
        assert!(
            errors.is_empty(),
            "Option<Int> annotation should be valid: {:?}",
            errors
        );
    }

    // ------------------------------------------------------------------
    // Extern function type-checking tests (DWARF-FFI Phase 2)
    //
    // WILL FAIL — RED PHASE
    //
    // These tests verify that Decl::Extern declarations are registered in
    // the TypeRegistry as Func types and that calls to extern functions are
    // type-checked for argument count, argument types, and return types.
    //
    // They will fail until:
    //   1. resolve::register_decls handles Decl::Extern by registering a
    //      TypeDef::Func in the registry for each extern declaration
    //   2. TypeCheckPass::check() binds extern names in the TypeEnv so
    //      function bodies can reference them
    //   3. infer_call validates argument types against the extern's signature
    // ------------------------------------------------------------------

    #[test]
    /// An extern declaration should register the function's signature in the
    /// TypeRegistry so that calls to it type-check correctly.
    ///
    /// Setup:
    ///   extern "npm:math" fn add(a: Int, b: Int) -> Int
    ///   fn main() { add(1, 2) }
    ///
    /// Expected: no errors (extern is registered, call matches signature).
    ///
    /// Failure mode: Decl::Extern is ignored by register_decls and
    /// TypeCheckPass, so the call `add(1, 2)` produces "unknown variable: add".
    fn test_extern_function_registered_in_registry() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        let extern_decl = Decl::Extern {
            source: "npm:math".to_string(),
            name: "add".to_string(),
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
            is_pub: true,
            span: dummy_span(),
        };

        // A function that calls the extern with correct argument types
        let caller = Decl::Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "add".to_string(),
                    span: dummy_span(),
                }),
                args: vec![
                    Expr::Literal {
                        value: LiteralValue::Int(1),
                        span: dummy_span(),
                    },
                    Expr::Literal {
                        value: LiteralValue::Int(2),
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, caller];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Extern function 'add' should be registered and callable with \
             correct argument types, but got errors: {:?}",
            errors
        );
    }

    #[test]
    /// Calling an extern with a wrong argument type should produce a type
    /// mismatch error, not an "unknown variable" error.
    ///
    /// Setup:
    ///   extern "npm:math" fn add(a: Int, b: Int) -> Int
    ///   fn bad() { add(42, "hello") }
    ///
    /// Expected: error mentioning argument type mismatch (arg 1 is Str, expected Int).
    ///
    /// Failure mode: The extern is not registered, so the error is
    /// "unknown variable: add" instead of a type mismatch on the second argument.
    fn test_extern_wrong_argument_type_error() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        let extern_decl = Decl::Extern {
            source: "npm:math".to_string(),
            name: "add".to_string(),
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
            is_pub: true,
            span: dummy_span(),
        };

        // Call add(42, "hello") — second arg is Str, expected Int
        let caller = Decl::Function {
            name: "bad".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "add".to_string(),
                    span: dummy_span(),
                }),
                args: vec![
                    Expr::Literal {
                        value: LiteralValue::Int(42),
                        span: dummy_span(),
                    },
                    Expr::Literal {
                        value: LiteralValue::Str("hello".to_string()),
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, caller];
        let (_registry, errors) = pass.check(&decls);

        assert!(
            !errors.is_empty(),
            "Calling extern with wrong argument type should produce errors"
        );

        // The error should be about argument type mismatch, NOT "unknown variable".
        // This assertion fails in RED phase because the extern is not registered,
        // so the only error is "unknown variable: add".
        let has_type_error = errors
            .iter()
            .any(|e| e.message.contains("mismatch") || e.message.contains("argument"));
        assert!(
            has_type_error,
            "Expected a type mismatch or argument error for extern call with \
             wrong argument type, but got: {:?}",
            errors
        );
    }

    #[test]
    /// An extern function with generic type parameters should support type
    /// inference at call sites.
    ///
    /// Setup:
    ///   extern "npm:lodash" fn map<T, U>(arr: List<T>, f: Func<T, U>) -> List<U>
    ///   fn main() { map([1, 2, 3], fn(x: Int) -> Int { x * 2 }) }
    ///
    /// Expected: no errors — T and U are inferred as Int.
    ///
    /// Failure mode: Generic extern declarations are not supported. The
    /// type parameters <T, U> cannot be resolved, and the Func type
    /// annotation in parameters is not supported by resolve_hir_type_param.
    fn test_extern_generic_function_inference() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        // extern "npm:lodash" fn map(arr: List, f: Func) -> List
        // Note: HIR doesn't yet have a way to express type params on decls,
        // so we use the base generic names. The full generic inference is
        // aspirational — this test specifies the desired end state.
        let extern_decl = Decl::Extern {
            source: "npm:lodash".to_string(),
            name: "map".to_string(),
            params: vec![
                Param {
                    name: "arr".to_string(),
                    type_: Some(Type::Generic {
                        base: "List".to_string(),
                        args: vec![Type::Named("Int".to_string())],
                    }),
                },
                Param {
                    name: "f".to_string(),
                    type_: Some(Type::Func {
                        params: vec![Type::Named("Int".to_string())],
                        return_: Box::new(Type::Named("Int".to_string())),
                    }),
                },
            ],
            return_type: Some(Type::Generic {
                base: "List".to_string(),
                args: vec![Type::Named("Int".to_string())],
            }),
            is_pub: true,
            span: dummy_span(),
        };

        // fn main() { map([1, 2, 3], fn(x: Int) -> Int { x * 2 }) }
        let lambda = Expr::Lambda {
            params: vec![Param {
                name: "x".to_string(),
                type_: Some(Type::Named("Int".to_string())),
            }],
            body: Box::new(Expr::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::Variable {
                    name: "x".to_string(),
                    span: dummy_span(),
                }),
                rhs: Box::new(Expr::Literal {
                    value: LiteralValue::Int(2),
                    span: dummy_span(),
                }),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let caller = Decl::Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "map".to_string(),
                    span: dummy_span(),
                }),
                args: vec![
                    Expr::Array {
                        items: vec![
                            Expr::Literal {
                                value: LiteralValue::Int(1),
                                span: dummy_span(),
                            },
                            Expr::Literal {
                                value: LiteralValue::Int(2),
                                span: dummy_span(),
                            },
                            Expr::Literal {
                                value: LiteralValue::Int(3),
                                span: dummy_span(),
                            },
                        ],
                        span: dummy_span(),
                    },
                    lambda,
                ],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, caller];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Generic extern 'map' should support type inference at call sites: {:?}",
            errors
        );
    }

    #[test]
    /// An extern parameter typed as `Any` should accept values of any type.
    ///
    /// Setup:
    ///   extern "npm:console" fn log(msg: Any) -> ()
    ///   fn main() { log(42) }
    ///
    /// Expected: no errors — Any is compatible with Int, Str, Bool, etc.
    ///
    /// Failure mode: The `Any` type does not exist in the type system.
    /// resolve_hir_type_name("Any") returns None, producing an "unknown type"
    /// error during parameter resolution.
    fn test_any_type_accepts_any_value() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        let extern_decl = Decl::Extern {
            source: "npm:console".to_string(),
            name: "log".to_string(),
            params: vec![Param {
                name: "msg".to_string(),
                type_: Some(Type::Named("Any".to_string())),
            }],
            return_type: Some(Type::Named("Null".to_string())),
            is_pub: true,
            span: dummy_span(),
        };

        // Call log(42) — Int should be accepted by Any
        let caller = Decl::Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "log".to_string(),
                    span: dummy_span(),
                }),
                args: vec![Expr::Literal {
                    value: LiteralValue::Int(42),
                    span: dummy_span(),
                }],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, caller];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Any-typed extern parameter should accept Int values: {:?}",
            errors
        );
    }

    #[test]
    /// Calling a function name that was NOT declared as an extern should
    /// produce an error, while a declared extern should type-check fine.
    ///
    /// Setup:
    ///   extern "npm:math" fn add(a: Int, b: Int) -> Int
    ///   fn good() { add(1, 2) }     — should pass
    ///   fn bad()  { subtract(1, 2) } — should fail (not declared)
    ///
    /// Expected: exactly one error, mentioning "subtract".
    ///
    /// Failure mode: Neither extern registration nor name resolution works,
    /// so BOTH calls produce "unknown variable" errors. The test expects
    /// only one error (for "subtract"), but gets two.
    fn test_extern_not_found_error() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        let extern_decl = Decl::Extern {
            source: "npm:math".to_string(),
            name: "add".to_string(),
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
            is_pub: true,
            span: dummy_span(),
        };

        // fn good() { add(1, 2) } — declared extern, should type-check
        let good_caller = Decl::Function {
            name: "good".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "add".to_string(),
                    span: dummy_span(),
                }),
                args: vec![
                    Expr::Literal {
                        value: LiteralValue::Int(1),
                        span: dummy_span(),
                    },
                    Expr::Literal {
                        value: LiteralValue::Int(2),
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        // fn bad() { subtract(1, 2) } — NOT declared, should error
        let bad_caller = Decl::Function {
            name: "bad".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "subtract".to_string(),
                    span: dummy_span(),
                }),
                args: vec![
                    Expr::Literal {
                        value: LiteralValue::Int(1),
                        span: dummy_span(),
                    },
                    Expr::Literal {
                        value: LiteralValue::Int(2),
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, good_caller, bad_caller];
        let (_registry, errors) = pass.check(&decls);

        // Should have exactly one error: for "subtract" (not declared)
        assert!(
            !errors.is_empty(),
            "Calling an undeclared extern should produce at least one error"
        );

        // The error should mention "subtract", not "add"
        let mentions_subtract = errors.iter().any(|e| e.message.contains("subtract"));
        let mentions_add = errors.iter().any(|e| e.message.contains("add"));

        assert!(
            mentions_subtract,
            "Error should mention the undeclared function 'subtract': {:?}",
            errors
        );
        assert!(
            !mentions_add,
            "Error should NOT mention the declared extern 'add' — it should \
             type-check successfully: {:?}",
            errors
        );
    }

    #[test]
    /// Calling an extern with the wrong number of arguments should produce
    /// an argument count mismatch error.
    ///
    /// Setup:
    ///   extern "npm:math" fn pow(base: Int, exp: Int) -> Int
    ///   fn bad() { pow(2) }  — missing second argument
    ///
    /// Expected: error mentioning argument count mismatch.
    ///
    /// Failure mode: The extern is not registered, so the error is
    /// "unknown variable: pow" instead of an argument count error.
    fn test_extern_wrong_arg_count_error() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();

        let extern_decl = Decl::Extern {
            source: "npm:math".to_string(),
            name: "pow".to_string(),
            params: vec![
                Param {
                    name: "base".to_string(),
                    type_: Some(Type::Named("Int".to_string())),
                },
                Param {
                    name: "exp".to_string(),
                    type_: Some(Type::Named("Int".to_string())),
                },
            ],
            return_type: Some(Type::Named("Int".to_string())),
            is_pub: true,
            span: dummy_span(),
        };

        // Call pow(2) — missing second argument
        let caller = Decl::Function {
            name: "bad".to_string(),
            params: vec![],
            return_type: None,
            body: Expr::Call {
                func: Box::new(Expr::Variable {
                    name: "pow".to_string(),
                    span: dummy_span(),
                }),
                args: vec![Expr::Literal {
                    value: LiteralValue::Int(2),
                    span: dummy_span(),
                }],
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        };

        let decls = vec![extern_decl, caller];
        let (_registry, errors) = pass.check(&decls);

        assert!(
            !errors.is_empty(),
            "Calling extern with wrong arg count should produce errors"
        );

        // The error should mention argument count, NOT "unknown variable"
        let has_count_error = errors.iter().any(|e| {
            e.message.contains("count")
                || e.message.contains("argument")
                || e.message.contains("expected")
        });
        assert!(
            has_count_error,
            "Expected an argument count mismatch error for extern call with \
             too few arguments, but got: {:?}",
            errors
        );
    }

    // ------------------------------------------------------------------
    // Refined type annotation tests (DWARF-60: Refinement Type System)
    //
    // WILL FAIL — RED PHASE
    //
    // These tests verify that refined type annotations on function parameters
    // are accepted by the typechecker. Currently, the parameter annotation
    // handler in pass.rs (~line 121-131) has a catch-all `_` arm that produces
    // "unsupported type annotation for parameter" for any type that isn't
    // Named or Generic. Type::Refined falls into this catch-all.
    //
    // They will fail until:
    //   1. The parameter annotation handler accepts Type::Refined
    //   2. The resolver registers TypeDef::Refined instead of erasing
    // ------------------------------------------------------------------

    #[test]
    /// A function parameter annotated with `Int(0..100)` should type-check
    /// without producing the "unsupported type annotation" error.
    ///
    /// Setup:
    ///   fn f(x: Int(0..100)) { 42 }
    ///
    /// Expected: no errors.
    ///
    /// Failure mode: The catch-all `_` arm in the parameter annotation handler
    /// produces "unsupported type annotation for parameter 'x'" because
    /// Type::Refined is not handled.
    fn test_refined_type_annotation_on_parameter() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();
        let decls = vec![Decl::Function {
            name: "f".to_string(),
            params: vec![Param {
                name: "x".to_string(),
                type_: Some(Type::Refined {
                    base: Box::new(Type::Named("Int".to_string())),
                    constraint: dwarf_syntax::hir::RefConstraint::Range { min: 0, max: 100 },
                }),
            }],
            return_type: None,
            body: Expr::Literal {
                value: LiteralValue::Int(42),
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        }];
        let (_registry, errors) = pass.check(&decls);
        assert!(
            errors.is_empty(),
            "Refined type annotation 'Int(0..100)' on parameter should be valid, \
             but got errors: {:?}",
            errors
        );
    }

    #[test]
    /// A function parameter annotated with a refined type should NOT produce
    /// the "unsupported type annotation" error specifically.
    ///
    /// This test checks the error message content to ensure the failure mode
    /// is the catch-all error, not some other issue.
    fn test_refined_type_does_not_produce_unsupported_error() {
        // WILL FAIL — RED PHASE
        let pass = TypeCheckPass::new();
        let decls = vec![Decl::Function {
            name: "clamp".to_string(),
            params: vec![Param {
                name: "value".to_string(),
                type_: Some(Type::Refined {
                    base: Box::new(Type::Named("Int".to_string())),
                    constraint: dwarf_syntax::hir::RefConstraint::Range { min: 0, max: 255 },
                }),
            }],
            return_type: None,
            body: Expr::Variable {
                name: "value".to_string(),
                span: dummy_span(),
            },
            is_pub: true,
            span: dummy_span(),
        }];
        let (_registry, errors) = pass.check(&decls);

        // Check that none of the errors mention "unsupported type annotation"
        let has_unsupported_error = errors
            .iter()
            .any(|e| e.message.contains("unsupported type annotation"));
        assert!(
            !has_unsupported_error,
            "Refined type annotation should NOT produce 'unsupported type annotation' \
             error, but got: {:?}",
            errors
        );
    }
}
