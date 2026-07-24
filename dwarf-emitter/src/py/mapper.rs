//! Python type mapping — implements [`TypeMapper`] for Python output.
//!
//! Maps Dwarf types to their Python equivalents.

use crate::types::TypeMapper;
use dwarf_syntax::hir::Type;

/// Python implementation of [`TypeMapper`].
pub struct PythonMapper;

impl TypeMapper for PythonMapper {
    fn map_type(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" => "int".to_string(),
                "Float" => "float".to_string(),
                "String" => "str".to_string(),
                "Bool" => "bool".to_string(),
                "Null" => "None".to_string(),
                "Void" => "None".to_string(),
                "Any" => "Any".to_string(),
                _ => name.clone(),
            },
            Type::Record(fields) => {
                if fields.is_empty() {
                    return "dict".to_string();
                }
                // Python uses TypedDict or dataclass — for now use dict[str, Any]
                "dict".to_string()
            }
            Type::Union(variants) => {
                if variants.len() == 1 {
                    return self.map_type(&variants[0]);
                }
                // Python 3.10+ uses `X | Y` syntax
                variants
                    .iter()
                    .map(|t| self.map_type(t))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            Type::Func { params, return_ } => {
                let params_str: Vec<String> = params.iter().map(|p| self.map_type(p)).collect();
                format!(
                    "Callable[[{}], {}]",
                    params_str.join(", "),
                    self.map_type(return_)
                )
            }
            Type::Generic { base, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.map_type(a)).collect();
                format!("{}[{}]", base, args_str.join(", "))
            }
            Type::Refined { base, .. } => self.map_type(base),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Type;

    fn mapper() -> PythonMapper {
        PythonMapper
    }

    #[test]
    fn test_map_named_int() {
        assert_eq!(mapper().map_type(&Type::Named("Int".into())), "int");
    }

    #[test]
    fn test_map_named_float() {
        assert_eq!(mapper().map_type(&Type::Named("Float".into())), "float");
    }

    #[test]
    fn test_map_named_string() {
        assert_eq!(mapper().map_type(&Type::Named("String".into())), "str");
    }

    #[test]
    fn test_map_named_bool() {
        assert_eq!(mapper().map_type(&Type::Named("Bool".into())), "bool");
    }

    #[test]
    fn test_map_named_null() {
        assert_eq!(mapper().map_type(&Type::Named("Null".into())), "None");
    }

    #[test]
    fn test_map_named_void() {
        assert_eq!(mapper().map_type(&Type::Named("Void".into())), "None");
    }

    #[test]
    fn test_map_named_any() {
        assert_eq!(mapper().map_type(&Type::Named("Any".into())), "Any");
    }

    #[test]
    fn test_map_named_custom() {
        assert_eq!(mapper().map_type(&Type::Named("MyType".into())), "MyType");
    }

    #[test]
    fn test_map_union() {
        let ty = Type::Union(vec![
            Type::Named("Int".into()),
            Type::Named("String".into()),
        ]);
        assert_eq!(mapper().map_type(&ty), "int | str");
    }

    #[test]
    fn test_map_func() {
        let ty = Type::Func {
            params: vec![Type::Named("Int".into())],
            return_: Box::new(Type::Named("Bool".into())),
        };
        assert_eq!(mapper().map_type(&ty), "Callable[[int], bool]");
    }

    #[test]
    fn test_map_generic() {
        let ty = Type::Generic {
            base: "list".into(),
            args: vec![Type::Named("Int".into())],
        };
        assert_eq!(mapper().map_type(&ty), "list[int]");
    }
}
