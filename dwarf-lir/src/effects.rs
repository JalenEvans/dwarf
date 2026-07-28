//! Call graph construction from MIR declarations.
//!
//! The call graph tracks which functions call which, enabling effect
//! (pure/async) propagation through the graph and conflict detection.
//!
//! Construction is based on [`MirDecl`] nodes — only `Function` variants
//! produce graph nodes; type-/record-/union-def declarations are skipped.

use dwarf_mir::{MirDecl, MirExpr, MirStmt};
use std::collections::HashMap;

use crate::Effect;

/// A single node in the call graph, representing one function.
#[derive(Debug, Clone, PartialEq)]
pub struct CallGraphNode {
    /// Name of the function.
    pub name: String,
    /// Names of functions that this function calls.
    pub calls: Vec<String>,
    /// Names of functions that call this function.
    pub called_by: Vec<String>,
}

/// A directed call graph over a set of function declarations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallGraph {
    pub nodes: HashMap<String, CallGraphNode>,
}

impl CallGraph {
    /// Create an empty call graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a node by function name.
    pub fn get(&self, name: &str) -> Option<&CallGraphNode> {
        self.nodes.get(name)
    }

    /// Return the names of functions that directly call `name`.
    pub fn callers_of(&self, name: &str) -> Vec<&str> {
        self.nodes
            .get(name)
            .map(|n| n.called_by.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Return the names of functions directly called by `name`.
    pub fn callees_of(&self, name: &str) -> Vec<&str> {
        self.nodes
            .get(name)
            .map(|n| n.calls.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Returns `true` if the call graph contains at least one cycle
    /// (mutual or self recursion).
    pub fn has_cycle(&self) -> bool {
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut in_progress: std::collections::HashSet<String> = std::collections::HashSet::new();

        for node_name in self.nodes.keys() {
            if !visited.contains(node_name.as_str())
                && self.dfs_cycle(node_name, &mut visited, &mut in_progress)
            {
                return true;
            }
        }
        false
    }

    /// DFS-based cycle detection using a recursion stack.
    fn dfs_cycle(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(name.to_string());
        in_progress.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for callee in &node.calls {
                if in_progress.contains(callee) {
                    return true; // back edge found
                }
                if !visited.contains(callee) && self.dfs_cycle(callee, visited, in_progress) {
                    return true;
                }
            }
        }

        in_progress.remove(name);
        false
    }
}

/// Build a [`CallGraph`] from a slice of MIR declarations.
///
/// Only `MirDecl::Function` nodes contribute entries. Non-function decls
/// (type aliases, record definitions, union definitions) are silently ignored.
///
/// For each function declaration, the body is walked to find all
/// `MirExpr::Call` targets (via `MirExpr::Variable` in the `func` position)
/// and the graph's edges are populated bidirectionally.
pub fn build_call_graph(decls: &[MirDecl]) -> CallGraph {
    // Phase 1: Create nodes for all function declarations.
    let mut graph = CallGraph::new();

    for decl in decls {
        if let MirDecl::Function { name, .. } = decl {
            graph.nodes.insert(
                name.clone(),
                CallGraphNode {
                    name: name.clone(),
                    calls: Vec::new(),
                    called_by: Vec::new(),
                },
            );
        }
    }

    // Phase 2: Extract call targets from each function body and populate edges.
    // Build a set of known function names (owned strings) for existence checks.
    let known: std::collections::HashSet<String> = graph.nodes.keys().cloned().collect();

    for decl in decls {
        if let MirDecl::Function { name, body, .. } = decl {
            let callees: Vec<String> = extract_calls(body)
                .into_iter()
                .filter(|c| known.contains(c))
                .collect();

            if let Some(node) = graph.nodes.get_mut(name) {
                node.calls.extend(callees.iter().cloned());
            }

            for callee in &callees {
                if let Some(callee_node) = graph.nodes.get_mut(callee) {
                    callee_node.called_by.push(name.clone());
                }
            }
        }
    }

    graph
}

/// Extract names of functions called within a `MirExpr` body.
fn extract_calls(expr: &MirExpr) -> Vec<String> {
    let mut calls = Vec::new();
    extract_calls_inner(expr, &mut calls);
    calls
}

/// Recursively walk a `MirExpr` tree and collect names of function calls.
fn extract_calls_inner(expr: &MirExpr, calls: &mut Vec<String>) {
    match expr {
        MirExpr::Call { func, args, .. } => {
            if let MirExpr::Variable { name, .. } = func.as_ref() {
                calls.push(name.clone());
            }
            // Recurse into both func and args to find nested calls.
            extract_calls_inner(func, calls);
            for arg in args {
                extract_calls_inner(arg, calls);
            }
        }
        MirExpr::Literal { .. } | MirExpr::Variable { .. } | MirExpr::Wildcard { .. } => {}
        MirExpr::Member { obj, .. } => extract_calls_inner(obj, calls),
        MirExpr::If {
            cond, then, else_, ..
        } => {
            extract_calls_inner(cond, calls);
            extract_calls_inner(then, calls);
            if let Some(else_) = else_ {
                extract_calls_inner(else_, calls);
            }
        }
        MirExpr::Match { expr, arms, .. } => {
            extract_calls_inner(expr, calls);
            for arm in arms {
                extract_calls_inner(&arm.body, calls);
                if let Some(guard) = &arm.guard {
                    extract_calls_inner(guard, calls);
                }
            }
        }
        MirExpr::Loop { body, .. } => extract_calls_inner(body, calls),
        MirExpr::Block { stmts, .. } => {
            for stmt in stmts {
                match stmt {
                    MirStmt::Let { value, .. } => extract_calls_inner(value, calls),
                    MirStmt::Expr(e) => extract_calls_inner(e, calls),
                }
            }
        }
        MirExpr::Binary { lhs, rhs, .. } => {
            extract_calls_inner(lhs, calls);
            extract_calls_inner(rhs, calls);
        }
        MirExpr::Unary { expr, .. } => extract_calls_inner(expr, calls),
        MirExpr::Lambda { body, .. } => extract_calls_inner(body, calls),
        MirExpr::Record { fields, .. } => {
            for (_, field_expr) in fields {
                extract_calls_inner(field_expr, calls);
            }
        }
        MirExpr::Variant { arg, .. } => {
            if let Some(arg) = arg {
                extract_calls_inner(arg, calls);
            }
        }
        MirExpr::Array { items, .. } => {
            for item in items {
                extract_calls_inner(item, calls);
            }
        }
        MirExpr::Assign { target, value, .. } => {
            extract_calls_inner(target, calls);
            extract_calls_inner(value, calls);
        }
        MirExpr::ForAll { property, .. } => {
            extract_calls_inner(property, calls);
        }
        MirExpr::AssertConsistent { expr, .. } => {
            extract_calls_inner(expr, calls);
        }
        MirExpr::Try {
            body,
            guard,
            handler,
            ..
        } => {
            extract_calls_inner(body, calls);
            if let Some(g) = guard {
                extract_calls_inner(g, calls);
            }
            extract_calls_inner(handler, calls);
        }
        MirExpr::Throw { expr, .. } => extract_calls_inner(expr, calls),
        MirExpr::Propagate { expr, .. } => extract_calls_inner(expr, calls),
    }
}

/// Extract initial effects from function declarations.
///
/// Currently all functions default to [`Effect::Pure`] since Dwarf
/// doesn't have explicit effect annotations yet.
pub fn initial_effects(decls: &[MirDecl]) -> HashMap<String, Effect> {
    let mut effects = HashMap::new();
    for decl in decls {
        if let MirDecl::Function { name, .. } = decl {
            effects.insert(name.clone(), Effect::Pure);
        }
    }
    effects
}

/// Resolve effects for all functions by walking the call graph.
///
/// Starts with [`initial_effects`] as a baseline, applies `seeds` to
/// override initial effect assignments (e.g., marking specific functions
/// as async), then propagates effect information through the call graph
/// so that callers of async functions are also marked async.
pub fn resolve_effects(
    decls: &[MirDecl],
    callgraph: &CallGraph,
    seeds: &HashMap<String, Effect>,
) -> HashMap<String, Effect> {
    let mut effects = initial_effects(decls);

    // Apply seeds (override initial effects for specific functions)
    for (name, effect) in seeds {
        effects.insert(name.clone(), effect.clone());
    }

    let mut changed = true;

    while changed {
        changed = false;

        for (name, node) in &callgraph.nodes {
            if let Some(current_effect) = effects.get(name).cloned() {
                for callee in &node.calls {
                    if let Some(callee_effect) = effects.get(callee) {
                        if effect_level(callee_effect) > effect_level(&current_effect) {
                            effects.insert(name.clone(), callee_effect.clone());
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    effects
}

/// Numeric ranking of effect levels for propagation ordering.
///
/// Ordering: Pure (0) < Async (1) < Impure (2)
fn effect_level(effect: &Effect) -> u8 {
    match effect {
        Effect::Pure => 0,
        Effect::Async => 1,
        Effect::Impure => 2,
    }
}

/// Detect pure functions that call async functions.
///
/// Returns a list of `(pure_function_name, async_function_name)` pairs
/// representing edges in the call graph where a pure function invokes
/// an async function — a type mismatch that the language must reject.
pub fn detect_pure_async_conflicts(
    resolved_effects: &HashMap<String, Effect>,
    callgraph: &CallGraph,
) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();

    for (name, node) in &callgraph.nodes {
        if let Some(effect) = resolved_effects.get(name) {
            if *effect == Effect::Pure {
                for callee in &node.calls {
                    if let Some(callee_effect) = resolved_effects.get(callee) {
                        if *callee_effect == Effect::Async {
                            conflicts.push((name.clone(), callee.clone()));
                        }
                    }
                }
            }
        }
    }

    conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_mir::{MirDecl, MirExpr, MirLiteral};
    use dwarf_syntax::span::Span;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn span1() -> Span {
        Span::new(0, 0, 0)
    }

    fn make_func(name: &str, body: MirExpr) -> MirDecl {
        MirDecl::Function {
            name: name.into(),
            params: vec![],
            return_type: None,
            body,
            is_pub: true,
            is_generator: false,
            span: span1(),
        }
    }

    fn make_call(target: &str) -> MirExpr {
        MirExpr::Call {
            func: Box::new(MirExpr::Variable {
                name: target.into(),
                span: span1(),
            }),
            args: vec![],
            span: span1(),
        }
    }

    fn make_literal(val: i64) -> MirExpr {
        MirExpr::Literal {
            value: MirLiteral::Int(val),
            span: span1(),
        }
    }

    // ------------------------------------------------------------------
    // No calls — function with just a literal body
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_no_calls() {
        let decls = vec![make_func("a", make_literal(1))];
        let graph = build_call_graph(&decls);

        let node = graph.get("a").expect("node 'a' should exist");
        assert!(node.calls.is_empty(), "fn a should call nothing");
        assert!(node.called_by.is_empty(), "fn a should not be called");
    }

    // ------------------------------------------------------------------
    // Simple call — fn a() calls fn b()
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_simple_call() {
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);

        // a should have b in its calls
        let a = graph.get("a").expect("node 'a' should exist");
        assert_eq!(a.calls, vec!["b"]);
        assert!(a.called_by.is_empty(), "a should not be called by anyone");

        // b should have a in its called_by
        let b = graph.get("b").expect("node 'b' should exist");
        assert!(b.calls.is_empty(), "b should call nothing");
        assert_eq!(b.called_by, vec!["a"]);
    }

    // ------------------------------------------------------------------
    // Mutual call — a() calls b(), b() calls a()
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_mutual_call() {
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_call("a")),
        ];
        let graph = build_call_graph(&decls);

        let a = graph.get("a").expect("node 'a' should exist");
        assert_eq!(a.calls, vec!["b"]);
        assert_eq!(a.called_by, vec!["b"]);

        let b = graph.get("b").expect("node 'b' should exist");
        assert_eq!(b.calls, vec!["a"]);
        assert_eq!(b.called_by, vec!["a"]);
    }

    // ------------------------------------------------------------------
    // Transitive call — a() calls b(), b() calls c()
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_transitive() {
        let c_body = make_literal(1);
        let b_body = make_call("c");
        let a_body = make_call("b");

        let decls = vec![
            make_func("a", a_body),
            make_func("b", b_body),
            make_func("c", c_body),
        ];
        let graph = build_call_graph(&decls);

        let a = graph.get("a").expect("node 'a' should exist");
        assert_eq!(a.calls, vec!["b"]);
        assert!(a.called_by.is_empty(), "a should not be called by anyone");

        let b = graph.get("b").expect("node 'b' should exist");
        assert_eq!(b.calls, vec!["c"]);
        assert_eq!(b.called_by, vec!["a"]);

        let c = graph.get("c").expect("node 'c' should exist");
        assert!(c.calls.is_empty(), "c should call nothing");
        assert_eq!(c.called_by, vec!["b"]);
    }

    // ------------------------------------------------------------------
    // Cycle detection — mutual recursion
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_cycle_detection() {
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_call("a")),
        ];
        let graph = build_call_graph(&decls);
        assert!(
            graph.has_cycle(),
            "mutual recursion should be detected as a cycle"
        );
    }

    // ------------------------------------------------------------------
    // Self-recursion — fn a() calls itself
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_self_call() {
        let decls = vec![make_func("a", make_call("a"))];
        let graph = build_call_graph(&decls);

        let a = graph.get("a").expect("node 'a' should exist");
        assert_eq!(a.calls, vec!["a"]);
        assert_eq!(a.called_by, vec!["a"]);
        assert!(
            graph.has_cycle(),
            "self-recursion should be detected as a cycle"
        );
    }

    // ------------------------------------------------------------------
    // No cycle — a() calls b(), but b() does not call a()
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_no_cycle() {
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);
        assert!(
            !graph.has_cycle(),
            "a simple call without back-edge should not be a cycle"
        );
    }

    // ------------------------------------------------------------------
    // Non-function declarations are ignored
    // ------------------------------------------------------------------

    #[test]
    fn test_call_graph_skips_non_function_decls() {
        use dwarf_mir::MirField;

        let func = make_func("f", make_literal(42));
        let typedef = MirDecl::TypeDef {
            name: "MyInt".into(),
            type_: dwarf_syntax::hir::Type::Named("Int".into()),
            is_pub: true,
            span: span1(),
        };
        let recdef = MirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![MirField {
                name: "x".into(),
                type_: dwarf_syntax::hir::Type::Named("Int".into()),
            }],
            is_pub: true,
            span: span1(),
        };
        let uniondef = MirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![],
            is_pub: true,
            span: span1(),
        };

        let decls = vec![typedef, recdef, uniondef, func];
        let graph = build_call_graph(&decls);

        // Only "f" should be in the graph
        assert!(graph.get("f").is_some(), "function 'f' should exist");
        assert!(graph.get("MyInt").is_none(), "type defs should be skipped");
        assert!(
            graph.get("Point").is_none(),
            "record defs should be skipped"
        );
        assert!(
            graph.get("Option").is_none(),
            "union defs should be skipped"
        );
        assert_eq!(graph.nodes.len(), 1, "only one node should be present");
    }

