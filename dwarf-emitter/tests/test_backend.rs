//! Integration tests for the [`EmitterBackend`] trait.
//!
//! These tests define a mock backend, implement the trait, and verify
//! that every LIR construct produces the expected output. They test
//! the *contract* of the trait — any real backend should pass similar
//! tests after adapting the expected strings.

use dwarf_emitter::backend::EmitterBackend;
use dwarf_emitter::error::EmitterError;
use dwarf_lir::{
    Effect, LirArm, LirBinaryOp, LirDecl, LirExpr, LirField, LirLiteral, LirParam, LirPat, LirStmt,
    LirUnaryOp, LirVariant, TargetHint,
};
use dwarf_syntax::hir::{RefConstraint, Type};
use dwarf_syntax::span::Span;

// ------------------------------------------------------------------
// Mock backend — shared across all integration tests
// ------------------------------------------------------------------
//
// The mock produces deterministic debug-like strings from LIR nodes so
// we can assert on exact output.

struct MockBackend;

impl MockBackend {
    fn span() -> Span {
        Span::new(0, 0, 0)
    }

    fn hint_none() -> TargetHint {
        TargetHint::None
    }
}

impl EmitterBackend for MockBackend {
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
        Ok(out.trim_end().to_string())
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
                        Some(t) => format!("{}({})", v.name, self.emit_type(t).unwrap()),
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
            LirDecl::Extern { source, name, .. } => {
                Ok(format!("extern \"{}\" fn {}", source, name))
            }
        }
    }

    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        match expr {
            LirExpr::Literal { value, .. } => self.emit_literal(value),
            LirExpr::Variable { name, .. } => Ok(format!("var({name})")),
            LirExpr::Call { func, args, .. } => {
                let func_str = self.emit_expr(func)?;
                let args_str: Vec<String> =
                    args.iter().map(|a| self.emit_expr(a).unwrap()).collect();
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
                let items_str: Vec<String> =
                    items.iter().map(|i| self.emit_expr(i).unwrap()).collect();
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
            LirExpr::ForAll {
                type_,
                binding,
                property,
                ..
            } => {
                let ty_str = self.emit_type(type_)?;
                let binding_str = self.emit_pat(binding)?;
                let property_str = self.emit_expr(property)?;
                Ok(format!("forAll({ty_str}, {binding_str} => {property_str})"))
            }
            LirExpr::AssertConsistent { expr, .. } => {
                let inner = self.emit_expr(expr)?;
                Ok(format!("assert.consistent({})", inner))
            }
            LirExpr::Try {
                body,
                binding,
                guard,
                handler,
                ..
            } => {
                let body_str = self.emit_expr(body)?;
                let binding_str = self.emit_pat(binding)?;
                let guard_str = match guard {
                    Some(g) => format!(", guard {}", self.emit_expr(g)?),
                    None => String::new(),
                };
                let handler_str = self.emit_expr(handler)?;
                Ok(format!(
                    "try({}, {}, {}, {})",
                    body_str, binding_str, guard_str, handler_str
                ))
            }
            LirExpr::Throw { expr, .. } => {
                let inner = self.emit_expr(expr)?;
                Ok(format!("throw({})", inner))
            }
            LirExpr::Propagate { expr, .. } => {
                let inner = self.emit_expr(expr)?;
                Ok(format!("propagate({})", inner))
            }
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
                let params_str: Vec<String> =
                    params.iter().map(|p| self.emit_type(p).unwrap()).collect();
                let ret = self.emit_type(return_)?;
                Ok(format!("({}) -> {ret}", params_str.join(", ")))
            }
            Type::Generic { base, args } => {
                let args_str: Vec<String> =
                    args.iter().map(|a| self.emit_type(a).unwrap()).collect();
                Ok(format!("{base}<{}>", args_str.join(", ")))
            }
            Type::Refined { base, constraint } => {
                let base_str = self.emit_type(base)?;
                match constraint {
                    RefConstraint::Range { min, max } => Ok(format!("{base_str}({min}..{max})")),
                }
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

// ======================================================================
// emit_module tests
// ======================================================================

#[test]
fn test_emit_module_empty() {
    let mut backend = MockBackend;
    let result = backend.emit_module(&[]).unwrap();
    assert_eq!(result, "", "empty module should produce empty output");
}

#[test]
fn test_emit_module_single_func() {
    let mut backend = MockBackend;
    let decl = LirDecl::Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::Literal {
            value: LirLiteral::Int(0),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        is_generator: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_module(&[decl]).unwrap();
    assert_eq!(result, "pub fn main() [pure] [none] = 0");
}

#[test]
fn test_emit_module_two_decls() {
    let mut backend = MockBackend;
    let decls = vec![
        LirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![LirField {
                name: "x".into(),
                type_: Type::Named("Int".into()),
            }],
            is_pub: true,
            span: MockBackend::span(),
        },
        LirDecl::Function {
            name: "zero".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: false,
            is_generator: false,
            span: MockBackend::span(),
        },
    ];
    let result = backend.emit_module(&decls).unwrap();
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("record Point"));
    assert!(lines[1].contains("fn zero"));
}

// ======================================================================
// emit_decl tests
// ======================================================================

#[test]
fn test_emit_decl_func_public_pure() {
    let mut backend = MockBackend;
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
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
            rhs: Box::new(LirExpr::Variable {
                name: "b".into(),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: true,
        is_generator: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(
        result,
        "pub fn add(a: Int, b: Int) -> Int [pure] [none] = binary(var(a), +, var(b))"
    );
}

#[test]
fn test_emit_decl_func_async_effect() {
    let mut backend = MockBackend;
    let decl = LirDecl::Function {
        name: "fetch".into(),
        params: vec![],
        return_type: Some(Type::Named("String".into())),
        body: LirExpr::Literal {
            value: LirLiteral::Str("data".into()),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        },
        effect: Effect::Async,
        hint: TargetHint::Async,
        is_pub: true,
        is_generator: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(
        result,
        "pub fn fetch() -> String [async] [async] = \"data\""
    );
}

#[test]
fn test_emit_decl_func_private() {
    let mut backend = MockBackend;
    let decl = LirDecl::Function {
        name: "helper".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::Literal {
            value: LirLiteral::Null,
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        },
        effect: Effect::Impure,
        hint: TargetHint::None,
        is_pub: false,
        is_generator: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(result, "fn helper() [impure] [none] = null");
    assert!(
        !result.starts_with("pub"),
        "private function should not be prefixed with 'pub'"
    );
}

#[test]
fn test_emit_decl_record_def() {
    let mut backend = MockBackend;
    let decl = LirDecl::RecordDef {
        name: "Person".into(),
        fields: vec![
            LirField {
                name: "name".into(),
                type_: Type::Named("String".into()),
            },
            LirField {
                name: "age".into(),
                type_: Type::Named("Int".into()),
            },
        ],
        is_pub: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(result, "record Person { name: String, age: Int }");
}

#[test]
fn test_emit_decl_union_def() {
    let mut backend = MockBackend;
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
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(result, "pub union Option = Some(Int) | None");
}

#[test]
fn test_emit_decl_union_def_no_variants() {
    let mut backend = MockBackend;
    let decl = LirDecl::UnionDef {
        name: "Empty".into(),
        variants: vec![],
        is_pub: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl).unwrap();
    assert_eq!(result, "union Empty = ");
}

// ======================================================================
// emit_expr tests — every LirExpr variant
// ======================================================================

#[test]
fn test_emit_expr_literal_int() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Int(42),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "42");
}

#[test]
fn test_emit_expr_literal_float() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Float(3.5),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "3.5");
}

#[test]
fn test_emit_expr_literal_str() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Str("hello".into()),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "\"hello\"");
}

