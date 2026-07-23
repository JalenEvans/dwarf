//! MirPass — the MIR desugaring compilation pass.
//!
//! This pass transforms typed HIR into desugared MIR by running
//! the full desugaring pipeline: decorators → type alias expansion.

use crate::desugar::desugar_decorators;
use dwarf_syntax::hir::Decl;

/// The MIR desugaring pass.
///
/// Runs `desugar_decorators` (which internally handles decorator
/// desugaring, pipe/propagation/for-loop lowering, and type alias
/// expansion) to produce a `Vec<MirDecl>` from HIR declarations.
pub struct MirPass;

impl MirPass {
    pub fn new() -> Self {
        Self
    }

    /// Run the full desugaring pipeline on HIR declarations.
    pub fn run(&self, decls: &[Decl]) -> Vec<crate::MirDecl> {
        desugar_decorators(decls)
    }
}

impl Default for MirPass {
    fn default() -> Self {
        Self::new()
    }
}
