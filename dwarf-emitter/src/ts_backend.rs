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
//!
//! All other [`EmitterBackend`] methods currently panic with `unimplemented!()`.

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirUnaryOp, TargetHint,
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

    fn emit_module(&mut self, _decls: &[LirDecl]) -> Result<String, EmitterError> {
        unimplemented!("TypeScriptBackend::emit_module")
    }

    fn emit_decl(&mut self, _decl: &LirDecl) -> Result<String, EmitterError> {
        unimplemented!("TypeScriptBackend::emit_decl")
    }

    fn emit_expr(&mut self, _expr: &LirExpr) -> Result<String, EmitterError> {
        unimplemented!("TypeScriptBackend::emit_expr")
    }

    fn emit_pat(&mut self, _pat: &LirPat) -> Result<String, EmitterError> {
        unimplemented!("TypeScriptBackend::emit_pat")
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

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lir::{
        LirBinaryOp, LirLiteral, LirUnaryOp, TargetHint, Effect,
    };
    use dwarf_syntax::hir::Type;

    // ==================================================================
    // Creation tests
    // ==================================================================

    #[test]
    fn test_ts_backend_new() {
        let backend = TypeScriptBackend::new();
        assert!(backend.buffer.is_empty(), "new backend should have empty buffer");
        assert_eq!(backend.indent_level, 0, "new backend should have indent_level 0");
    }

    #[test]
    fn test_ts_backend_default() {
        let backend = TypeScriptBackend::default();
        assert!(backend.buffer.is_empty(), "default backend should have empty buffer");
        assert_eq!(backend.indent_level, 0, "default backend should have indent_level 0");
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
        assert_eq!(backend.emit_target_hint(&TargetHint::Async).unwrap(), "async ");
    }

    #[test]
    fn test_emit_target_hint_optional() {
        let mut backend = TypeScriptBackend::new();
        assert_eq!(backend.emit_target_hint(&TargetHint::Optional).unwrap(), "?");
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
        assert_eq!(backend.emit_type(&ty).unwrap(), "{ x: number; y: number }");
    }

    // ==================================================================
    // Stub methods — should panic with unimplemented!()
    // ==================================================================

    #[test]
    #[should_panic(expected = "not implemented")]
    fn test_emit_expr_unimplemented() {
        use dwarf_lir::LirExpr;
        use dwarf_syntax::span::Span;
        let mut backend = TypeScriptBackend::new();
        let expr = LirExpr::Literal {
            value: LirLiteral::Int(0),
            hint: TargetHint::None,
            span: Span::new(0, 0, 0),
        };
        let _ = backend.emit_expr(&expr);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn test_emit_pat_unimplemented() {
        let mut backend = TypeScriptBackend::new();
        let _ = backend.emit_pat(&LirPat::Wildcard);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn test_emit_decl_unimplemented() {
        use dwarf_syntax::span::Span;
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: TargetHint::None,
                span: Span::new(0, 0, 0),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: Span::new(0, 0, 0),
        };
        let _ = backend.emit_decl(&decl);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn test_emit_module_unimplemented() {
        use dwarf_syntax::span::Span;
        let mut backend = TypeScriptBackend::new();
        let decl = LirDecl::Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            body: LirExpr::Literal {
                value: LirLiteral::Int(0),
                hint: TargetHint::None,
                span: Span::new(0, 0, 0),
            },
            effect: Effect::Pure,
            hint: TargetHint::None,
            is_pub: true,
            span: Span::new(0, 0, 0),
        };
        let _ = backend.emit_module(&[decl]);
    }
}
