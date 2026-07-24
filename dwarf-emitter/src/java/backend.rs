//! Java backend implementation.
//!
//! TODO: Phase 1 implementation

use dwarf_lir::{
    Effect, LirBinaryOp, LirDecl, LirExpr, LirLiteral, LirPat, LirUnaryOp, TargetHint,
};
use dwarf_syntax::hir::Type;

use crate::backend::EmitterBackend;
use crate::error::EmitterError;

/// A backend that emits Java code from LIR declarations.
pub struct JavaBackend;

impl JavaBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmitterBackend for JavaBackend {
    type Output = String;

    fn emit_module(&mut self, _decls: &[LirDecl]) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java module emission not yet implemented".into()))
    }

    fn emit_decl(&mut self, _decl: &LirDecl) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java decl emission not yet implemented".into()))
    }

    fn emit_expr(&mut self, _expr: &LirExpr) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java expr emission not yet implemented".into()))
    }

    fn emit_pat(&mut self, _pat: &LirPat) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java pat emission not yet implemented".into()))
    }

    fn emit_type(&mut self, _ty: &Type) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java type emission not yet implemented".into()))
    }

    fn emit_literal(&mut self, _lit: &LirLiteral) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java literal emission not yet implemented".into()))
    }

    fn emit_binary_op(&mut self, _op: &LirBinaryOp) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java binary op emission not yet implemented".into()))
    }

    fn emit_unary_op(&mut self, _op: &LirUnaryOp) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java unary op emission not yet implemented".into()))
    }

    fn emit_target_hint(&mut self, _hint: &TargetHint) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java target hint emission not yet implemented".into()))
    }

    fn emit_effect(&mut self, _effect: &Effect) -> Result<String, EmitterError> {
        Err(EmitterError::UnsupportedFeature("Java effect emission not yet implemented".into()))
    }
}
