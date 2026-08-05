//! Module dependency graph for import resolution and cycle detection.
//!
//! This module operates on HIR declarations during the early pipeline stages
//! to build a directed graph of module dependencies and detect cycles in the
//! import graph.

use dwarf_syntax::hir::Decl;
use std::collections::{HashMap, HashSet};

/// A node representing a module in the dependency graph.
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// The module name (e.g., "std", "io", "collections").
    pub name: String,
    /// Modules this module directly imports from.
    pub imports: Vec<String>,
    /// Modules that directly import this module.
    pub imported_by: Vec<String>,
}

/// The module dependency graph.
///
/// For single-file compilation, the "current" file is represented as a
/// synthetic module node named `"current"`.
#[derive(Debug, Clone)]
pub struct ModuleGraph {
    pub modules: HashMap<String, ModuleNode>,
}

impl ModuleGraph {
    /// Create an empty module graph.
    pub fn new() -> Self {
        ModuleGraph {
            modules: HashMap::new(),
        }
    }

    /// Build the module graph from HIR declarations.
    ///
    /// Scans all `Decl::Import` declarations and creates directed edges
    /// from the importing file (synthesised as `"current"`) to the
    /// root module being imported.
    ///
    /// A dotted import path like `"std.io"` is resolved to its root module
    /// `"std"` — sub-module granularity is not tracked at this level.
    pub fn build(decls: &[Decl]) -> Self {
        let mut graph = ModuleGraph::new();

        // Collect all import declarations
        for decl in decls {
            if let Decl::Import { module, .. } = decl {
                // Split module path like "std.io" -> root "std"
                let root_module = module.split('.').next().unwrap_or(module).to_string();

                // Ensure the importing file has a node
                // (we use a synthetic "current" file for single-file compilation)
                graph
                    .modules
                    .entry("current".to_string())
                    .or_insert_with(|| ModuleNode {
                        name: "current".to_string(),
                        imports: Vec::new(),
                        imported_by: Vec::new(),
                    });

                // Ensure the imported module has a node
                graph
                    .modules
                    .entry(root_module.clone())
                    .or_insert_with(|| ModuleNode {
                        name: root_module.clone(),
                        imports: Vec::new(),
                        imported_by: Vec::new(),
                    });

                // Add edge: current -> root_module
                if let Some(node) = graph.modules.get_mut("current") {
                    if !node.imports.contains(&root_module) {
                        node.imports.push(root_module.clone());
                    }
                }
                if let Some(node) = graph.modules.get_mut(&root_module) {
                    if !node.imported_by.contains(&"current".to_string()) {
                        node.imported_by.push("current".to_string());
                    }
                }
            }
        }

        graph
    }

    /// Check whether the graph contains a cycle (mutual/recursive imports).
    ///
    /// Uses DFS with a back-edge detection set (`in_progress`). Returns `true`
    /// if any cycle is found.
    pub fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut in_progress = HashSet::new();

