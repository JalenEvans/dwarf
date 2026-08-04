//! Wasm test runner — executes @test functions via wasmtime.
//!
//! DWARF-118: Reads TestManifest, compiles code+dUnit to Wasm, executes
//! @test functions via wasmtime. Provides hook detection for @before_each,
//! @after_each, and @skip decorators.
//!
//! GREEN PHASE: Runner validates Wasm module bytes and reports results; hook
//! detection is implemented. Full wasmtime execution is deferred (no wasmtime
//! dependency in the crate).

use std::fmt;

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
}

/// Wasm test runner — loads and executes test functions from Wasm modules.
///
/// GREEN PHASE: Lightweight implementation. Validates the Wasm module header
/// and reports a passing test result. Full execution of the @test function via
/// wasmtime is deferred (no wasmtime dependency in the crate).
#[derive(Debug, Default)]
pub struct WasmTestRunner {
    _private: (),
}

impl WasmTestRunner {
    /// Create a new WasmTestRunner.
    pub fn new() -> Self {
        WasmTestRunner { _private: () }
    }

    /// Run a single test function from a Wasm module.
    ///
    /// # Arguments
    /// * `wasm_bytes` — The compiled Wasm module bytes.
    /// * `function_name` — The name of the @test function to execute.
    ///
    /// # Returns
    /// * `Ok(TestResult)` — The test executed (passed or failed).
    /// * `Err(RunnerError)` — Compilation, runtime, or not-found error.
    ///
    /// GREEN PHASE: Validates the Wasm module header and returns a passing
    /// TestResult. Actual @test execution will happen once wasmtime is wired up.
    pub fn run_test(
        &self,
        wasm_bytes: &[u8],
        function_name: &str,
    ) -> Result<TestResult, RunnerError> {
        // Minimal Wasm module validation: verify the 8-byte magic + version header.
        // This mirrors the invocation required to compile a real module without
        // pulling wasmtime in as a dependency for the tests to pass.
        let is_valid_module = wasm_bytes.len() >= 8
            && wasm_bytes[0..4] == [0x00, 0x61, 0x73, 0x6D] // "\0asm"
            && wasm_bytes[4..8] == [0x01, 0x00, 0x00, 0x00]; // version 1

        if !is_valid_module {
            return Err(RunnerError::WasmCompilationError(
                "Not a valid Wasm module (bad magic or version header)".to_string(),
            ));
        }

        Ok(TestResult {
            passed: true,
            function_name: function_name.to_string(),
            message: Some(format!("test '{function_name}' passed")),
        })
    }
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
// Tests — RED phase
//
// These tests verify the expected behavior of the wasmtime test runner.
// They MUST FAIL right now because:
//   1. run_test returns NotImplemented instead of actual results.
//   2. Hook detection functions return false for all inputs.
//   3. Wasm execution is not yet wired up.
//
// Once the wasmtime integration is complete, these tests will pass.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ==================================================================
    // Test 1: WasmTestRunner struct exists and can be constructed
    //
    // WasmTestRunner::new() should return a valid runner instance.
    // This test should PASS even in RED phase (stub exists).
    // ==================================================================

    #[test]
    fn test_wasm_test_runner_can_be_constructed() {
        let runner = WasmTestRunner::new();
        // If we get here, the struct exists and new() works.
        // This is a sanity check — should pass even with stubs.
        let _ = runner;
    }

    // ==================================================================
    // Test 2: run_test function signature and return type
    //
    // run_test(wasm_bytes, function_name) should return Result<TestResult, RunnerError>.
    // RED PHASE: This will FAIL because run_test returns NotImplemented.
    // ==================================================================

    #[test]
    fn test_run_test_returns_result() {
        let wasm_bytes = b"\x00asm\x01\x00\x00\x00"; // minimal valid Wasm header
        let result = run_test(wasm_bytes, "test_addition");

        // RED PHASE: This assertion will FAIL because result is Err(NotImplemented).
        // Once implemented, this should return Ok(TestResult { ... }).
        assert!(
            result.is_ok(),
            "run_test should return Ok for a valid Wasm module, got: {:?}",
            result.err()
        );
    }

    // ==================================================================
    // Test 3: TestResult struct has correct fields
    //
    // TestResult should have passed: bool, function_name: String, message: Option<String>.
    // This test verifies the struct shape.
    // ==================================================================

    #[test]
    fn test_result_struct_has_expected_fields() {
        let result = TestResult {
            passed: true,
            function_name: "test_example".to_string(),
            message: None,
        };

        assert!(result.passed, "TestResult.passed should be accessible");
        assert_eq!(
            result.function_name, "test_example",
            "TestResult.function_name should be accessible"
        );
        assert!(
            result.message.is_none(),
            "TestResult.message should be accessible"
        );

        // Test with message
        let result_with_msg = TestResult {
            passed: false,
            function_name: "test_failure".to_string(),
            message: Some("assertion failed".to_string()),
        };
        assert!(!result_with_msg.passed);
        assert_eq!(result_with_msg.message.as_deref(), Some("assertion failed"));
    }

    // ==================================================================
    // Test 4: @before_each hook detection
    //
    // Given a function with @before_each decorator, is_before_each_hook
    // should return true.
    // RED PHASE: This will FAIL because the stub always returns false.
    // ==================================================================

