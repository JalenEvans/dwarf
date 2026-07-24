//! TypeScript type mapping — implements [`TypeMapper`] for TypeScript output.
//!
//! Maps Dwarf types to their TypeScript equivalents.

use dwarf_syntax::hir::Type;
use crate::types::TypeMapper;

/// Default TypeScript implementation of [`TypeMapper`].
///
/// Maps Dwarf's built-in types to their TypeScript equivalents and
/// handles compound types (records, unions, functions, generics).
pub struct TypeScriptMapper;

impl TypeMapper for TypeScriptMapper {
    fn map_type(&self, ty: &Type) -> String {
        match ty {
            Type::Named(name) => match name.as_str() {
                "Int" | "Float" => "number".to_string(),
                "String" => "string".to_string(),
                "Bool" => "boolean".to_string(),
                "Null" => "null".to_string(),
                "Void" => "void".to_string(),
                "Any" => "any".to_string(),
                _ => name.clone(),
            },
            Type::Record(fields) => {
                if fields.is_empty() {
                    return "{}".to_string();
                }
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(name, ty)| format!("{}: {}", name, self.map_type(ty)))
                    .collect();
                format!("{{ {} }}", fields_str.join("; "))
            }
            Type::Union(variants) => variants
                .iter()
                .map(|t| self.map_type(t))
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Func { params, return_ } => {
                let params_str: Vec<String> = params.iter().map(|p| self.map_type(p)).collect();
                format!("({}) => {}", params_str.join(", "), self.map_type(return_))
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

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn mapper() -> TypeScriptMapper {
        TypeScriptMapper
    }

    // -----------------------------------------------------------------------
    // Named type mapping
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_named_int() {
        let mapper = mapper();
        let ty = Type::Named("Int".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number");
    }

    #[test]
    fn test_map_named_float() {
        let mapper = mapper();
        let ty = Type::Named("Float".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number");
    }

    #[test]
    fn test_map_named_string() {
        let mapper = mapper();
        let ty = Type::Named("String".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "string");
    }

    #[test]
    fn test_map_named_bool() {
        let mapper = mapper();
        let ty = Type::Named("Bool".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "boolean");
    }

    #[test]
    fn test_map_named_null() {
        let mapper = mapper();
        let ty = Type::Named("Null".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "null");
    }

    #[test]
    fn test_map_named_void() {
        let mapper = mapper();
        let ty = Type::Named("Void".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "void");
    }

    #[test]
    fn test_map_named_any() {
        let mapper = mapper();
        let ty = Type::Named("Any".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "any");
    }

    #[test]
    fn test_map_named_custom_type() {
        let mapper = mapper();
        let ty = Type::Named("UserDefined".to_string());
        let result = mapper.map_type(&ty);
        assert_eq!(result, "UserDefined");
    }

    // -----------------------------------------------------------------------
    // Record types
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_record_type() {
        let mapper = mapper();
        let ty = Type::Record(vec![
            ("x".to_string(), Box::new(Type::Named("Int".to_string()))),
            ("y".to_string(), Box::new(Type::Named("Int".to_string()))),
        ]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "{ x: number; y: number }");
    }

    #[test]
    fn test_map_record_type_empty() {
        let mapper = mapper();
        let ty = Type::Record(vec![]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "{}");
    }

    #[test]
    fn test_map_record_type_nested() {
        let mapper = mapper();
        let inner = Type::Record(vec![(
            "a".to_string(),
            Box::new(Type::Named("Int".to_string())),
        )]);
        let ty = Type::Record(vec![("inner".to_string(), Box::new(inner))]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "{ inner: { a: number } }");
    }

    // -----------------------------------------------------------------------
    // Union types
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_union_type() {
        let mapper = mapper();
        let ty = Type::Union(vec![
            Type::Named("Int".to_string()),
            Type::Named("String".to_string()),
        ]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number | string");
    }

    #[test]
    fn test_map_union_type_single() {
        let mapper = mapper();
        let ty = Type::Union(vec![Type::Named("Int".to_string())]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number");
    }

    #[test]
    fn test_map_union_type_multiple() {
        let mapper = mapper();
        let ty = Type::Union(vec![
            Type::Named("Int".to_string()),
            Type::Named("String".to_string()),
            Type::Named("Bool".to_string()),
            Type::Named("Null".to_string()),
        ]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "number | string | boolean | null");
    }

    // -----------------------------------------------------------------------
    // Function types
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_func_type() {
        let mapper = mapper();
        let ty = Type::Func {
            params: vec![
                Type::Named("Int".to_string()),
                Type::Named("String".to_string()),
            ],
            return_: Box::new(Type::Named("Bool".to_string())),
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "(number, string) => boolean");
    }

    #[test]
    fn test_map_func_type_no_params() {
        let mapper = mapper();
        let ty = Type::Func {
            params: vec![],
            return_: Box::new(Type::Named("Int".to_string())),
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "() => number");
    }

    #[test]
    fn test_map_func_type_void_return() {
        let mapper = mapper();
        let ty = Type::Func {
            params: vec![Type::Named("String".to_string())],
            return_: Box::new(Type::Named("Void".to_string())),
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "(string) => void");
    }

    // -----------------------------------------------------------------------
    // Generic types
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_generic_single_arg() {
        let mapper = mapper();
        let ty = Type::Generic {
            base: "Array".to_string(),
            args: vec![Type::Named("Int".to_string())],
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "Array<number>");
    }

    #[test]
    fn test_map_generic_multiple_args() {
        let mapper = mapper();
        let ty = Type::Generic {
            base: "Map".to_string(),
            args: vec![
                Type::Named("String".to_string()),
                Type::Named("Int".to_string()),
            ],
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "Map<string, number>");
    }

    #[test]
    fn test_map_generic_nested() {
        let mapper = mapper();
        let inner = Type::Generic {
            base: "Array".to_string(),
            args: vec![Type::Named("Int".to_string())],
        };
        let ty = Type::Generic {
            base: "Promise".to_string(),
            args: vec![inner],
        };
        let result = mapper.map_type(&ty);
        assert_eq!(result, "Promise<Array<number>>");
    }

    // -----------------------------------------------------------------------
    // Edge cases — deeply nested compound types
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_type_deeply_nested() {
        let mapper = mapper();
        // Record containing a Union containing a Func:
        // { handler: (number) => string | boolean }
        let func = Type::Func {
            params: vec![Type::Named("Int".to_string())],
            return_: Box::new(Type::Union(vec![
                Type::Named("String".to_string()),
                Type::Named("Bool".to_string()),
            ])),
        };
        let ty = Type::Record(vec![("handler".to_string(), Box::new(func))]);
        let result = mapper.map_type(&ty);
        assert_eq!(result, "{ handler: (number) => string | boolean }");
    }
}
