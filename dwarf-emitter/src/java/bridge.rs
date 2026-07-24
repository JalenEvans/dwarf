//! Structural-to-nominal bridge for Java code generation.
//!
//! Converts Dwarf's structural types (inline records and unions) into Java's
//! nominal type system by generating deterministic, unique names. Handles
//! memoization so the same structural type always maps to the same name.
//!
//! # Naming conventions
//!
//! - **Records**: `Record_{Field1}_{Field2}` — field names are PascalCased
//! - **Unions**: `Union_{Variant1}_{Variant2}` — variant type strings are PascalCased

use std::collections::HashMap;
use dwarf_syntax::hir::Type;
use crate::naming;
use crate::types::TypeMapper;

/// A key that uniquely identifies a structural type by its shape.
///
/// Two structural types with the same fields (names + mapped type strings)
/// or the same variant types produce the same key, enabling deduplication.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum StructuralTypeKey {
    /// Record structural type: field names paired with their mapped type strings.
    Record(Vec<(String, String)>),
    /// Union structural type: mapped type strings of each variant.
    Union(Vec<String>),
}

/// Bridges Dwarf's structural types to Java's nominal type system.
///
/// Maintains a registry that maps structural type shapes to generated
/// Java class/interface names. The same structural type always yields
/// the same name (memoization). Different types that happen to produce
/// the same generated name get a numeric suffix for disambiguation.
pub struct StructuralNominalBridge {
    registry: HashMap<StructuralTypeKey, String>,
    counter: usize,
}

impl StructuralNominalBridge {
    /// Creates a new empty bridge.
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            counter: 0,
        }
    }

    /// Register a record structural type (e.g. `{x: Int, y: Int}`).
    ///
    /// Returns the generated Java class name. The same fields (by name
    /// and mapped type) always return the same name.
    pub fn register_record(
        &mut self,
        fields: &[(String, Type)],
        type_mapper: &dyn TypeMapper,
    ) -> String {
        // Build the key from original field names and their mapped type strings.
        let field_key: Vec<(String, String)> = fields
            .iter()
            .map(|(name, ty)| (name.clone(), type_mapper.map_type(ty)))
            .collect();

        let key = StructuralTypeKey::Record(field_key);

        // Memoization — return existing name if this shape has been registered.
        if let Some(name) = self.registry.get(&key) {
            return name.clone();
        }

        // Generate a deterministic name from the field names.
        let raw_name = if fields.is_empty() {
            "Record_Empty".to_string()
        } else {
            let parts: Vec<String> = fields
                .iter()
                .map(|(name, _)| {
                    let sanitized: String = name
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    naming::to_pascal_case(&sanitized)
                })
                .collect();
            format!("Record_{}", parts.join("_"))
        };

        let name = self.dedup_name(&raw_name);
        self.registry.insert(key, name.clone());
        name
    }

    /// Register a union structural type (e.g. `Int | String`).
    ///
    /// Returns the generated Java sealed-interface name. The same variant
    /// types always return the same name.
    pub fn register_union(
        &mut self,
        variants: &[Type],
        type_mapper: &dyn TypeMapper,
    ) -> String {
        // Build the key from mapped variant type strings.
        let variant_key: Vec<String> = variants
            .iter()
            .map(|ty| type_mapper.map_type(ty))
            .collect();

        let key = StructuralTypeKey::Union(variant_key);

        // Memoization — return existing name if this shape has been registered.
        if let Some(name) = self.registry.get(&key) {
            return name.clone();
        }

        // Generate a deterministic name from the variant type strings.
        let raw_name = if variants.is_empty() {
            "Union_Empty".to_string()
        } else {
            let parts: Vec<String> = variants
                .iter()
                .map(|ty| {
                    let type_str = type_mapper.map_type(ty);
                    let sanitized: String = type_str
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    naming::to_pascal_case(&sanitized)
                })
                .collect();
            format!("Union_{}", parts.join("_"))
        };

        let name = self.dedup_name(&raw_name);
        self.registry.insert(key, name.clone());
        name
    }

    /// Returns a reference to the registry of generated type names.
    pub fn generated_types(&self) -> &HashMap<StructuralTypeKey, String> {
        &self.registry
    }

    /// Clears all registered types, resetting the bridge.
    pub fn clear(&mut self) {
        self.registry.clear();
        self.counter = 0;
    }

    /// Ensure the generated name does not collide with any existing name in
    /// the registry. If it does, append a monotonic counter as a suffix.
    fn dedup_name(&mut self, name: &str) -> String {
        if !self.registry.values().any(|v| v == name) {
            return name.to_string();
        }
        let result = format!("{}_{}", name, self.counter);
        self.counter += 1;
        result
    }
}

