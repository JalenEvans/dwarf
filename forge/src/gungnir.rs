//! Gungnir — the forge Z3 subprocess bridge (DWARF-120).
//!
//! This module owns the process side of `@gungnir` formal verification:
//!
//!   1. resolve the Z3 solver binary (`DWARF_Z3` env is authoritative, else
//!      `z3` on `$PATH`),
//!   2. parse each `.kzd` file into [`Decl`]s and discover `@gungnir`
//!      functions via `dwarf_lib::gungnir::discover_gungnir`,
//!   3. for each function, feed `build_verification_query`'s SMT-LIB2 script to
//!      `z3 -in` on stdin, read stdout, and map it through
//!      `parse_smt_output` into a `Verdict`,
//!   4. enforce the `--timeout-ms` budget by killing a solver that exceeds it
//!      and reporting the function as `unproven`.
//!
//! The pure SMT-LIB2 translator / verdict parser live in `dwarf-lib`; this
//! module only bridges them to a real Z3 process.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use dwarf_lexer::pass::TokenizePass;
use dwarf_lib::gungnir::{
    build_verification_query, discover_gungnir, parse_smt_output, unsupported_reason, Verdict,
};
use dwarf_parser::Parser;

/// Options for a single `forge gungnir` run.
#[derive(Debug, Clone)]
pub struct GungnirOptions {
    /// Emit a structured JSON report instead of the human-readable text.
    pub json: bool,
    /// Per-query solver budget in milliseconds.
    pub timeout_ms: u64,
}

/// A per-function verification result for one source file.
#[derive(Debug, Clone)]
pub struct GungnirResult {
    /// Source file path, as passed on the command line.
    pub file: String,
    /// Name of the verified function.
    pub function: String,
    /// The solver verdict for this function.
    pub verdict: Verdict,
    /// Whether the function's contract carries an `@invariant`.
    ///
    /// Drives the honest AC-1 disclosure: `@invariant` is verified as an ENTRY
    /// (data-consistency) invariant, not as an inductive check across all
    /// reachable states, and the human report says so.
    pub has_invariant: bool,
}

impl GungnirResult {
    /// Human-readable report line, e.g.
    /// `gungnir: abs — proved` or `gungnir: identity — counterexample (a = 5)`.
    fn report_line(&self) -> String {
        let base = match &self.verdict {
            Verdict::Counterexample { model } => {
                let bindings = format_model(model);
                if bindings.is_empty() {
                    format!("gungnir: {} — counterexample", self.function)
                } else {
                    format!("gungnir: {} — counterexample ({})", self.function, bindings)
                }
            }
            Verdict::Proved => format!("gungnir: {} — proved", self.function),
            Verdict::Unproven { reason } => {
                format!("gungnir: {} — unproven ({})", self.function, reason)
            }
            Verdict::Error { reason } => {
                format!("gungnir: {} — error ({})", self.function, reason)
            }
        };
        // AC-1: disclose that @invariant is verified as an ENTRY invariant so
        // `proved` is not oversold as an inductive check across all states.
        if self.has_invariant {
            format!("{} (entry-invariant)", base)
        } else {
            base
        }
    }

    /// Structured JSON form of this result.
    fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "file".to_string(),
            serde_json::Value::String(self.file.clone()),
        );
        obj.insert(
            "function".to_string(),
            serde_json::Value::String(self.function.clone()),
        );
        obj.insert(
            "status".to_string(),
            serde_json::Value::String(self.verdict.label().to_string()),
        );
        match &self.verdict {
            Verdict::Counterexample { model } => {
                obj.insert(
                    "model".to_string(),
                    serde_json::Value::String(format_model(model)),
                );
            }
            Verdict::Unproven { reason } | Verdict::Error { reason } => {
                obj.insert(
                    "reason".to_string(),
                    serde_json::Value::String(reason.clone()),
                );
            }
            Verdict::Proved => {}
        }
        serde_json::Value::Object(obj)
    }
}

