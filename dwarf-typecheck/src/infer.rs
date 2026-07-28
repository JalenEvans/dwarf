//! Expression type inference for the Dwarf compiler.
//!
//! Provides [`TypeEnv`] for tracking variable bindings and [`infer_expr`]
//! for inferring the types of HIR expressions.

use std::collections::HashMap;

use dwarf_syntax::hir::Type as HirType;
use dwarf_syntax::hir::{BinaryOp, Expr, LiteralValue, MatchArm, Param, Pat, Stmt, UnaryOp};

use crate::compat;
use crate::registry::TypeRegistry;
use crate::types::{FieldDef, TypeDef, TypeId};

/// Type environment mapping variable names to their inferred types.
#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: HashMap<String, TypeId>,
}

impl TypeEnv {
    /// Create an empty type environment.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Bind a variable name to a type ID.
    pub fn bind(&mut self, name: String, type_id: TypeId) {
        self.bindings.insert(name, type_id);
    }

    /// Look up the type of a variable by name.
    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        self.bindings.get(name).copied()
    }

    /// Returns `true` if no variables are bound.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Returns the number of bound variables.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve a named HIR type to a primitive TypeId.
///
/// Handles common casing conventions for built-in types.
fn resolve_hir_type_name(name: &str) -> Option<TypeId> {
    match name {
        "int" | "Int" => Some(0),
        "float" | "Float" => Some(1),
        "str" | "Str" | "string" | "String" => Some(2),
        "bool" | "Bool" => Some(3),
        "null" | "Null" => Some(4),
        _ => None,
    }
}

/// Resolve an HIR type annotation to a TypeId.
///
/// This is a minimal resolver for use during inference (e.g. lambda param
/// annotations). Named types must be primitives or user-defined types already
/// in the name map.
fn resolve_hir_type_param(hir_type: &HirType) -> Result<TypeId, String> {
    match hir_type {
        HirType::Named(name) => {
            resolve_hir_type_name(name).ok_or_else(|| format!("unknown type: {}", name))
        }
        HirType::Generic { base, args: _ } => {
            resolve_hir_type_name(base).ok_or_else(|| format!("unknown type: {}", base))
        }
        HirType::Record(_) => {
            Err("inline record types are not supported in parameter annotations".to_string())
        }
        HirType::Union(_) => {
            Err("inline union types are not supported in parameter annotations".to_string())
        }
        HirType::Func { .. } => {
            Err("function types are not supported in parameter annotations".to_string())
        }
        HirType::Refined { base, .. } => resolve_hir_type_param(base),
    }
}

/// Infer the type of an expression given a type environment.
///
/// Returns the `TypeId` of the inferred type, or an `Err` with a description
/// if inference fails (type mismatch, unknown variable, etc.).
///
/// The registry may be mutated to register anonymous types (e.g. function
/// types for lambdas, record types for record expressions).
pub fn infer_expr(
    expr: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    match expr {
        // 1. Literal expressions
        Expr::Literal { value, .. } => infer_literal(value),

        // 2. Variable references
        Expr::Variable { name, .. } => env
            .lookup(name)
            .ok_or_else(|| format!("unknown variable: {}", name)),

        // 3. Binary operations
        Expr::Binary { op, lhs, rhs, .. } => infer_binary(op, lhs, rhs, env, registry),

        // 4. Unary operations
        Expr::Unary { op, expr, .. } => infer_unary(op, expr, env, registry),

        // 5. Block expressions
        Expr::Block { stmts, .. } => infer_block(stmts, env, registry),

        // 6. If expressions
        Expr::If {
            cond, then, else_, ..
        } => infer_if(cond, then, else_.as_deref(), env, registry),

        // 7. Lambda expressions
        Expr::Lambda { params, body, .. } => infer_lambda(params, body, env, registry),

        // 8. Call expressions
        Expr::Call { func, args, .. } => infer_call(func, args, env, registry),

        // 9. Record expressions
        Expr::Record { fields, .. } => infer_record(fields, env, registry),

        // 10. Member access
        Expr::Member { obj, field, .. } => infer_member_access(obj, field, env, registry),

        // 11. Match expressions
        Expr::Match { expr, arms, .. } => infer_match(expr, arms, env, registry),

        // 12. Array expressions (List<T>)
        Expr::Array { items, .. } => infer_array(items, env, registry),

        // 13. Wildcard expressions (placeholder, infers to Null)
        Expr::Wildcard { .. } => infer_wildcard(),

        // 14. Variant expressions (e.g. None, Some(42))
        Expr::Variant { name, arg, .. } => infer_variant(name, arg.as_deref(), env, registry),

        // 15. Pipe expressions (lhs |> rhs)
        Expr::Pipe { lhs, rhs, .. } => infer_pipe(lhs, rhs, env, registry),

        // 16. Propagate expressions (?expr)
        Expr::Propagate { expr, .. } => infer_propagate(expr, env, registry),

        // 17. For loop expressions (for x in iterable { body })
        Expr::For {
            binding,
            iterable,
            body,
            ..
        } => infer_for(binding, iterable, body, env, registry),

        // 18. Assign expressions (target = value)
        Expr::Assign { target, value, .. } => infer_assign(target, value, env, registry),

        // 19. ForAll expression (property-based testing)
        Expr::ForAll {
            type_,
            binding,
            property,
            ..
        } => infer_forall(type_, binding, property, env, registry),

        // 20. AssertConsistent expression (pass-through)
        Expr::AssertConsistent { expr, .. } => infer_assert_consistent(expr, env, registry),
    }
}

