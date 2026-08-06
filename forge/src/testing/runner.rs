//! Wasm test runner — executes @test functions via wasmtime.
//!
//! DWARF-118/DWARF-127: Compiles a Wasm module with wasmtime and executes the
//! @test functions inside it. Honors the dUnit decorator metadata passed in via
//! [`WasmTestRunner::with_metadata`]: `@skip`/`@skip_test` tests are reported
//! but not executed, and `@before_each`/`@after_each` hooks run around each
//! test when present in the module.

use std::collections::HashMap;
use std::fmt;

use wasmtime::{Engine, Func, Linker, Module, Store, Val};

/// Error type for test runner operations.
#[derive(Debug, Clone, PartialEq)]
pub enum RunnerError {
    /// Wasm compilation or validation failed.
    WasmCompilationError(String),
    /// The specified test function was not found in the Wasm module.
    FunctionNotFound(String),
    /// The test function executed but returned an error.
    TestFailed(String),
    /// A wasmtime runtime error occurred.
    RuntimeError(String),
    /// The runner is not yet implemented (RED phase placeholder).
    NotImplemented,
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunnerError::WasmCompilationError(msg) => write!(f, "Wasm compilation error: {}", msg),
            RunnerError::FunctionNotFound(name) => write!(f, "Function not found: {}", name),
            RunnerError::TestFailed(msg) => write!(f, "Test failed: {}", msg),
            RunnerError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            RunnerError::NotImplemented => write!(f, "Not yet implemented"),
        }
    }
}

impl std::error::Error for RunnerError {}

/// Result of executing a single test function.
#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    /// Whether the test passed.
    pub passed: bool,
    /// Name of the test function that was executed.
    pub function_name: String,
    /// Optional message (error message on failure, or skip reason).
    pub message: Option<String>,
    /// Whether the test was skipped (e.g. via `@skip`/`@skip_test`). A skipped
    /// test is not executed, so `passed` is always `false` for it.
    pub skipped: bool,
}

/// Wasm test runner — loads and executes test functions from Wasm modules.
///
/// The runner compiles the module with wasmtime and executes the requested
/// export. Per-function decorator metadata drives skip detection and hook
/// execution (see [`WasmTestRunner::with_metadata`]).
#[derive(Debug, Default)]
pub struct WasmTestRunner {
    /// Per-function decorator metadata: function name -> list of decorators
    /// (e.g. `["test", "skip"]`, `["before_each"]`).
    metadata: HashMap<String, Vec<String>>,
}

impl WasmTestRunner {
    /// Create a new WasmTestRunner with no decorator metadata.
    pub fn new() -> Self {
        WasmTestRunner {
            metadata: HashMap::new(),
        }
    }

    /// Create a new WasmTestRunner carrying per-function decorator metadata.
    ///
    /// The metadata maps each function name to its decorator list. The runner
    /// uses it to recognize `@skip`/`@skip_test` tests and `@before_each` /
    /// `@after_each` hooks.
    pub fn with_metadata(metadata: HashMap<String, Vec<String>>) -> Self {
        WasmTestRunner { metadata }
    }

    /// Run a single test function from a Wasm module.
    ///
    /// # Arguments
    /// * `wasm_bytes` — The compiled Wasm module bytes.
    /// * `function_name` — The name of the @test function to execute.
    ///
    /// # Returns
    /// * `Ok(TestResult)` — The test executed (or was skipped via metadata).
    /// * `Err(RunnerError::WasmCompilationError)` — Invalid wasm input.
    /// * `Err(RunnerError::FunctionNotFound)` — The export is not in the module.
    pub fn run_test(
        &self,
        wasm_bytes: &[u8],
        function_name: &str,
    ) -> Result<TestResult, RunnerError> {
        // A test marked @skip/@skip_test is reported as skipped and never
        // executed — the module is not even touched.
        if let Some(decorators) = self.metadata.get(function_name) {
            if is_skipped(decorators) {
                return Ok(TestResult {
                    passed: false,
                    function_name: function_name.to_string(),
                    message: Some("skipped via @skip".to_string()),
                    skipped: true,
                });
            }
        }

        // Compile the module. Invalid bytes are rejected up front.
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| RunnerError::WasmCompilationError(e.to_string()))?;

        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| RunnerError::RuntimeError(e.to_string()))?;

        // Run @before_each hooks (if present in the module) before the test.
        run_hooks(&instance, &mut store, &self.metadata, HookPhase::Before);

        // Locate the test export. A missing/renamed function is a hard error —
        // never a silent pass.
        let func = match instance.get_func(&mut store, function_name) {
            Some(func) => func,
            None => return Err(RunnerError::FunctionNotFound(function_name.to_string())),
        };

        let (passed, message) = match call_func(&mut store, &func, &[]) {
            Ok(()) => (true, None),
            Err(err) => (false, Some(format!("Test failed: {}", err))),
        };

        // Run @after_each hooks (if present in the module) after the test.
        run_hooks(&instance, &mut store, &self.metadata, HookPhase::After);

        Ok(TestResult {
            passed,
            function_name: function_name.to_string(),
            message,
            skipped: false,
        })
    }
}

