//! Structured JSON output module for the Dwarf CLI.
//!
//! Provides shared types and utilities for JSON-serializable output
//! across all subcommands (`check`, `emit`, `build`, `test`).
//!
//! # Structured JSON Output
//!
//! The shared envelope wraps command-specific payloads:
//!
//! ```json
//! {
//!   "version": "1.0.0",
//!   "duration_ms": 123,
//!   "command": "check",
//!   "payload": { ... }
//! }
//! ```
//!
//! # Red Phase (TDD)
//!
//! This module contains stub/skeleton implementations that compile but
//! deliberately fail the tests below. The Green phase will replace these
//! stubs with correct implementations.

use serde::{Deserialize, Serialize};
use serde::Serializer;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Stub: version constant — intentionally wrong for Red phase
// ---------------------------------------------------------------------------

/// Version of the JSON output format.
///
// **Stub:** returns `"0.0.0"` instead of `"1.0.0"` to force Red-phase failure.
pub const OUTPUT_VERSION: &str = "1.0.0";

// ---------------------------------------------------------------------------
// Shared envelope
// ---------------------------------------------------------------------------

/// Shared JSON envelope wrapping all command outputs.
///
/// Every subcommand's JSON output is wrapped in this envelope, which provides
/// a consistent `version`, `duration_ms`, `command`, and `payload` structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutputEnvelope<T: Serialize> {
    pub version: String,
    pub duration_ms: u64,
    pub command: String,
    pub payload: T,
}

impl<T: Serialize> OutputEnvelope<T> {
    /// Create a new output envelope.
    ///
    // **Stub:** version is set to `OUTPUT_VERSION` (wrong), `duration_ms` is 0.
    #[allow(dead_code)]
    pub fn new(command: &str, payload: T) -> Self {
        Self {
            version: OUTPUT_VERSION.to_string(),
            duration_ms: 0,
            command: command.to_string(),
            payload,
        }
    }

    /// Create an envelope with duration computed from a start time.
    pub fn from_start(command: &str, payload: T, start: Instant) -> Self {
        Self {
            version: OUTPUT_VERSION.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            command: command.to_string(),
            payload,
        }
    }
}

// ---------------------------------------------------------------------------
// Command-specific payload types
// ---------------------------------------------------------------------------

/// Payload for the `check` subcommand.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckPayload {
    pub files: Vec<FileCheckResult>,
}

/// Result for a single file in `check` output.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileCheckResult {
    pub file: String,
    pub success: bool,
    pub errors: Vec<StructuredDiagnostic>,
}

/// Payload for the `emit` subcommand.
#[derive(Debug, Serialize, Deserialize)]
pub struct EmitPayload {
    pub files: Vec<FileEmitResult>,
}

/// Result for a single file and target in `emit` output.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEmitResult {
    pub file: String,
    pub target: String,
    pub success: bool,
    pub output: String,
    pub extension: String,
    pub errors: Vec<StructuredDiagnostic>,
}

/// Payload for the `build` subcommand.
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildPayload {
    pub files: Vec<FileBuildResult>,
}

/// Result for a single file and target in `build` output.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileBuildResult {
    pub file: String,
    pub target: String,
    pub success: bool,
    pub output_path: String,
    pub errors: Vec<StructuredDiagnostic>,
}

/// Payload for the `test` subcommand.
#[derive(Debug, Serialize, Deserialize)]
pub struct TestPayload {
    pub ok: bool,
    pub results: Vec<TestResultItem>,
}

