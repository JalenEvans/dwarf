//! Java backend implementation.
//!
//! Implements the [`EmitterBackend`] trait for Java code generation.
//! Provides full implementations for all LIR constructs: literals,
//! binary/unary ops, target hints, effects, types, patterns,
//! expressions, declarations, and module emission.

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;
use crate::format::CodeBuffer;
use crate::java::bridge::StructuralNominalBridge;
use crate::java::mapper::JavaMapper;
use crate::naming::{to_camel_case, to_pascal_case};
use crate::types::TypeMapper;

/// A backend that emits Java code from LIR declarations.
pub struct JavaBackend {
    buffer: CodeBuffer,
    type_mapper: JavaMapper,
    bridge: StructuralNominalBridge,
    package_name: String,
    version: String,
    needs_completable_future: bool,
    needs_optional: bool,
    needs_jqwik: bool,
    has_forall: bool,
    needs_option: bool,
    needs_result: bool,
    needs_list_utils: bool,
    needs_string_utils: bool,
    needs_math_utils: bool,
    needs_io_utils: bool,
    java_extern_imports: Vec<String>,
}

impl JavaBackend {
    pub fn new(package_name: &str, version: &str) -> Self {
        Self {
            buffer: CodeBuffer::with_indent_size(4),
            type_mapper: JavaMapper,
            bridge: StructuralNominalBridge::new(),
            package_name: package_name.to_string(),
            version: version.to_string(),
            needs_completable_future: false,
            needs_optional: false,
            needs_jqwik: false,
            has_forall: false,
            needs_option: false,
            needs_result: false,
            needs_list_utils: false,
            needs_string_utils: false,
            needs_math_utils: false,
            needs_io_utils: false,
            java_extern_imports: Vec::new(),
        }
    }

    pub fn buffer(&self) -> &CodeBuffer {
        &self.buffer
    }

    pub fn into_output(self) -> String {
        self.buffer.into_string()
    }
}

impl Default for JavaBackend {
    fn default() -> Self {
        Self::new("dwarf.gen", "0.1.0")
    }
}

impl EmitterBackend for JavaBackend {
    type Output = String;

    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
        if decls.is_empty() {
            return Ok(String::new());
        }

        // Reset import flags and scan all decls
        self.needs_completable_future = false;
        self.needs_optional = false;
        self.needs_jqwik = false;
        self.has_forall = false;
        for decl in decls {
            Self::scan_decl_for_imports(
                decl,
                &mut self.needs_completable_future,
                &mut self.needs_optional,
            );
            // Scan types for stdlib references
            match decl {
                LirDecl::Function {
                    params,
                    return_type,
                    body,
                    ..
                } => {
                    for param in params {
                        if let Some(ref ty) = param.type_ {
                            Self::scan_type_for_stdlib(
                                ty,
                                &mut self.needs_option,
                                &mut self.needs_result,
                                &mut self.needs_list_utils,
                                &mut self.needs_string_utils,
                                &mut self.needs_math_utils,
                            );
                        }
                    }
                    if let Some(ref ty) = return_type {
                        Self::scan_type_for_stdlib(
                            ty,
                            &mut self.needs_option,
                            &mut self.needs_result,
                            &mut self.needs_list_utils,
                            &mut self.needs_string_utils,
                            &mut self.needs_math_utils,
                        );
                    }
                    // Also scan expression for stdlib calls
                    Self::scan_expr_for_stdlib(
                        body,
                        &mut self.needs_io_utils,
                        &mut self.needs_string_utils,
                        &mut self.needs_math_utils,
                        &mut self.needs_result,
                    );
                }
                LirDecl::RecordDef { fields, .. } => {
                    for field in fields {
                        Self::scan_type_for_stdlib(
                            &field.type_,
                            &mut self.needs_option,
                            &mut self.needs_result,
                            &mut self.needs_list_utils,
                            &mut self.needs_string_utils,
                            &mut self.needs_math_utils,
                        );
                    }
                }
                LirDecl::UnionDef { variants, .. } => {
                    for variant in variants {
                        if let Some(ref arg_type) = variant.arg {
                            Self::scan_type_for_stdlib(
                                arg_type,
                                &mut self.needs_option,
                                &mut self.needs_result,
                                &mut self.needs_list_utils,
                                &mut self.needs_string_utils,
                                &mut self.needs_math_utils,
                            );
                        }
                    }
                }
                LirDecl::Extern { source, name, .. } => {
                    // Register Java imports for java: extern sources.
                    // Non-java sources (e.g. npm:, py:) are silently ignored.
                    if let Some(package) = source.strip_prefix("java:") {
                        let import_line = format!("import {}.{};", package, name);
                        if !self.java_extern_imports.contains(&import_line) {
                            self.java_extern_imports.push(import_line);
                        }
                    }
                }
            }
            // Detect ForAll declarations for jqwik import and class name
            if let LirDecl::Function { body, .. } = decl {
                if matches!(*body, LirExpr::ForAll { .. }) {
                    self.needs_jqwik = true;
                    self.has_forall = true;
                }
            }
        }

        let mut buf = CodeBuffer::with_indent_size(4);