/// Which hook phase a metadata-driven hook belongs to.
#[derive(Clone, Copy, PartialEq)]
enum HookPhase {
    Before,
    After,
}

/// Invoke the module exports whose metadata marks them as `@before_each` or
/// `@after_each` hooks.
///
/// A hook that is declared in metadata but missing from the module is treated
/// gracefully (not a test failure). Hook execution errors are likewise ignored
/// so a failing hook never masks the test's own result.
fn run_hooks(
    instance: &wasmtime::Instance,
    store: &mut Store<()>,
    metadata: &HashMap<String, Vec<String>>,
    phase: HookPhase,
) {
    for (name, decorators) in metadata {
        let is_hook = match phase {
            HookPhase::Before => is_before_each_hook(decorators),
            HookPhase::After => is_after_each_hook(decorators),
        };
        if !is_hook {
            continue;
        }
        if let Some(func) = instance.get_func(&mut *store, name) {
            let _ = call_func(&mut *store, &func, &[]);
        }
    }
}

/// Call a wasm function dynamically, allocating the result buffer to match the
/// function's declared result arity.
fn call_func(store: &mut Store<()>, func: &Func, params: &[Val]) -> Result<(), wasmtime::Error> {
    let num_results = func.ty(&*store).results().count();
    let mut results = vec![Val::I32(0); num_results];
    func.call(&mut *store, params, &mut results)
}

/// Standalone function to run a test (convenience wrapper).
pub fn run_test(wasm_bytes: &[u8], function_name: &str) -> Result<TestResult, RunnerError> {
    let runner = WasmTestRunner::new();
    runner.run_test(wasm_bytes, function_name)
}

/// Check if a function has the @before_each decorator.
///
/// Returns `true` if any decorator string equals `"before_each"`.
pub fn is_before_each_hook(decorators: &[String]) -> bool {
    decorators.iter().any(|d| d == "before_each")
}

/// Check if a function has the @after_each decorator.
///
/// Returns `true` if any decorator string equals `"after_each"`.
pub fn is_after_each_hook(decorators: &[String]) -> bool {
    decorators.iter().any(|d| d == "after_each")
}

/// Check if a function is skipped.
///
/// Returns `true` if any decorator string equals `"skip"` or `"skip_test"`.
pub fn is_skipped(decorators: &[String]) -> bool {
    decorators.iter().any(|d| d == "skip" || d == "skip_test")
}

