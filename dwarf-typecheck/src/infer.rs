//! Expression type inference for the Dwarf compiler.
//!
//! Provides [`TypeEnv`] for tracking variable bindings and [`infer_expr`]
//! for inferring the types of HIR expressions.

use std::collections::HashMap;

use dwarf_syntax::hir::Type as HirType;
use dwarf_syntax::hir::{BinaryOp, Expr, LiteralValue, MatchArm, Param, Pat, Stmt, UnaryOp};

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

        // 12. Other expressions (placeholder stubs)
        Expr::Pipe { .. }
        | Expr::Propagate { .. }
        | Expr::For { .. }
        | Expr::ForAll { .. }
        | Expr::Assign { .. }
        | Expr::Array { .. }
        | Expr::Wildcard { .. }
        | Expr::Variant { .. } => Ok(0),
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
