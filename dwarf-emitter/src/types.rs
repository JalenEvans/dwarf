//! Type mapping module for the Dwarf emitter.
//!
//! Provides the [`TypeMapper`] trait and backend-specific implementations
//! that translate Dwarf HIR types into target language type strings.

use dwarf_syntax::hir::Type;

/// Maps Dwarf types to target language type strings.
pub trait TypeMapper {
    /// Convert a Dwarf [`Type`] into its string representation in the target language.
    fn map_type(&self, ty: &Type) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Type;

    #[test]
    fn test_map_type_trait_object() {
        // Verify the trait is object-safe by using it through `dyn TypeMapper`.
        // We use a concrete mapper to verify the trait dispatch works.
        let mapper: &dyn TypeMapper = &crate::ts::mapper::TypeScriptMapper;
        let ty = Type::Named("Int".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number");
    }
}
