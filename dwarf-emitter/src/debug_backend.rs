//! A debug backend that formats LIR constructs into human-readable strings.
//!
//! This backend implements [`EmitterBackend`] with `Output = String` and
//! produces a structured, parenthesised representation of the LIR. It is
//! intended for CLI debugging, testing, and as a reference for implementing
//! real backends.
//!
//! # Example output
//!
//! ```ignore
//! fn main() [pure] [none] = 0
//! pub fn add(a: Int, b: Int) -> Int [pure] [none] = binary(var(a), +, var(b))
//! ```

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;

/// A debug backend that renders LIR to a human-readable debug format.
///
/// Each LIR construct is rendered as a structured, parenthesised string.
/// This is the same format used by the test-only `MockBackend` but is
/// available for production CLI use (e.g., `dwarf emit`).
pub struct DebugBackend;

impl DebugBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebugBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterBackend for DebugBackend {
    type Output = String;

    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
        if decls.is_empty() {
            return Ok(String::new());
        }
        let mut out = String::new();
        for decl in decls {
            out.push_str(&self.emit_decl(decl)?);
            out.push('\n');
        }
        // Trim trailing newline
        let trimmed = out.trim_end().to_string();
        Ok(trimmed)
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
                is_pub,
                ..
            } => {
                let vis = if *is_pub { "pub " } else { "" };
                let eff = self.emit_effect(effect)?;
                let h = self.emit_target_hint(hint)?;
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let ty = match &p.type_ {
                            Some(t) => format!(": {}", self.emit_type(t).unwrap()),
                            None => String::new(),
                        };
                        format!("{}{}", p.name, ty)
                    })
                    .collect();
                let ret = match return_type {
                    Some(t) => format!(" -> {}", self.emit_type(t).unwrap()),
                    None => String::new(),
                };
                let body_str = self.emit_expr(body)?;
                Ok(format!(
                    "{}fn {}({}){ret} [{}] [{}] = {body_str}",
                    vis,
                    name,
                    params_str.join(", "),
                    eff,
                    h,
                ))
            }
            LirDecl::RecordDef {
                name,
                fields,
                is_pub,
                ..
            } => {
                let vis = if *is_pub { "pub " } else { "" };
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.emit_type(&f.type_).unwrap();
                        format!("{}: {}", f.name, ty)
                    })
                    .collect();
                Ok(format!(
                    "{}record {} {{ {} }}",
                    vis,
                    name,
                    fields_str.join(", ")
                ))
            }
            LirDecl::UnionDef {
                name,
                variants,
                is_pub,
                ..
            } => {
                let vis = if *is_pub { "pub " } else { "" };
                let variants_str: Vec<String> = variants
                    .iter()
                    .map(|v| match &v.arg {
                        Some(t) => {
                            format!("{}({})", v.name, self.emit_type(t).unwrap())
                        }
                        None => v.name.clone(),
                    })
                    .collect();
                Ok(format!(
                    "{}union {} = {}",
                    vis,
                    name,
                    variants_str.join(" | ")
                ))
            }
        }
    }

    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        match expr {
            LirExpr::Literal { value, .. } => self.emit_literal(value),
            LirExpr::Variable { name, .. } => Ok(format!("var({name})")),
            LirExpr::Call { func, args, .. } => {
                let func_str = self.emit_expr(func)?;
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.emit_expr(a).unwrap())
                    .collect();
                Ok(format!("call({}, [{}])", func_str, args_str.join(", ")))
            }
            LirExpr::Member { obj, field, .. } => {
                let obj_str = self.emit_expr(obj)?;
                Ok(format!("member({obj_str}, {field})"))
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                let cond_str = self.emit_expr(cond)?;
                let then_str = self.emit_expr(then)?;
                let else_str = match else_ {
                    Some(e) => format!(", {}", self.emit_expr(e).unwrap()),
                    None => String::new(),
                };
                Ok(format!("if({cond_str}, {then_str}{else_str})"))
            }
            LirExpr::Match { expr, arms, .. } => {
                let expr_str = self.emit_expr(expr)?;
                let arms_str: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let pat = self.emit_pat(&arm.pattern).unwrap();
                        let guard = match &arm.guard {
                            Some(g) => format!(" if {}", self.emit_expr(g).unwrap()),
                            None => String::new(),
                        };
                        let body = self.emit_expr(&arm.body).unwrap();
                        format!("{pat}{guard} => {body}")
                    })
                    .collect();
                Ok(format!("match({expr_str}, [{}])", arms_str.join("; ")))
            }
            LirExpr::Block { stmts, .. } => {
                let stmts_str: Vec<String> = stmts
                    .iter()
                    .map(|s| match s {
                        LirStmt::Let { pat, value } => {
                            let pat_str = self.emit_pat(pat).unwrap();
                            let val_str = self.emit_expr(value).unwrap();
                            format!("let {} = {val_str}", pat_str)
                        }
                        LirStmt::Expr(e) => self.emit_expr(e).unwrap(),
                    })
                    .collect();
                Ok(format!("block([{}])", stmts_str.join("; ")))
            }
            LirExpr::Assign { target, value, .. } => {
                let t = self.emit_expr(target)?;
                let v = self.emit_expr(value)?;
                Ok(format!("assign({t}, {v})"))
            }
            LirExpr::Lambda { params, body, .. } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let ty = match &p.type_ {
                            Some(t) => format!(": {}", self.emit_type(t).unwrap()),
                            None => String::new(),
                        };
                        format!("{}{}", p.name, ty)
                    })
                    .collect();
                let body_str = self.emit_expr(body)?;
                Ok(format!("lambda(({}), {body_str})", params_str.join(", ")))
            }
            LirExpr::Record { fields, .. } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, val)| {
                        let v = self.emit_expr(val).unwrap();
                        format!("{name}: {v}")
                    })
                    .collect();
                Ok(format!(
                    "record({{{fields_str}}})",
                    fields_str = fields_str.join(", ")
                ))
            }
            LirExpr::Variant { name, arg, .. } => match arg {
                Some(a) => {
                    let a_str = self.emit_expr(a)?;
                    Ok(format!("variant({name}, {a_str})"))
                }
                None => Ok(format!("variant({name})")),
            },
            LirExpr::Array { items, .. } => {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|i| self.emit_expr(i).unwrap())
                    .collect();
                Ok(format!("array([{}])", items_str.join(", ")))
            }
            LirExpr::Binary { op, lhs, rhs, .. } => {
                let op_str = self.emit_binary_op(op)?;
                let lhs_str = self.emit_expr(lhs)?;
                let rhs_str = self.emit_expr(rhs)?;
                Ok(format!("binary({lhs_str}, {op_str}, {rhs_str})"))
            }
            LirExpr::Unary { op, expr, .. } => {
                let op_str = self.emit_unary_op(op)?;
                let e_str = self.emit_expr(expr)?;
                Ok(format!("unary({op_str}, {e_str})"))
            }
            LirExpr::Wildcard { .. } => Ok("wildcard".into()),
        }
    }

    fn emit_pat(&mut self, pat: &LirPat) -> Result<String, EmitterError> {
        match pat {
            LirPat::Wildcard => Ok("_".into()),
            LirPat::Literal(lit) => self.emit_literal(lit),
            LirPat::Variable(name) => Ok(name.clone()),
            LirPat::Variant { name, arg } => match arg {
                Some(a) => {
                    let a_str = self.emit_pat(a)?;
                    Ok(format!("{name}({a_str})"))
                }
                None => Ok(name.clone()),
            },
            LirPat::Record { fields, rest } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, pat)| {
                        let p = self.emit_pat(pat).unwrap();
                        format!("{name}: {p}")
                    })
                    .collect();
                let rest_str = if *rest { ", .." } else { "" };
                Ok(format!("{{ {}{} }}", fields_str.join(", "), rest_str))
            }
        }
    }

    fn emit_type(&mut self, ty: &Type) -> Result<String, EmitterError> {
        match ty {
            Type::Named(name) => Ok(name.clone()),
            Type::Record(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| {
                        let t = self.emit_type(ty).unwrap();
                        format!("{name}: {t}")
                    })
                    .collect();
                Ok(format!("record({})", fields_str.join(", ")))
            }
            Type::Union(variants) => {
                let vars_str: Vec<String> = variants
                    .iter()
                    .map(|v| self.emit_type(v).unwrap())
                    .collect();
                Ok(format!("union({})", vars_str.join(" | ")))
            }
            Type::Func { params, return_ } => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| self.emit_type(p).unwrap())
                    .collect();
                let ret = self.emit_type(return_)?;
                Ok(format!("({}) -> {ret}", params_str.join(", ")))
            }
            Type::Generic { base, args } => {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.emit_type(a).unwrap())
                    .collect();
                Ok(format!("{base}<{}>", args_str.join(", ")))
            }
        }
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
            LirBinaryOp::Add => Ok("+".into()),
            LirBinaryOp::Sub => Ok("-".into()),
            LirBinaryOp::Mul => Ok("*".into()),
            LirBinaryOp::Div => Ok("/".into()),
            LirBinaryOp::Eq => Ok("==".into()),
            LirBinaryOp::Ne => Ok("!=".into()),
            LirBinaryOp::Lt => Ok("<".into()),
            LirBinaryOp::Gt => Ok(">".into()),
            LirBinaryOp::Le => Ok("<=".into()),
            LirBinaryOp::Ge => Ok(">=".into()),
            LirBinaryOp::And => Ok("&&".into()),
            LirBinaryOp::Or => Ok("||".into()),
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
            TargetHint::None => Ok("none".into()),
            TargetHint::Async => Ok("async".into()),
            TargetHint::Optional => Ok("optional".into()),
            TargetHint::Result => Ok("result".into()),
            TargetHint::ReactComponent => Ok("react_component".into()),
        }
    }

    fn emit_effect(&mut self, effect: &Effect) -> Result<String, EmitterError> {
        match effect {
            Effect::Pure => Ok("pure".into()),
            Effect::Async => Ok("async".into()),
            Effect::Impure => Ok("impure".into()),
        }
    }
}