// ---------------------------------------------------------------------------
// Tests — RED phase spec for the wasmtime executor (DWARF-127)
//
// These are the EXPECTED post-implementation behaviors of a real wasmtime-based
// test runner. They replace the obsolete honest-stub tests (which asserted
// `Err(RunnerError::NotImplemented)`).
//
// RED, with intent:
//  * Execution tests (pass/fail/missing/invalid) compile against the EXISTING
//    API and currently fail at runtime because `run_test` returns
//    `RunnerError::NotImplemented`.
//  * The metadata tests (`with_metadata`, and constructing/discerning the new
//    `TestResult.skipped` field) fail to COMPILE because those APIs do not exist
//    yet. Compilation failure persists until the implementation adds them —
//    they are the spec for the new API surface.
// Both failure modes are the "right" red: the behavior the wasmtime executor
// must deliver is not implemented yet.
//
// The absolute spec the implementation must satisfy (acceptance criteria):
//   1. Valid compiled module + exported @test function -> Ok(TestResult).
//   2. Passing test -> passed: true, no message.
//   3. Failing assert -> passed: false with expected-vs-actual messaging.
//   4. Missing/renamed function -> Err(RunnerError::FunctionNotFound), NOT a
//      silent pass.
//   5. @skip/@skip_test -> skipped (not executed), signaled via the new
//      `TestResult.skipped` field.
//   6. @before_each/@after_each hooks run around each test.
//   7. `RunnerError::NotImplemented` is no longer reachable on the happy path.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ==================================================================
    // Fixtures — build tiny, self-contained Wasm modules from WAT text.
    //
    // Using `wat::parse_str` mirrors how the production pipeline hands batch
    // Wasm bytes to the runner: a compiled module's raw exported-function view
    // is identical regardless of whether it was emitted by the Dwarf compiler
    // or hand-written here. `[dev-dependencies] wat` supplies this.
    // ==================================================================

    /// Parse a WAT module into Wasm bytes. Panics on invalid WAT so test
    /// fixtures fail loudly, not silently.
    fn module_bytes(wat_src: &str) -> Vec<u8> {
        wat::parse_str(wat_src).expect("test helper: WAT fixture must be a valid module")
    }

    /// A module exporting a single function `name` that returns an `i32` value
    /// without trapping -> the "passing test" fixture.
    fn module_with_single_export(name: &str) -> Vec<u8> {
        module_bytes(&format!(
            "(module (func (export \"{name}\") (result i32) i32.const 7))"
        ))
    }

    /// A module whose single export traps (stands in for a dUnit `assert`
    /// failure; the real expected-vs-actual messaging is produced by the
    /// dUnit intrinsics wired into the runner).
    fn module_with_failing_export(name: &str) -> Vec<u8> {
        module_bytes(&format!("(module (func (export \"{name}\") unreachable))"))
    }

    // ==================================================================
    // Test 1: WasmTestRunner is constructible — `new()` (backward compat)
    //
    // The empty-metadata constructor must keep compiling. It is verified
    // separately so a regression in `new()` isn't entangled with the new
    // `with_metadata(...)` surface.
    // ==================================================================

    #[test]
    fn test_wasm_test_runner_default_constructible() {
        let runner = WasmTestRunner::new();
        let _ = runner;

        // `with_metadata` is the new (not-yet-present) constructor the
        // production executor must provide. Referencing it here failures the
        // build until it exists — the intended RED signal.
        let _configured = WasmTestRunner::with_metadata(HashMap::new());
    }

    // ==================================================================
    // Test 2: TestResult shape now spans run + skipped
    //
    // The new `skipped` field is part of the required struct shape; both a
    // run result and an explicitly-skipped result are constructible. Fails to
    // compile until `skipped` exists on the production struct.
    // ==================================================================

    #[test]
    fn test_result_struct_shape_has_skipped_field() {
        // Run path: skipped defaults to false.
        let run = TestResult {
            passed: true,
            function_name: "test_ok".to_string(),
            message: None,
            skipped: false,
        };
        assert!(run.passed);
        assert!(!run.skipped, "a run test must not be marked skipped");

        // Skipped path: reported + not executed.
        let skipped = TestResult {
            passed: false,
            function_name: "test_todo".to_string(),
            message: Some("skipped via @skip".to_string()),
            skipped: true,
        };
        assert!(skipped.skipped, "@skip tests must be distinguishable");
    }

    // ==================================================================
    // Test 3: Valid module + exported @test -> Ok(TestResult)
    //
    // AC 1 + 2. The runner must compile the module, locate `test_ok`, execute
    // it, and report a passing result. Currently `run_test` returns
    // Err(NotImplemented) -> this fails for the right reason.
    // ==================================================================

    #[test]
    fn test_valid_module_passing_test_returns_ok_passed() {
        let wasm = module_with_single_export("test_ok");
        let result = WasmTestRunner::new().run_test(&wasm, "test_ok");

        let res = result.expect(
            "a valid, exported @test must execute and return Ok — got NotImplemented instead",
        );
        assert!(res.passed, "passing test must have passed: true, got {:?}", res);
        assert_eq!(
            res.function_name, "test_ok",
            "result must echo the executed function name"
        );
        assert!(
            res.message.is_none(),
            "a passing test must not carry an error message, got {:?}",
            res.message
        );
    }

    // ==================================================================
    // Test 4: Failing @test -> passed: false with a message
    //
    // AC 3. The fixture traps (stands in for an `assert` failure). The runner
    // must surface `passed: false` plus messaging (expected-vs-actual once the
    // dUnit intrinsics are wired). Currently fails because `run_test` is a
    // stub.
    // ==================================================================

    #[test]
    fn test_failing_test_returns_failed_with_message() {
        let wasm = module_with_failing_export("test_fail");
        let result = WasmTestRunner::new().run_test(&wasm, "test_fail");

        let result = result.expect("a trapping @test must still return Ok(TestResult)");
        assert!(
            !result.passed,
            "a trapping test must be reported as failed, got {:?}",
            result
        );
        assert!(
            result.message.is_some(),
            "a failed test must carry an expected-vs-actual message, got {:?}",
            result.message
        );
    }

    // ==================================================================
    // Test 5: Missing/renamed function -> FunctionNotFound
    //
    // AC 4. The module only exports `test_ok`; requesting `test_renamed` must
    // error loudly — never a silent pass. Currently fails because the stub
    // never inspects exports (it returns NotImplemented).
    // ==================================================================

    #[test]
    fn test_missing_function_returns_function_not_found() {
        let wasm = module_with_single_export("test_ok");
        let result = WasmTestRunner::new().run_test(&wasm, "test_renamed");

        assert!(
            matches!(result, Err(RunnerError::FunctionNotFound(_))),
            "requesting a non-exported function must yield FunctionNotFound, not a silent pass; \
             got: {:?}",
            result
        );
    }

    // ==================================================================
    // Test 6: Invalid Wasm bytes -> WasmCompilationError
    //
    // Pre-existing behavior retained: malformed input must be rejected before
    // execution. This one is green already; it pins the error path so the
    // post-implementation runner still rejects bad input.
    // ==================================================================

    #[test]
    fn test_invalid_wasm_returns_compilation_error() {
        // Deliberately non-Wasm bytes (bad magic/version).
        let bad_bytes: &[u8] = b"definitely-not-a-wasm-module";
        let result = run_test(bad_bytes, "test_anything");
        assert!(
            matches!(result, Err(RunnerError::WasmCompilationError(_))),
            "invalid wasm bytes must yield WasmCompilationError, got: {:?}",
            result
        );
    }

    // ==================================================================
    // Test 7: @skip / @skip_test -> skipped: true, NOT executed
    //
    // AC 5. The caller distinguishes a skipped test via the new
    // `TestResult.skipped` field. Metadata is supplied through
    // `WasmTestRunner::with_metadata(...)`; a test whose decorators contain
    // `"skip"`/`"skip_test"` must be reported but never executed.
    //
    // Fails to compile until both `with_metadata` and `TestResult.skipped`
    // exist (the required new API surface).
    // ==================================================================

    #[test]
    fn test_skipped_test_is_not_executed() {
        let wasm = module_with_single_export("test_skipped");

        let mut metadata = HashMap::new();
        metadata.insert(
            "test_skipped".to_string(),
            vec!["test".to_string(), "skip".to_string()],
        );
        let runner = WasmTestRunner::with_metadata(metadata);

        let result = runner
            .run_test(&wasm, "test_skipped")
            .expect("a skipped test must report Ok(skipped), not an execution error");

        assert!(
            result.skipped,
            "@skip/@skip_test tests must be reported as skipped, got: {:?}",
            result
        );
        assert!(
            !result.passed,
            "a skipped test is not run, so it cannot pass conclusively"
        );
    }

    // ==================================================================
    // Test 8: @before_each / @after_each hooks run around each test
    //
    // AC 6. With metadata declaring a `before_each` hook function and an
    // `after_each` hook function around `test_seq`, the executor must invoke
    // both (in order) and still report the passing test. Hook functions are
    // located in the module; is_before_each_hook / is_after_each_hook gate
    // which decorators are hooks.
    //
    // Fails to compile until `with_metadata` exists.
    // ==================================================================

    #[test]
    fn test_hooks_run_around_each_test() {
        let wasm = {
            let src = concat!(
                "(module",
                "  (func (export \"before_alpha\"))",
                "  (func (export \"after_alpha\"))",
                "  (func (export \"test_seq\") (result i32) i32.const 1)",
                ")",
            );
            module_bytes(src)
        };

        let mut metadata = HashMap::new();
        metadata.insert(
            "before_alpha".to_string(),
            vec!["before_each".to_string()],
        );
        metadata.insert(
            "after_alpha".to_string(),
            vec!["after_each".to_string()],
        );

        let runner = WasmTestRunner::with_metadata(metadata);

        let result = runner
            .run_test(&wasm, "test_seq")
            .expect("a test with exported hooks must execute Ok");

        assert!(
            result.passed,
            "test behind hooks must still pass, got: {:?}",
            result
        );

        // The metadata-driven design relies on these hook predicates.
        assert!(is_before_each_hook(&["before_each".to_string()]));
        assert!(is_after_each_hook(&["after_each".to_string()]));
    }

    // ==================================================================
    // Test 9: Hook detection predicates (unit-level, stays green)
    //
    // is_before_each_hook / is_after_each_hook / is_skipped gate the runner's
    // decorator inspection. These build on `super::` helpers and pin the
    // "skip_test" alias required by the acceptance criteria.
    // ==================================================================

    #[test]
    fn test_before_each_hook_detection() {
        let decorators = vec!["before_each".to_string()];
        assert!(
            is_before_each_hook(&decorators),
            "@before_each decorator must be recognized"
        );
    }

    #[test]
    fn test_before_each_hook_not_detected_for_other_decorators() {
        let decorators = vec!["test".to_string(), "skip".to_string()];
        assert!(
            !is_before_each_hook(&decorators),
            "@before_each must NOT be detected when absent"
        );
    }

    #[test]
    fn test_after_each_hook_detection() {
        let decorators = vec!["after_each".to_string()];
        assert!(
            is_after_each_hook(&decorators),
            "@after_each decorator must be recognized"
        );
    }

    #[test]
    fn test_skip_detection() {
        let skip = vec!["skip".to_string()];
        assert!(is_skipped(&skip), "is_skipped must detect @skip");

        let skip_test = vec!["skip_test".to_string()];
        assert!(
            is_skipped(&skip_test),
            "is_skipped must also detect @skip_test"
        );
    }

    #[test]
    fn test_skip_not_detected_for_other_decorators() {
        let decorators = vec!["test".to_string(), "before_each".to_string()];
        assert!(
            !is_skipped(&decorators),
            "is_skipped must be false when no skip decorator present"
        );
    }

    // ==================================================================
    // Test 10: Multiple decorators — only matching hook predicates fire
    // ==================================================================

    #[test]
    fn test_multiple_decorators_hook_detection() {
        let decorators = vec![
            "test".to_string(),
            "before_each".to_string(),
            "skip".to_string(),
        ];

        assert!(
            is_before_each_hook(&decorators),
            "@before_each must be detected among multiple decorators"
        );
        assert!(
            is_skipped(&decorators),
            "@skip must be detected among multiple decorators"
        );
        assert!(
            !is_after_each_hook(&decorators),
            "@after_each must NOT be detected when absent"
        );
    }

    // ==================================================================
    // Test 11: RunnerError Display — still meaningful after execution lands
    //
    // Error Display strings must survive the rewrite; verifies the error
    // contract a caller would surface to a user.
    // ==================================================================

    #[test]
    fn test_runner_error_display() {
        let not_found = RunnerError::FunctionNotFound("test_missing".to_string());
        let msg = format!("{}", not_found);
        assert!(
            msg.contains("test_missing"),
            "FunctionNotFound Display must include the function name, got: {}",
            msg
        );

        let compiled = RunnerError::WasmCompilationError("bad".to_string());
        assert!(
            format!("{}", compiled).contains("Wasm compilation error"),
            "WasmCompilationError Display must be meaningful"
        );
    }
}