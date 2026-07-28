//! Python backend implementation.
//!
//! Implements the [`EmitterBackend`] trait for Python code generation.
//! Currently provides real implementations for literals, binary ops,
//! unary ops, target hints, effects, declarations, and module emission.

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirStmt, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;
use crate::format::CodeBuffer;
use crate::naming::{to_pascal_case, to_snake_case};
use crate::py::mapper::PythonMapper;
use crate::types::TypeMapper;

/// Python indentation width in spaces (4-space convention).
const PY_INDENT: &str = "    ";

/// A backend that emits Python code from LIR declarations.
///
/// Each method accepts a reference to a LIR construct and produces
/// a Python string representation. The `Output` type is `String`,
/// containing the complete emitted module.
pub struct PythonBackend {
    buffer: CodeBuffer,
    type_mapper: PythonMapper,
    version: String,
    needs_dataclass: bool,
    needs_typing_union: bool,
    needs_hypothesis: bool,
    needs_option: bool,
    needs_result: bool,
    needs_list_utils: bool,
    needs_string_utils: bool,
    needs_math_utils: bool,
    needs_io_utils: bool,
}

impl PythonBackend {
    /// Create a new `PythonBackend` with an empty buffer (4-space indent).
    pub fn new() -> Self {
        Self {
            buffer: CodeBuffer::with_indent_size(4),
            type_mapper: PythonMapper,
            version: String::new(),
            needs_dataclass: false,
            needs_typing_union: false,
            needs_hypothesis: false,
            needs_option: false,
            needs_result: false,
            needs_list_utils: false,
            needs_string_utils: false,
            needs_math_utils: false,
            needs_io_utils: false,
        }
    }

    /// Return a reference to the internal [`CodeBuffer`].
    pub fn buffer(&self) -> &CodeBuffer {
        &self.buffer
    }

    /// Consume the backend and return the accumulated output as a `String`.
    pub fn into_output(self) -> String {
        self.buffer.into_string()
    }

    /// Set the version string for the generated file header.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Emit a block body (stmts) as a single-line `{ ... }` string.
    ///
    /// For Let statements we produce `pat = value` (Python has no `let` keyword).
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

    /// Emit the body of a function declaration (multi-line with Python indentation).
    ///
    /// Handles `Block` bodies by inlining statements, and wraps bare expressions
    /// with a `return` statement.
    fn emit_function_body(&mut self, body: &LirExpr) -> Result<Vec<String>, EmitterError> {
        match body {
            LirExpr::Block { stmts, .. } => {
                if stmts.is_empty() {
                    return Ok(vec![format!("{}pass", PY_INDENT)]);
                }
                let mut lines: Vec<String> = Vec::new();
                for (i, stmt) in stmts.iter().enumerate() {
                    let is_last = i == stmts.len() - 1;
                    match stmt {
                        LirStmt::Let { pat, value } => {
                            let val_str = self.emit_expr(value)?;
                            let pat_str = self.emit_pat(pat)?;
                            lines.push(format!("{}{} = {}", PY_INDENT, pat_str, val_str));
                        }
                        LirStmt::Expr(expr) => {
                            let expr_str = self.emit_expr(expr)?;
                            if is_last {
                                lines.push(format!("{}return {}", PY_INDENT, expr_str));
                            } else {
                                lines.push(format!("{}{}", PY_INDENT, expr_str));
                            }
                        }
                    }
                }
                Ok(lines)
            }
            other => {
                let expr_str = self.emit_expr(other)?;
                Ok(vec![format!("{}return {}", PY_INDENT, expr_str)])
            }
        }
    }