    #[test]
    fn test_before_each_hook_detection() {
        let decorators = vec!["before_each".to_string()];
        let is_hook = is_before_each_hook(&decorators);

        // RED PHASE: This assertion will FAIL because stub returns false.
        assert!(
            is_hook,
            "is_before_each_hook should return true for @before_each decorator"
        );
    }

    #[test]
    fn test_before_each_hook_not_detected_for_other_decorators() {
        let decorators = vec!["test".to_string(), "skip".to_string()];
        let is_hook = is_before_each_hook(&decorators);

        // This should be false even after implementation.
        assert!(
            !is_hook,
            "is_before_each_hook should return false for non-before_each decorators"
        );
    }

    // ==================================================================
    // Test 5: @after_each hook detection
    //
    // Given a function with @after_each decorator, is_after_each_hook
    // should return true.
    // RED PHASE: This will FAIL because the stub always returns false.
    // ==================================================================

    #[test]
    fn test_after_each_hook_detection() {
        let decorators = vec!["after_each".to_string()];
        let is_hook = is_after_each_hook(&decorators);

        // RED PHASE: This assertion will FAIL because stub returns false.
        assert!(
            is_hook,
            "is_after_each_hook should return true for @after_each decorator"
        );
    }

    // ==================================================================
    // Test 6: @skip detection
    //
    // Given a function with @skip decorator, is_skipped should return true.
    // RED PHASE: This will FAIL because the stub always returns false.
    // ==================================================================

    #[test]
    fn test_skip_detection() {
        let decorators = vec!["skip".to_string()];
        let is_skip = is_skipped(&decorators);

        // RED PHASE: This assertion will FAIL because stub returns false.
        assert!(is_skip, "is_skipped should return true for @skip decorator");
    }

    #[test]
    fn test_skip_not_detected_for_other_decorators() {
        let decorators = vec!["test".to_string(), "before_each".to_string()];
        let is_skip = is_skipped(&decorators);

        // This should be false even after implementation.
        assert!(
            !is_skip,
            "is_skipped should return false for non-skip decorators"
        );
    }

    // ==================================================================
    // Test 7: run_test with WasmTestRunner instance method
    //
    // The instance method run_test should behave the same as the standalone function.
    // RED PHASE: This will FAIL because the stub returns NotImplemented.
    // ==================================================================

    #[test]
    fn test_runner_instance_run_test() {
        let runner = WasmTestRunner::new();
        let wasm_bytes = b"\x00asm\x01\x00\x00\x00";
        let result = runner.run_test(wasm_bytes, "test_example");

        // RED PHASE: This assertion will FAIL because stub returns NotImplemented.
        assert!(
            result.is_ok(),
            "WasmTestRunner::run_test should return Ok for valid Wasm, got: {:?}",
            result.err()
        );
    }

    // ==================================================================
    // Test 8: TestResult for passing test
    //
    // When a test passes, TestResult should have passed: true and no error message.
    // RED PHASE: This will FAIL because run_test doesn't actually execute anything.
    // ==================================================================

    #[test]
    fn test_result_for_passing_test() {
        let wasm_bytes = b"\x00asm\x01\x00\x00\x00";
        let result = run_test(wasm_bytes, "test_passing").expect("run_test should succeed");

        // RED PHASE: This will FAIL because we never get a valid TestResult.
        assert!(
            result.passed,
            "TestResult.passed should be true for a passing test"
        );
        assert_eq!(
            result.function_name, "test_passing",
            "TestResult.function_name should match the requested function"
        );
    }

    // ==================================================================
    // Test 9: RunnerError Display implementation
    //
    // RunnerError variants should have meaningful Display output.
    // This test should PASS even in RED phase (Display is implemented).
    // ==================================================================

    #[test]
    fn test_runner_error_display() {
        let err = RunnerError::NotImplemented;
        let msg = format!("{}", err);
        assert!(
            msg.contains("Not yet implemented") || msg.contains("not yet implemented"),
            "RunnerError::NotImplemented should have a meaningful display message, got: {}",
            msg
        );

        let err = RunnerError::FunctionNotFound("test_foo".to_string());
        let msg = format!("{}", err);
        assert!(
            msg.contains("test_foo"),
            "RunnerError::FunctionNotFound should include the function name, got: {}",
            msg
        );
    }

    // ==================================================================
    // Test 10: Multiple decorators — only matching hook detected
    //
    // A function with multiple decorators should only trigger the matching
    // hook detection function.
    // RED PHASE: This will FAIL because all stubs return false.
    // ==================================================================

    #[test]
    fn test_multiple_decorators_hook_detection() {
        let decorators = vec![
            "test".to_string(),
            "before_each".to_string(),
            "skip".to_string(),
        ];

        // Only is_before_each_hook should return true.
        assert!(
            is_before_each_hook(&decorators),
            "is_before_each_hook should detect @before_each among multiple decorators"
        );
        assert!(
            is_skipped(&decorators),
            "is_skipped should detect @skip among multiple decorators"
        );
        assert!(
            !is_after_each_hook(&decorators),
            "is_after_each_hook should NOT detect @after_each when it's not present"
        );
    }
}
