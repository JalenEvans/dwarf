//! Java type mapping — implements [`TypeMapper`] for Java output.
//!
//! Maps Dwarf types to their Java equivalents.

use crate::types::TypeMapper;
use dwarf_syntax::hir::Type;

/// Java implementation of [`TypeMapper`].
pub struct JavaMapper;

impl TypeMapper for JavaMapper {
    fn map_type(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" => "int".to_string(),
                "Float" => "double".to_string(),
                "String" => "String".to_string(),
                "Bool" => "boolean".to_string(),
                "Null" => "Void".to_string(),
                "Void" => "void".to_string(),
                "Any" => "Object".to_string(),
                _ => name.clone(),
            },
            Type::Record(fields) => {
                if fields.is_empty() {
                    return "Record".to_string();
                }
                // Java records are generated via the structural-to-nominal bridge
                "Record".to_string()
            }
            Type::Union(variants) => {
                if variants.len() == 1 {
                    return self.map_type(&variants[0]);
                }
                // Java uses sealed interfaces for unions
                "sealed".to_string()
            }
            Type::Func { params, return_ } => {
                let params_str: Vec<String> = params.iter().map(|p| self.map_type(p)).collect();
                format!(
                    "Function<{}, {}>",
                    if params_str.len() == 1 {
                        params_str[0].clone()
                    } else {
                        "Object[]".to_string()
                    },
                    self.map_type(return_)
                )
            }
            Type::Generic { base, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.map_type(a)).collect();
                format!("{}<{}>", base, args_str.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Type;

    fn mapper() -> JavaMapper {
        JavaMapper
    }

    #[test]
    fn test_map_named_int() {
        assert_eq!(mapper().map_type(&Type::Named("Int".into())), "int");
    }

    #[test]
    fn test_map_named_float() {
        assert_eq!(mapper().map_type(&Type::Named("Float".into())), "double");
    }

    #[test]
    fn test_map_named_string() {
        assert_eq!(mapper().map_type(&Type::Named("String".into())), "String");
    }

    #[test]
    fn test_map_named_bool() {
        assert_eq!(mapper().map_type(&Type::Named("Bool".into())), "boolean");
    }

    #[test]
    fn test_map_named_null() {
        assert_eq!(mapper().map_type(&Type::Named("Null".into())), "Void");
    }

    #[test]
    fn test_map_named_void() {
        assert_eq!(mapper().map_type(&Type::Named("Void".into())), "void");
    }

    #[test]
    fn test_map_named_any() {
        assert_eq!(mapper().map_type(&Type::Named("Any".into())), "Object");
    }

    #[test]
    fn test_map_named_custom() {
        assert_eq!(mapper().map_type(&Type::Named("MyClass".into())), "MyClass");
    }

    #[test]
    fn test_map_generic() {
        let ty = Type::Generic {
            base: "List".into(),
            args: vec![Type::Named("String".into())],
        };
        assert_eq!(mapper().map_type(&ty), "List<String>");
    }
}