/// Result for a single test case.
#[derive(Debug, Serialize, Deserialize)]
pub struct TestResultItem {
    pub file: String,
    pub passed: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Structured diagnostic
// ---------------------------------------------------------------------------

/// A structured diagnostic with consistent format across all subcommands.
///
/// ```json
/// {
///   "code": "DWARF-E-TYPE-0001",
///   "severity": "error",
///   "message": "Type mismatch: expected Int, got String",
///   "file": "input.kzd",
///   "line": 42,
///   "col": 10,
///   "related": [],
///   "fix": null
/// }
/// ```
/// Serialize severity as lowercase.
fn serialize_severity<S>(severity: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&severity.to_lowercase())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructuredDiagnostic {
    pub code: String,
    #[serde(serialize_with = "serialize_severity")]
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub related: Vec<RelatedLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// A related source location attached to a diagnostic.
#[derive(Debug, Serialize, Deserialize)]
pub struct RelatedLocation {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Output format handler
// ---------------------------------------------------------------------------

/// Controls whether output is human-readable TTY text or structured JSON.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    #[allow(dead_code)]
    Tty,
    Json,
}

/// Produce formatted output from an envelope.
pub fn format_output<T: Serialize + std::fmt::Debug>(
    format: OutputFormat,
    envelope: &OutputEnvelope<T>,
) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(envelope).unwrap(),
        OutputFormat::Tty => format!(
            "Command: {}\nDuration: {}ms\n\n{:?}",
            envelope.command, envelope.duration_ms, envelope.payload
        ),
    }
}