/// Entry point for the `forge gungnir` subcommand.
///
/// Exits non-zero when the solver is unavailable or any input file cannot be
/// read / parsed. Verification verdicts themselves (including counterexamples
/// and unproven results) are reported, not treated as tool failures.
pub fn run_gungnir(files: Vec<PathBuf>, json: bool, timeout_ms: u64) {
    let z3 = match resolve_z3() {
        Some(z3) => z3,
        None => {
            eprint!("{}", z3_install_hints());
            std::process::exit(1);
        }
    };

    let mut results = Vec::new();
    let mut had_error = false;

    for file in &files {
        match verify_file(file, &z3, timeout_ms) {
            Ok(mut rs) => results.append(&mut rs),
            Err(e) => {
                eprintln!("forge: error — {}: {}", file.display(), e);
                had_error = true;
            }
        }
    }

    if json {
        let payloads: Vec<serde_json::Value> = results.iter().map(|r| r.to_json()).collect();
        let output = serde_json::to_string_pretty(&serde_json::json!({ "gungnir": payloads }))
            .unwrap_or_else(|_| "{}".to_string());
        println!("{}", output);
    } else {
        for r in &results {
            println!("{}", r.report_line());
        }
        println!("{}", summary_line(&results));
    }

    if had_error {
        std::process::exit(1);
    }
}

/// Human-readable summary of all results, e.g.
/// `forge: gungnir — 1 proved, 0 counterexample, 0 unproven, 0 error`.
fn summary_line(results: &[GungnirResult]) -> String {
    let mut proved = 0usize;
    let mut counterexample = 0usize;
    let mut unproven = 0usize;
    let mut error = 0usize;
    for r in results {
        match &r.verdict {
            Verdict::Proved => proved += 1,
            Verdict::Counterexample { .. } => counterexample += 1,
            Verdict::Unproven { .. } => unproven += 1,
            Verdict::Error { .. } => error += 1,
        }
    }
    format!(
        "forge: gungnir — {} proved, {} counterexample, {} unproven, {} error",
        proved, counterexample, unproven, error
    )
}

// ---------------------------------------------------------------------------
// Z3 binary resolution
// ---------------------------------------------------------------------------

