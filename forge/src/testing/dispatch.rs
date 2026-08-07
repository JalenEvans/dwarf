//! Wasm test-runner dispatch — routes `forge test --target wasm` into the
//! wasmtime executor instead of the legacy Jest passthrough.
//!
//! [`run_wasm_tests`] reads each `.kzd` file, compiles it to WAT via
//! `DwarfCompiler`, parses the WAT with `wat`, discovers the exported `@test`
//! functions, and runs each through the wasmtime [`WasmTestRunner`]. The
//! `Commands::Test` handler in `forge/src/main.rs` calls [`run_wasm_tests`]
//! whenever [`is_wasm_target`] matches the `--target`.

use std::path::PathBuf;

use dwarf_cli::output::TestResultItem;
use dwarf_lib::{CompileOptions, Diagnostic, DwarfCompiler, Severity};
use wasmtime::{Engine, Module};

use super::runner::WasmTestRunner;

/// Whether a CLI `--target` value should route through the wasmtime runner.
///
/// Only the literal `"wasm"` is routed. Every other target (`ts`, `py`,
/// `java`, ...) falls through to the legacy runner. This is the DISPATCH
/// DECISION at the heart of DWARF-129.
pub fn is_wasm_target(target: &str) -> bool {
    target == "wasm"
}

/// Execute every @test function in each `.kzd` file under the wasmtime runner
/// and return one [`TestResultItem`] per test function.
///
/// Each file is read, compiled via `DwarfCompiler` to target `"wasm"` (WAT),
/// parsed by `wat` into module bytes, then each `@test` export is run through
/// `WasmTestRunner`. A per-test item is produced for every test function (its
/// file, pass/fail, and an expected-vs-actual message on failure). When a file
/// fails to compile or its WAT does not parse, the file yields a single
/// `passed: false` item carrying the compile/parse error.
///
/// `filter`, when `Some`, restricts execution to tests whose function name
/// matches the pattern (all tests run when `None`).
///
/// When `draupnir` is `true`, the Draupnir PBT runtime sources are prepended to
/// each file before compilation so its declarations (e.g. `for_all`) resolve
/// during typechecking (DWARF-130). The runtime methods desugar to callable
/// signatures and the wasm backend emits the runtime; constructs genuinely
/// outside the wasm i32 subset (e.g. a string literal in a property body)
/// still surface as `unsupported feature` diagnostics and fail the run, which
/// is the intended signal.
///
pub fn run_wasm_tests(
    files: &[PathBuf],
    filter: Option<&str>,
    draupnir: bool,
) -> Vec<TestResultItem> {
    let mut results = Vec::new();

    // Precompute the Draupnir runtime source once (empty when disabled). It is
    // prepended to every file so the runtime and the user source desugar and
    // typecheck together in a single compilation unit.
    let draupnir_runtime = if draupnir {
        dwarf_lib::draupnir::runtime_sources()
    } else {
        String::new()
    };

    for file in files {
        let file_str = file.to_string_lossy().into_owned();

        // 1. Read the source file.
        let source = match std::fs::read_to_string(file) {
            Ok(source) => source,
            Err(e) => {
                results.push(TestResultItem {
                    file: file_str,
                    passed: false,
                    message: format!("failed to read {}: {e}", file.display()),
                });
                continue;
            }
        };

        // 2. Compile to the wasm target (WAT text).
        //
        //    When `draupnir` is set, prepend the runtime source so its
        //    declarations are visible to the typechecker. The pipeline runs on
        //    the combined source — the runtime and user code are one unit.
        let compiler = DwarfCompiler::new();
        let options = CompileOptions {
            target: "wasm".to_string(),
            ..Default::default()
        };
        let program = if draupnir_runtime.is_empty() {
            source
        } else {
            format!("{}\n\n{}", draupnir_runtime, source)
        };
        let compile_result = match compiler.compile(&program, &file_str, options) {
            Ok(result) => result,
            Err(errors) => {
                let message = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                results.push(TestResultItem {
                    file: file_str,
                    passed: false,
                    message,
                });
                continue;
            }
        };

        // A hard error diagnostic means the file did not compile — report a
        // single per-file failure carrying the compiler diagnostics.
        if compile_result
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error))
        {
            results.push(TestResultItem {
                file: file_str,
                passed: false,
                message: diagnostics_to_message(&compile_result.diagnostics),
            });
            continue;
        }

        // 3. Parse the emitted WAT into module bytes.
        let wasm = match wat::parse_str(&compile_result.output) {
            Ok(bytes) => bytes,
            Err(e) => {
                results.push(TestResultItem {
                    file: file_str,
                    passed: false,
                    message: format!("WAT parse error: {e}"),
                });
                continue;
            }
        };

        // 4. Discover the @test candidates: the module's exports. Hook
        //    functions (`before_each`/`after_each`) are not tests.
        let test_names = match discover_test_exports(&wasm) {
            Ok(names) => names,
            Err(e) => {
                results.push(TestResultItem {
                    file: file_str,
                    passed: false,
                    message: e,
                });
                continue;
            }
        };

        // 5. Execute each test export through the wasmtime runner, honoring
        //    the optional name filter. One result item per test function.
        let runner = WasmTestRunner::new();
        for name in test_names {
            if let Some(pattern) = filter {
                if !name.contains(pattern) {
                    continue;
                }
            }
            match runner.run_test(&wasm, &name) {
                Ok(result) => {
                    // Carry the test function name in the message so the CLI
                    // can report WHICH test ran (the `TestResultItem` shape
                    // has no dedicated name field).
                    let message = match result.message {
                        Some(m) => format!("{name}: {m}"),
                        None => format!("{name}: passed"),
                    };
                    results.push(TestResultItem {
                        file: file_str.clone(),
                        passed: result.passed,
                        message,
                    })
                }
                Err(e) => results.push(TestResultItem {
                    file: file_str.clone(),
                    passed: false,
                    message: format!("{name}: {e}"),
                }),
            }
        }
    }

    results
}

/// Discover the exported function names of a compiled wasm module, excluding
/// `before_each`/`after_each` hook exports. Compiled modules are inspected with
/// the wasmtime engine (already a forge dependency) rather than parsing WAT
/// text by hand.
fn discover_test_exports(wasm: &[u8]) -> Result<Vec<String>, String> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).map_err(|e| format!("wasm module error: {e}"))?;
    let names: Vec<String> = module
        .exports()
        .map(|export| export.name().to_string())
        .filter(|name| !name.starts_with("before_each") && !name.starts_with("after_each"))
        .collect();
    Ok(names)
}

/// Render a list of compiler diagnostics into a single human-readable message.
fn diagnostics_to_message(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| format!("[{}] {}: {}", d.code, d.severity, d.message))
        .collect::<Vec<_>>()
        .join("\n")
}