// ---------------------------------------------------------------------------
// Inference helpers
// ---------------------------------------------------------------------------

/// Infer the type of a literal value.
fn infer_literal(value: &LiteralValue) -> Result<TypeId, String> {
    match value {
        LiteralValue::Int(_) => Ok(0),
        LiteralValue::Float(_) => Ok(1),
        LiteralValue::Str(_) | LiteralValue::RawStr(_) => Ok(2),
        LiteralValue::Bool(_) => Ok(3),
        LiteralValue::Null => Ok(4),
    }
}

/// Infer the type of a binary operation.
///
/// Rules:
/// - Arithmetic (Add/Sub/Mul/Div): both operands Int → Int.
///   Add also supports Str + Str → Str (string concatenation).
/// - Comparison (Eq/Ne/Lt/Gt/Le/Ge): both operands same type → Bool.
/// - Logical (And/Or): both operands Bool → Bool.
fn infer_binary(
    op: &BinaryOp,
    lhs: &Expr,
    rhs: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let lhs_type = infer_expr(lhs, env, registry)?;
    let rhs_type = infer_expr(rhs, env, registry)?;

    match op {
        // String concatenation: Str + Str → Str
        BinaryOp::Add if lhs_type == 2 && rhs_type == 2 => Ok(2),
        // Arithmetic: both must be Int
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            if lhs_type == 0 && rhs_type == 0 {
                Ok(0)
            } else {
                Err(format!(
                    "type mismatch: expected Int, got lhs={} rhs={}",
                    lhs_type, rhs_type
                ))
            }
        }
        // Comparison: both must be the same type → Bool
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
            if lhs_type == rhs_type {
                Ok(3)
            } else {
                Err(format!(
                    "type mismatch: cannot compare lhs={} with rhs={}",
                    lhs_type, rhs_type
                ))
            }
        }
        // Logical: both must be Bool
        BinaryOp::And | BinaryOp::Or => {
            if lhs_type == 3 && rhs_type == 3 {
                Ok(3)
            } else {
                Err(format!(
                    "type mismatch: expected Bool, got lhs={} rhs={}",
                    lhs_type, rhs_type
                ))
            }
        }
    }
}

/// Infer the type of a unary operation.
///
/// Rules:
/// - Neg: operand must be Int → Int.
/// - Not: operand must be Bool → Bool.
fn infer_unary(
    op: &UnaryOp,
    expr: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let operand_type = infer_expr(expr, env, registry)?;

    match op {
        UnaryOp::Neg => {
            if operand_type == 0 {
                Ok(0)
            } else {
                Err(format!(
                    "type mismatch: expected Int for negation, got {}",
                    operand_type
                ))
            }
        }
        UnaryOp::Not => {
            if operand_type == 3 {
                Ok(3)
            } else {
                Err(format!(
                    "type mismatch: expected Bool for not, got {}",
                    operand_type
                ))
            }
        }
    }
}

/// Infer the type of a block expression.
///
/// Statements are processed in order. `let` bindings extend the environment
/// for subsequent statements. The type of the last expression becomes the
/// block's type. Empty blocks default to Int (0).
fn infer_block(
    stmts: &[Stmt],
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let mut local_env = env.clone();
    let mut last_type = 0;

    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr) => {
                last_type = infer_expr(expr, &local_env, registry)?;
            }
            Stmt::Let(pat, expr) => {
                let type_id = infer_expr(expr, &local_env, registry)?;
                bind_pat(pat, type_id, &mut local_env);
                last_type = type_id;
            }
        }
    }

    Ok(last_type)
}