        for name in self.modules.keys() {
            if !visited.contains(name.as_str())
                && self.dfs_cycle(name, &mut visited, &mut in_progress)
            {
                return true;
            }
        }
        false
    }

    /// Depth-first search for cycle detection.
    ///
    /// Returns `true` if a cycle is reachable from `name`.
    fn dfs_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) -> bool {
        visited.insert(name.to_string());
        in_progress.insert(name.to_string());

        if let Some(node) = self.modules.get(name) {
            for import in &node.imports {
                if in_progress.contains(import.as_str()) {
                    // Found a back-edge → cycle detected.
                    return true;
                }
                if !visited.contains(import.as_str())
                    && self.dfs_cycle(import, visited, in_progress)
                {
                    return true;
                }
            }
        }

        in_progress.remove(name);
        false
    }

    /// Return all modules that directly import from `name`.
    ///
    /// These are the modules that *depend on* `name`.
    pub fn dependents_of(&self, name: &str) -> Vec<&str> {
        self.modules
            .get(name)
            .map(|n| n.imported_by.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Return all modules that `name` directly imports.
    ///
    /// These are the modules that `name` *depends on*.
    pub fn dependencies_of(&self, name: &str) -> Vec<&str> {
        self.modules
            .get(name)
            .map(|n| n.imports.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::hir::Decl;
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    fn import_decl(module: &str, names: &[&str], is_pub: bool) -> Decl {
        Decl::Import {
            module: module.to_string(),
            names: names.iter().map(|s| s.to_string()).collect(),
            is_pub,
            span: dummy_span(),
        }
    }

    // ------------------------------------------------------------------
    // build() — basic cases
    // ------------------------------------------------------------------

    #[test]
    fn test_empty_declarations_returns_empty_graph() {
        let decls: Vec<Decl> = vec![];
        let graph = ModuleGraph::build(&decls);
        assert!(
            graph.modules.is_empty(),
            "empty decls should yield empty graph"
        );
    }

    #[test]
    fn test_single_import_creates_edge() {
        let decls = vec![import_decl("std", &["println"], false)];
        let graph = ModuleGraph::build(&decls);

        assert!(
            graph.modules.contains_key("current"),
            "should have 'current' node"
        );
        assert!(graph.modules.contains_key("std"), "should have 'std' node");

        let deps = graph.dependencies_of("current");
        assert_eq!(deps, vec!["std"], "current should depend on std");

        let dependents = graph.dependents_of("std");
        assert_eq!(
            dependents,
            vec!["current"],
            "std should be imported by current"
        );
    }

    #[test]
    fn test_multiple_imports_creates_multiple_edges() {
        let decls = vec![
            import_decl("std", &["println"], false),
            import_decl("collections", &["HashMap"], true),
            import_decl("io", &["read"], false),
        ];
        let graph = ModuleGraph::build(&decls);

        let deps = graph.dependencies_of("current");
        assert_eq!(deps.len(), 3, "current should depend on 3 modules");
        assert!(deps.contains(&"std"));
        assert!(deps.contains(&"collections"));
        assert!(deps.contains(&"io"));
    }

    #[test]
    fn test_no_imports_returns_empty_graph() {
        // Declarations that are NOT imports should be ignored.
        let decls = vec![Decl::Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            body: dwarf_syntax::hir::Expr::Literal {
                value: dwarf_syntax::hir::LiteralValue::Int(0),
                span: dummy_span(),
            },
            is_pub: true,
            decorators: vec![],
            span: dummy_span(),
        }];
        let graph = ModuleGraph::build(&decls);
        assert!(
            graph.modules.is_empty(),
            "only non-import decls should yield empty graph"
        );
    }

    #[test]
    fn test_duplicate_imports_deduplicated() {
        let decls = vec![
            import_decl("std", &["println"], false),
            import_decl("std", &["format"], false),
        ];
        let graph = ModuleGraph::build(&decls);

        let deps = graph.dependencies_of("current");
        assert_eq!(
            deps.len(),
            1,
            "duplicate imports to same module should be deduplicated"
        );
        assert_eq!(deps[0], "std");
    }

    #[test]
    fn test_dotted_module_path_resolves_to_root() {
        let decls = vec![import_decl("std.io", &["read"], false)];
        let graph = ModuleGraph::build(&decls);

        assert!(
            graph.modules.contains_key("std"),
            "dotted path 'std.io' should resolve to root 'std'"
        );
        assert!(
            !graph.modules.contains_key("std.io"),
            "dotted path should NOT create submodule node"
        );

        let deps = graph.dependencies_of("current");
        assert_eq!(
            deps,
            vec!["std"],
            "current should depend on root 'std', not 'std.io'"
        );
    }

    #[test]
    fn test_pub_import_still_creates_edge() {
        // Public vs private imports don't affect the graph structure.
        let decls = vec![import_decl("net", &["connect"], true)];
        let graph = ModuleGraph::build(&decls);
        assert!(
            graph.modules.contains_key("net"),
            "pub import should still add the module node"
        );
        assert_eq!(graph.dependencies_of("current").len(), 1);
    }

    // ------------------------------------------------------------------
    // has_cycle() — cycle detection
    // ------------------------------------------------------------------

    #[test]
    fn test_no_cycle_in_empty_graph() {
        let graph = ModuleGraph::new();
        assert!(!graph.has_cycle(), "empty graph has no cycles");
    }

    #[test]
    fn test_no_cycle_in_acyclic_graph() {
        // Build: current -> std -> collections
        // No cycles.
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "current".to_string(),
            ModuleNode {
                name: "current".to_string(),
                imports: vec!["std".to_string()],
                imported_by: vec![],
            },
        );
        graph.modules.insert(
            "std".to_string(),
            ModuleNode {
                name: "std".to_string(),
                imports: vec!["collections".to_string()],
                imported_by: vec!["current".to_string()],
            },
        );
        graph.modules.insert(
            "collections".to_string(),
            ModuleNode {
                name: "collections".to_string(),
                imports: vec![],
                imported_by: vec!["std".to_string()],
            },
        );
        assert!(!graph.has_cycle(), "acyclic graph should have no cycles");
    }

    #[test]
    fn test_cycle_detected_in_mutual_import() {
        // Build: a <-> b (mutual import)
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "a".to_string(),
            ModuleNode {
                name: "a".to_string(),
                imports: vec!["b".to_string()],
                imported_by: vec!["b".to_string()],
            },
        );
        graph.modules.insert(
            "b".to_string(),
            ModuleNode {
                name: "b".to_string(),
                imports: vec!["a".to_string()],
                imported_by: vec!["a".to_string()],
            },
        );
        assert!(
            graph.has_cycle(),
            "mutual import a <-> b should be detected as a cycle"
        );
    }

    #[test]
    fn test_cycle_detected_in_triangular_import() {
        // Build: a -> b -> c -> a (triangular cycle)
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "a".to_string(),
            ModuleNode {
                name: "a".to_string(),
                imports: vec!["b".to_string()],
                imported_by: vec!["c".to_string()],
            },
        );
        graph.modules.insert(
            "b".to_string(),
            ModuleNode {
                name: "b".to_string(),
                imports: vec!["c".to_string()],
                imported_by: vec!["a".to_string()],
            },
        );
        graph.modules.insert(
            "c".to_string(),
            ModuleNode {
                name: "c".to_string(),
                imports: vec!["a".to_string()],
                imported_by: vec!["b".to_string()],
            },
        );
        assert!(
            graph.has_cycle(),
            "triangular cycle a -> b -> c -> a should be detected"
        );
    }

    #[test]
    fn test_self_import_is_cycle() {
        // Build: a -> a (self-import)
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "a".to_string(),
            ModuleNode {
                name: "a".to_string(),
                imports: vec!["a".to_string()],
                imported_by: vec!["a".to_string()],
            },
        );
        assert!(
            graph.has_cycle(),
            "self-import a -> a should be detected as a cycle"
        );
    }

    // ------------------------------------------------------------------
    // dependents_of() / dependencies_of()
    // ------------------------------------------------------------------

    #[test]
    fn test_dependents_of_unknown_module() {
        let graph = ModuleGraph::new();
        assert!(graph.dependents_of("nonexistent").is_empty());
    }

    #[test]
    fn test_dependencies_of_unknown_module() {
        let graph = ModuleGraph::new();
        assert!(graph.dependencies_of("nonexistent").is_empty());
    }

    #[test]
    fn test_dependents_of_module_with_no_dependents() {
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "standalone".to_string(),
            ModuleNode {
                name: "standalone".to_string(),
                imports: vec![],
                imported_by: vec![],
            },
        );
        assert!(graph.dependents_of("standalone").is_empty());
    }

    #[test]
    fn test_dependencies_of_module_with_no_dependencies() {
        let mut graph = ModuleGraph::new();
        graph.modules.insert(
            "leaf".to_string(),
            ModuleNode {
                name: "leaf".to_string(),
                imports: vec![],
                imported_by: vec!["current".to_string()],
            },
        );
        assert!(graph.dependencies_of("leaf").is_empty());
    }

    #[test]
    fn test_dependents_and_dependencies_round_trip() {
        let decls = vec![
            import_decl("std", &["println"], false),
            import_decl("collections", &["HashMap"], false),
        ];
        let graph = ModuleGraph::build(&decls);

        // current -> std, collections
        let deps = graph.dependencies_of("current");
        assert_eq!(deps.len(), 2);

        // std, collections are imported by current
        let std_dependents = graph.dependents_of("std");
        assert_eq!(std_dependents, vec!["current"]);

        let collections_dependents = graph.dependents_of("collections");
        assert_eq!(collections_dependents, vec!["current"]);
    }

    // ------------------------------------------------------------------
    // Graph properties
    // ------------------------------------------------------------------

    #[test]
    fn test_new_graph_is_empty() {
        let graph = ModuleGraph::new();
        assert!(graph.modules.is_empty());
    }

    #[test]
    fn test_graph_clone() {
        let decls = vec![import_decl("std", &["println"], false)];
        let graph = ModuleGraph::build(&decls);
        let cloned = graph.clone();
        assert_eq!(graph.modules.len(), cloned.modules.len());
        assert!(cloned.modules.contains_key("current"));
        assert!(cloned.modules.contains_key("std"));
    }

    #[test]
    fn test_graph_debug_output() {
        let decls = vec![import_decl("std", &["println"], false)];
        let graph = ModuleGraph::build(&decls);
        let debug = format!("{graph:?}");
        assert!(debug.contains("current"));
        assert!(debug.contains("std"));
    }
}