#[test]
fn test_emit_expr_literal_bool_true() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Bool(true),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "true");
}

#[test]
fn test_emit_expr_literal_bool_false() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Bool(false),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "false");
}

#[test]
fn test_emit_expr_literal_null() {
    let mut backend = MockBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Null,
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "null");
}

#[test]
fn test_emit_expr_variable() {
    let mut backend = MockBackend;
    let expr = LirExpr::Variable {
        name: "myVar".into(),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "var(myVar)");
}

#[test]
fn test_emit_expr_call_no_args() {
    let mut backend = MockBackend;
    let expr = LirExpr::Call {
        func: Box::new(LirExpr::Variable {
            name: "f".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        args: vec![],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "call(var(f), [])");
}

#[test]
fn test_emit_expr_call_with_args() {
    let mut backend = MockBackend;
    let expr = LirExpr::Call {
        func: Box::new(LirExpr::Variable {
            name: "add".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        args: vec![
            LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
            LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
        ],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "call(var(add), [1, 2])");
}

#[test]
fn test_emit_expr_member_access() {
    let mut backend = MockBackend;
    let expr = LirExpr::Member {
        obj: Box::new(LirExpr::Variable {
            name: "point".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        field: "x".into(),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "member(var(point), x)");
}

#[test]
fn test_emit_expr_if_with_else() {
    let mut backend = MockBackend;
    let expr = LirExpr::If {
        cond: Box::new(LirExpr::Literal {
            value: LirLiteral::Bool(true),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        then: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        else_: Some(Box::new(LirExpr::Literal {
            value: LirLiteral::Int(2),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        })),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "if(true, 1, 2)");
}

#[test]
fn test_emit_expr_if_no_else() {
    let mut backend = MockBackend;
    let expr = LirExpr::If {
        cond: Box::new(LirExpr::Literal {
            value: LirLiteral::Bool(false),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        then: Box::new(LirExpr::Literal {
            value: LirLiteral::Null,
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        else_: None,
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "if(false, null)");
}

#[test]
fn test_emit_expr_match_single_arm() {
    let mut backend = MockBackend;
    let expr = LirExpr::Match {
        expr: Box::new(LirExpr::Variable {
            name: "x".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        arms: vec![LirArm {
            pattern: LirPat::Wildcard,
            guard: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
        }],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "match(var(x), [_ => 0])");
}

#[test]
fn test_emit_expr_match_multi_arm() {
    let mut backend = MockBackend;
    let expr = LirExpr::Match {
        expr: Box::new(LirExpr::Variable {
            name: "x".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        arms: vec![
            LirArm {
                pattern: LirPat::Literal(LirLiteral::Int(0)),
                guard: None,
                body: LirExpr::Literal {
                    value: LirLiteral::Str("zero".into()),
                    hint: MockBackend::hint_none(),
                    span: MockBackend::span(),
                },
            },
            LirArm {
                pattern: LirPat::Wildcard,
                guard: None,
                body: LirExpr::Literal {
                    value: LirLiteral::Str("other".into()),
                    hint: MockBackend::hint_none(),
                    span: MockBackend::span(),
                },
            },
        ],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(
        backend.emit_expr(&expr).unwrap(),
        "match(var(x), [0 => \"zero\"; _ => \"other\"])"
    );
}

#[test]
fn test_emit_expr_match_with_guard() {
    let mut backend = MockBackend;
    let expr = LirExpr::Match {
        expr: Box::new(LirExpr::Variable {
            name: "n".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        arms: vec![LirArm {
            pattern: LirPat::Variable("val".into()),
            guard: Some(LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
            body: LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
        }],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(
        backend.emit_expr(&expr).unwrap(),
        "match(var(n), [val if true => 1])"
    );
}

#[test]
fn test_emit_expr_block() {
    let mut backend = MockBackend;
    let expr = LirExpr::Block {
        stmts: vec![
            LirStmt::Let {
                pat: LirPat::Variable("x".into()),
                value: LirExpr::Literal {
                    value: LirLiteral::Int(1),
                    hint: MockBackend::hint_none(),
                    span: MockBackend::span(),
                },
            },
            LirStmt::Expr(LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
        ],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "block([let x = 1; 2])");
}

#[test]
fn test_emit_expr_block_empty() {
    let mut backend = MockBackend;
    let expr = LirExpr::Block {
        stmts: vec![],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "block([])");
}

#[test]
fn test_emit_expr_assign() {
    let mut backend = MockBackend;
    let expr = LirExpr::Assign {
        target: Box::new(LirExpr::Variable {
            name: "x".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        value: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "assign(var(x), 42)");
}

#[test]
fn test_emit_expr_lambda_no_params() {
    let mut backend = MockBackend;
    let expr = LirExpr::Lambda {
        params: vec![],
        body: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "lambda((), 42)");
}

#[test]
fn test_emit_expr_lambda_with_params() {
    let mut backend = MockBackend;
    let expr = LirExpr::Lambda {
        params: vec![
            LirParam {
                name: "a".into(),
                type_: None,
            },
            LirParam {
                name: "b".into(),
                type_: Some(Type::Named("Int".into())),
            },
        ],
        body: Box::new(LirExpr::Binary {
            op: LirBinaryOp::Add,
            lhs: Box::new(LirExpr::Variable {
                name: "a".into(),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
            rhs: Box::new(LirExpr::Variable {
                name: "b".into(),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            }),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(
        backend.emit_expr(&expr).unwrap(),
        "lambda((a, b: Int), binary(var(a), +, var(b)))"
    );
}

#[test]
fn test_emit_expr_record_empty() {
    let mut backend = MockBackend;
    let expr = LirExpr::Record {
        fields: vec![],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "record({})");
}

#[test]
fn test_emit_expr_record_with_fields() {
    let mut backend = MockBackend;
    let expr = LirExpr::Record {
        fields: vec![
            (
                "x".into(),
                LirExpr::Literal {
                    value: LirLiteral::Int(10),
                    hint: MockBackend::hint_none(),
                    span: MockBackend::span(),
                },
            ),
            (
                "y".into(),
                LirExpr::Literal {
                    value: LirLiteral::Int(20),
                    hint: MockBackend::hint_none(),
                    span: MockBackend::span(),
                },
            ),
        ],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "record({x: 10, y: 20})");
}

#[test]
fn test_emit_expr_variant_with_arg() {
    let mut backend = MockBackend;
    let expr = LirExpr::Variant {
        name: "Some".into(),
        arg: Some(Box::new(LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        })),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "variant(Some, 1)");
}

#[test]
fn test_emit_expr_variant_no_arg() {
    let mut backend = MockBackend;
    let expr = LirExpr::Variant {
        name: "None".into(),
        arg: None,
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "variant(None)");
}

#[test]
fn test_emit_expr_array_empty() {
    let mut backend = MockBackend;
    let expr = LirExpr::Array {
        items: vec![],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "array([])");
}

#[test]
fn test_emit_expr_array_with_items() {
    let mut backend = MockBackend;
    let expr = LirExpr::Array {
        items: vec![
            LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
            LirExpr::Literal {
                value: LirLiteral::Int(2),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
            LirExpr::Literal {
                value: LirLiteral::Int(3),
                hint: MockBackend::hint_none(),
                span: MockBackend::span(),
            },
        ],
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "array([1, 2, 3])");
}

#[test]
fn test_emit_expr_binary_add() {
    let mut backend = MockBackend;
    let expr = LirExpr::Binary {
        op: LirBinaryOp::Add,
        lhs: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(1),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        rhs: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(2),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "binary(1, +, 2)");
}

#[test]
fn test_emit_expr_binary_eq() {
    let mut backend = MockBackend;
    let expr = LirExpr::Binary {
        op: LirBinaryOp::Eq,
        lhs: Box::new(LirExpr::Variable {
            name: "a".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        rhs: Box::new(LirExpr::Variable {
            name: "b".into(),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(
        backend.emit_expr(&expr).unwrap(),
        "binary(var(a), ==, var(b))"
    );
}

#[test]
fn test_emit_expr_unary_neg() {
    let mut backend = MockBackend;
    let expr = LirExpr::Unary {
        op: LirUnaryOp::Neg,
        expr: Box::new(LirExpr::Literal {
            value: LirLiteral::Int(5),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "unary(-, 5)");
}

#[test]
fn test_emit_expr_unary_not() {
    let mut backend = MockBackend;
    let expr = LirExpr::Unary {
        op: LirUnaryOp::Not,
        expr: Box::new(LirExpr::Literal {
            value: LirLiteral::Bool(true),
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        }),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "unary(!, true)");
}

#[test]
fn test_emit_expr_wildcard() {
    let mut backend = MockBackend;
    let expr = LirExpr::Wildcard {
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    assert_eq!(backend.emit_expr(&expr).unwrap(), "wildcard");
}

// ======================================================================
// emit_pat tests — every LirPat variant
// ======================================================================

#[test]
fn test_emit_pat_wildcard() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_pat(&LirPat::Wildcard).unwrap(), "_");
}

#[test]
fn test_emit_pat_literal_int() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_pat(&LirPat::Literal(LirLiteral::Int(42)))
            .unwrap(),
        "42"
    );
}

#[test]
fn test_emit_pat_literal_str() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_pat(&LirPat::Literal(LirLiteral::Str("hi".into())))
            .unwrap(),
        "\"hi\""
    );
}

#[test]
fn test_emit_pat_variable() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_pat(&LirPat::Variable("myBinding".into()))
            .unwrap(),
        "myBinding"
    );
}

#[test]
fn test_emit_pat_variant_with_nested_pat() {
    let mut backend = MockBackend;
    let pat = LirPat::Variant {
        name: "Some".into(),
        arg: Some(Box::new(LirPat::Variable("inner".into()))),
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "Some(inner)");
}

#[test]
fn test_emit_pat_variant_no_arg() {
    let mut backend = MockBackend;
    let pat = LirPat::Variant {
        name: "None".into(),
        arg: None,
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "None");
}

#[test]
fn test_emit_pat_variant_with_literal_arg() {
    let mut backend = MockBackend;
    let pat = LirPat::Variant {
        name: "Exactly".into(),
        arg: Some(Box::new(LirPat::Literal(LirLiteral::Int(7)))),
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "Exactly(7)");
}

#[test]
fn test_emit_pat_record_no_rest() {
    let mut backend = MockBackend;
    let pat = LirPat::Record {
        fields: vec![
            ("x".into(), LirPat::Wildcard),
            ("y".into(), LirPat::Variable("val".into())),
        ],
        rest: false,
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _, y: val }");
}

#[test]
fn test_emit_pat_record_with_rest() {
    let mut backend = MockBackend;
    let pat = LirPat::Record {
        fields: vec![("name".into(), LirPat::Variable("n".into()))],
        rest: true,
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "{ name: n, .. }");
}

#[test]
fn test_emit_pat_record_empty() {
    let mut backend = MockBackend;
    let pat = LirPat::Record {
        fields: vec![],
        rest: false,
    };
    assert_eq!(backend.emit_pat(&pat).unwrap(), "{  }");
}

// ======================================================================
// emit_type tests — every Type variant
// ======================================================================

#[test]
fn test_emit_type_named() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_type(&Type::Named("String".into())).unwrap(),
        "String"
    );
}

#[test]
fn test_emit_type_record_single() {
    let mut backend = MockBackend;
    let ty = Type::Record(vec![(
        "name".into(),
        Box::new(Type::Named("String".into())),
    )]);
    assert_eq!(backend.emit_type(&ty).unwrap(), "record(name: String)");
}

#[test]
fn test_emit_type_record_nested() {
    let mut backend = MockBackend;
    let ty = Type::Record(vec![(
        "meta".into(),
        Box::new(Type::Record(vec![(
            "key".into(),
            Box::new(Type::Named("String".into())),
        )])),
    )]);
    assert_eq!(
        backend.emit_type(&ty).unwrap(),
        "record(meta: record(key: String))"
    );
}

#[test]
fn test_emit_type_union() {
    let mut backend = MockBackend;
    let ty = Type::Union(vec![
        Type::Named("Int".into()),
        Type::Named("String".into()),
    ]);
    assert_eq!(backend.emit_type(&ty).unwrap(), "union(Int | String)");
}

#[test]
fn test_emit_type_union_single() {
    let mut backend = MockBackend;
    let ty = Type::Union(vec![Type::Named("Never".into())]);
    assert_eq!(backend.emit_type(&ty).unwrap(), "union(Never)");
}

#[test]
fn test_emit_type_func() {
    let mut backend = MockBackend;
    let ty = Type::Func {
        params: vec![Type::Named("Int".into())],
        return_: Box::new(Type::Named("Bool".into())),
    };
    assert_eq!(backend.emit_type(&ty).unwrap(), "(Int) -> Bool");
}

#[test]
fn test_emit_type_func_multi_param() {
    let mut backend = MockBackend;
    let ty = Type::Func {
        params: vec![
            Type::Named("Int".into()),
            Type::Named("String".into()),
            Type::Named("Float".into()),
        ],
        return_: Box::new(Type::Named("Bool".into())),
    };
    assert_eq!(
        backend.emit_type(&ty).unwrap(),
        "(Int, String, Float) -> Bool"
    );
}

#[test]
fn test_emit_type_func_no_params() {
    let mut backend = MockBackend;
    let ty = Type::Func {
        params: vec![],
        return_: Box::new(Type::Named("Int".into())),
    };
    assert_eq!(backend.emit_type(&ty).unwrap(), "() -> Int");
}

#[test]
fn test_emit_type_generic_single_arg() {
    let mut backend = MockBackend;
    let ty = Type::Generic {
        base: "Array".into(),
        args: vec![Type::Named("Int".into())],
    };
    assert_eq!(backend.emit_type(&ty).unwrap(), "Array<Int>");
}

#[test]
fn test_emit_type_generic_multi_args() {
    let mut backend = MockBackend;
    let ty = Type::Generic {
        base: "Map".into(),
        args: vec![Type::Named("String".into()), Type::Named("Int".into())],
    };
    assert_eq!(backend.emit_type(&ty).unwrap(), "Map<String, Int>");
}

#[test]
fn test_emit_type_generic_nested() {
    let mut backend = MockBackend;
    let ty = Type::Generic {
        base: "Array".into(),
        args: vec![Type::Generic {
            base: "Option".into(),
            args: vec![Type::Named("Int".into())],
        }],
    };
    assert_eq!(backend.emit_type(&ty).unwrap(), "Array<Option<Int>>");
}

// ======================================================================
// emit_literal tests — every LirLiteral variant
// ======================================================================

#[test]
fn test_emit_literal_int_positive() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_literal(&LirLiteral::Int(0)).unwrap(), "0");
    assert_eq!(backend.emit_literal(&LirLiteral::Int(999)).unwrap(), "999");
}

#[test]
fn test_emit_literal_int_negative() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_literal(&LirLiteral::Int(-42)).unwrap(), "-42");
}

#[test]
fn test_emit_literal_float() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_literal(&LirLiteral::Float(3.5)).unwrap(),
        "3.5"
    );
    assert_eq!(
        backend.emit_literal(&LirLiteral::Float(-1.5)).unwrap(),
        "-1.5"
    );
}

#[test]
fn test_emit_literal_str_empty() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_literal(&LirLiteral::Str(String::new()))
            .unwrap(),
        "\"\""
    );
}

#[test]
fn test_emit_literal_str_non_empty() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_literal(&LirLiteral::Str("hello world".into()))
            .unwrap(),
        "\"hello world\""
    );
}

#[test]
fn test_emit_literal_bool_true() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_literal(&LirLiteral::Bool(true)).unwrap(),
        "true"
    );
}

#[test]
fn test_emit_literal_bool_false() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_literal(&LirLiteral::Bool(false)).unwrap(),
        "false"
    );
}

#[test]
fn test_emit_literal_null() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_literal(&LirLiteral::Null).unwrap(), "null");
}

// ======================================================================
// emit_binary_op tests — every operator
// ======================================================================

#[test]
fn test_emit_binary_op_add() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Add).unwrap(), "+");
}

#[test]
fn test_emit_binary_op_sub() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Sub).unwrap(), "-");
}

#[test]
fn test_emit_binary_op_mul() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Mul).unwrap(), "*");
}

#[test]
fn test_emit_binary_op_div() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Div).unwrap(), "/");
}