/// Bind pattern variables to their inferred types in the environment.
fn bind_pat(pat: &Pat, type_id: TypeId, env: &mut TypeEnv) {
    match pat {
        Pat::Variable(name) => {
            env.bind(name.clone(), type_id);
        }
        Pat::Wildcard | Pat::Literal(_) => {
            // No binding needed.
        }
        Pat::Variant { name: _, arg } => {
            if let Some(arg_pat) = arg.as_ref() {
                bind_pat(arg_pat, type_id, env);
            }
        }
        Pat::Record { fields, rest: _ } => {
            for (_, field_pat) in fields {
                bind_pat(field_pat, type_id, env);
            }
        }
    }
}

/// Infer the type of an if expression.
///
/// The condition must be Bool. Both arms must have the same type.
fn infer_if(
    cond: &Expr,
    then: &Expr,
    else_: Option<&Expr>,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let cond_type = infer_expr(cond, env, registry)?;
    if cond_type != 3 {
        return Err("if condition must be Bool".to_string());
    }

    let then_type = infer_expr(then, env, registry)?;

    if let Some(else_expr) = else_ {
        let else_type = infer_expr(else_expr, env, registry)?;
        if then_type != else_type {
            return Err("if arms have mismatched types".to_string());
        }
    }

    Ok(then_type)
}

/// Infer the type of a lambda expression.
///
/// Each parameter with a type annotation is resolved and added to the
/// environment. The body is then inferred, and a `Func` type is registered
/// in the registry.
fn infer_lambda(
    params: &[Param],
    body: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let mut param_types = Vec::new();
    let mut local_env = env.clone();

    for param in params {
        let param_type = match &param.type_ {
            Some(hir_type) => resolve_hir_type_param(hir_type)?,
            None => return Err("lambda parameter without type annotation".to_string()),
        };
        param_types.push(param_type);
        local_env.bind(param.name.clone(), param_type);
    }

    let return_type = infer_expr(body, &local_env, registry)?;

    let func_type = TypeDef::Func(param_types, return_type);
    let type_id = registry.register(func_type);
    Ok(type_id)
}

/// Infer the type of a call expression.
///
/// The callee must be a function type. The argument types must match the
/// parameter types in count and type. Returns the function's return type.
fn infer_call(
    func: &Expr,
    args: &[Expr],
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let callee_type_id = infer_expr(func, env, registry)?;

    // Clone the relevant data to avoid holding an immutable borrow on registry
    // while passing &mut registry to infer_expr.
    let (param_types, return_type) = match registry.get(callee_type_id) {
        Some(TypeDef::Func(params, ret)) => (params.clone(), *ret),
        Some(_) => return Err("callee is not a function".to_string()),
        None => return Err(format!("unknown callee type ID: {}", callee_type_id)),
    };

    if args.len() != param_types.len() {
        return Err(format!(
            "argument count mismatch: expected {}, got {}",
            param_types.len(),
            args.len()
        ));
    }

    for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
        let arg_type = infer_expr(arg, env, registry)?;
        if arg_type != *param_type {
            return Err(format!(
                "argument {} type mismatch: expected {}, got {}",
                i, param_type, arg_type
            ));
        }
    }

    Ok(return_type)
}

/// Infer the type of a record expression.
///
/// Each field's expression is inferred and a `Record` type is registered
/// in the registry.
fn infer_record(
    fields: &[(String, Expr)],
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let mut field_defs = Vec::new();
    for (name, expr) in fields {
        let field_type = infer_expr(expr, env, registry)?;
        field_defs.push(FieldDef {
            name: name.clone(),
            type_id: field_type,
        });
    }

    let record_type = TypeDef::Record(field_defs);
    let type_id = registry.register(record_type);
    Ok(type_id)
}

/// Infer the type of a member access expression (e.g. `point.x`).
///
/// The object must be a record type. Returns the type of the named field.
fn infer_member_access(
    obj: &Expr,
    field: &str,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let obj_type_id = infer_expr(obj, env, registry)?;

    let obj_def = registry
        .get(obj_type_id)
        .ok_or_else(|| format!("unknown type ID: {}", obj_type_id))?;

    match obj_def {
        TypeDef::Record(fields) => fields
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.type_id)
            .ok_or_else(|| format!("record has no field named '{}'", field)),
        _ => Err("member access on non-record type".to_string()),
    }
}