    /// Map a Dwarf type to a Hypothesis strategy generator expression.
    ///
    /// | Dwarf Type | Hypothesis generator |
    /// |---|---|
    /// | `Int` | `st.integers()` |
    /// | `String` | `st.text()` |
    /// | `Bool` | `st.booleans()` |
    /// | `List<X>` | `st.lists(<X_gen>)` |
    /// | other | `st.just(None)` |
    fn type_to_st_generator(&mut self, ty: &Type) -> Result<String, EmitterError> {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" => Ok("st.integers()".to_string()),
                "String" => Ok("st.text()".to_string()),
                "Bool" => Ok("st.booleans()".to_string()),
                "Float" => Ok("st.floats()".to_string()),
                _ => Ok("st.just(None)".to_string()),
            },
            Type::Generic { base, args } => match base.as_str() {
                "List" if args.len() == 1 => {
                    let elem_gen = self.type_to_st_generator(&args[0])?;
                    Ok(format!("st.lists({})", elem_gen))
                }
                "Option" if args.len() == 1 => {
                    let inner_gen = self.type_to_st_generator(&args[0])?;
                    Ok(format!("st.one_of(st.none(), {})", inner_gen))
                }
                "Result" if args.len() == 2 => {
                    let ok_gen = self.type_to_st_generator(&args[0])?;
                    let err_gen = self.type_to_st_generator(&args[1])?;
                    Ok(format!("st.one_of({}, {})", ok_gen, err_gen))
                }
                "Map" if args.len() == 2 => {
                    let value_gen = self.type_to_st_generator(&args[1])?;
                    Ok(format!("st.dictionaries(st.text(), {})", value_gen))
                }
                _ => Ok("st.just(None)".to_string()),
            },
            _ => Ok("st.just(None)".to_string()),
        }
    }

    /// Build the header comment string, optionally including the version.
    fn header_comment(&self) -> String {
        if self.version.is_empty() {
            "# Generated by Dwarf — edit the .kzd source, not this file".to_string()
        } else {
            format!(
                "# Generated by Dwarf v{} — edit the .kzd source, not this file",
                self.version
            )
        }
    }

    /// Scan a type for stdlib references and mark needed imports.
    fn register_stdlib_imports(&mut self, ty: &Type) {
        match ty {
            Type::Generic { base, args } => {
                match base.as_str() {
                    "Option" => self.needs_option = true,
                    "Result" => self.needs_result = true,
                    "List" => self.needs_list_utils = true,
                    "Math" => self.needs_math_utils = true,
                    _ => {}
                }
                for arg in args {
                    self.register_stdlib_imports(arg);
                }
            }
            Type::Record(fields) => {
                for (_, field_type) in fields {
                    self.register_stdlib_imports(field_type);
                }
            }
            Type::Union(variants) => {
                for variant in variants {
                    self.register_stdlib_imports(variant);
                }
            }
            Type::Func { params, return_ } => {
                for param in params {
                    self.register_stdlib_imports(param);
                }
                self.register_stdlib_imports(return_);
            }
            Type::Refined { base, .. } => self.register_stdlib_imports(base),
            Type::Named(_) => {}
        }
    }

    /// Walk an expression tree and mark needed stdlib imports.
    fn scan_expr_for_stdlib(&mut self, expr: &LirExpr) {
        match expr {
            LirExpr::Call { func, args, .. } => {
                // Check for module-style calls: String.split, etc.
                if let LirExpr::Member { obj, .. } = func.as_ref() {
                    if let LirExpr::Variable { name, .. } = obj.as_ref() {
                        match name.as_str() {
                            "String" => self.needs_string_utils = true,
                            "List" => self.needs_list_utils = true,
                            "Math" => self.needs_math_utils = true,
                            _ => {}
                        }
                    }
                }
                // Check for bare function calls: print, readFile, writeFile
                if let LirExpr::Variable { name, .. } = func.as_ref() {
                    match name.as_str() {
                        "print" | "readFile" | "writeFile" => self.needs_io_utils = true,
                        _ => {}
                    }
                }
                for arg in args {
                    self.scan_expr_for_stdlib(arg);
                }
            }
            LirExpr::Block { stmts, .. } => {
                for stmt in stmts {
                    match stmt {
                        LirStmt::Let { value, .. } => self.scan_expr_for_stdlib(value),
                        LirStmt::Expr(e) => self.scan_expr_for_stdlib(e),
                    }
                }
            }
            LirExpr::Lambda { body, .. } => self.scan_expr_for_stdlib(body),
            LirExpr::If { cond, then, else_, .. } => {
                self.scan_expr_for_stdlib(cond);
                self.scan_expr_for_stdlib(then);
                if let Some(el) = else_ {
                    self.scan_expr_for_stdlib(el);
                }
            }
            LirExpr::Match { expr, arms, .. } => {
                self.scan_expr_for_stdlib(expr);
                for arm in arms {
                    self.scan_expr_for_stdlib(&arm.body);
                }
            }
            LirExpr::Binary { lhs, rhs, .. } => {
                self.scan_expr_for_stdlib(lhs);
                self.scan_expr_for_stdlib(rhs);
            }
            LirExpr::Unary { expr, .. } => self.scan_expr_for_stdlib(expr),
            LirExpr::Assign { target, value, .. } => {
                self.scan_expr_for_stdlib(target);
                self.scan_expr_for_stdlib(value);
            }
            LirExpr::Member { obj, .. } => self.scan_expr_for_stdlib(obj),
            LirExpr::Record { fields, .. } => {
                for (_, val) in fields {
                    self.scan_expr_for_stdlib(val);
                }
            }
            LirExpr::Array { items, .. } => {
                for item in items {
                    self.scan_expr_for_stdlib(item);
                }
            }
            LirExpr::Variant { arg, .. } => {
                if let Some(a) = arg {
                    self.scan_expr_for_stdlib(a);
                }
            }
            LirExpr::ForAll { property, .. } => self.scan_expr_for_stdlib(property),
            LirExpr::AssertConsistent { expr, .. } => self.scan_expr_for_stdlib(expr),
            LirExpr::Variable { .. }
            | LirExpr::Literal { .. }
            | LirExpr::Wildcard { .. } => {}
        }
    }
}