#[test]
fn test_emit_binary_op_eq() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Eq).unwrap(), "==");
}

#[test]
fn test_emit_binary_op_ne() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ne).unwrap(), "!=");
}

#[test]
fn test_emit_binary_op_lt() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Lt).unwrap(), "<");
}

#[test]
fn test_emit_binary_op_gt() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Gt).unwrap(), ">");
}

#[test]
fn test_emit_binary_op_le() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Le).unwrap(), "<=");
}

#[test]
fn test_emit_binary_op_ge() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ge).unwrap(), ">=");
}

#[test]
fn test_emit_binary_op_and() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::And).unwrap(), "&&");
}

#[test]
fn test_emit_binary_op_or() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_binary_op(&LirBinaryOp::Or).unwrap(), "||");
}

// ======================================================================
// emit_unary_op tests — every operator
// ======================================================================

#[test]
fn test_emit_unary_op_neg() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_unary_op(&LirUnaryOp::Neg).unwrap(), "-");
}

#[test]
fn test_emit_unary_op_not() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_unary_op(&LirUnaryOp::Not).unwrap(), "!");
}

// ======================================================================
// emit_target_hint tests — every variant
// ======================================================================

#[test]
fn test_emit_target_hint_none() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_target_hint(&TargetHint::None).unwrap(), "none");
}

