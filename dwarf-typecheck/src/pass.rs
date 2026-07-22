//! TypeCheckPass — the type-checking compilation pass.
//!
//! This pass runs the full type-check pipeline on parsed HIR declarations:
//! resolution of type declarations, then expression type inference.

use dwarf_syntax::hir::Decl;

use crate::error::{TypeCheckError, TYPE_ERROR_CODES};
use crate::infer::{infer_expr, TypeEnv};
use crate::registry::TypeRegistry;
use crate::resolve;

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

        // Phase 1: Register all type declarations (RecordDef, UnionDef, TypeDef)
        let _result = resolve::register_decls(&mut registry, decls);
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
}
