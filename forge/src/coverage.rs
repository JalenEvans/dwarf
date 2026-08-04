//! Coverage reporter for the `forge coverage` subcommand (DWARF-118).
//!
//! Reads Dwarf source files, analyzes functions, `@test` decorators, branch
//! edges, and `@gungnir` verification status, then emits a human-readable
//! coverage report (or JSON when `--json` is passed).
//!
//! GREEN PHASE: This is a first working implementation. It performs a
//! lightweight static analysis over the parsed HIR:
//!   - functions tested  = functions with an `@test`, covered by a
//!     `test_<name>` naming convention, or targeted by `@tested(<name>)`
//!   - edges covered     = branch edges (if/match/try/for) inside tested
//!     functions
//!   - gungnir status    = verified when no `@gungnir` function is left
//!     untested

use std::collections::HashSet;
use std::path::PathBuf;
use std::process;

use dwarf_lexer::pass::TokenizePass;
use dwarf_parser::Parser;
use dwarf_syntax::hir::{Decl, Decorator, Expr, MatchArm, Stmt};
use dwarf_syntax::test_manifest::{collect_test_manifest, TestManifest};

/// Per-function coverage summary.
#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub tested: bool,
    pub gungnir: bool,
    pub edges_total: usize,
    pub edges_covered: usize,
}

/// Aggregate coverage report for a single source file.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub file: String,
    pub functions_total: usize,
    pub functions_tested: usize,
    pub edges_total: usize,
    pub edges_covered: usize,
    pub gungnir_functions: usize,
    pub gungnir_unverified: usize,
    pub functions: Vec<FunctionCoverage>,
}

impl CoverageReport {
    /// Branch coverage as a percentage (100% when there are no branches).
    fn branch_coverage_pct(&self) -> f64 {
        if self.edges_total == 0 {
            100.0
        } else {
            self.edges_covered as f64 * 100.0 / self.edges_total as f64
        }
    }

    /// Overall `@gungnir` verification status.
    fn gungnir_status(&self) -> &'static str {
        if self.gungnir_unverified == 0 {
            "verified"
        } else {
            "unverified"
        }
    }

    /// Emit a JSON representation of this report.
    fn to_json(&self) -> serde_json::Value {
        let functions: Vec<serde_json::Value> = self
            .functions
            .iter()
            .map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "tested": f.tested,
                    "gungnir": f.gungnir,
                    "edges_total": f.edges_total,
                    "edges_covered": f.edges_covered,
                })
            })
            .collect();
        serde_json::json!({
            "file": self.file,
            "functions_total": self.functions_total,
            "functions_tested": self.functions_tested,
            "edges_total": self.edges_total,
            "edges_covered": self.edges_covered,
            "branch_coverage_pct": self.branch_coverage_pct(),
            "gungnir_status": self.gungnir_status(),
            "gungnir_functions": self.gungnir_functions,
            "gungnir_unverified": self.gungnir_unverified,
            "functions": functions,
        })
    }

    /// Emit a human-readable representation of this report.
    fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Coverage report: {}\n", self.file));
        out.push_str(&format!(
            "  Functions: {} total, {} tested\n",
            self.functions_total, self.functions_tested
        ));
        out.push_str(&format!(
            "  Edges: {} total, {} covered\n",
            self.edges_total, self.edges_covered
        ));
        out.push_str(&format!(
            "  Branch coverage: {:.1}%\n",
            self.branch_coverage_pct()
        ));
        out.push_str(&format!("  gungnir status: {}\n", self.gungnir_status()));
        for f in &self.functions {
            let tested = if f.tested { "tested" } else { "untested" };
            let gungnir = if f.gungnir { " @gungnir" } else { "" };
            out.push_str(&format!(
                "    - {}: {} ({} edge(s) total, {} covered){}\n",
                f.name, tested, f.edges_total, f.edges_covered, gungnir
            ));
        }
        out
    }
}

