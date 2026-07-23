//! A TypeScript backend that produces real TypeScript code from LIR.
//!
//! This backend implements [`EmitterBackend`] with `Output = String` and
//! produces TypeScript source code suitable for further compilation or
//! direct execution in a TypeScript runtime.
//!
//! # Status
//!
//! This backend is a work in progress. Only the following methods have real
//! implementations:
//!
//! - `emit_literal` — produces TypeScript literal syntax
//! - `emit_binary_op` — produces TypeScript binary operators (with spaces)
//! - `emit_unary_op` — produces TypeScript unary operators
//! - `emit_target_hint` — produces target-hint strings
//! - `emit_effect` — produces effect-annotation strings
//! - `emit_type` — delegates to [`TypeScriptMapper`]
//! - `emit_pat` — produces TypeScript pattern syntax
//!
//! All [`EmitterBackend`] methods now have real implementations.

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;
use crate::format::CodeBuffer;
use crate::imports::ImportManager;
use crate::types::{TypeMapper, TypeScriptMapper};

/// A backend that emits TypeScript code from LIR declarations.
///
/// Each method accepts a reference to a LIR construct and produces a
/// TypeScript string representation. The `Output` type is `String`,
/// containing the complete emitted module.
pub struct TypeScriptBackend {
    buffer: CodeBuffer,
    type_mapper: TypeScriptMapper,
    imports: ImportManager,
    indent_level: usize,
}

impl TypeScriptBackend {
    /// Create a new `TypeScriptBackend` with an empty buffer, a fresh
    /// [`TypeScriptMapper`], and an empty [`ImportManager`].
    pub fn new() -> Self {
        Self {
            buffer: CodeBuffer::new(),
            type_mapper: TypeScriptMapper,
            imports: ImportManager::new(),
            indent_level: 0,
        }
    }

    /// Return a reference to the internal [`CodeBuffer`].
    pub fn buffer(&self) -> &CodeBuffer {
        &self.buffer
    }

    /// Return a mutable reference to the internal [`CodeBuffer`].
    pub fn buffer_mut(&mut self) -> &mut CodeBuffer {
        &mut self.buffer
    }

    /// Consume the backend and return the accumulated output as a `String`.
    pub fn into_output(self) -> String {
        self.buffer.into_string()
    }
}