    // ------------------------------------------------------------------
    // initial_effects — baseline effect assignment
    // ------------------------------------------------------------------

    #[test]
    fn test_initial_effects_all_pure() {
        let decls = vec![
            make_func("a", make_literal(1)),
            make_func("b", make_call("a")),
        ];
        let effects = initial_effects(&decls);

        assert_eq!(effects.len(), 2, "should produce an entry per function");
        assert_eq!(effects.get("a"), Some(&Effect::Pure));
        assert_eq!(effects.get("b"), Some(&Effect::Pure));
    }

    #[test]
    fn test_initial_effects_skips_non_functions() {
        use dwarf_mir::MirField;

        let func = make_func("f", make_literal(42));
        let typedef = MirDecl::TypeDef {
            name: "MyInt".into(),
            type_: dwarf_syntax::hir::Type::Named("Int".into()),
            is_pub: true,
            span: span1(),
        };
        let recdef = MirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![MirField {
                name: "x".into(),
                type_: dwarf_syntax::hir::Type::Named("Int".into()),
            }],
            is_pub: true,
            span: span1(),
        };
        let uniondef = MirDecl::UnionDef {
            name: "Option".into(),
            variants: vec![],
            is_pub: true,
            span: span1(),
        };

        let decls = vec![typedef, recdef, uniondef, func];
        let effects = initial_effects(&decls);