impl Default for PythonBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterBackend for PythonBackend {
    type Output = String;

    fn emit_module(&mut self, decls: &[LirDecl]) -> Result<String, EmitterError> {
        if decls.is_empty() {
            return Ok(String::new());
        }

        // First pass: collect required imports
        for decl in decls {
            match decl {
                LirDecl::RecordDef { .. } => self.needs_dataclass = true,
                LirDecl::UnionDef { .. } => self.needs_typing_union = true,
                LirDecl::Function { body, .. } => {
                    if matches!(*body, LirExpr::ForAll { .. }) {
                        self.needs_hypothesis = true;
                    }
                }
            }
        }

        // Second pass: walk types and expressions for stdlib usage
        for decl in decls {
            match decl {
                LirDecl::Function {
                    params,
                    return_type,
                    body,
                    ..
                } => {
                    for param in params {
                        if let Some(ref ty) = param.type_ {
                            self.register_stdlib_imports(ty);
                        }
                    }
                    if let Some(ref ty) = return_type {
                        self.register_stdlib_imports(ty);
                    }
                    self.scan_expr_for_stdlib(body);
                }
                LirDecl::RecordDef { fields, .. } => {
                    for field in fields {
                        self.register_stdlib_imports(&field.type_);
                    }
                }
                LirDecl::UnionDef { variants, .. } => {
                    for variant in variants {
                        if let Some(ref arg_type) = variant.arg {
                            self.register_stdlib_imports(arg_type);
                        }
                    }
                }
            }
        }

        let mut parts: Vec<String> = Vec::new();

        // Header comment
        parts.push(self.header_comment());

        // Imports
        let mut imports: Vec<String> = Vec::new();
        if self.needs_dataclass {
            imports.push("from dataclasses import dataclass".to_string());
        }
        if self.needs_typing_union {
            imports.push("from typing import Union".to_string());
        }
        if self.needs_hypothesis {
            imports.push("from hypothesis import given, strategies as st".to_string());
        }
        if self.needs_option {
            imports.push(
                "from dwarf_runtime.option import Option, some, none, is_some, is_none"
                    .to_string(),
            );
        }
        if self.needs_result {
            imports.push(
                "from dwarf_runtime.result import Result, ok, err, is_ok, is_err".to_string(),
            );
        }
        if self.needs_list_utils {
            imports.push(
                "from dwarf_runtime.list_utils import map_list, filter_list, reduce_list, sum_list, sort_list, reverse_list, list_length"
                    .to_string(),
            );
        }
        if self.needs_string_utils {
            imports.push(
                "from dwarf_runtime.string_utils import split, to_upper, to_lower, reverse, contains, trim, string_length"
                    .to_string(),
            );
        }
        if self.needs_math_utils {
            imports.push("from dwarf_runtime.math_utils import abs, max, min".to_string());
        }
        if self.needs_io_utils {
            imports.push(
                "from dwarf_runtime.io_utils import print_out, read_file, write_file".to_string(),
            );
        }

        if !imports.is_empty() {
            parts.push(String::new()); // blank line after header
            parts.extend(imports);
            // Two blank lines after imports (Python convention)
            parts.push(String::new());
            parts.push(String::new());
        } else {
            parts.push(String::new()); // blank line after header, no imports
        }

        // Emit each declaration
        for (i, decl) in decls.iter().enumerate() {
            if i > 0 {
                parts.push(String::new()); // blank line between declarations
            }
            let decl_str = self.emit_decl(decl)?;
            for line in decl_str.lines() {
                parts.push(line.to_string());
            }
        }

        // No trailing newline
        Ok(parts.join("\n"))
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
                let mut lines: Vec<String> = Vec::new();

                // Async prefix
                let async_prefix = if *hint == TargetHint::Async || *effect == Effect::Async {
                    "async "
                } else {
                    ""
                };

                // Function name in snake_case
                let fn_name = to_snake_case(name);

                // Build parameter list
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if let Some(type_) = &p.type_ {
                            format!("{}: {}", p.name, self.type_mapper.map_type(type_))
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect();

                // Return type annotation
                let ret_str = if let Some(rt) = return_type {
                    format!(" -> {}", self.type_mapper.map_type(rt))
                } else {
                    String::new()
                };

                // Def line: `[async] def name(params) -> ret:`
                lines.push(format!(
                    "{}def {}({}){}:",
                    async_prefix,
                    fn_name,
                    params_str.join(", "),
                    ret_str
                ));

                // Check for ForAll (property-based testing) body — emit @given decorator
                if let LirExpr::ForAll {
                    type_,
                    binding,
                    property,
                    ..
                } = body
                {
                    let gen_str = self.type_to_st_generator(type_)?;
                    let binding_str = self.emit_pat(binding)?;
                    let prop_str = self.emit_expr(property)?;
                    // Replace the def line to use the ForAll binding as parameter
                    let last_idx = lines.len() - 1;
                    lines[last_idx] = format!("def {}({}):", fn_name, binding_str);
                    // Insert @given decorator above the def
                    lines.insert(last_idx, format!("@given({})", gen_str));
                    // Use the property expression as the function body with assert
                    lines.push(format!("{}assert {}", PY_INDENT, prop_str));
                    return Ok(lines.join("\n"));
                }

                // Body — multi-line with 4-space Python indentation
                let body_lines = self.emit_function_body(body)?;
                lines.extend(body_lines);

                Ok(lines.join("\n"))
            }

            LirDecl::RecordDef { name, fields, .. } => {
                self.needs_dataclass = true;

                let mut lines: Vec<String> = Vec::new();

                // @dataclass decorator
                lines.push("@dataclass".to_string());

                // class PascalCaseName:
                let class_name = to_pascal_case(name);
                lines.push(format!("class {}:", class_name));

                // Fields: `    field_name: type`
                for field in fields {
                    let type_str = self.type_mapper.map_type(&field.type_);
                    lines.push(format!("{}{}: {}", PY_INDENT, field.name, type_str));
                }

                Ok(lines.join("\n"))
            }

            LirDecl::UnionDef { name, variants, .. } => {
                self.needs_typing_union = true;

                let union_name = to_pascal_case(name);

                // Map each variant to its Python type; variants without args → None
                let variants_str: Vec<String> = variants
                    .iter()
                    .map(|v| {
                        if let Some(arg_type) = &v.arg {
                            self.type_mapper.map_type(arg_type)
                        } else {
                            "None".to_string()
                        }
                    })
                    .collect();

                Ok(format!(
                    "{} = Union[{}]",
                    union_name,
                    variants_str.join(", ")
                ))
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
                            return Ok(format!("assert {} == {}", a, b));
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
                    Ok(format!("await {}", call))
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
                        // Python ternary: then_val if cond else else_val
                        Ok(format!("{} if {} else {}", then_str, cond_str, else_str))
                    }
                    None => Ok(then_str),
                }
            }
            LirExpr::Match { expr, arms, .. } => {
                let expr_str = self.emit_expr(expr)?;
                if arms.is_empty() {
                    return Ok(String::new());
                }
                // Build ternary chain: process arms right-to-left.
                // Wildcard arm becomes the else/default case.
                let mut chain = String::new();
                for arm in arms.iter().rev() {
                    let body_str = self.emit_expr(&arm.body)?;
                    if matches!(arm.pattern, LirPat::Wildcard) {
                        if chain.is_empty() && arms.len() == 1 {
                            // Single wildcard arm with no other arms — emit a
                            // ternary that references `_` so the wildcard pattern
                            // is visible in the generated output.
                            chain = format!("{} if _ == {} else None", body_str, expr_str);
                        } else {
                            // Wildcard is the else/default in a multi-arm match.
                            chain = body_str;
                        }
                    } else {
                        let pat_str = match &arm.pattern {
                            LirPat::Literal(lit) => self.emit_literal(lit)?,
                            LirPat::Variable(name) => name.clone(),
                            LirPat::Variant { name, .. } => format!("'{}'", name),
                            LirPat::Record { .. } => "_".to_string(),
                            LirPat::Wildcard => unreachable!(),
                        };
                        let condition = format!("{} == {}", expr_str, pat_str);
                        if chain.is_empty() {
                            chain = format!("{} if {} else None", body_str, condition);
                        } else {
                            chain = format!("{} if {} else {}", body_str, condition, chain);
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
                let params_str: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let body_str = self.emit_expr(body)?;
                Ok(format!("lambda {}: {}", params_str.join(", "), body_str))
            }
            LirExpr::Record { fields, .. } => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, expr)| {
                        let val = self.emit_expr(expr)?;
                        Ok(format!("'{}': {}", name, val))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{{{}}}", fields_str.join(", ")))
            }
            LirExpr::Variant { name, arg, .. } => match arg {
                Some(expr) => {
                    let val = self.emit_expr(expr)?;
                    Ok(format!("{{'tag': '{}', 'value': {}}}", name, val))
                }
                None => Ok(format!("'{}'", name)),
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
                    "# forAll<{ty_str}>({binding_str} => {property_str})"
                ))
            }
            LirExpr::AssertConsistent { expr, .. } => self.emit_expr(expr),
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
                        Ok(format!("'{}': {}", name, p))
                    })
                    .collect::<Result<Vec<_>, EmitterError>>()?;
                let rest_str = if *rest { ", **rest" } else { "" };
                Ok(format!("{{{}{}}}", fields_str.join(", "), rest_str))
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
            LirLiteral::Bool(v) => {
                if *v {
                    Ok("True".into())
                } else {
                    Ok("False".into())
                }
            }
            LirLiteral::Null => Ok("None".into()),
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
            LirBinaryOp::And => Ok(" and ".into()),
            LirBinaryOp::Or => Ok(" or ".into()),
        }
    }

    fn emit_unary_op(&mut self, op: &LirUnaryOp) -> Result<String, EmitterError> {
        match op {
            LirUnaryOp::Neg => Ok("-".into()),
            LirUnaryOp::Not => Ok("not ".into()),
        }
    }

    fn emit_target_hint(&mut self, hint: &TargetHint) -> Result<String, EmitterError> {
        match hint {
            TargetHint::None => Ok(String::new()),
            TargetHint::Async => Ok("async ".into()),
            TargetHint::Optional => Ok(String::new()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::{LirArm, LirBinaryOp, LirLiteral, LirParam, LirUnaryOp, TargetHint};
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
    // Expression emission — every LirExpr variant
    // ==================================================================

    #[test]
    fn test_emit_expr_literal_int() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(42),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "42");
    }

    #[test]
    fn test_emit_expr_literal_float() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Float(3.5),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "3.5");
    }

    #[test]
    fn test_emit_expr_literal_str() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Str("hello".into()),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "\"hello\"");
    }

    #[test]
    fn test_emit_expr_literal_bool() {
        let mut backend = PythonBackend::new();
        let expr_true = LirExpr::Literal {
            value: LirLiteral::Bool(true),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr_true).unwrap(), "True");
        let expr_false = LirExpr::Literal {
            value: LirLiteral::Bool(false),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr_false).unwrap(), "False");
    }

    #[test]
    fn test_emit_expr_literal_null() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Null,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "None");
    }

    #[test]
    fn test_emit_expr_variable() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Variable {
            name: "x".into(),
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "x");
    }

    #[test]
    fn test_emit_expr_call() {
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "fetch".into(),
                hint: hint_none(),
                span: s(),
            }),
            args: vec![LirExpr::Variable {
                name: "url".into(),
                hint: hint_none(),
                span: s(),
            }],
            hint: TargetHint::Async,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "await fetch(url)");
    }

    #[test]
    fn test_emit_assert_single_arg() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "assert".into(),
                hint: TargetHint::None,
                span: s(),
            }),
            args: vec![LirExpr::Literal {
                value: LirLiteral::Bool(true),
                hint: TargetHint::None,
                span: s(),
            }],
            hint: TargetHint::None,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "assert True");
    }

    #[test]
    fn test_emit_assert_eq_two_args() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "assert_eq".into(),
                hint: TargetHint::None,
                span: s(),
            }),
            args: vec![
                LirExpr::Literal {
                    value: LirLiteral::Int(42),
                    hint: TargetHint::None,
                    span: s(),
                },
                LirExpr::Literal {
                    value: LirLiteral::Int(42),
                    hint: TargetHint::None,
                    span: s(),
                },
            ],
            hint: TargetHint::None,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "assert 42 == 42");
    }

    #[test]
    fn test_emit_regular_call_unaffected_by_assert() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Call {
            func: Box::new(LirExpr::Variable {
                name: "foo".into(),
                hint: TargetHint::None,
                span: s(),
            }),
            args: vec![LirExpr::Literal {
                value: LirLiteral::Int(1),
                hint: TargetHint::None,
                span: s(),
            }],
            hint: TargetHint::None,
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "foo(1)");
    }

    #[test]
    fn test_emit_expr_member() {
        let mut backend = PythonBackend::new();
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
    fn test_emit_expr_if_ternary() {
        let mut backend = PythonBackend::new();
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
        // Python ternary: then_val if cond else else_val
        assert_eq!(
            backend.emit_expr(&expr).unwrap(),
            "thenVal if cond else elseVal"
        );
    }

    #[test]
    fn test_emit_expr_if_no_else() {
        let mut backend = PythonBackend::new();
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
    fn test_emit_expr_match_ternary() {
        let mut backend = PythonBackend::new();
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
            "\"one\" if x == 1 else \"other\""
        );
    }

    #[test]
    fn test_emit_expr_match_multi_arm() {
        let mut backend = PythonBackend::new();
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
                    pattern: LirPat::Literal(LirLiteral::Int(2)),
                    guard: None,
                    body: LirExpr::Literal {
                        value: LirLiteral::Str("two".into()),
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
            "\"one\" if x == 1 else \"two\" if x == 2 else \"other\""
        );
    }

    #[test]
    fn test_emit_expr_match_no_wildcard() {
        let mut backend = PythonBackend::new();
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
                    pattern: LirPat::Literal(LirLiteral::Int(2)),
                    guard: None,
                    body: LirExpr::Literal {
                        value: LirLiteral::Str("two".into()),
                        hint: hint_none(),
                        span: s(),
                    },
                },
            ],
            hint: hint_none(),
            span: s(),
        };
        let result = backend.emit_expr(&expr).unwrap();
        assert!(
            result.contains("if x == 1"),
            "should contain first condition"
        );
        assert!(
            result.contains("if x == 2"),
            "should contain second condition"
        );
        assert!(result.contains("None"), "should fallback to None");
    }

    #[test]
    fn test_emit_expr_block() {
        let mut backend = PythonBackend::new();
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
        assert_eq!(backend.emit_expr(&expr).unwrap(), "{ x = 1; return x; }");
    }

    #[test]
    fn test_emit_expr_assign() {
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
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
        assert_eq!(backend.emit_expr(&expr).unwrap(), "lambda a, b: a + b");
    }

    #[test]
    fn test_emit_expr_record() {
        let mut backend = PythonBackend::new();
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
            "{'x': 1, 'y': \"hello\"}"
        );
    }

    #[test]
    fn test_emit_expr_variant_no_arg() {
        let mut backend = PythonBackend::new();
        let expr = LirExpr::Variant {
            name: "None".into(),
            arg: None,
            hint: hint_none(),
            span: s(),
        };
        assert_eq!(backend.emit_expr(&expr).unwrap(), "'None'");
    }

    #[test]
    fn test_emit_expr_variant_with_arg() {
        let mut backend = PythonBackend::new();
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
            "{'tag': 'Ok', 'value': 42}"
        );
    }

    #[test]
    fn test_emit_expr_array() {
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
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
        assert_eq!(backend.emit_expr(&expr).unwrap(), "a == b");
    }

    #[test]
    fn test_emit_expr_unary_neg() {
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
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
        assert_eq!(backend.emit_expr(&expr).unwrap(), "not flag");
    }

    #[test]
    fn test_emit_expr_wildcard() {
        let mut backend = PythonBackend::new();
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
        let mut backend = PythonBackend::new();
        assert_eq!(backend.emit_pat(&LirPat::Wildcard).unwrap(), "_");
    }

    #[test]
    fn test_emit_pat_literal() {
        let mut backend = PythonBackend::new();
        let result = backend
            .emit_pat(&LirPat::Literal(LirLiteral::Int(42)))
            .unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn test_emit_pat_variable() {
        let mut backend = PythonBackend::new();
        let result = backend.emit_pat(&LirPat::Variable("myVar".into())).unwrap();
        assert_eq!(result, "myVar");
    }

    #[test]
    fn test_emit_pat_variant_no_arg() {
        let mut backend = PythonBackend::new();
        let pat = LirPat::Variant {
            name: "None".into(),
            arg: None,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "None");
    }

    #[test]
    fn test_emit_pat_variant_with_arg() {
        let mut backend = PythonBackend::new();
        let pat = LirPat::Variant {
            name: "Some".into(),
            arg: Some(Box::new(LirPat::Variable("inner".into()))),
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "Some(inner)");
    }

    #[test]
    fn test_emit_pat_record_no_rest() {
        let mut backend = PythonBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{'x': _}");
    }

    #[test]
    fn test_emit_pat_record_with_rest() {
        let mut backend = PythonBackend::new();
        let pat = LirPat::Record {
            fields: vec![("x".into(), LirPat::Wildcard)],
            rest: true,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{'x': _, **rest}");
    }

    #[test]
    fn test_emit_pat_record_empty() {
        let mut backend = PythonBackend::new();
        let pat = LirPat::Record {
            fields: vec![],
            rest: false,
        };
        assert_eq!(backend.emit_pat(&pat).unwrap(), "{}");
    }

    // ==================================================================
    // Type emission — delegates to PythonMapper
    // ==================================================================

    #[test]
    fn test_emit_type_int() {
        let mut backend = PythonBackend::new();
        assert_eq!(
            backend.emit_type(&Type::Named("Int".into())).unwrap(),
            "int"
        );
    }

    #[test]
    fn test_emit_type_string() {
        let mut backend = PythonBackend::new();
        assert_eq!(
            backend.emit_type(&Type::Named("String".into())).unwrap(),
            "str"
        );
    }

    #[test]
    fn test_emit_type_bool() {
        let mut backend = PythonBackend::new();
        assert_eq!(
            backend.emit_type(&Type::Named("Bool".into())).unwrap(),
            "bool"
        );
    }

    #[test]
    fn test_emit_type_union() {
        let mut backend = PythonBackend::new();
        let ty = Type::Union(vec![
            Type::Named("Int".into()),
            Type::Named("String".into()),
        ]);
        assert_eq!(backend.emit_type(&ty).unwrap(), "int | str");
    }
}