#[test]
fn test_emit_target_hint_async() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_target_hint(&TargetHint::Async).unwrap(),
        "async"
    );
}

#[test]
fn test_emit_target_hint_optional() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_target_hint(&TargetHint::Optional).unwrap(),
        "optional"
    );
}

#[test]
fn test_emit_target_hint_result() {
    let mut backend = MockBackend;
    assert_eq!(
        backend.emit_target_hint(&TargetHint::Result).unwrap(),
        "result"
    );
}

#[test]
fn test_emit_target_hint_react_component() {
    let mut backend = MockBackend;
    assert_eq!(
        backend
            .emit_target_hint(&TargetHint::ReactComponent)
            .unwrap(),
        "react_component"
    );
}

// ======================================================================
// emit_effect tests — every variant
// ======================================================================

#[test]
fn test_emit_effect_pure() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_effect(&Effect::Pure).unwrap(), "pure");
}

#[test]
fn test_emit_effect_async() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_effect(&Effect::Async).unwrap(), "async");
}

#[test]
fn test_emit_effect_impure() {
    let mut backend = MockBackend;
    assert_eq!(backend.emit_effect(&Effect::Impure).unwrap(), "impure");
}

// ======================================================================
// Error handling tests — methods returning EmitterError
// ======================================================================

/// A backend that always fails, for testing error propagation.
struct AlwaysFailBackend;