impl Default for StructuralNominalBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Type;
    use crate::java::mapper::JavaMapper;
    use crate::types::TypeMapper;

    /// Helper — a mapper that returns the debug representation of a type.
    /// Useful for tests where we want to verify type-string-based naming.
    struct PassthroughMapper;

    impl TypeMapper for PassthroughMapper {
        fn map_type(&self, ty: &Type) -> String {
            match ty {
                Type::Named(name) => name.clone(),
                Type::Record(fields) => {
                    let f: Vec<String> = fields
                        .iter()
                        .map(|(n, t)| format!("{}:{}", n, self.map_type(t)))
                        .collect();
                    format!("{{{}}}", f.join(","))
                }
                Type::Union(variants) => {
                    let v: Vec<String> = variants.iter().map(|t| self.map_type(t)).collect();
                    format!("({})", v.join("|"))
                }
                Type::Func { params, return_ } => {
                    let p: Vec<String> = params.iter().map(|t| self.map_type(t)).collect();
                    format!("({})->{}", p.join(","), self.map_type(return_))
                }
                Type::Generic { base, args } => {
                    let a: Vec<String> = args.iter().map(|t| self.map_type(t)).collect();
                    format!("{}<{}>", base, a.join(","))
                }
            }
        }
    }

    fn java_mapper() -> JavaMapper {
        JavaMapper
    }

    fn passthrough_mapper() -> PassthroughMapper {
        PassthroughMapper
    }

    // ------------------------------------------------------------------
    // Test 1: Records generate expected structural names
    // ------------------------------------------------------------------

    #[test]
    fn test_generates_name_for_record() {
        let mut bridge = StructuralNominalBridge::new();
        let fields = vec![
            ("x".to_string(), Type::Named("Int".into())),
            ("y".to_string(), Type::Named("Int".into())),
        ];
        let name = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name, "Record_X_Y");
    }

    // ------------------------------------------------------------------
    // Test 2: Unions generate expected structural names
    // ------------------------------------------------------------------

    #[test]
    fn test_generates_name_for_union() {
        let mut bridge = StructuralNominalBridge::new();
        let variants = vec![Type::Named("Int".into()), Type::Named("String".into())];
        let name = bridge.register_union(&variants, &java_mapper());
        assert_eq!(name, "Union_Int_String");
    }

    // ------------------------------------------------------------------
    // Test 3: Same record registered twice returns the same name
    // ------------------------------------------------------------------

    #[test]
    fn test_deduplication() {
        let mut bridge = StructuralNominalBridge::new();
        let fields = vec![
            ("x".to_string(), Type::Named("Int".into())),
            ("y".to_string(), Type::Named("Int".into())),
        ];
        let name1 = bridge.register_record(&fields, &java_mapper());
        let name2 = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name1, name2);
        assert_eq!(name1, "Record_X_Y");
    }

    // ------------------------------------------------------------------
    // Test 4: Different records get different names
    // ------------------------------------------------------------------

    #[test]
    fn test_different_records_different_names() {
        let mut bridge = StructuralNominalBridge::new();
        let fields_a = vec![
            ("x".to_string(), Type::Named("Int".into())),
            ("y".to_string(), Type::Named("Int".into())),
        ];
        let fields_b = vec![
            ("name".to_string(), Type::Named("String".into())),
            ("value".to_string(), Type::Named("Float".into())),
        ];
        let name_a = bridge.register_record(&fields_a, &java_mapper());
        let name_b = bridge.register_record(&fields_b, &java_mapper());
        assert_ne!(name_a, name_b);
        assert_eq!(name_a, "Record_X_Y");
        assert_eq!(name_b, "Record_Name_Value");
    }

    // ------------------------------------------------------------------
    // Test 5: Record containing generic types
    // ------------------------------------------------------------------

    #[test]
    fn test_record_with_generic_types() {
        let mut bridge = StructuralNominalBridge::new();
        let fields = vec![(
            "items".to_string(),
            Type::Generic {
                base: "Array".to_string(),
                args: vec![Type::Named("Int".into())],
            },
        )];
        let name = bridge.register_record(&fields, &java_mapper());

        // Name is based on field names, not types.
        assert!(name.contains("Items"));

        // Verify the key includes type information by checking the map directly.
        let key = StructuralTypeKey::Record(vec![(
            "items".to_string(),
            "Array<int>".to_string(),
        )]);
        assert!(bridge.generated_types().contains_key(&key));
        assert_eq!(bridge.generated_types().get(&key), Some(&name));
    }

    // ------------------------------------------------------------------
    // Test 6: clear() resets the bridge
    // ------------------------------------------------------------------

    #[test]
    fn test_clear_resets() {
        let mut bridge = StructuralNominalBridge::new();
        let fields = vec![
            ("x".to_string(), Type::Named("Int".into())),
            ("y".to_string(), Type::Named("Int".into())),
        ];
        let name_before = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name_before, "Record_X_Y");

        bridge.clear();
        assert!(bridge.generated_types().is_empty());

        // After clear, registering the same type starts fresh.
        let name_after = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name_after, "Record_X_Y");
    }

    // ------------------------------------------------------------------
    // Test 7: Empty record generates a valid name
    // ------------------------------------------------------------------

    #[test]
    fn test_empty_record() {
        let mut bridge = StructuralNominalBridge::new();
        let fields: Vec<(String, Type)> = vec![];
        let name = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name, "Record_Empty");
    }

    // ------------------------------------------------------------------
    // Test 8: Empty union generates a valid name
    // ------------------------------------------------------------------

    #[test]
    fn test_empty_union() {
        let mut bridge = StructuralNominalBridge::new();
        let variants: Vec<Type> = vec![];
        let name = bridge.register_union(&variants, &java_mapper());
        assert_eq!(name, "Union_Empty");
    }

    // ------------------------------------------------------------------
    // Test 9: Generated names follow PascalCase convention
    // ------------------------------------------------------------------

    #[test]
    fn test_name_pascal_case() {
        let mut bridge = StructuralNominalBridge::new();

        // Record with multi-word field names
        let fields = vec![
            ("first_name".to_string(), Type::Named("String".into())),
            ("last_name".to_string(), Type::Named("String".into())),
        ];
        let name = bridge.register_record(&fields, &java_mapper());
        assert_eq!(name, "Record_FirstName_LastName");

        // Union with type names that need PascalCase
        let mut bridge2 = StructuralNominalBridge::new();
        let variants = vec![
            Type::Named("int".into()),   // lower-case
            Type::Named("string".into()), // lower-case
        ];
        let name2 = bridge2.register_union(&variants, &passthrough_mapper());
        assert_eq!(name2, "Union_Int_String");
    }

    // ------------------------------------------------------------------
    // Test 10: generated_types() accessor
    // ------------------------------------------------------------------

    #[test]
    fn test_generated_types_accessor() {
        let mut bridge = StructuralNominalBridge::new();

        // Register a record
        let fields = vec![
            ("x".to_string(), Type::Named("Int".into())),
            ("y".to_string(), Type::Named("Int".into())),
        ];
        let rec_name = bridge.register_record(&fields, &java_mapper());

        // Register a union
        let variants = vec![Type::Named("Int".into()), Type::Named("String".into())];
        let union_name = bridge.register_union(&variants, &java_mapper());

        // Check the map contains exactly two entries
        let map = bridge.generated_types();
        assert_eq!(map.len(), 2);

        // Verify expected keys exist
        let rec_key = StructuralTypeKey::Record(vec![
            ("x".to_string(), "int".to_string()),
            ("y".to_string(), "int".to_string()),
        ]);
        let union_key = StructuralTypeKey::Union(vec![
            "int".to_string(),
            "String".to_string(),
        ]);

        assert!(map.contains_key(&rec_key));
        assert!(map.contains_key(&union_key));
        assert_eq!(map.get(&rec_key), Some(&rec_name));
        assert_eq!(map.get(&union_key), Some(&union_name));
    }

    // ------------------------------------------------------------------
    // Test: Counter-based deduplication when names collide
    // ------------------------------------------------------------------

    #[test]
    fn test_name_collision_uses_counter() {
        let mut bridge = StructuralNominalBridge::new();

        // Two records with same field names but different types will collide
        // on the generated name but have different keys.
        let fields_a = vec![("value".to_string(), Type::Named("Int".into()))];
        let fields_b = vec![("value".to_string(), Type::Named("String".into()))];

        let name_a = bridge.register_record(&fields_a, &java_mapper());
        let name_b = bridge.register_record(&fields_b, &java_mapper());

        // Both should be valid, distinct names
        assert_eq!(name_a, "Record_Value");
        assert_eq!(name_b, "Record_Value_0");
    }
}