// ---------------------------------------------------------------------------
// Tests (Red Phase — expected to fail)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use std::time::Duration;

    // ===================================================================
    // 1. Shared JSON Wrapper Structure
    // ===================================================================

    #[test]
    fn test_output_envelope_struct_exists() {
        let payload = serde_json::json!({"key": "value"});
        let envelope = OutputEnvelope::new("check", payload);

        // FAIL (Red): version is "0.0.0", not "1.0.0"
        assert_eq!(envelope.version, "1.0.0", "Version should be 1.0.0");
        assert_eq!(envelope.command, "check");
        assert_eq!(envelope.duration_ms, 0);
    }

    #[test]
    fn test_output_envelope_serializes_correctly() {
        let payload = serde_json::json!({"key": "value"});
        let envelope = OutputEnvelope::new("check", payload);
        let json = serde_json::to_value(&envelope).unwrap();

        // FAIL (Red): version is "0.0.0" in JSON
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["command"], "check");
        assert_eq!(json["duration_ms"], 0);
        assert!(json.get("payload").is_some());
    }

    #[test]
    fn test_output_envelope_duration_computed_from_start() {
        let start = Instant::now();
        // Simulate measurable work
        std::thread::sleep(Duration::from_millis(5));
        let payload = serde_json::json!({"result": "ok"});
        let envelope = OutputEnvelope::from_start("check", payload, start);

        // FAIL (Red): duration_ms is 0 (stub), not >= 5
        assert!(
            envelope.duration_ms >= 5,
            "duration_ms should be at least 5ms, got {}",
            envelope.duration_ms
        );
    }

    #[test]
    fn test_output_envelope_version_is_string_constant() {
        // FAIL (Red): OUTPUT_VERSION is "0.0.0"
        assert_eq!(OUTPUT_VERSION, "1.0.0");
    }

    // ===================================================================
    // 2. Command-Specific Payloads
    // ===================================================================

    #[test]
    fn test_check_payload_serialization() {
        let diag = StructuredDiagnostic {
            code: "E001".into(),
            severity: "error".into(),
            message: "test error".into(),
            file: "test.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let result = FileCheckResult {
            file: "test.kzd".into(),
            success: false,
            errors: vec![diag],
        };
        let payload = CheckPayload {
            files: vec![result],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["files"][0]["file"], "test.kzd");
        assert!(!json["files"][0]["success"].as_bool().unwrap());
        assert_eq!(json["files"][0]["errors"][0]["code"], "E001");
    }

    #[test]
    fn test_emit_payload_serialization() {
        let result = FileEmitResult {
            file: "test.kzd".into(),
            target: "ts".into(),
            success: true,
            output: "console.log('hello')".into(),
            extension: "ts".into(),
            errors: vec![],
        };
        let payload = EmitPayload {
            files: vec![result],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["files"][0]["target"], "ts");
        assert_eq!(json["files"][0]["output"], "console.log('hello')");
        assert_eq!(json["files"][0]["extension"], "ts");
    }

    #[test]
    fn test_build_payload_serialization() {
        let result = FileBuildResult {
            file: "test.kzd".into(),
            target: "ts".into(),
            success: true,
            output_path: "dist/ts/test.ts".into(),
            errors: vec![],
        };
        let payload = BuildPayload {
            files: vec![result],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["files"][0]["output_path"], "dist/ts/test.ts");
    }

    #[test]
    fn test_test_payload_serialization() {
        let item = TestResultItem {
            file: "test.kzd".into(),
            passed: true,
            message: "All tests passed".into(),
        };
        let payload = TestPayload {
            ok: true,
            results: vec![item],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(json["ok"].as_bool().unwrap());
        assert_eq!(json["results"][0]["file"], "test.kzd");
        assert!(json["results"][0]["passed"].as_bool().unwrap());
    }

    // ===================================================================
    // 3. Structured Diagnostic Format
    // ===================================================================

    #[test]
    fn test_structured_diagnostic_all_fields() {
        let diag = StructuredDiagnostic {
            code: "DWARF-E-TYPE-0001".into(),
            severity: "error".into(),
            message: "Type mismatch: expected Int, got String".into(),
            file: "input.kzd".into(),
            line: 42,
            col: 10,
            related: vec![],
            fix: None,
        };
        assert_eq!(diag.code, "DWARF-E-TYPE-0001");
        assert_eq!(diag.severity, "error");
        assert_eq!(diag.message, "Type mismatch: expected Int, got String");
        assert_eq!(diag.file, "input.kzd");
        assert_eq!(diag.line, 42);
        assert_eq!(diag.col, 10);
        assert!(diag.related.is_empty());
        assert!(diag.fix.is_none());
    }

    #[test]
    fn test_structured_diagnostic_severity_error() {
        let diag = StructuredDiagnostic {
            code: "DWARF-E-TYPE-0001".into(),
            severity: "error".into(),
            message: "test".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["severity"], "error");
    }

    #[test]
    fn test_structured_diagnostic_severity_warning() {
        let diag = StructuredDiagnostic {
            code: "W001".into(),
            // FAIL (Red): uppercase "WARNING" — should serialize as lowercase "warning"
            severity: "WARNING".into(),
            message: "test".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["severity"], "warning");
    }

    #[test]
    fn test_structured_diagnostic_severity_info() {
        let diag = StructuredDiagnostic {
            code: "I001".into(),
            severity: "INFO".into(),
            message: "test".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["severity"], "info");
    }

    #[test]
    fn test_structured_diagnostic_with_related() {
        let related = vec![RelatedLocation {
            file: "other.kzd".into(),
            line: 10,
            col: 5,
            message: "previously defined here".into(),
        }];
        let diag = StructuredDiagnostic {
            code: "E002".into(),
            severity: "error".into(),
            message: "duplicate definition".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related,
            fix: None,
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert_eq!(json["related"][0]["file"], "other.kzd");
        assert_eq!(json["related"][0]["line"], 10);
        assert_eq!(json["related"][0]["message"], "previously defined here");
    }

    #[test]
    fn test_structured_diagnostic_with_fix() {
        let diff = [
            "--- a/file.kzd",
            "+++ b/file.kzd",
            "@@ -1 +1 @@",
            "-foo",
            "+bar",
        ]
        .join("\n");
        let diag = StructuredDiagnostic {
            code: "E003".into(),
            severity: "error".into(),
            message: "incorrect value".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: Some(diff),
        };
        let json = serde_json::to_value(&diag).unwrap();
        assert!(json["fix"].is_string());
        assert!(json["fix"].as_str().unwrap().contains("+bar"));
    }

    #[test]
    fn test_structured_diagnostic_fix_is_optional() {
        let diag = StructuredDiagnostic {
            code: "E004".into(),
            severity: "error".into(),
            message: "no fix available".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let json = serde_json::to_value(&diag).unwrap();
        // When fix is None, the field should not appear (skip_serializing_if)
        // or be null. Either is acceptable for the API.
        if json.get("fix").is_some() {
            assert!(json["fix"].is_null());
        }
    }

    // ===================================================================
    // 4. Output Format Handler (TTY vs JSON)
    // ===================================================================

    #[test]
    fn test_output_format_enum_variants() {
        let tty = OutputFormat::Tty;
        let json = OutputFormat::Json;
        assert_ne!(tty, json);
    }

    #[test]
    fn test_format_output_json_mode_returns_valid_json() {
        let payload = serde_json::json!({"ok": true});
        let envelope = OutputEnvelope::new("test", payload);
        let output = format_output(OutputFormat::Json, &envelope);

        // FAIL (Red): output is empty string, not valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&output);
        assert!(
            parsed.is_ok(),
            "JSON mode should produce valid JSON, got: {:?}",
            parsed.err()
        );
    }

    #[test]
    fn test_format_output_json_mode_contains_envelope() {
        let payload = serde_json::json!({"ok": true});
        let envelope = OutputEnvelope::new("test", payload);
        let output = format_output(OutputFormat::Json, &envelope);

        // FAIL (Red): output is empty string, won't parse as JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("duration_ms").is_some());
        assert!(parsed.get("command").is_some());
        assert!(parsed.get("payload").is_some());
    }

    #[test]
    fn test_format_output_tty_mode_returns_text() {
        let payload = serde_json::json!({"ok": true});
        let envelope = OutputEnvelope::new("test", payload);
        let output = format_output(OutputFormat::Tty, &envelope);

        // FAIL (Red): output is empty string
        assert!(
            !output.is_empty(),
            "TTY mode should produce non-empty text"
        );
    }

    // ===================================================================
    // 5. Build Command JSON Flag
    // ===================================================================

    #[test]
    fn test_build_cli_accepts_json_flag() {
        // FAIL (Red): Build subcommand in main.rs does not have a --json flag yet.
        let cmd = crate::Cli::command();
        let matches = cmd.try_get_matches_from([
            "dwarf-cli",
            "build",
            "file.kzd",
            "--target",
            "ts",
            "--json",
        ]);
        assert!(
            matches.is_ok(),
            "Build subcommand should accept --json flag, but got: {:?}",
            matches.err()
        );
        if let Ok(ref m) = matches {
            let (_subcommand, sub_m) = m.subcommand().unwrap();
            assert!(sub_m.get_flag("json"));
        }
    }

    // ===================================================================
    // 6. Error Output Format
    // ===================================================================

    #[test]
    fn test_error_output_has_code_location_and_message() {
        let diag = StructuredDiagnostic {
            code: "DWARF-E-TYPE-0001".into(),
            severity: "error".into(),
            message: "Type mismatch: expected Int, got String".into(),
            file: "input.kzd".into(),
            line: 42,
            col: 10,
            related: vec![],
            fix: None,
        };
        assert_eq!(diag.code, "DWARF-E-TYPE-0001");
        assert_eq!(diag.file, "input.kzd");
        assert_eq!(diag.line, 42);
        assert_eq!(diag.col, 10);
    }

    #[test]
    fn test_multiple_errors_all_included() {
        let diag1 = StructuredDiagnostic {
            code: "E001".into(),
            severity: "error".into(),
            message: "first error".into(),
            file: "f.kzd".into(),
            line: 1,
            col: 1,
            related: vec![],
            fix: None,
        };
        let diag2 = StructuredDiagnostic {
            code: "E002".into(),
            severity: "error".into(),
            message: "second error".into(),
            file: "f.kzd".into(),
            line: 5,
            col: 3,
            related: vec![],
            fix: None,
        };
        let errors = vec![diag1, diag2];
        assert_eq!(errors.len(), 2);

        let json = serde_json::to_value(&errors).unwrap();
        assert_eq!(json[0]["code"], "E001");
        assert_eq!(json[0]["message"], "first error");
        assert_eq!(json[1]["code"], "E002");
        assert_eq!(json[1]["message"], "second error");
    }

    #[test]
    fn test_empty_diagnostics_produces_empty_errors_array() {
        let errors: Vec<StructuredDiagnostic> = vec![];
        let json = serde_json::to_value(&errors).unwrap();
        assert!(json.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_check_result_with_empty_errors() {
        let result = FileCheckResult {
            file: "clean.kzd".into(),
            success: true,
            errors: vec![],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["file"], "clean.kzd");
        assert!(json["success"].as_bool().unwrap());
        assert!(json["errors"].as_array().unwrap().is_empty());
    }
}