        assert_eq!(
            effects.len(),
            1,
            "only the function declaration should yield an effect entry"
        );
        assert_eq!(
            effects.get("f"),
            Some(&Effect::Pure),
            "the function should default to Pure"
        );
        assert!(
            !effects.contains_key("MyInt"),
            "TypeDef declarations should be skipped"
        );
        assert!(
            !effects.contains_key("Point"),
            "RecordDef declarations should be skipped"
        );
        assert!(
            !effects.contains_key("Option"),
            "UnionDef declarations should be skipped"
        );
    }

    // ------------------------------------------------------------------
    // resolve_effects — call-graph effect propagation
    // ------------------------------------------------------------------

    #[test]
    fn test_resolve_no_calls() {
        let decls = vec![make_func("a", make_literal(1))];
        let graph = build_call_graph(&decls);
        let effects = resolve_effects(&decls, &graph, &HashMap::new());

        assert_eq!(effects.len(), 1, "should have an entry for 'a'");
        assert_eq!(
            effects.get("a"),
            Some(&Effect::Pure),
            "function with no calls and no async body stays Pure"
        );
    }

    #[test]
    fn test_resolve_transitive_async_detection() {
        // a calls b, b calls async c → a and b become async through propagation
        // Seed c as explicitly async to test propagation
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_call("c")),
            make_func("c", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);
        let mut seeds = HashMap::new();
        seeds.insert("c".into(), Effect::Async);
        let effects = resolve_effects(&decls, &graph, &seeds);

        assert_eq!(effects.len(), 3, "all three functions should have effects");
        assert_eq!(
            effects.get("a"),
            Some(&Effect::Async),
            "a calls b which transitively calls async c → a is async"
        );
        assert_eq!(
            effects.get("b"),
            Some(&Effect::Async),
            "b calls async c directly → b is async"
        );
        assert_eq!(
            effects.get("c"),
            Some(&Effect::Async),
            "c is seeded as explicitly async"
        );
    }

    // ------------------------------------------------------------------
    // detect_pure_async_conflicts — pure → async edge detection
    // ------------------------------------------------------------------

    #[test]
    fn test_detect_pure_calls_async() {
        // a(pure) calls b(async) → one conflict (a, b)
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);

        let mut resolved_effects = HashMap::new();
        resolved_effects.insert("a".into(), Effect::Pure);
        resolved_effects.insert("b".into(), Effect::Async);

        let conflicts = detect_pure_async_conflicts(&resolved_effects, &graph);

        assert_eq!(conflicts.len(), 1, "one pure→async edge should be reported");
        assert_eq!(
            conflicts[0],
            ("a".to_string(), "b".to_string()),
            "a is pure and calls async b"
        );
    }

    #[test]
    fn test_detect_no_conflict() {
        // a(async) calls b(async) → no pure→async conflict
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);

        let mut resolved_effects = HashMap::new();
        resolved_effects.insert("a".into(), Effect::Async);
        resolved_effects.insert("b".into(), Effect::Async);

        let conflicts = detect_pure_async_conflicts(&resolved_effects, &graph);

        assert!(
            conflicts.is_empty(),
            "no conflict when both caller and callee are async"
        );
    }

    #[test]
    fn test_detect_multiple_conflicts() {
        // a(pure)→b(async) and c(pure)→d(async) = two conflicts
        let decls = vec![
            make_func("a", make_call("b")),
            make_func("b", make_literal(1)),
            make_func("c", make_call("d")),
            make_func("d", make_literal(1)),
        ];
        let graph = build_call_graph(&decls);

        let mut resolved_effects = HashMap::new();
        resolved_effects.insert("a".into(), Effect::Pure);
        resolved_effects.insert("b".into(), Effect::Async);
        resolved_effects.insert("c".into(), Effect::Pure);
        resolved_effects.insert("d".into(), Effect::Async);

        let conflicts = detect_pure_async_conflicts(&resolved_effects, &graph);

        assert_eq!(conflicts.len(), 2, "two separate pure→async edges");
        assert!(
            conflicts.contains(&("a".to_string(), "b".to_string())),
            "first conflict: a (pure) calls b (async)"
        );
        assert!(
            conflicts.contains(&("c".to_string(), "d".to_string())),
            "second conflict: c (pure) calls d (async)"
        );
    }
}