/// Infer the type of a match expression.
///
/// The scrutinee type is validated, and all arm bodies must have the same type.
fn infer_match(
    expr: &Expr,
    arms: &[MatchArm],
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Validate the scrutinee is a known expression.
    let _scrutinee_type = infer_expr(expr, env, registry)?;

    if arms.is_empty() {
        return Err("match expression has no arms".to_string());
    }

    let first_type = infer_expr(&arms[0].body, env, registry)?;

    for arm in arms.iter().skip(1) {
        let arm_type = infer_expr(&arm.body, env, registry)?;
        if arm_type != first_type {
            return Err(format!(
                "match arms have mismatched types: {} vs {}",
                first_type, arm_type
            ));
        }
    }

    Ok(first_type)
}

/// Infer the type of an array literal expression.
///
/// All elements must have the same type (checked via structural compatibility).
/// Empty arrays infer to `List<Null>`.
///
/// Returns a `GenericInstance` with `base = List` (lazily registered empty
/// record) and `args = [element_type]`.
fn infer_array(
    items: &[Expr],
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let list_base = registry.get_or_create_list_base();

    if items.is_empty() {
        // Empty array: List<Null>
        return Ok(registry.register(TypeDef::GenericInstance {
            base: list_base,
            args: vec![4], // Null as element type
        }));
    }

    // Infer first element type
    let elem_type = infer_expr(&items[0], env, registry)?;

    // Check remaining elements are compatible with the first
    for item in &items[1..] {
        let t = infer_expr(item, env, registry)?;
        if !compat::check(registry, elem_type, t).compatible {
            return Err(format!(
                "Type mismatch in array literal: expected type {}, got {}",
                elem_type, t
            ));
        }
    }

    // Return List<elem_type>
    Ok(registry.register(TypeDef::GenericInstance {
        base: list_base,
        args: vec![elem_type],
    }))
}

/// Infer the type of a wildcard expression `_`.
///
/// A wildcard is a placeholder that always infers to the Null type (4).
fn infer_wildcard() -> Result<TypeId, String> {
    Ok(4) // Null type
}

/// Infer the type of a variant expression (e.g. `None`, `Some(42)`).
///
/// Searches all registered types in the registry for a `TypeDef::Union`
/// containing a variant with the matching `name`. If found, validates the
/// argument (payload) against the variant's expected type and returns the
/// union's `TypeId`.
///
/// # Errors
///
/// - `"Unknown variant '{name}'"` if no registered union contains a variant
///   with the given name.
/// - `"Variant '{name}' does not accept an argument"` if `arg` is `Some` but
///   the variant definition has no expected payload type.
/// - `"Variant '{name}' requires an argument"` if `arg` is `None` but the
///   variant definition expects a payload type.
/// - Compat check errors if the inferred arg type does not match the expected
///   payload type.
fn infer_variant(
    name: &str,
    arg: Option<&Expr>,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Search every type in the registry for a Union containing this variant.
    // We extract variant info first to avoid holding a borrow on registry
    // while calling infer_expr (which needs &mut registry).
    for id in 0..registry.len() {
        let resolved_id = registry.resolve(id);

        // Look for a matching variant and extract its expected payload type.
        // The borrow on registry is dropped before we call infer_expr below.
        let expected_type = match registry.get(resolved_id) {
            Some(TypeDef::Union(variants)) => {
                variants.iter().find(|v| v.name == name).map(|v| v.type_id)
            }
            _ => None,
        };

        if let Some(expected_type) = expected_type {
            match (arg, expected_type) {
                (Some(arg_expr), Some(expected)) => {
                    // Variant with payload: validate arg type
                    let inferred_arg_type = infer_expr(arg_expr, env, registry)?;
                    let compat_result = compat::check(registry, expected, inferred_arg_type);
                    if !compat_result.compatible {
                        return Err(format!(
                            "type mismatch for variant '{}': expected type {}, got {}",
                            name, expected, inferred_arg_type
                        ));
                    }
                    return Ok(resolved_id);
                }
                (Some(_), None) => {
                    // Variant is unit (no payload) but an argument was provided
                    return Err(format!("Variant '{}' does not accept an argument", name));
                }
                (None, Some(_)) => {
                    // Variant expects a payload but no argument was provided
                    return Err(format!("Variant '{}' requires an argument", name));
                }
                (None, None) => {
                    // Unit variant without argument: valid
                    return Ok(resolved_id);
                }
            }
        }
    }

    // No union found containing this variant name
    Err(format!("Unknown variant '{}'", name))
}