/// Resolve the Z3 solver binary.
///
/// `DWARF_Z3` is authoritative when set: a set-but-nonexistent path counts as
/// "solver not found" (no `$PATH` fallback). When unset, `z3` is looked up on
/// `$PATH`.
fn resolve_z3() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("DWARF_Z3") {
        let candidate = PathBuf::from(path);
        return candidate.is_file().then_some(candidate);
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let candidate = dir.join("z3");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Install instructions printed when the Z3 solver cannot be resolved.
fn z3_install_hints() -> String {
    let mut out = String::new();
    out.push_str("forge: error — Z3 solver not found.\n");
    out.push_str("forge: gungnir verification requires the Z3 SMT solver.\n");
    out.push_str("forge:\n");
    out.push_str("forge: Install Z3 and make sure it is on your $PATH, or point the\n");
    out.push_str("forge: DWARF_Z3 environment variable at a Z3 binary:\n");
    out.push_str("forge:\n");
    out.push_str("forge:   Ubuntu / Debian:  sudo apt install z3\n");
    out.push_str("forge:   Fedora:           sudo dnf install z3\n");
    out.push_str("forge:   macOS (Homebrew): brew install z3\n");
    out.push_str("forge:   Windows:          winget install Microsoft.Z3\n");
    out.push_str("forge:   From source:      https://github.com/Z3Prover/z3\n");
    out.push_str("forge:\n");
    out.push_str("forge: Then re-run:  forge gungnir <files...>\n");
    out
}

// ---------------------------------------------------------------------------
// File pipeline: read → parse → discover → verify
// ---------------------------------------------------------------------------

/// Verify every `@gungnir` function in a single `.kzd` file.
fn verify_file(file: &Path, z3: &Path, timeout_ms: u64) -> Result<Vec<GungnirResult>, String> {
    let file_str = file.to_string_lossy().to_string();
    let source = std::fs::read_to_string(file).map_err(|e| format!("cannot read file: {}", e))?;

    let tokenizer = TokenizePass;
    let tokens = tokenizer
        .tokenize(&source)
        .map_err(|e| format!("lexer error: {}", e))?;

    let mut parser = Parser::new(tokens);
    let (decls, _parse_errors) = parser.parse();

    let functions = discover_gungnir(&decls);

    let mut results = Vec::new();
    for f in &functions {
        let has_invariant = f.contract.invariant.is_some();

        // Soundness gate: reject functions outside the verifiable v1 subset
        // BEFORE building/running a query so we never report a false verdict.
        if let Some(reason) = unsupported_reason(f) {
            results.push(GungnirResult {
                file: file_str.clone(),
                function: f.name.clone(),
                verdict: Verdict::Unproven {
                    reason: format!("unsupported: {}", reason),
                },
                has_invariant,
            });
            continue;
        }

        // A function with NO post-condition has nothing to disprove; reporting
        // it as `counterexample` would conflate "no contract" with "violated".
        // Report `unproven` instead.
        if f.contract.post.is_none() {
            results.push(GungnirResult {
                file: file_str.clone(),
                function: f.name.clone(),
                verdict: Verdict::Unproven {
                    reason: "no post-condition".to_string(),
                },
                has_invariant,
            });
            continue;
        }

        let query = build_verification_query(f);
        let verdict = match run_solver(z3, &query, timeout_ms) {
            Ok(stdout) => parse_smt_output(&stdout),
            Err(SolverError::Timeout) => Verdict::Unproven {
                reason: format!("solver exceeded {}ms budget", timeout_ms),
            },
            Err(SolverError::Spawn(e)) => {
                return Err(format!("failed to start z3: {}", e));
            }
            Err(SolverError::Io(e)) => {
                return Err(format!("z3 I/O error: {}", e));
            }
        };
        results.push(GungnirResult {
            file: file_str.clone(),
            function: f.name.clone(),
            verdict,
            has_invariant,
        });
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Z3 subprocess bridge with a hard timeout
// ---------------------------------------------------------------------------

/// Failure modes from the solver subprocess bridge.
enum SolverError {
    /// The `z3` binary could not be spawned.
    Spawn(std::io::Error),
    /// Reading/writing the solver's pipes failed.
    Io(std::io::Error),
    /// The solver did not finish within the time budget (it was killed).
    Timeout,
}

/// Run `z3 -in` on `query` (stdin), collect stdout, and enforce `timeout_ms`.
///
/// When the solver exceeds the budget it is killed and [`SolverError::Timeout`]
/// is returned; the caller maps that to an `unproven` verdict.
fn run_solver(z3: &Path, query: &str, timeout_ms: u64) -> Result<String, SolverError> {
    let mut child = Command::new(z3)
        .arg("-in")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(SolverError::Spawn)?;

    // Feed the SMT-LIB2 script and close stdin (EOF) so z3 starts solving.
    {
        let mut stdin = child.stdin.take().expect("z3 stdin pipe");
        let mut data = query.as_bytes().to_vec();
        data.push(b'\n');
        stdin.write_all(&data).map_err(|e| {
            let _ = child.kill();
            SolverError::Io(e)
        })?;
    }

    // Read stdout to EOF on a background thread; the main thread enforces the
    // deadline so a hung solver cannot block forge indefinitely.
    let mut stdout = child.stdout.take().expect("z3 stdout pipe");
    let (tx, rx) = mpsc::channel::<std::io::Result<Vec<u8>>>();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let result = std::io::Read::read_to_end(&mut stdout, &mut buf).map(|_| buf);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(Ok(bytes)) => {
            let _ = child.wait();
            let _ = reader.join();
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
        Ok(Err(e)) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            Err(SolverError::Io(e))
        }
        Err(_) => {
            // Deadline elapsed (or the channel disconnected): the solver did
            // not produce output in time. Kill it and report unproven.
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            Err(SolverError::Timeout)
        }
    }
}

// ---------------------------------------------------------------------------
// Counterexample model formatting
// ---------------------------------------------------------------------------

/// Render a z3 model as `name = value` bindings, e.g. `a = 5, result = 5`.
///
/// The model is an S-expression of `(define-fun <name> () <sort> <value>)`
/// forms; we extract the concrete bindings and drop the sort/arity noise.
fn format_model(model: &str) -> String {
    model_bindings(model)
        .iter()
        .map(|(name, value)| format!("{} = {}", name, value))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Extract `(name, value)` pairs from every `define-fun` form in a model.
///
/// Z3 wraps the model in an outer S-expression that groups all
/// `(define-fun <name> () <sort> <value>)` forms, so we walk the entire parsed
/// tree (at any depth) rather than assuming a flat top level.
fn model_bindings(model: &str) -> Vec<(String, String)> {
    let toks = tokenize(model);
    let mut bindings = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        match parse_sexpr(&toks, i) {
            Ok((expr, next)) => {
                collect_bindings(&expr, &mut bindings);
                i = next;
            }
            Err(_) => break,
        }
    }
    bindings
}

/// Recursively collect bindings from a parsed S-expression.
fn collect_bindings(expr: &Sexpr, out: &mut Vec<(String, String)>) {
    if let Sexpr::List(items) = expr {
        // Shape: (define-fun <name> () <sort> <value>...)
        if items.len() >= 4 {
            if let (Sexpr::Atom(kw), Sexpr::Atom(name)) = (&items[0], &items[1]) {
                if kw == "define-fun" && !name.is_empty() {
                    let value: Vec<String> = items[4..].iter().map(sexpr_str).collect();
                    if !value.is_empty() {
                        out.push((name.clone(), value.join(" ")));
                    }
                }
            }
        }
        for item in items {
            collect_bindings(item, out);
        }
    }
}

/// A minimal S-expression token.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LParen,
    RParen,
    Atom(String),
}

/// Tokenize an S-expression string into parens + atoms.
fn tokenize(s: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut current = String::new();
    for c in s.chars() {
        match c {
            '(' => {
                if !current.is_empty() {
                    toks.push(Tok::Atom(std::mem::take(&mut current)));
                }
                toks.push(Tok::LParen);
            }
            ')' => {
                if !current.is_empty() {
                    toks.push(Tok::Atom(std::mem::take(&mut current)));
                }
                toks.push(Tok::RParen);
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    toks.push(Tok::Atom(std::mem::take(&mut current)));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        toks.push(Tok::Atom(current));
    }
    toks
}

/// A parsed S-expression.
#[derive(Debug, Clone, PartialEq)]
enum Sexpr {
    Atom(String),
    List(Vec<Sexpr>),
}

/// Parse one S-expression starting at `toks[pos]`; returns it and the next
/// unconsumed index.
fn parse_sexpr(toks: &[Tok], pos: usize) -> Result<(Sexpr, usize), ()> {
    match toks.get(pos) {
        Some(Tok::Atom(a)) => Ok((Sexpr::Atom(a.clone()), pos + 1)),
        Some(Tok::LParen) => {
            let mut items = Vec::new();
            let mut i = pos + 1;
            loop {
                match toks.get(i) {
                    None => return Err(()),
                    Some(Tok::RParen) => return Ok((Sexpr::List(items), i + 1)),
                    _ => {
                        let (item, next) = parse_sexpr(toks, i)?;
                        items.push(item);
                        i = next;
                    }
                }
            }
        }
        _ => Err(()),
    }
}

/// Render a parsed S-expression back to a compact string.
fn sexpr_str(e: &Sexpr) -> String {
    match e {
        Sexpr::Atom(a) => a.clone(),
        Sexpr::List(items) => {
            let inner: Vec<String> = items.iter().map(sexpr_str).collect();
            format!("({})", inner.join(" "))
        }
    }
}