        // Header comment
        buf.push_line(format!(
            "// Generated by Dwarf v{} — edit the .kzd source, not this file",
            self.version
        ));
        buf.push_empty();

        // Package declaration
        buf.push_line(format!("package {};", self.package_name));
        buf.push_empty();

        // Import statements
        if self.needs_completable_future {
            buf.push_line("import java.util.concurrent.CompletableFuture;");
        }
        if self.needs_optional {
            buf.push_line("import java.util.Optional;");
        }
        if self.needs_jqwik {
            buf.push_line("import net.jqwik.api.*;");
        }
        if self.needs_option {
            buf.push_line("import dwarf.gen.Option;");
        }
        if self.needs_result {
            buf.push_line("import dwarf.gen.Result;");
        }
        if self.needs_list_utils {
            buf.push_line("import dwarf.gen.ListUtils;");
        }
        if self.needs_string_utils {
            buf.push_line("import dwarf.gen.StringUtils;");
        }
        if self.needs_math_utils {
            buf.push_line("import dwarf.gen.MathUtils;");
        }
        if self.needs_io_utils {
            buf.push_line("import dwarf.gen.IOUtils;");
        }
        // FFI imports from java: extern declarations
        for import_line in &self.java_extern_imports {
            buf.push_line(import_line);
        }
        if self.needs_completable_future
            || self.needs_optional
            || self.needs_jqwik
            || self.needs_option
            || self.needs_result
            || self.needs_list_utils
            || self.needs_string_utils
            || self.needs_math_utils
            || self.needs_io_utils
            || !self.java_extern_imports.is_empty()
        {
            buf.push_empty();
        }

        // Class declaration — use PropertyTests when ForAll declarations are present
        let class_name = if self.has_forall {
            "PropertyTests"
        } else {
            "Main"
        };
        buf.push_line(format!("public class {} {{", class_name));
        buf.indent();

        // Emit each declaration
        for (i, decl) in decls.iter().enumerate() {
            if i > 0 {
                buf.push_empty();
            }
            let decl_str = self.emit_decl(decl)?;
            for line in decl_str.lines() {
                buf.push_line(line);
            }
        }

        buf.dedent();
        buf.push_line("}");

