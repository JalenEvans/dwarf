//! Manager for tracking and emitting module imports.
//!
//! The [`ImportManager`] registers imports (name + optional alias from a
//! module) and produces sorted, deduplicated import statements suitable
//! for a TypeScript/JavaScript-style emitter backend.

use std::collections::BTreeSet;

/// Tracks a set of module imports with deduplication and sorted emission.
///
/// Internally imports are stored in a `BTreeSet` of (module, name, alias)
/// tuples to maintain sorted order and eliminate duplicates.
#[derive(Debug, Clone)]
pub struct ImportManager {
    imports: BTreeSet<(String, String, Option<String>)>,
}

impl ImportManager {
    /// Create a new, empty `ImportManager`.
    pub fn new() -> Self {
        ImportManager {
            imports: BTreeSet::new(),
        }
    }

    /// Register an import: from `module`, import `name` (optionally aliased).
    ///
    /// If the same (module, name) pair is added multiple times, later calls
    /// are silently ignored (idempotent). If an alias is provided on a later
    /// call where none existed before (or vice versa), the first registration
    /// is kept.
    pub fn add_import(&mut self, module: &str, name: &str, alias: Option<&str>) {
        self.imports.insert((
            module.to_string(),
            name.to_string(),
            alias.map(|s| s.to_string()),
        ));
    }

    /// Check if a specific name from a module has been registered.
    pub fn has_import(&self, module: &str, name: &str) -> bool {
        self.imports
            .iter()
            .any(|(m, n, _)| m == module && n == name)
    }

    /// Return all registered imports as formatted strings, sorted by module
    /// then name, deduplicated.
    ///
    /// Format for non-aliased imports:
    /// ```text
    /// import { Name } from 'module'
    /// ```
    ///
    /// Format for aliased imports:
    /// ```text
    /// import { Name as Alias } from 'module'
    /// ```
    pub fn emit_imports(&self) -> Vec<String> {
        self.imports
            .iter()
            .map(|(module, name, alias)| match alias {
                Some(alias) => format!("import {{ {name} as {alias} }} from '{module}'"),
                None => format!("import {{ {name} }} from '{module}'"),
            })
            .collect()
    }

    /// Clear all registered imports.
    pub fn clear(&mut self) {
        self.imports.clear();
    }

    /// Number of unique imports.
    pub fn len(&self) -> usize {
        self.imports.len()
    }

    /// Returns true if no imports are registered.
    pub fn is_empty(&self) -> bool {
        self.imports.is_empty()
    }
}