// ---------------------------------------------------------------------------
// Pipe and Propagate inference
// ---------------------------------------------------------------------------

/// Infer the type of a pipe expression `lhs |> rhs`.
///
/// The pipe operator is syntactic sugar for `rhs(lhs)`:
/// 1. Infer `lhs` type — this is the argument value.
/// 2. Infer `rhs` — it must be a `TypeDef::Func(param_types, return_type)`.
/// 3. Validate that `lhs` type is compatible with the first parameter of `rhs`.
/// 4. Return `rhs`'s return type.
fn infer_pipe(
    lhs: &Expr,
    rhs: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let arg_type = infer_expr(lhs, env, registry)?;
    let rhs_type = infer_expr(rhs, env, registry)?;
    let resolved = registry.resolve(rhs_type);

    // Clone param data to avoid holding an immutable borrow on registry
    let (param_types, return_type) = match registry.get(resolved) {
        Some(TypeDef::Func(params, ret)) => (params.clone(), *ret),
        Some(_) => return Err("Pipe target must be a function".to_string()),
        None => return Err("Pipe target type not found in registry".to_string()),
    };

    if param_types.is_empty() {
        return Err("Pipe target must accept at least one parameter".to_string());
    }

    let compat_result = compat::check(registry, param_types[0], arg_type);
    if !compat_result.compatible {
        return Err(
            "Type mismatch in pipe: expected argument type compatible with parameter type"
                .to_string(),
        );
    }

    Ok(return_type)
}

/// Infer the type of a propagate expression `?expr`.
///
/// The propagate operator unwraps a `Result<T, E>` or `Option<T>`:
/// 1. Infer the inner `expr` type.
/// 2. Look it up — it must be a `TypeDef::Union` with variants that follow
///    the Result/Option pattern.
/// 3. Find the "success" variant (`Ok` or `Some`) and extract its payload type.
/// 4. Return the payload type.
/// 5. If it's not a union (or doesn't have Ok/Some variants), return an error.
fn infer_propagate(
    expr: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    let inner_type = infer_expr(expr, env, registry)?;
    let resolved = registry.resolve(inner_type);

    match registry.get(resolved) {
        Some(TypeDef::Union(variants)) => {
            // Look for a "success" variant (Ok or Some) with a payload
            for variant in variants {
                let success_names = ["Ok", "Some"];
                if success_names.contains(&variant.name.as_str()) {
                    if let Some(payload_type) = variant.type_id {
                        return Ok(payload_type);
                    }
                }
            }
            // Check for unit success variants (no payload)
            for variant in variants {
                let success_names = ["Ok", "Some"];
                if success_names.contains(&variant.name.as_str()) {
                    return Err("Cannot propagate a unit variant (no payload)".to_string());
                }
            }
            Err("Propagate target must be a Result or Option type".to_string())
        }
        Some(_) => Err("Propagate target must be a union type (Result/Option)".to_string()),
        None => Err("Propagate target type not found".to_string()),
    }
}

// ---------------------------------------------------------------------------
// For loop and Assign inference (Phase 4)
// ---------------------------------------------------------------------------

/// Infer the type of a for loop expression `for binding in iterable { body }`.
///
/// Semantics:
/// 1. Infer the `iterable` type — it must be a `List<T>` (GenericInstance
///    with base == List base and args containing the element type).
/// 2. Extract the element type `T` from the List's generic arguments.
/// 3. Create a new scope with the `binding` variable mapped to `T`.
/// 4. Infer the `body` expression in the new scope.
/// 5. Return Null (4) — for loops are control-flow expressions that don't
///    produce a meaningful value.
///
/// # Errors
///
/// - If the iterable is not a `List<T>`, returns an error.
/// - If the binding pattern is unsupported (not Variable or Wildcard),
///   returns an error.
fn infer_for(
    binding: &Pat,
    iterable: &Expr,
    body: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Infer the iterable type
    let iter_type = infer_expr(iterable, env, registry)?;
    let resolved = registry.resolve(iter_type);

    // Get the list base before matching to avoid conflicting borrows
    let list_base = registry.get_or_create_list_base();

    match registry.get(resolved) {
        Some(TypeDef::GenericInstance { base, args }) => {
            if *base != list_base {
                return Err("For loop iterable must be a List".to_string());
            }
            if args.is_empty() {
                return Err("For loop iterable List has no element type".to_string());
            }
            let elem_type = args[0];

            // Create new scope with binding
            let mut inner_env = env.clone();
            match binding {
                Pat::Variable(name) => {
                    inner_env.bind(name.clone(), elem_type);
                }
                Pat::Wildcard => {
                    // Binding ignored
                }
                _ => {
                    return Err("Unsupported binding pattern in for loop".to_string());
                }
            }

            // Infer body in new scope
            infer_expr(body, &inner_env, registry)?;

            // For loops return Null (unit / control-flow)
            Ok(4)
        }
        Some(_) => Err("For loop iterable must be a List".to_string()),
        None => Err("For loop iterable type not found".to_string()),
    }
}

