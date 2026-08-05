//! Compilation passes for the Dwarf typechecker.
//!
//! Each pass performs a specific analysis or transformation on the HIR
//! declarations, producing diagnostics or enriched metadata.

pub mod edge_analysis;
pub mod test_coverage;