impl Default for TypeScriptBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterBackend for TypeScriptBackend {
    type Output = String;

    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
        if decls.is_empty() {
            return Ok(String::new());
        }
        let mut buf = CodeBuffer::new();
        for (i, decl) in decls.iter().enumerate() {
            if i > 0 {
                buf.push_empty();
            }
            let decl_str = self.emit_decl(decl)?;
            // Push each line of the decl with proper indentation
            for line in decl_str.lines() {
                buf.push_line(line);
            }
        }
        Ok(buf.into_string().trim_end().to_string())
    }

    fn emit_decl(&mut self, decl: &LirDecl) -> Result<String, EmitterError> {
        match decl {
            LirDecl::Function {
                name,
                params,
                return_type,
                body,
                effect,
                hint,
                ..
            } => {
                let async_prefix = if *hint == TargetHint::Async || *effect == Effect::Async {
                    "async "
                } else {
                    ""
                };
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| match &p.type_ {
                        Some(ty) => format!("{}: {}", p.name, self.type_mapper.map_type(ty)),
                        None => p.name.clone(),
                    })
                    .collect();
                let ret_str = match return_type {
                    Some(ty) => format!(": {}", self.type_mapper.map_type(ty)),
                    None => String::new(),
                };
                let header = format!(
                    "{}function {}({}){}",
                    async_prefix,
                    name,
                    params_str.join(", "),
                    ret_str
                );

                // If the body is a Block, use proper multi-line formatting
                match body {
                    LirExpr::Block { stmts, .. } => {
                        let mut buf = CodeBuffer::new();
                        buf.push_line(format!("{} {{", header));
                        buf.indent();
                        for (i, stmt) in stmts.iter().enumerate() {
                            let is_last = i == stmts.len() - 1;
                            match stmt {
                                LirStmt::Let { pat, value } => {
                                    let val_str = self.emit_expr(value)?;
                                    let pat_str = self.emit_pat_inline(pat);
                                    buf.push_line(format!("let {} = {};", pat_str, val_str));
                                }
                                LirStmt::Expr(expr) => {
                                    let expr_str = self.emit_expr(expr)?;
                                    if is_last {
                                        buf.push_line(format!("return {};", expr_str));
                                    } else {
                                        buf.push_line(format!("{};", expr_str));
                                    }
                                }
                            }
                        }
                        buf.dedent();
                        buf.push_line("}");
                        Ok(buf.into_string().trim_end().to_string())
                    }
                    other => {
                        let body_str = self.emit_expr(other)?;
                        Ok(format!("{} {{ return {}; }}", header, body_str))
                    }
                }
            }
            LirDecl::RecordDef { name, fields, .. } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|f| format!("{}: {};", f.name, self.type_mapper.map_type(&f.type_)))
                    .collect();
                Ok(format!("interface {} {{ {} }}", name, fields_str.join(" ")))
            }
            LirDecl::UnionDef { name, variants, .. } => {
                let variants_str: Vec<String> = variants
                    .iter()
                    .map(|v| match &v.arg {
                        Some(_ty) => v.name.clone(),
                        None => v.name.clone(),
                    })
                    .collect();
                Ok(format!("type {} = {};", name, variants_str.join(" | ")))
            }
        }
    }

    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        match expr {
            LirExpr::Literal { value, .. } => self.emit_literal(value),
            LirExpr::Variable { name, .. } => Ok(name.clone()),
            LirExpr::Call {
                func, args, hint, ..
            } => {
                let func_str = self.emit_expr(func)?;
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.emit_expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let call = format!("{}({})", func_str, args_str.join(", "));
                if *hint == TargetHint::Async {
                    Ok(format!("await {}", call))
                } else {
                    Ok(call)
                }
            }
            LirExpr::Member {
                obj, field, hint, ..
            } => {
                let obj_str = self.emit_expr(obj)?;
                let op = if *hint == TargetHint::Optional {
                    "?."
                } else {
                    "."
                };
                Ok(format!("{}{}{}", obj_str, op, field))
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                let cond_str = self.emit_expr(cond)?;
                let then_str = self.emit_expr(then)?;
                match else_ {
                    Some(else_expr) => {
                        let else_str = self.emit_expr(else_expr)?;
                        Ok(format!("{} ? {} : {}", cond_str, then_str, else_str))
                    }
                    None => Ok(then_str),
                }
            }
            LirExpr::Match {
                expr, arms, ..
            } => {
                let expr_str = self.emit_expr(expr)?;
                if arms.is_empty() {
                    return Ok(String::new());
                }
                let mut chain = String::new();
                for (i, arm) in arms.iter().enumerate() {
                    let body_str = self.emit_expr(&arm.body)?;
                    let is_last = i == arms.len() - 1;
                    let is_wildcard_default = is_last && matches!(arm.pattern, LirPat::Wildcard);

                    if is_wildcard_default {
                        if chain.is_empty() {
                            chain = body_str;
                        } else {
                            chain = format!("{} : {}", chain, body_str);
                        }
                    } else {
                        let pat_str = match &arm.pattern {
                            LirPat::Literal(lit) => self.emit_literal(lit)?,
                            LirPat::Wildcard => "_".to_string(),
                            LirPat::Variable(name) => name.clone(),
                            LirPat::Variant { name, .. } => format!("\"{}\"", name),
                            LirPat::Record { .. } => "_".to_string(),
                        };
                        let condition = format!("{} === {}", expr_str, pat_str);
                        if chain.is_empty() {
                            chain = format!("{} ? {}", condition, body_str);
                        } else {
                            chain = format!("{} : {} ? {}", chain, condition, body_str);
                        }
                    }
                }
                Ok(chain)
            }
            LirExpr::Block { stmts, .. } => self.emit_block_body(stmts),
            LirExpr::Assign { target, value, .. } => {
                let target_str = self.emit_expr(target)?;
                let value_str = self.emit_expr(value)?;
                Ok(format!("{} = {}", target_str, value_str))
            }
            LirExpr::Lambda { params, body, .. } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| match &p.type_ {
                        Some(ty) => format!("{}: {}", p.name, self.type_mapper.map_type(ty)),
                        None => p.name.clone(),
                    })
                    .collect();
                let body_str = self.emit_expr(body)?;
                Ok(format!("({}) => {}", params_str.join(", "), body_str))
            }
            LirExpr::Record { fields, .. } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, expr)| {
                        let val = self.emit_expr(expr)?;
                        Ok(format!("{}: {}", name, val))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{{ {} }}", fields_str.join(", ")))
            }
            LirExpr::Variant { name, arg, .. } => match arg {
                Some(expr) => {
                    let val = self.emit_expr(expr)?;
                    Ok(format!("{{ tag: \"{}\", value: {} }}", name, val))
                }
                None => Ok(format!("\"{}\"", name)),
            },
            LirExpr::Array { items, .. } => {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|i| self.emit_expr(i))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("[{}]", items_str.join(", ")))
            }
            LirExpr::Binary { op, lhs, rhs, .. } => {
                let lhs_str = self.emit_expr(lhs)?;
                let rhs_str = self.emit_expr(rhs)?;
                let op_str = self.emit_binary_op(op)?;
                Ok(format!("{}{}{}", lhs_str, op_str, rhs_str))
            }
            LirExpr::Unary { op, expr, .. } => {
                let expr_str = self.emit_expr(expr)?;
                let op_str = self.emit_unary_op(op)?;
                Ok(format!("{}{}", op_str, expr_str))
            }
            LirExpr::Wildcard { .. } => Ok("_".to_string()),
        }
    }

    fn emit_pat(&mut self, pat: &LirPat) -> Result<String, EmitterError> {
        match pat {
            LirPat::Wildcard => Ok("_".to_string()),
            LirPat::Literal(lit) => self.emit_literal(lit),
            LirPat::Variable(name) => Ok(name.clone()),
            LirPat::Variant { name, arg } => match arg {
                Some(a) => {
                    let inner = self.emit_pat(a)?;
                    Ok(format!("{}: {}", name, inner))
                }
                None => Ok(name.clone()),
            },
            LirPat::Record { fields, rest } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, pat)| {
                        let p = self.emit_pat(pat)?;
                        Ok(format!("{}: {}", name, p))
                    })
                    .collect::<Result<Vec<_>, EmitterError>>()?;
                let rest_str = if *rest { ", ..." } else { "" };
                Ok(format!("{{ {}{} }}", fields_str.join(", "), rest_str))
            }
        }
    }

    fn emit_type(&mut self, ty: &Type) -> Result<String, EmitterError> {
        Ok(self.type_mapper.map_type(ty))
    }

    fn emit_literal(&mut self, lit: &LirLiteral) -> Result<String, EmitterError> {
        match lit {
            LirLiteral::Int(v) => Ok(format!("{v}")),
            LirLiteral::Float(v) => Ok(format!("{v}")),
            LirLiteral::Str(v) => Ok(format!("\"{v}\"")),
            LirLiteral::Bool(v) => Ok(format!("{v}")),
            LirLiteral::Null => Ok("null".into()),
        }
    }

    fn emit_binary_op(&mut self, op: &LirBinaryOp) -> Result<String, EmitterError> {
        match op {
            LirBinaryOp::Add => Ok(" + ".into()),
            LirBinaryOp::Sub => Ok(" - ".into()),
            LirBinaryOp::Mul => Ok(" * ".into()),
            LirBinaryOp::Div => Ok(" / ".into()),
            LirBinaryOp::Eq => Ok(" === ".into()),
            LirBinaryOp::Ne => Ok(" !== ".into()),
            LirBinaryOp::Lt => Ok(" < ".into()),
            LirBinaryOp::Gt => Ok(" > ".into()),
            LirBinaryOp::Le => Ok(" <= ".into()),
            LirBinaryOp::Ge => Ok(" >= ".into()),
            LirBinaryOp::And => Ok(" && ".into()),
            LirBinaryOp::Or => Ok(" || ".into()),
        }
    }

    fn emit_unary_op(&mut self, op: &LirUnaryOp) -> Result<String, EmitterError> {
        match op {
            LirUnaryOp::Neg => Ok("-".into()),
            LirUnaryOp::Not => Ok("!".into()),
        }
    }

    fn emit_target_hint(&mut self, hint: &TargetHint) -> Result<String, EmitterError> {
        match hint {
            TargetHint::None => Ok(String::new()),
            TargetHint::Async => Ok("async ".into()),
            TargetHint::Optional => Ok("?".into()),
            TargetHint::Result => Ok(String::new()),
            TargetHint::ReactComponent => Ok(String::new()),
        }
    }

    fn emit_effect(&mut self, effect: &Effect) -> Result<String, EmitterError> {
        match effect {
            Effect::Pure => Ok(String::new()),
            Effect::Async => Ok("await ".into()),
            Effect::Impure => Ok(String::new()),
        }
    }
}