/// Entry point for the `forge coverage` subcommand.
pub fn run_coverage(
    files: Vec<PathBuf>,
    json: bool,
    _quick: bool,
    _skip_edge_check: bool,
    _test_coverage: Option<String>,
) {
    let mut reports = Vec::new();
    let mut had_error = false;

    for file in &files {
        match analyze_file(file) {
            Ok(report) => reports.push(report),
            Err(e) => {
                eprintln!("Error analyzing {}: {}", file.display(), e);
                had_error = true;
            }
        }
    }

    if json {
        let payloads: Vec<serde_json::Value> = reports.iter().map(|r| r.to_json()).collect();
        let output = serde_json::to_string_pretty(&serde_json::json!({ "coverage": payloads }))
            .unwrap_or_else(|_| "{}".to_string());
        println!("{}", output);
    } else {
        for report in &reports {
            print!("{}", report.to_text());
        }
    }

    if had_error {
        process::exit(1);
    }
}

/// Analyze a single source file and produce a coverage report.
fn analyze_file(file: &PathBuf) -> Result<CoverageReport, String> {
    let file_str = file.to_string_lossy().to_string();
    let source =
        std::fs::read_to_string(file).map_err(|e| format!("cannot read file: {}", e))?;

    let tokenizer = TokenizePass;
    let tokens = tokenizer
        .tokenize(&source)
        .map_err(|e| format!("lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let (decls, _parse_errors) = parser.parse();

    // Names covered by tests: @test functions themselves, `test_<name>`
    // naming convention, and `@tested(<name>)` targets.
    let manifest: TestManifest = collect_test_manifest(&decls);
    let mut covered_names: HashSet<String> = HashSet::new();

    for test_fn in &manifest.tests {
        covered_names.insert(test_fn.function_name.clone());
        // `test_<fn>` covers `<fn>`
        if let Some(target) = test_fn.function_name.strip_prefix("test_") {
            covered_names.insert(target.to_string());
        }
    }
    for decl in &decls {
        if let Decl::Function { decorators, .. } = decl {
            for d in decorators {
                if let Decorator::Tested { fn_name } = d {
                    covered_names.insert(fn_name.clone());
                }
            }
        }
    }

    let mut functions = Vec::new();
    let mut edges_total = 0usize;
    let mut edges_covered = 0usize;
    let mut functions_tested = 0usize;
    let mut gungnir_functions = 0usize;
    let mut gungnir_unverified = 0usize;

    for decl in &decls {
        if let Decl::Function {
            name,
            decorators,
            body,
            ..
        } = decl
        {
            let tested = covered_names.contains(name);
            let has_gungnir = decorators.iter().any(|d| matches!(d, Decorator::Gungnir));
            let fn_edges = count_expr_edges(body);

            if has_gungnir {
                gungnir_functions += 1;
                if !tested {
                    gungnir_unverified += 1;
                }
            }
            if tested {
                functions_tested += 1;
                edges_covered += fn_edges;
            }
            edges_total += fn_edges;

            functions.push(FunctionCoverage {
                name: name.clone(),
                tested,
                gungnir: has_gungnir,
                edges_total: fn_edges,
                edges_covered: if tested { fn_edges } else { 0 },
            });
        }
    }

    Ok(CoverageReport {
        file: file_str,
        functions_total: functions.len(),
        functions_tested,
        edges_total,
        edges_covered,
        gungnir_functions,
        gungnir_unverified,
        functions,
    })
}

/// Count branch edges in an expression tree.
///
/// Edge accounting:
///   - `if`                 → 2 (then + else/fall-through)
///   - `match`              → 1 per arm (plus guard conditions)
///   - `try`                → 2 (try + handler)
///   - `for`                → 2 (loop entry + exit)
fn count_expr_edges(expr: &Expr) -> usize {
    match expr {
        Expr::Literal { .. } | Expr::Variable { .. } | Expr::Wildcard { .. } => 0,
        Expr::Call { func, args, .. } => {
            count_expr_edges(func) + args.iter().map(count_expr_edges).sum::<usize>()
        }
        Expr::Member { obj, .. } | Expr::OptionalAccess { obj, .. } => count_expr_edges(obj),
        Expr::If {
            cond, then, else_, ..
        } => {
            let mut edges = 2;
            edges += count_expr_edges(cond) + count_expr_edges(then);
            if let Some(e) = else_ {
                edges += count_expr_edges(e);
            }
            edges
        }
        Expr::Match { expr, arms, .. } => {
            let mut edges = arms.len().max(1);
            edges += count_expr_edges(expr);
            for arm in arms {
                edges += count_match_arm_edges(arm);
            }
            edges
        }
        Expr::Block { stmts, .. } => stmts.iter().map(count_stmt_edges).sum(),
        Expr::Pipe { lhs, rhs, .. } => count_expr_edges(lhs) + count_expr_edges(rhs),
        Expr::Propagate { expr, .. } => count_expr_edges(expr),
        Expr::Try {
            body,
            guard,
            handler,
            ..
        } => {
            let mut edges = 2;
            edges += count_expr_edges(body) + count_expr_edges(handler);
            if let Some(g) = guard {
                edges += count_expr_edges(g);
            }
            edges
        }
        Expr::Throw { expr, .. } => count_expr_edges(expr),
        Expr::For {
            iterable, body, ..
        } => {
            let mut edges = 2;
            edges += count_expr_edges(iterable) + count_expr_edges(body);
            edges
        }
        Expr::Assign { target, value, .. } => count_expr_edges(target) + count_expr_edges(value),
        Expr::Lambda { body, .. } => count_expr_edges(body),
        Expr::Record { fields, .. } => fields.iter().map(|(_, e)| count_expr_edges(e)).sum(),
        Expr::Variant { arg, .. } => arg.as_ref().map(|e| count_expr_edges(e)).unwrap_or(0),
        Expr::Array { items, .. } => items.iter().map(count_expr_edges).sum(),
        Expr::Binary { lhs, rhs, .. } => count_expr_edges(lhs) + count_expr_edges(rhs),
        Expr::Unary { expr, .. } => count_expr_edges(expr),
        Expr::ForAll { property, .. } => count_expr_edges(property),
        Expr::AssertConsistent { expr, .. } => count_expr_edges(expr),
        Expr::NonNullAssert { expr, .. } => count_expr_edges(expr),
    }
}

/// Count branch edges contributed by a statement.
fn count_stmt_edges(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Let(_, expr) => count_expr_edges(expr),
        Stmt::Expr(expr) => count_expr_edges(expr),
    }
}

/// Count branch edges contributed by a match arm (including its guard).
fn count_match_arm_edges(arm: &MatchArm) -> usize {
    let mut edges = count_expr_edges(&arm.body);
    if let Some(guard) = &arm.guard {
        edges += count_expr_edges(guard);
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(source: &str) -> CoverageReport {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("t.kzd");
        std::fs::write(&path, source).expect("write");
        analyze_file(&path).expect("analyze")
    }

    #[test]
    fn test_functions_tested_counted() {
        let report = analyze(
            "fn add(a: Int, b: Int) -> Int { a + b }\n\
             fn sub(a: Int, b: Int) -> Int { a - b }\n\
             @test fn test_add() { add(1, 2) }",
        );
        assert_eq!(report.functions_total, 3);
        assert!(report.functions_tested >= 1);
    }

    #[test]
    fn test_edges_counted_for_if_else() {
        let report = analyze(
            "fn divide(a: Int, b: Int) -> Int { if b == 0 { 0 } else { a / b } }",
        );
        assert_eq!(report.edges_total, 2);
    }

    #[test]
    fn test_gungnir_verified_by_default() {
        let report = analyze("fn main() { 42 }");
        assert_eq!(report.gungnir_status(), "verified");
    }

    #[test]
    fn test_json_output_contains_keys() {
        let report = analyze("fn main() { 42 }");
        let json = report.to_json();
        assert_eq!(json["gungnir_status"], "verified");
        assert!(json.get("functions_total").is_some());
    }
}