impl EmitterBackend for AlwaysFailBackend {
    type Output = String;

    fn emit_module(&mut self, _decls: &[LirDecl]) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("module emission".into()))
    }

    fn emit_decl(&mut self, _decl: &LirDecl) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("decl".into()))
    }

    fn emit_expr(&mut self, _expr: &LirExpr) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("expr".into()))
    }

    fn emit_pat(&mut self, _pat: &LirPat) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("pat".into()))
    }

    fn emit_type(&mut self, _ty: &Type) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("type".into()))
    }

    fn emit_literal(&mut self, _lit: &LirLiteral) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("literal".into()))
    }

    fn emit_binary_op(&mut self, _op: &LirBinaryOp) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("binary op".into()))
    }

    fn emit_unary_op(&mut self, _op: &LirUnaryOp) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("unary op".into()))
    }

    fn emit_target_hint(&mut self, _hint: &TargetHint) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("target hint".into()))
    }

    fn emit_effect(&mut self, _effect: &Effect) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("effect".into()))
    }
}

#[test]
fn test_always_fail_emit_module() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_module(&[]);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        EmitterError::UnsupportedFeature("module emission".into())
    );
}

#[test]
fn test_always_fail_emit_decl() {
    let mut backend = AlwaysFailBackend;
    let decl = LirDecl::Function {
        name: "f".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::Literal {
            value: LirLiteral::Null,
            hint: MockBackend::hint_none(),
            span: MockBackend::span(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: false,
        is_generator: false,
        span: MockBackend::span(),
    };
    let result = backend.emit_decl(&decl);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_expr() {
    let mut backend = AlwaysFailBackend;
    let expr = LirExpr::Literal {
        value: LirLiteral::Int(0),
        hint: MockBackend::hint_none(),
        span: MockBackend::span(),
    };
    let result = backend.emit_expr(&expr);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_pat() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_pat(&LirPat::Wildcard);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_type() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_type(&Type::Named("Int".into()));
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_literal() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_literal(&LirLiteral::Null);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_binary_op() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_binary_op(&LirBinaryOp::Add);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_unary_op() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_unary_op(&LirUnaryOp::Neg);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_target_hint() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_target_hint(&TargetHint::None);
    assert!(result.is_err());
}

#[test]
fn test_always_fail_emit_effect() {
    let mut backend = AlwaysFailBackend;
    let result = backend.emit_effect(&Effect::Pure);
    assert!(result.is_err());
}

// ======================================================================
// Trait object safety — verify `dyn EmitterBackend` compiles
// ======================================================================

#[test]
fn test_trait_can_be_used_as_trait_object() {
    fn takes_dyn(_backend: &mut dyn EmitterBackend<Output = String>) {}
    let mut backend = MockBackend;
    takes_dyn(&mut backend);
}

#[test]
fn test_trait_object_always_fail() {
    fn takes_dyn(backend: &mut dyn EmitterBackend<Output = String>) {
        let result = backend.emit_expr(&LirExpr::Wildcard {
            hint: TargetHint::None,
            span: MockBackend::span(),
        });
        assert!(result.is_err());
    }
    let mut backend = AlwaysFailBackend;
    takes_dyn(&mut backend);
}

// ======================================================================
// Trail's core contract: mock backend Round-Trip
// Each LIR construct produces a deterministic output
// ======================================================================

#[test]
fn test_mock_backend_all_methods_return_ok() {
    // Call every method at least once to ensure they all return Ok
    let mut b = MockBackend;
    assert!(b.emit_module(&[]).is_ok());
    assert!(b.emit_literal(&LirLiteral::Int(0)).is_ok());
    assert!(b.emit_literal(&LirLiteral::Float(0.0)).is_ok());
    assert!(b.emit_literal(&LirLiteral::Str("".into())).is_ok());
    assert!(b.emit_literal(&LirLiteral::Bool(false)).is_ok());
    assert!(b.emit_literal(&LirLiteral::Null).is_ok());
    assert!(b.emit_binary_op(&LirBinaryOp::Add).is_ok());
    assert!(b.emit_unary_op(&LirUnaryOp::Neg).is_ok());
    assert!(b.emit_target_hint(&TargetHint::None).is_ok());
    assert!(b.emit_effect(&Effect::Pure).is_ok());
    assert!(b.emit_type(&Type::Named("T".into())).is_ok());
    assert!(b.emit_pat(&LirPat::Wildcard).is_ok());
    let decl = LirDecl::Function {
        name: "t".into(),
        params: vec![],
        return_type: None,
        body: LirExpr::Literal {
            value: LirLiteral::Null,
            hint: TargetHint::None,
            span: MockBackend::span(),
        },
        effect: Effect::Pure,
        hint: TargetHint::None,
        is_pub: false,
        is_generator: false,
        span: MockBackend::span(),
    };
    assert!(b.emit_decl(&decl).is_ok());
    let expr = LirExpr::Literal {
        value: LirLiteral::Null,
        hint: TargetHint::None,
        span: MockBackend::span(),
    };
    assert!(b.emit_expr(&expr).is_ok());
}