/// Infer the type of an assignment expression `target = value`.
///
/// Semantics:
/// 1. Infer the `target` type (e.g. looking up a variable in the environment).
/// 2. Infer the `value` type.
/// 3. Check that the two types are structurally compatible.
/// 4. Return Null (4) — assignment is a statement whose value is discarded.
///
/// # Errors
///
/// - If the target type and value type are incompatible, returns an error.
fn infer_assign(
    target: &Expr,
    value: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Infer the target type (e.g. resolves a variable reference)
    let target_type = infer_expr(target, env, registry)?;

    // Infer the value type
    let value_type = infer_expr(value, env, registry)?;

    // Check compatibility: value must be assignable to target
    if !compat::check(registry, target_type, value_type).compatible {
        return Err(
            "Assignment type mismatch: target and value types are incompatible".to_string(),
        );
    }

    // Assignment returns Null (unit)
    Ok(4)
}

// ---------------------------------------------------------------------------
// ForAll and AssertConsistent inference (Phase 5)
// ---------------------------------------------------------------------------

/// Infer the type of a forAll expression `forAll(x: Int) { property }`.
///
/// Semantics:
/// 1. Resolve the HIR type annotation to a TypeId (via `resolve_hir_type_name`).
/// 2. Bind the variable to that type in a new scope.
/// 3. Infer the property expression in the new scope.
/// 4. Verify the property type is Bool (TypeId 3).
/// 5. Return Bool.
///
/// # Errors
///
/// - If the type annotation is unknown or unsupported, returns an error.
/// - If the binding pattern is unsupported (not Variable or Wildcard),
///   returns an error.
/// - If the property does not evaluate to Bool, returns an error.
fn infer_forall(
    type_: &HirType,
    binding: &Pat,
    property: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Resolve the type annotation (only named types supported for now)
    let bound_type = match type_ {
        HirType::Named(name) => resolve_hir_type_name(name.as_str())
            .ok_or_else(|| format!("Unknown type '{}' in forAll", name)),
        HirType::Refined { base, .. } => {
            // Refined types like `Int(0..100)` delegate to their base
            match base.as_ref() {
                HirType::Named(name) => resolve_hir_type_name(name.as_str())
                    .ok_or_else(|| format!("Unknown base type '{}' in forAll", name)),
                _ => Err(
                    "Only named base types are supported in forAll refined bindings".to_string(),
                ),
            }
        }
        _ => Err("Only named types are supported in forAll bindings".to_string()),
    }?;

    // Create new scope with binding
    let mut inner_env = env.clone();
    match binding {
        Pat::Variable(name) => {
            inner_env.bind(name.clone(), bound_type);
        }
        Pat::Wildcard => {
            // Binding ignored
        }
        _ => {
            return Err("Unsupported binding pattern in forAll".to_string());
        }
    }

    // Infer property in the new scope
    let property_type = infer_expr(property, &inner_env, registry)?;

    // Property must be Bool
    if property_type != 3 {
        return Err("forAll property must evaluate to Bool".to_string());
    }

    Ok(3) // Bool
}

/// Infer the type of an assertConsistent expression `assertConsistent(expr)`.
///
/// Semantics:
/// - Pure pass-through: infer the inner expression and return its type unchanged.
///
/// This is a no-op from the type system's perspective — it's a hint to the
/// compiler that the expression should produce consistent results across all
/// targets.
fn infer_assert_consistent(
    expr: &Expr,
    env: &TypeEnv,
    registry: &mut TypeRegistry,
) -> Result<TypeId, String> {
    // Pure pass-through — defer to inner expression's type
    infer_expr(expr, env, registry)
}