        // Trim trailing newline from CodeBuffer::into_string()
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
                is_pub,
                ..
            } => {
                let access = if *is_pub { "public " } else { "" };
                let is_async = *hint == TargetHint::Async || *effect == Effect::Async;

                // Determine return type string
                let ret_type = if is_async {
                    // Wrap return type in CompletableFuture<>
                    let inner = match return_type {
                        Some(ty) => self.type_mapper.map_type(ty),
                        None => "Void".to_string(),
                    };
                    format!("CompletableFuture<{}>", inner)
                } else {
                    match return_type {
                        Some(ty) => self.type_mapper.map_type(ty),
                        None => "void".to_string(),
                    }
                };

                // Method name in camelCase
                let method_name = to_camel_case(name);

                // Parameters
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        let type_str = match &p.type_ {
                            Some(ty) => self.type_mapper.map_type(ty),
                            None => "Object".to_string(),
                        };
                        format!("{} {}", type_str, p.name)
                    })
                    .collect();

                let header = format!(
                    "{access}static {ret} {name}({params})",
                    access = access,
                    ret = ret_type,
                    name = method_name,
                    params = params_str.join(", ")
                );

                // Check for ForAll (property-based testing) body — emit @Property + @ForAll
                if let LirExpr::ForAll {
                    type_,
                    binding,
                    property,
                    ..
                } = body
                {
                    let (java_type, annotation) = self.type_to_jqwik_info(type_)?;
                    let binding_str = self.emit_pat(binding)?;
                    let prop_str = self.emit_expr(property)?;
                    let access = if *is_pub { "public " } else { "" };
                    let mut body_buf = CodeBuffer::with_indent_size(4);
                    body_buf.push_line("@Property");
                    body_buf.push_line(format!(
                        "{}boolean {}({} {} {}) {{",
                        access, method_name, annotation, java_type, binding_str
                    ));
                    body_buf.indent();
                    body_buf.push_line(format!("return {};", prop_str));
                    body_buf.dedent();
                    body_buf.push_line("}");
                    return Ok(body_buf.into_string().trim_end().to_string());
                }

                // Body handling
                match body {
                    LirExpr::Block { stmts, .. } => {
                        let mut body_buf = CodeBuffer::with_indent_size(4);
                        body_buf.push_line(format!("{} {{", header));
                        body_buf.indent();
                        for (i, stmt) in stmts.iter().enumerate() {
                            let is_last = i == stmts.len() - 1;
                            match stmt {
                                LirStmt::Let { pat, value } => {
                                    let val_str = self.emit_expr(value)?;
                                    let pat_str = self.emit_pat(pat)?;
                                    body_buf.push_line(format!("{} = {};", pat_str, val_str));
                                }
                                LirStmt::Expr(expr) => {
                                    let expr_str = self.emit_expr(expr)?;
                                    if is_last {
                                        body_buf.push_line(format!("return {};", expr_str));
                                    } else {
                                        body_buf.push_line(format!("{};", expr_str));
                                    }
                                }
                            }
                        }
                        body_buf.dedent();
                        body_buf.push_line("}");
                        Ok(body_buf.into_string().trim_end().to_string())
                    }
                    other => {
                        let body_str = self.emit_expr(other)?;
                        if body_str.starts_with("throw ") {
                            Ok(format!("{} {{ {}; }}", header, body_str))
                        } else if body_str.contains('\n') {
                            let mut body_buf = CodeBuffer::with_indent_size(4);
                            body_buf.push_line(format!("{} {{", header));
                            body_buf.indent();
                            for line in body_str.lines() {
                                body_buf.push_line(line);
                            }
                            body_buf.dedent();
                            body_buf.push_line("}");
                            Ok(body_buf.into_string().trim_end().to_string())
                        } else {
                            Ok(format!("{} {{ return {}; }}", header, body_str))
                        }
                    }
                }
            }
            LirDecl::RecordDef {
                name,
                fields,
                is_pub,
                ..
            } => {
                let access = if *is_pub { "public " } else { "" };
                let class_name = to_pascal_case(name);
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let type_str = self.type_mapper.map_type(&f.type_);
                        format!("{} {}", type_str, f.name)
                    })
                    .collect();
                Ok(format!(
                    "{access}record {name}({fields}) {{ }}",
                    access = access,
                    name = class_name,
                    fields = fields_str.join(", ")
                ))
            }
            LirDecl::UnionDef {
                name,
                variants,
                is_pub,
                ..
            } => {
                let access = if *is_pub { "public " } else { "" };
                let union_name = to_pascal_case(name);

                // Build the sealed interface with permits clause
                let variant_names: Vec<String> =
                    variants.iter().map(|v| to_pascal_case(&v.name)).collect();

                // We need to return multiple lines: the sealed interface + each variant record.
                // Build them as a single string with newlines.
                let mut result = String::new();

                // Sealed interface
                result.push_str(&format!(
                    "{access}sealed interface {name} permits {permits} {{ }}",
                    access = access,
                    name = union_name,
                    permits = variant_names.join(", ")
                ));

                // Variant records
                for variant in variants {
                    result.push('\n');
                    let v_name = to_pascal_case(&variant.name);
                    match &variant.arg {
                        Some(arg_type) => {
                            let type_str = self.type_mapper.map_type(arg_type);
                            result.push_str(&format!(
                                "{access}record {name}({type_} arg) implements {union} {{ }}",
                                access = access,
                                name = v_name,
                                type_ = type_str,
                                union = union_name
                            ));
                        }
                        None => {
                            result.push_str(&format!(
                                "{access}record {name}() implements {union} {{ }}",
                                access = access,
                                name = v_name,
                                union = union_name
                            ));
                        }
                    }
                }

                Ok(result)
            }
            LirDecl::Extern { .. } => {
                // The import statement (emitted in emit_module) handles visibility.
                // Emit nothing for the declaration itself, regardless of source.
                Ok(String::new())
            }
        }
    }

    fn emit_expr(&mut self, expr: &LirExpr) -> Result<String, EmitterError> {
        match expr {
            LirExpr::Literal { value, hint, .. } => match hint {
                TargetHint::Optional => match value {
                    LirLiteral::Null => Ok("Optional.empty()".to_string()),
                    _ => {
                        let val = self.emit_literal(value)?;
                        Ok(format!("Optional.of({})", val))
                    }
                },
                _ => self.emit_literal(value),
            },
            LirExpr::Variable { name, .. } => Ok(name.clone()),
            LirExpr::Call {
                func, args, hint, ..
            } => {
                // Check for assert/assert_eq
                if let LirExpr::Variable { name, .. } = func.as_ref() {
                    match name.as_str() {
                        "assert" if args.len() == 1 => {
                            let arg = self.emit_expr(&args[0])?;
                            return Ok(format!("assert {}", arg));
                        }
                        "assert_eq" if args.len() == 2 => {
                            let a = self.emit_expr(&args[0])?;
                            let b = self.emit_expr(&args[1])?;
                            return Ok(format!("assertEquals({}, {})", a, b));
                        }
                        _ => {}
                    }
                }
                let func_str = self.emit_expr(func)?;
                let args_str: Vec<String> = args
                    .iter()
                    .map(|a| self.emit_expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                let call = format!("{}({})", func_str, args_str.join(", "));
                if *hint == TargetHint::Async {
                    Ok(format!("CompletableFuture.supplyAsync(() -> {})", call))
                } else {
                    Ok(call)
                }
            }
            LirExpr::Member { obj, field, .. } => {
                let obj_str = self.emit_expr(obj)?;
                Ok(format!("{}.{}", obj_str, field))
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
            LirExpr::Match { expr, arms, .. } => {
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
                        if chain.is_empty() && arms.len() == 1 {
                            // Single wildcard arm — emit ternary with _ placeholder
                            chain = format!("_ == {} ? {} : {}", expr_str, body_str, body_str);
                        } else if chain.is_empty() {
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
                        let condition = format!("{} == {}", expr_str, pat_str);
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
                        Some(ty) => format!("{} {}", self.type_mapper.map_type(ty), p.name),
                        None => p.name.clone(),
                    })
                    .collect();
                let body_str = self.emit_expr(body)?;
                Ok(format!("({}) -> {}", params_str.join(", "), body_str))
            }
            LirExpr::Record { fields, .. } => {
                // Use structural bridge to generate a nominal record name
                let field_types: Vec<(String, Type)> = fields
                    .iter()
                    .map(|(name, _)| (name.clone(), Type::Named("Object".into())))
                    .collect();
                let record_name = self.bridge.register_record(&field_types, &self.type_mapper);
                let values: Vec<String> = fields
                    .iter()
                    .map(|(_, expr)| self.emit_expr(expr))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("new {}({})", record_name, values.join(", ")))
            }
            LirExpr::Variant { name, arg, .. } => match arg {
                Some(expr) => {
                    let val = self.emit_expr(expr)?;
                    Ok(format!("new {}({})", name, val))
                }
                None => Ok(format!("{}.INSTANCE", name)),
            },
            LirExpr::Array { items, .. } => {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|i| self.emit_expr(i))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("new Object[]{{{}}}", items_str.join(", ")))
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
            LirExpr::ForAll {
                type_,
                binding,
                property,
                ..
            } => {
                let ty_str = self.emit_type(type_)?;
                let binding_str = self.emit_pat(binding)?;
                let property_str = self.emit_expr(property)?;
                // ForAll is a property-based testing construct; emit as a comment
                // with the sub-expression for now.
                Ok(format!(
                    "/* forAll<{ty_str}>({binding_str} -> {property_str}) */"
                ))
            }
            LirExpr::AssertConsistent { expr, .. } => self.emit_expr(expr),
            LirExpr::Try {
                body,
                binding,
                guard,
                handler,
                ..
            } => {
                let body_str = self.emit_expr(body)?;
                let binding_str = self.emit_pat(binding)?;
                let handler_str = self.emit_expr(handler)?;
                match guard {
                    Some(guard_expr) => {
                        let guard_str = self.emit_expr(guard_expr)?;
                        Ok(format!(
                            "try {{\n    {}\n}} catch (Exception {}) {{\n    if ({}) {{\n        {}\n    }} else {{\n        throw {};\n    }}\n}}",
                            body_str, binding_str, guard_str, handler_str, binding_str
                        ))
                    }
                    None => Ok(format!(
                        "try {{\n    {}\n}} catch (Exception {}) {{\n    {}\n}}",
                        body_str, binding_str, handler_str
                    )),
                }
            }
            LirExpr::Throw { expr, .. } => {
                let expr_str = self.emit_expr(expr)?;
                match expr.as_ref() {
                    // In Dwarf, error constructors are emitted as call expressions
                    // (e.g. `Error("msg")`). In Java these must be prefixed with
                    // `new`. For any other expression (variable, literal, etc.)
                    // emit it unchanged.
                    LirExpr::Call { .. } => Ok(format!("throw new {}", expr_str)),
                    _ => Ok(format!("throw {}", expr_str)),
                }
            }
            LirExpr::Propagate { expr, .. } => {
                let expr_str = self.emit_expr(expr)?;
                Ok(format!(
                    "((java.util.function.Supplier<Object>)(() -> {{ Object __v = {}; if (Result.isErr(__v)) {{ return __v; }} return __v.value; }})).get()",
                    expr_str
                ))
            }
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
                    Ok(format!("{}({})", name, inner))
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
            LirBinaryOp::Eq => Ok(" == ".into()),
            LirBinaryOp::Ne => Ok(" != ".into()),
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
            TargetHint::Async => Ok(String::new()), // handled via CompletableFuture at type level
            TargetHint::Optional => Ok(String::new()),
            TargetHint::Result => Ok(String::new()),
            TargetHint::ReactComponent => Ok(String::new()),
        }
    }

    fn emit_effect(&mut self, effect: &Effect) -> Result<String, EmitterError> {
        match effect {
            Effect::Pure => Ok(String::new()),
            Effect::Async => Ok(String::new()), // handled via CompletableFuture at type level
            Effect::Impure => Ok(String::new()),
        }
    }
}