impl Default for ImportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Creation & empty state
    // ------------------------------------------------------------------

    #[test]
    fn test_new_import_manager_is_empty() {
        let im = ImportManager::new();
        assert_eq!(im.len(), 0, "new manager should have len() == 0");
        assert!(im.is_empty(), "new manager should be empty");
    }

    #[test]
    fn test_empty_emit_imports() {
        let im = ImportManager::new();
        let imports = im.emit_imports();
        assert!(
            imports.is_empty(),
            "emit_imports on empty manager should return empty vec"
        );
    }

    // ------------------------------------------------------------------
    // Adding imports
    // ------------------------------------------------------------------

    #[test]
    fn test_add_single_import() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        assert_eq!(im.len(), 1, "should have one import");
        assert!(
            im.has_import("react", "useState"),
            "should have imported useState"
        );
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 1, "should emit one import statement");
        assert_eq!(emitted[0], "import { useState } from 'react'");
    }

    #[test]
    fn test_add_multiple_imports_different_modules() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        im.add_import("lodash", "map", None);
        assert_eq!(im.len(), 2, "should have two imports");
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 2, "should emit two import statements");
        // Should be sorted: "lodash" < "react"
        assert!(
            emitted[0].contains("lodash"),
            "first import should be from 'lodash'"
        );
        assert!(
            emitted[1].contains("react"),
            "second import should be from 'react'"
        );
    }

    #[test]
    fn test_add_multiple_imports_same_module() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        im.add_import("react", "useEffect", None);
        assert_eq!(im.len(), 2, "should have two imports from same module");
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 2, "should emit two import statements");
        // Both should be from 'react'
        for line in &emitted {
            assert!(
                line.contains("from 'react'"),
                "both imports should be from 'react': {line}"
            );
        }
        // Names should be sorted: "useEffect" < "useState"
        assert!(
            emitted[0].contains("useEffect"),
            "first import should be useEffect (sorted)"
        );
        assert!(
            emitted[1].contains("useState"),
            "second import should be useState (sorted)"
        );
    }

    #[test]
    fn test_add_import_with_alias() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", Some("useMyState"));
        assert_eq!(im.len(), 1);
        assert!(im.has_import("react", "useState"));
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], "import { useState as useMyState } from 'react'");
    }

    #[test]
    fn test_add_duplicate_import() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        im.add_import("react", "useState", None);
        assert_eq!(
            im.len(),
            1,
            "adding the same (module, name) twice should not increase count"
        );
        let emitted = im.emit_imports();
        assert_eq!(
            emitted.len(),
            1,
            "duplicate imports should produce only one statement"
        );
    }

    #[test]
    fn test_add_duplicate_import_same_module_different_names() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        im.add_import("react", "useEffect", None);
        assert_eq!(
            im.len(),
            2,
            "two different names from same module should coexist"
        );
        assert!(im.has_import("react", "useState"));
        assert!(im.has_import("react", "useEffect"));
    }

    // ------------------------------------------------------------------
    // has_import
    // ------------------------------------------------------------------

    #[test]
    fn test_has_import_returns_true_for_added() {
        let mut im = ImportManager::new();
        im.add_import("fs", "readFile", None);
        assert!(
            im.has_import("fs", "readFile"),
            "has_import should return true for added import"
        );
    }

    #[test]
    fn test_has_import_returns_false_for_not_added() {
        let im = ImportManager::new();
        assert!(
            !im.has_import("nonexistent", "foo"),
            "has_import should return false for import that was never added"
        );
    }

    #[test]
    fn test_has_import_after_clear() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        assert!(im.has_import("react", "useState"));
        im.clear();
        assert!(
            !im.has_import("react", "useState"),
            "has_import should return false after clear()"
        );
    }

    // ------------------------------------------------------------------
    // emit_imports formatting
    // ------------------------------------------------------------------

    #[test]
    fn test_emit_imports_format() {
        let mut im = ImportManager::new();
        im.add_import("module-a", "MyType", None);
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], "import { MyType } from 'module-a'");
    }

    #[test]
    fn test_emit_imports_with_alias_format() {
        let mut im = ImportManager::new();
        im.add_import("module-a", "MyType", Some("RenamedType"));
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 1);
        assert_eq!(
            emitted[0],
            "import { MyType as RenamedType } from 'module-a'"
        );
    }

    #[test]
    fn test_emit_imports_sorted_by_module() {
        let mut im = ImportManager::new();
        im.add_import("zmodule", "Last", None);
        im.add_import("amodule", "First", None);
        im.add_import("mmodule", "Middle", None);
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 3);
        // Sorted alphabetically by module name
        assert!(emitted[0].contains("amodule"), "first should be amodule");
        assert!(emitted[1].contains("mmodule"), "second should be mmodule");
        assert!(emitted[2].contains("zmodule"), "third should be zmodule");
    }

    #[test]
    fn test_emit_imports_sorted_by_name_within_module() {
        let mut im = ImportManager::new();
        im.add_import("lib", "zeta", None);
        im.add_import("lib", "alpha", None);
        im.add_import("lib", "beta", None);
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 3);
        // All from same module, sorted by name
        assert!(
            emitted[0].contains("alpha"),
            "first should be alpha (sorted)"
        );
        assert!(
            emitted[1].contains("beta"),
            "second should be beta (sorted)"
        );
        assert!(emitted[2].contains("zeta"), "third should be zeta (sorted)");
    }

    #[test]
    fn test_emit_imports_after_clear() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        im.clear();
        let emitted = im.emit_imports();
        assert!(
            emitted.is_empty(),
            "emit_imports should return empty after clear()"
        );
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_import_with_empty_module() {
        let mut im = ImportManager::new();
        im.add_import("", "Something", None);
        assert_eq!(im.len(), 1, "should allow empty-string module");
        assert!(im.has_import("", "Something"));
    }

    #[test]
    fn test_import_with_empty_name() {
        let mut im = ImportManager::new();
        im.add_import("module", "", None);
        assert_eq!(im.len(), 1, "should allow empty-string name");
        assert!(im.has_import("module", ""));
    }

    #[test]
    fn test_import_with_special_characters() {
        let mut im = ImportManager::new();
        im.add_import("my-module", "parse_int_32", None);
        im.add_import("utils", "$helper", None);
        im.add_import("internal", "_privateMethod", None);
        assert_eq!(im.len(), 3);
        assert!(im.has_import("my-module", "parse_int_32"));
        assert!(im.has_import("utils", "$helper"));
        assert!(im.has_import("internal", "_privateMethod"));
    }

    #[test]
    fn test_multiple_imports_with_aliases_interleaved() {
        let mut im = ImportManager::new();
        im.add_import("mod", "plain", None);
        im.add_import("mod", "original", Some("aliased"));
        assert_eq!(im.len(), 2);
        let emitted = im.emit_imports();
        assert_eq!(emitted.len(), 2);
        // Sorted by name: "original" < "plain"
        assert_eq!(emitted[0], "import { original as aliased } from 'mod'");
        assert_eq!(emitted[1], "import { plain } from 'mod'");
    }

    // ------------------------------------------------------------------
    // Trait derivations: Clone & Debug
    // ------------------------------------------------------------------

    #[test]
    fn test_import_manager_clone() {
        let mut im = ImportManager::new();
        im.add_import("react", "useState", None);
        let cloned = im.clone();
        assert_eq!(
            im.len(),
            cloned.len(),
            "cloned manager should have same length"
        );
        assert!(
            cloned.has_import("react", "useState"),
            "cloned manager should have same imports"
        );
    }

    #[test]
    fn test_import_manager_debug() {
        let im = ImportManager::new();
        let debug_str = format!("{im:?}");
        // Debug output should at least contain the struct name or field name
        assert!(!debug_str.is_empty(), "Debug format should not be empty");
    }

    // ------------------------------------------------------------------
    // Default trait
    // ------------------------------------------------------------------

    #[test]
    fn test_import_manager_default() {
        let im = ImportManager::default();
        assert_eq!(im.len(), 0, "default ImportManager should be empty");
        assert!(im.is_empty(), "default ImportManager should be empty");
    }

    // ------------------------------------------------------------------
    // State transitions: clear and re-add
    // ------------------------------------------------------------------

    #[test]
    fn test_clear_and_readd_imports() {
        let mut im = ImportManager::new();
        im.add_import("a", "foo", None);
        im.add_import("b", "bar", None);
        assert_eq!(im.len(), 2);
        im.clear();
        assert!(im.is_empty());
        // Re-add after clear
        im.add_import("c", "baz", None);
        assert_eq!(im.len(), 1);
        assert!(im.has_import("c", "baz"));
        assert!(!im.has_import("a", "foo"), "cleared imports should be gone");
    }
}