// ------------------------------------------------------------------
// Internal helpers on TypeScriptBackend
// ------------------------------------------------------------------

impl TypeScriptBackend {
    /// Emit a block body (stmts) as a single-line `{ ... }` string.
    ///
    /// For Let statements we use `emit_pat_inline` which produces a slightly
    /// different format than the trait `emit_pat` method — it uses `()` for
    /// variant args and `..` for record rest, matching TypeScript destructuring
    /// conventions.
    fn emit_block_body(&mut self, stmts: &[LirStmt]) -> Result<String, EmitterError> {
        if stmts.is_empty() {
            return Ok("{}".to_string());
        }
        let mut parts: Vec<String> = Vec::new();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            match stmt {
                LirStmt::Let { pat, value } => {
                    let val_str = self.emit_expr(value)?;
                    let pat_str = self.emit_pat_inline(pat);
                    parts.push(format!("let {} = {}", pat_str, val_str));
                }
                LirStmt::Expr(expr) => {
                    let expr_str = self.emit_expr(expr)?;
                    if is_last {
                        parts.push(format!("return {}", expr_str));
                    } else {
                        parts.push(expr_str);
                    }
                }
            }
        }
        Ok(format!("{{ {}; }}", parts.join("; ")))
    }

    /// Inline pattern emission helper for `let` statement destructuring.
    ///
    /// This produces a slightly different format than the trait `emit_pat`
    /// method: it uses `Variant(inner)` syntax (matching TypeScript
    /// destructuring) instead of `Variant: inner`, and `..` for record rest
    /// instead of `...`.
    fn emit_pat_inline(&mut self, pat: &LirPat) -> String {
        match pat {
            LirPat::Wildcard => "_".to_string(),
            LirPat::Literal(lit) => self
                .emit_literal(lit)
                .unwrap_or_else(|_| "_".to_string()),
            LirPat::Variable(name) => name.clone(),
            LirPat::Variant { name, arg } => match arg {
                Some(arg_pat) => format!("{}({})", name, self.emit_pat_inline(arg_pat)),
                None => name.clone(),
            },
            LirPat::Record { fields, rest } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(fname, pat)| format!("{}: {}", fname, self.emit_pat_inline(pat)))
                    .collect();
                let mut result = format!("{{ {} ", fields_str.join(", "));
                if *rest {
                    result.push_str(", ..");
                }
                result.push('}');
                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::{
        Effect, LirArm, LirBinaryOp, LirField, LirLiteral, LirParam, LirUnaryOp, LirVariant,
        TargetHint,
    };
    use dwarf_syntax::hir::Type;
    use dwarf_syntax::span::Span;

    // ==================================================================
    // Helpers
    // ==================================================================

    fn s() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint_none() -> TargetHint {
        TargetHint::None
    }

    // ==================================================================
    // Creation tests
    // ==================================================================

    #[test]
    fn test_ts_backend_new() {
        let backend = TypeScriptBackend::new();
        assert!(
            backend.buffer.is_empty(),
            "new backend should have empty buffer"
        );
        assert_eq!(
            backend.indent_level, 0,
            "new backend should have indent_level 0"
        );
    }

    #[test]
    fn test_ts_backend_default() {
        let backend = TypeScriptBackend::default();
        assert!(
            backend.buffer.is_empty(),
            "default backend should have empty buffer"
        );
        assert_eq!(
            backend.indent_level, 0,
            "default backend should have indent_level 0"
        );
    }

    // ==================================================================
    // Literal emission — real implementations
    // ==================================================================

    #[test]
    fn test_emit_literal_int() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Int(42)).unwrap(), "42");
    }

    #[test]
    fn test_emit_literal_float() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Float(3.5)).unwrap(), "3.5");
    }

    #[test]
    fn test_emit_literal_str() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_literal(&LirLiteral::Str("hello".into())).unwrap(),
            "\"hello\""
        );
    }

    #[test]
    fn test_emit_literal_bool_true() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Bool(true)).unwrap(), "true");
    }

    #[test]
    fn test_emit_literal_bool_false() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Bool(false)).unwrap(), "false");
    }

    #[test]
    fn test_emit_literal_null() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_literal(&LirLiteral::Null).unwrap(), "null");
    }

    // ==================================================================
    // Binary operator emission — real implementations
    // ==================================================================

    #[test]
    fn test_emit_binary_op_add() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Add).unwrap(), " + ");
    }

    #[test]
    fn test_emit_binary_op_sub() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Sub).unwrap(), " - ");
    }

    #[test]
    fn test_emit_binary_op_mul() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Mul).unwrap(), " * ");
    }

    #[test]
    fn test_emit_binary_op_eq() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Eq).unwrap(), " === ");
    }

    #[test]
    fn test_emit_binary_op_ne() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ne).unwrap(), " !== ");
    }

    #[test]
    fn test_emit_binary_op_and() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::And).unwrap(), " && ");
    }

    #[test]
    fn test_emit_binary_op_or() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Or).unwrap(), " || ");
    }

    // ==================================================================
    // Unary operator emission — real implementations
    // ==================================================================

    #[test]
    fn test_emit_unary_op_neg() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Neg).unwrap(), "-");
    }

    #[test]
    fn test_emit_unary_op_not() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Not).unwrap(), "!");
    }

    // ==================================================================
    // Target hint emission — real implementations
    // ==================================================================

    #[test]
    fn test_emit_target_hint_none() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::None).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_async() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_target_hint(&TargetHint::Async).unwrap(),
            "async "
        );
    }

    #[test]
    fn test_emit_target_hint_optional() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_target_hint(&TargetHint::Optional).unwrap(),
            "?"
        );
    }

    #[test]
    fn test_emit_target_hint_result() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::Result).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_react_component() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_target_hint(&TargetHint::ReactComponent).unwrap(),
            ""
        );
    }

    // ==================================================================
    // Effect emission — real implementations
    // ==================================================================

    #[test]
    fn test_emit_effect_pure() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Pure).unwrap(), "");
    }

    #[test]
    fn test_emit_effect_async() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Async).unwrap(), "await ");
    }

    #[test]
    fn test_emit_effect_impure() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_effect(&Effect::Impure).unwrap(), "");
    }

    // ==================================================================
    // Type emission — delegates to TypeScriptMapper
    // ==================================================================

    #[test]
    fn test_emit_type_int() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_type(&Type::Named("Int".into())).unwrap(),
            "number"
        );
    }

    #[test]
    fn test_emit_type_string() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(
            backend.emit_type(&Type::Named("String".into())).unwrap(),
            "string"
        );
    }

    #[test]
    fn test_emit_type_record() {
        let mut backend = TypeScriptBackend::new();
        let ty = Type::Record(vec![
            ("x".into(), Box::new(Type::Named("Int".into()))),
            ("y".into(), Box::new(Type::Named("Int".into()))),
        ]);
        assert_eq!(
            backend.emit_type(&ty).unwrap(),
            "{ x: number; y: number }"
        );
    }

    // ==================================================================
    // Expression emission — every LirExpr variant
    // ==================================================================

    #[test]
    fn test_emit_expr_literal() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "42");
    }

    #[test]
    fn test_emit_expr_variable() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Variable {
            name: "x".into(),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "x");
    }

    #[test]
    fn test_emit_expr_call() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: hint_none(),
                span: s(),
            }),
            args: vec![
                LirExpr::Variable {
                    name: "x".into(),
                    hint: hint_none(),
                    span: s(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: hint_none(),
                    span: s(),
                },
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "f(x, 1)");
    }

    #[test]
    fn test_emit_expr_call_async() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "f".into(),
                hint: hint_none(),
                span: s(),
            }),
            args: vec![LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }],
            hint: TargetHint::Async,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "await f(x)");
    }

    #[test]
    fn test_emit_expr_member() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Member {
            obj: Box::new(LirExpr::Variable {
                name: "obj".into(),
                hint: hint_none(),
                span: s(),
            }),
            field: "field".into(),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "obj.field");
    }

    #[test]
    fn test_emit_expr_member_optional() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Member {
            obj: Box::new(LirExpr::Variable {
                name: "obj".into(),
                hint: hint_none(),
                span: s(),
            }),
            field: "field".into(),
            hint: TargetHint::Optional,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "obj?.field");
    }

    #[test]
    fn test_emit_expr_if_ternary() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::If {
            cond: Box::new(LirExpr::Variable {
                name: "cond".into(),
                hint: hint_none(),
                span: s(),
            }),
            then: Box::new(LirExpr::Variable {
                name: "thenVal".into(),
                hint: hint_none(),
                span: s(),
            }),
            else_: Some(Box::new(LirExpr::Variable {
                name: "elseVal".into(),
                hint: hint_none(),
                span: s(),
            })),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "cond ? thenVal : elseVal"
        );
    }

    #[test]
    fn test_emit_expr_if_no_else() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::If {
            cond: Box::new(LirExpr::Literal {
                value: LirLiteral::Bool(false),
                hint: hint_none(),
                span: s(),
            }),
            then: Box::new(LirExpr::Variable {
                name: "thenVal".into(),
                hint: hint_none(),
                span: s(),
            }),
            else_: None,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "thenVal");
    }

    #[test]
    fn test_emit_expr_match() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Match {
            expr: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            arms: vec![
                LirArm {
                    pattern: LirPat::Literal(LirLiteral::Int(1)),
                    guard: None,
                    body: LirExpr::Literal {
                        value: LirLiteral::Str("one".into()),
                        hint: hint_none(),
                        span: s(),
                    },
                },
                LirArm {
                    pattern: LirPat::Wildcard,
                    guard: None,
                    body: LirExpr::Literal {
                        value: LirLiteral::Str("other".into()),
                        hint: hint_none(),
                        span: s(),
                    },
                },
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "x === 1 ? \"one\" : \"other\""
        );
    }

    #[test]
    fn test_emit_expr_block() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Block {
            stmts: vec![
                LirStmt::Let {
                    pat: LirPat::Variable("x".into()),
                    value: LirExpr::Literal {
                        value: LirLiteral::Int(1),
                        hint: hint_none(),
                        span: s(),
                    },
                },
                LirStmt::Expr(LirExpr::Variable {
                    name: "x".into(),
                    hint: hint_none(),
                    span: s(),
                }),
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "{ let x = 1; return x; }"
        );
    }

    #[test]
    fn test_emit_expr_assign() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Assign {
            target: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            value: Box::new(LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "x = 42");
    }

    #[test]
    fn test_emit_expr_lambda() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Lambda {
            params: vec![
                LirParam {
                    name: "a".into(),
                    type_: Some(Type::Named("Int".into())),
                },
                LirParam {
                    name: "b".into(),
                    type_: None,
                },
            ],
            body: Box::new(LirExpr::Binary {
                op: LirBinaryOp::Add,
                lhs: Box::new(LirExpr::Variable {
                    name: "a".into(),
                    hint: hint_none(),
                    span: s(),
                }),
                rhs: Box::new(LirExpr::Variable {
                    name: "b".into(),
                    hint: hint_none(),
                    span: s(),
                }),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "(a: number, b) => a + b"
        );
    }

    #[test]
    fn test_emit_expr_record() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Record {
            fields: vec![
                (
                    "x".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Int(1),
                        hint: hint_none(),
                        span: s(),
                    },
                ),
                (
                    "y".into(),
                    LirExpr::Literal {
                        value: LirLiteral::Str("hello".into()),
                        hint: hint_none(),
                        span: s(),
                    },
                ),
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "{ x: 1, y: \"hello\" }"
        );
    }

    #[test]
    fn test_emit_expr_variant_no_arg() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Variant {
            name: "None".into(),
            arg: None,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "\"None\"");
    }

    #[test]
    fn test_emit_expr_variant_with_arg() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Variant {
            name: "Ok".into(),
            arg: Some(Box::new(LirExpr::Literal {
                value: LirLiteral::Int(42),
                hint: hint_none(),
                span: s(),
            })),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "{ tag: \"Ok\", value: 42 }"
        );
    }

    #[test]
    fn test_emit_expr_array() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Array {
            items: vec![
                LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: hint_none(),
                    span: s(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(2),
                    hint: hint_none(),
                    span: s(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(3),
                    hint: hint_none(),
                    span: s(),
                },
            ],
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "[1, 2, 3]");
    }

    #[test]
    fn test_emit_expr_binary() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(LirExpr::Variable {
                name: "a".into(),
                hint: hint_none(),
                span: s(),
            }),
            rhs: Box::new(LirExpr::Variable {
                name: "b".into(),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "a + b");
    }

    #[test]
    fn test_emit_expr_binary_eq() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Binary {
            op: LirBinaryOp::Eq,
            lhs: Box::new(LirExpr::Variable {
                name: "a".into(),
                hint: hint_none(),
                span: s(),
            }),
            rhs: Box::new(LirExpr::Variable {
                name: "b".into(),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "a === b");
    }

    #[test]
    fn test_emit_expr_unary_neg() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Unary {
            op: LirUnaryOp::Neg,
            expr: Box::new(LirExpr::Variable {
                name: "x".into(),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "-x");
    }

    #[test]
    fn test_emit_expr_unary_not() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Unary {
            op: LirUnaryOp::Not,
            expr: Box::new(LirExpr::Variable {
                name: "flag".into(),
                hint: hint_none(),
                span: s(),
            }),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "!flag");
    }

    #[test]
    fn test_emit_expr_wildcard() {
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Wildcard {
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "_");
    }

    // ==================================================================
    // Pattern emission — all LirPat variants
    // ==================================================================

    #[test]
    fn test_emit_pat_wildcard() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_pat(&LirPat::Wildcard).unwrap(), "_");
    }

    #[test]
    fn test_emit_pat_literal() {
        let mut backend = TypeScriptBackend::new();
        let result = backend.emit_pat(&LirPat::Literal(LirLiteral::Int(42))).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_emit_pat_variable() {
        let mut backend = TypeScriptBackend::new();
        let result = backend.emit_pat(&LirPat::Variable("myVar".into())).unwrap();
        assert_eq!(result, "myVar");
    }

    #[test]
    fn test_emit_pat_variant_no_arg() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "None");
    }

    #[test]
    fn test_emit_pat_variant_with_arg() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirPat::Variable("inner".into()))),
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "Some: inner");
    }

    #[test]
    fn test_emit_pat_record_no_rest() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _ }");
    }

    #[test]
    fn test_emit_pat_record_with_rest() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: true,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _, ... }");
    }

    #[test]
    fn test_emit_pat_record_empty() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Record {
            fields: vec![],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{  }");
    }

    #[test]
    fn test_emit_pat_nested_variant_in_record() {
        let mut backend = TypeScriptBackend::new();
        let pat = LirPat::Record {
            fields: vec![(
                "opt".into(),
                LirPat::Variant {
                    name: "Some".into(),
                    arg: Some(Box::new(LirPat::Variable("val".into()))),
                },
            )],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ opt: Some: val }");
    }

    // ==================================================================
    // Declaration emission
    // ==================================================================

    #[test]
    fn test_emit_decl_function() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: Some(Type::Named("Void".into())),
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: s(),
        };
        assert_eq!(
            backend.emit_decl(&decl).unwrap(),
            "function main(): void { return 0; }"
        );
    }

    #[test]
    fn test_emit_decl_function_async() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "fetchData".into(),
            params: vec![LirParam {
                name: "url".into(),
                type_: Some(Type::Named("String".into())),
            }],
            return_type: Some(Type::Generic {
                base: "Promise".into(),
                args: vec![Type::Named("String".into())],
            }),
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::Async,
                span: s(),
            },
            effect: Effect::Async,
            hint: TargetHint::Async,
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(result.starts_with("async function fetchData"));
        assert!(result.contains("url: string"));
        assert!(result.contains("Promise<string>"));
    }

    #[test]
    fn test_emit_decl_record() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                LirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                LirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: s(),
        };
        assert_eq!(
            backend.emit_decl(&decl).unwrap(),
            "interface Point { x: number; y: number; }"
        );
    }

    #[test]
    fn test_emit_decl_union() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![
                LirVariant {
                    name: "Some".into(),
                    arg: Some(Type::Named("Int".into())),
                },
                LirVariant {
                    name: "None".into(),
                    arg: None,
                },
            ],
            is_pub: true,
            span: s(),
        };
        assert_eq!(
            backend.emit_decl(&decl).unwrap(),
            "type Option = Some | None;"
        );
    }

    #[test]
    fn test_emit_decl_function_with_params() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "add".into(),
            params: vec![
                LirParam {
                    name: "a".into(),
                    type_: Some(Type::Named("Int".into())),
                },
                LirParam {
                    name: "b".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            return_type: Some(Type::Named("Int".into())),
            body: LirExpr::Binary {
                op: LirBinaryOp::Add,
                lhs: Box::new(LirExpr::Variable {
                    name: "a".into(),
                    hint: hint_none(),
                    span: s(),
                }),
                rhs: Box::new(LirExpr::Variable {
                    name: "b".into(),
                    hint: hint_none(),
                    span: s(),
                }),
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: s(),
        };
        assert_eq!(
            backend.emit_decl(&decl).unwrap(),
            "function add(a: number, b: number): number { return a + b; }"
        );
    }

    // ==================================================================
    // Module emission
    // ==================================================================

    #[test]
    fn test_emit_module_empty() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_module(&[]).unwrap(), "");
    }

    #[test]
    fn test_emit_module_single_function() {
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "main".into(),
            params: vec![],
            return_type: Some(Type::Named("Void".into())),
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_module(&[decl]).unwrap();
        assert!(result.contains("function main"));
    }

    #[test]
    fn test_emit_module_mixed() {
        let mut backend = TypeScriptBackend::new();
        let func_decl = LirDecl::Function {
            name: "getOrigin".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: hint_none(),
                span: s(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: s(),
        };
        let record_decl = LirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                LirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                LirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: s(),
        };
        let result = backend.emit_module(&[record_decl, func_decl]).unwrap();
        assert!(result.contains("interface Point"));
        assert!(result.contains("function getOrigin"));
    }
}