// ------------------------------------------------------------------
// Internal helpers on JavaBackend
// ------------------------------------------------------------------

impl JavaBackend {
    /// Map a Dwarf type to a (Java type, jqwik annotations) pair for
    /// property-based testing with `@ForAll`.
    ///
    /// | Dwarf Type | Java type | jqwik annotation |
    /// |---|---|---|
    /// | `Int` | `int` | `@ForAll @IntGenerator("int")` |
    /// | `String` | `String` | `@ForAll @StringGenerator` |
    /// | `Bool` | `boolean` | `@ForAll` |
    /// | other | `Object` | `@ForAll` |
    fn type_to_jqwik_info(&self, ty: &Type) -> Result<(String, String), EmitterError> {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" => Ok(("int".into(), "@ForAll".into())),
                "String" => Ok(("String".into(), "@ForAll".into())),
                "Bool" => Ok(("boolean".into(), "@ForAll".into())),
                _ => Ok(("Object".into(), "@ForAll".into())),
            },
            _ => Ok(("Object".into(), "@ForAll".into())),
        }
    }
    /// Emit a block body (stmts) as a single-line `{ ... }` string.
    ///
    /// For Let statements we produce `pat = value` (Java has no `let` keyword).
    /// The last expression statement gets a `return` prefix.
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
                    let pat_str = self.emit_pat(pat)?;
                    parts.push(format!("{} = {}", pat_str, val_str));
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

    /// Scan a type for stdlib references and mark needed imports.
    #[allow(clippy::only_used_in_recursion)]
    fn scan_type_for_stdlib(
        ty: &Type,
        needs_option: &mut bool,
        needs_result: &mut bool,
        needs_list: &mut bool,
        needs_string: &mut bool,
        needs_math: &mut bool,
    ) {
        match ty {
            Type::Generic { base, args } => {
                match base.as_str() {
                    "Option" => *needs_option = true,
                    "Result" => *needs_result = true,
                    "List" => *needs_list = true,
                    _ => {}
                }
                for arg in args {
                    Self::scan_type_for_stdlib(
                        arg,
                        needs_option,
                        needs_result,
                        needs_list,
                        needs_string,
                        needs_math,
                    );
                }
            }
            Type::Record(fields) => {
                for (_, field_type) in fields {
                    Self::scan_type_for_stdlib(
                        field_type,
                        needs_option,
                        needs_result,
                        needs_list,
                        needs_string,
                        needs_math,
                    );
                }
            }
            Type::Union(variants) => {
                for variant in variants {
                    Self::scan_type_for_stdlib(
                        variant,
                        needs_option,
                        needs_result,
                        needs_list,
                        needs_string,
                        needs_math,
                    );
                }
            }
            Type::Func { params, return_ } => {
                for param in params {
                    Self::scan_type_for_stdlib(
                        param,
                        needs_option,
                        needs_result,
                        needs_list,
                        needs_string,
                        needs_math,
                    );
                }
                Self::scan_type_for_stdlib(
                    return_,
                    needs_option,
                    needs_result,
                    needs_list,
                    needs_string,
                    needs_math,
                );
            }
            Type::Refined { base, .. } => Self::scan_type_for_stdlib(
                base,
                needs_option,
                needs_result,
                needs_list,
                needs_string,
                needs_math,
            ),
            Type::KeyOf(inner) => Self::scan_type_for_stdlib(
                inner,
                needs_option,
                needs_result,
                needs_list,
                needs_string,
                needs_math,
            ),
            Type::IndexedAccess { obj, .. } => Self::scan_type_for_stdlib(
                obj,
                needs_option,
                needs_result,
                needs_list,
                needs_string,
                needs_math,
            ),
            Type::Named(_) => {}
        }
    }

    /// Scan an expression for stdlib calls (I/O, String, Math, Result, etc.)
    fn scan_expr_for_stdlib(
        expr: &LirExpr,
        needs_io: &mut bool,
        needs_string: &mut bool,
        needs_math: &mut bool,
        needs_result: &mut bool,
    ) {
        match expr {
            LirExpr::Call { func, args, .. } => {
                if let LirExpr::Variable { name, .. } = func.as_ref() {
                    match name.as_str() {
                        "print" | "readFile" | "writeFile" => *needs_io = true,
                        _ => {}
                    }
                }
                if let LirExpr::Member { obj, .. } = func.as_ref() {
                    if let LirExpr::Variable { name, .. } = obj.as_ref() {
                        match name.as_str() {
                            "String" => *needs_string = true,
                            "Math" => *needs_math = true,
                            _ => {}
                        }
                    }
                }
                for arg in args {
                    Self::scan_expr_for_stdlib(
                        arg,
                        needs_io,
                        needs_string,
                        needs_math,
                        needs_result,
                    );
                }
            }
            LirExpr::Block { stmts, .. } => {
                for stmt in stmts {
                    match stmt {
                        LirStmt::Let { value, .. } => Self::scan_expr_for_stdlib(
                            value,
                            needs_io,
                            needs_string,
                            needs_math,
                            needs_result,
                        ),
                        LirStmt::Expr(e) => Self::scan_expr_for_stdlib(
                            e,
                            needs_io,
                            needs_string,
                            needs_math,
                            needs_result,
                        ),
                    }
                }
            }
            LirExpr::Lambda { body, .. } => {
                Self::scan_expr_for_stdlib(body, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                Self::scan_expr_for_stdlib(cond, needs_io, needs_string, needs_math, needs_result);
                Self::scan_expr_for_stdlib(then, needs_io, needs_string, needs_math, needs_result);
                if let Some(el) = else_ {
                    Self::scan_expr_for_stdlib(
                        el,
                        needs_io,
                        needs_string,
                        needs_math,
                        needs_result,
                    );
                }
            }
            LirExpr::Binary { lhs, rhs, .. } => {
                Self::scan_expr_for_stdlib(lhs, needs_io, needs_string, needs_math, needs_result);
                Self::scan_expr_for_stdlib(rhs, needs_io, needs_string, needs_math, needs_result);
            }
            LirExpr::Unary { expr, .. } => {
                Self::scan_expr_for_stdlib(expr, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::Assign { target, value, .. } => {
                Self::scan_expr_for_stdlib(
                    target,
                    needs_io,
                    needs_string,
                    needs_math,
                    needs_result,
                );
                Self::scan_expr_for_stdlib(value, needs_io, needs_string, needs_math, needs_result);
            }
            LirExpr::Member { obj, .. } => {
                Self::scan_expr_for_stdlib(obj, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::Record { fields, .. } => {
                for (_, val) in fields {
                    Self::scan_expr_for_stdlib(
                        val,
                        needs_io,
                        needs_string,
                        needs_math,
                        needs_result,
                    );
                }
            }
            LirExpr::Array { items, .. } => {
                for item in items {
                    Self::scan_expr_for_stdlib(
                        item,
                        needs_io,
                        needs_string,
                        needs_math,
                        needs_result,
                    );
                }
            }
            LirExpr::Variant { arg: Some(a), .. } => {
                Self::scan_expr_for_stdlib(a, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::Variant { arg: None, .. } => {}
            LirExpr::ForAll { property, .. } => Self::scan_expr_for_stdlib(
                property,
                needs_io,
                needs_string,
                needs_math,
                needs_result,
            ),
            LirExpr::AssertConsistent { expr, .. } => {
                Self::scan_expr_for_stdlib(expr, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::Try {
                body,
                guard,
                handler,
                ..
            } => {
                Self::scan_expr_for_stdlib(body, needs_io, needs_string, needs_math, needs_result);
                if let Some(g) = guard {
                    Self::scan_expr_for_stdlib(g, needs_io, needs_string, needs_math, needs_result);
                }
                Self::scan_expr_for_stdlib(
                    handler,
                    needs_io,
                    needs_string,
                    needs_math,
                    needs_result,
                );
            }
            LirExpr::Throw { expr, .. } => {
                Self::scan_expr_for_stdlib(expr, needs_io, needs_string, needs_math, needs_result)
            }
            LirExpr::Propagate { expr, .. } => {
                *needs_result = true;
                Self::scan_expr_for_stdlib(expr, needs_io, needs_string, needs_math, needs_result);
            }
            _ => {}
        }
    }

    /// Recursively scan a declaration for imports needed by the module.
    fn scan_decl_for_imports(decl: &LirDecl, needs_cf: &mut bool, needs_opt: &mut bool) {
        match decl {
            LirDecl::Function {
                effect, hint, body, ..
            } => {
                if *effect == Effect::Async || *hint == TargetHint::Async {
                    *needs_cf = true;
                }
                Self::scan_expr_for_imports(body, needs_cf, needs_opt);
            }
            LirDecl::RecordDef { .. } | LirDecl::UnionDef { .. } | LirDecl::Extern { .. } => {}
        }
    }

    /// Recursively scan an expression for imports needed by the module.
    fn scan_expr_for_imports(expr: &LirExpr, needs_cf: &mut bool, needs_opt: &mut bool) {
        match expr {
            LirExpr::Literal { hint, .. } => {
                if *hint == TargetHint::Optional {
                    *needs_opt = true;
                }
            }
            LirExpr::Variable { .. } => {}
            LirExpr::Call {
                hint, args, func, ..
            } => {
                if *hint == TargetHint::Async {
                    *needs_cf = true;
                }
                Self::scan_expr_for_imports(func, needs_cf, needs_opt);
                for arg in args {
                    Self::scan_expr_for_imports(arg, needs_cf, needs_opt);
                }
            }
            LirExpr::Member { obj, .. } => {
                Self::scan_expr_for_imports(obj, needs_cf, needs_opt);
            }
            LirExpr::If {
                cond, then, else_, ..
            } => {
                Self::scan_expr_for_imports(cond, needs_cf, needs_opt);
                Self::scan_expr_for_imports(then, needs_cf, needs_opt);
                if let Some(e) = else_ {
                    Self::scan_expr_for_imports(e, needs_cf, needs_opt);
                }
            }
            LirExpr::Match { expr, arms, .. } => {
                Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
                for arm in arms {
                    Self::scan_expr_for_imports(&arm.body, needs_cf, needs_opt);
                }
            }
            LirExpr::Block { stmts, .. } => {
                for stmt in stmts {
                    match stmt {
                        LirStmt::Let { value, .. } => {
                            Self::scan_expr_for_imports(value, needs_cf, needs_opt);
                        }
                        LirStmt::Expr(e) => {
                            Self::scan_expr_for_imports(e, needs_cf, needs_opt);
                        }
                    }
                }
            }
            LirExpr::Assign { target, value, .. } => {
                Self::scan_expr_for_imports(target, needs_cf, needs_opt);
                Self::scan_expr_for_imports(value, needs_cf, needs_opt);
            }
            LirExpr::Lambda { body, .. } => {
                Self::scan_expr_for_imports(body, needs_cf, needs_opt);
            }
            LirExpr::Record { fields, .. } => {
                for (_, expr) in fields {
                    Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
                }
            }
            LirExpr::Variant { arg, .. } => {
                if let Some(e) = arg {
                    Self::scan_expr_for_imports(e, needs_cf, needs_opt);
                }
            }
            LirExpr::Array { items, .. } => {
                for item in items {
                    Self::scan_expr_for_imports(item, needs_cf, needs_opt);
                }
            }
            LirExpr::Binary { lhs, rhs, .. } => {
                Self::scan_expr_for_imports(lhs, needs_cf, needs_opt);
                Self::scan_expr_for_imports(rhs, needs_cf, needs_opt);
            }
            LirExpr::Unary { expr, .. } => {
                Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
            }
            LirExpr::Wildcard { .. } => {}
            LirExpr::ForAll { property, .. } => {
                Self::scan_expr_for_imports(property, needs_cf, needs_opt);
            }
            LirExpr::AssertConsistent { expr, .. } => {
                Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
            }
            LirExpr::Try {
                body,
                guard,
                handler,
                ..
            } => {
                Self::scan_expr_for_imports(body, needs_cf, needs_opt);
                if let Some(g) = guard {
                    Self::scan_expr_for_imports(g, needs_cf, needs_opt);
                }
                Self::scan_expr_for_imports(handler, needs_cf, needs_opt);
            }
            LirExpr::Throw { expr, .. } => {
                Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
            }
            LirExpr::Propagate { expr, .. } => {
                Self::scan_expr_for_imports(expr, needs_cf, needs_opt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::{Effect, LirBinaryOp, LirLiteral, LirUnaryOp, TargetHint};
    use dwarf_syntax::hir::Type;

    // ==================================================================
    // Helpers
    // ==================================================================

    fn make_backend() -> JavaBackend {
        JavaBackend::new("dwarf.gen", "0.1.0")
    }

    // ==================================================================
    // Creation tests
    // ==================================================================

    #[test]
    fn test_java_backend_new() {
        let backend = JavaBackend::new("com.example", "1.0.0");
        assert!(
            backend.buffer.is_empty(),
            "new backend should have empty buffer"
        );
    }

    #[test]
    fn test_java_backend_default() {
        let backend = JavaBackend::default();
        assert!(
            backend.buffer.is_empty(),
            "default backend should have empty buffer"
        );
        assert_eq!(backend.package_name, "dwarf.gen");
        assert_eq!(backend.version, "0.1.0");
    }

    #[test]
    fn test_into_output_empty() {
        let backend = JavaBackend::new("test", "0.1.0");
        assert_eq!(backend.into_output(), "");
    }

    // ==================================================================
    // Literal emission
    // ==================================================================

    #[test]
    fn test_emit_literal_int() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_literal(&LirLiteral::Int(42)).unwrap(), "42");
    }

    #[test]
    fn test_emit_literal_float() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_literal(&LirLiteral::Float(3.5)).unwrap(),
            "3.5"
        );
    }

    #[test]
    fn test_emit_literal_str() {
        let mut backend = make_backend();
        assert_eq!(
            backend
                .emit_literal(&LirLiteral::Str("hello".into()))
                .unwrap(),
            "\"hello\""
        );
    }

    #[test]
    fn test_emit_literal_bool_true() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_literal(&LirLiteral::Bool(true)).unwrap(),
            "true"
        );
    }

    #[test]
    fn test_emit_literal_bool_false() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_literal(&LirLiteral::Bool(false)).unwrap(),
            "false"
        );
    }

    #[test]
    fn test_emit_literal_null() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_literal(&LirLiteral::Null).unwrap(), "null");
    }

    // ==================================================================
    // Binary operator emission
    // ==================================================================

    #[test]
    fn test_emit_binary_op_add() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Add).unwrap(), " + ");
    }

    #[test]
    fn test_emit_binary_op_sub() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Sub).unwrap(), " - ");
    }

    #[test]
    fn test_emit_binary_op_mul() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Mul).unwrap(), " * ");
    }

    #[test]
    fn test_emit_binary_op_div() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Div).unwrap(), " / ");
    }

    #[test]
    fn test_emit_binary_op_eq() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Eq).unwrap(), " == ");
    }

    #[test]
    fn test_emit_binary_op_ne() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ne).unwrap(), " != ");
    }

    #[test]
    fn test_emit_binary_op_lt() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Lt).unwrap(), " < ");
    }

    #[test]
    fn test_emit_binary_op_gt() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Gt).unwrap(), " > ");
    }

    #[test]
    fn test_emit_binary_op_le() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Le).unwrap(), " <= ");
    }

    #[test]
    fn test_emit_binary_op_ge() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Ge).unwrap(), " >= ");
    }

    #[test]
    fn test_emit_binary_op_and() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::And).unwrap(), " && ");
    }

    #[test]
    fn test_emit_binary_op_or() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_binary_op(&LirBinaryOp::Or).unwrap(), " || ");
    }

    // ==================================================================
    // Unary operator emission
    // ==================================================================

    #[test]
    fn test_emit_unary_op_neg() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Neg).unwrap(), "-");
    }

    #[test]
    fn test_emit_unary_op_not() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_unary_op(&LirUnaryOp::Not).unwrap(), "!");
    }

    // ==================================================================
    // Target hint emission — all return empty strings for Java
    // ==================================================================

    #[test]
    fn test_emit_target_hint_none() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_target_hint(&TargetHint::None).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_async() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_target_hint(&TargetHint::Async).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_optional() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_target_hint(&TargetHint::Optional).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_result() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_target_hint(&TargetHint::Result).unwrap(), "");
    }

    #[test]
    fn test_emit_target_hint_react_component() {
        let mut backend = make_backend();
        assert_eq!(
            backend
                .emit_target_hint(&TargetHint::ReactComponent)
                .unwrap(),
            ""
        );
    }

    // ==================================================================
    // Effect emission — all return empty strings for Java
    // ==================================================================

    #[test]
    fn test_emit_effect_pure() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_effect(&Effect::Pure).unwrap(), "");
    }

    #[test]
    fn test_emit_effect_async() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_effect(&Effect::Async).unwrap(), "");
    }

    #[test]
    fn test_emit_effect_impure() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_effect(&Effect::Impure).unwrap(), "");
    }

    // ==================================================================
    // Type emission — delegates to JavaMapper
    // ==================================================================

    #[test]
    fn test_emit_type_int() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_type(&Type::Named("Int".into())).unwrap(),
            "int"
        );
    }

    #[test]
    fn test_emit_type_string() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_type(&Type::Named("String".into())).unwrap(),
            "String"
        );
    }

    #[test]
    fn test_emit_type_bool() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_type(&Type::Named("Bool".into())).unwrap(),
            "boolean"
        );
    }

    #[test]
    fn test_emit_type_generic() {
        let mut backend = make_backend();
        let ty = Type::Generic {
            base: "List".into(),
            args: vec![Type::Named("String".into())],
        };
        assert_eq!(backend.emit_type(&ty).unwrap(), "List<String>");
    }

    // ==================================================================
    // Pattern emission
    // ==================================================================

    #[test]
    fn test_emit_pat_wildcard() {
        let mut backend = make_backend();
        assert_eq!(backend.emit_pat(&LirPat::Wildcard).unwrap(), "_");
    }

    #[test]
    fn test_emit_pat_literal() {
        let mut backend = make_backend();
        assert_eq!(
            backend
                .emit_pat(&LirPat::Literal(LirLiteral::Int(42)))
                .unwrap(),
            "42"
        );
    }

    #[test]
    fn test_emit_pat_variable() {
        let mut backend = make_backend();
        assert_eq!(
            backend.emit_pat(&LirPat::Variable("myVar".into())).unwrap(),
            "myVar"
        );
    }

    #[test]
    fn test_emit_pat_variant_no_arg() {
        let mut backend = make_backend();
        let pat = LirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "None");
    }

    #[test]
    fn test_emit_pat_variant_with_arg() {
        let mut backend = make_backend();
        let pat = LirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirPat::Variable("inner".into()))),
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "Some(inner)");
    }

    #[test]
    fn test_emit_pat_record_no_rest() {
        let mut backend = make_backend();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _ }");
    }

    #[test]
    fn test_emit_pat_record_with_rest() {
        let mut backend = make_backend();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: true,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: _, ... }");
    }

    #[test]
    fn test_emit_pat_record_empty() {
        let mut backend = make_backend();
        let pat = LirPat::Record {
            fields: vec![],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{  }");
    }

    #[test]
    fn test_emit_pat_record_multiple_fields() {
        let mut backend = make_backend();
        let pat = LirPat::Record {
            fields: vec![
                ("x".into(), LirPat::Variable("a".into())),
                ("y".into(), LirPat::Variable("b".into())),
            ],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{ x: a, y: b }");
    }

    // ==================================================================
    // Real implementation tests for emit_module / emit_decl / emit_expr
    // ==================================================================

    #[test]
    fn test_emit_module_empty() {
        let mut backend = make_backend();
        let result = backend.emit_module(&[]).unwrap();
        assert_eq!(result, "", "empty module should produce empty string");
    }

    #[test]
    fn test_emit_decl_function_simple() {
        let mut backend = make_backend();
        let decl = LirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: TargetHint::None,
                span: dwarf_syntax::span::Span::new(0, 0, 0),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            is_generator: false,
            span: dwarf_syntax::span::Span::new(0, 0, 0),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(
            result.contains("static void f("),
            "should contain static method signature"
        );
        assert!(
            result.contains("return 0;"),
            "should contain return statement"
        );
    }

    #[test]
    fn test_emit_expr_literal_int() {
        let mut backend = make_backend();
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: TargetHint::None,
            span: dwarf_syntax::span::Span::new(0, 0, 0),
        };
        let result = backend.emit_expr(&expr).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_emit_decl_function_public_has_public_modifier() {
        let mut backend = make_backend();
        let decl = LirDecl::Function {
            name: "pubFn".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: dwarf_syntax::span::Span::new(0, 0, 0),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            is_generator: false,
            span: dwarf_syntax::span::Span::new(0, 0, 0),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(
            result.starts_with("public "),
            "public fn should have public modifier"
        );
    }

    #[test]
    fn test_emit_decl_function_private_has_no_public_modifier() {
        let mut backend = make_backend();
        let decl = LirDecl::Function {
            name: "privFn".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Null,
                hint: TargetHint::None,
                span: dwarf_syntax::span::Span::new(0, 0, 0),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: false,
            is_generator: false,
            span: dwarf_syntax::span::Span::new(0, 0, 0),
        };
        let result = backend.emit_decl(&decl).unwrap();
        assert!(
            !result.starts_with("public "),
            "private fn should not have public modifier"
        );
    }
}
