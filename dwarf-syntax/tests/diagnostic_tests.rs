//! Tests for source location utilities and diagnostic formatting.

use dwarf_syntax::diagnostic::{byte_to_line_col, extract_line, format_diagnostic};

#[test]
fn test_byte_to_line_col_returns_correct_pos() {
    let source = "line1\nline2\nline3";
    // Byte 6 is 'l' of "line2"
    let (line, col) = byte_to_line_col(source, 6).unwrap();
    assert_eq!(line, 2);
    assert_eq!(col, 1);
}

#[test]
fn test_byte_to_line_col_first_line() {
    let source = "hello world";
    let (line, col) = byte_to_line_col(source, 0).unwrap();
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn test_byte_to_line_col_out_of_bounds() {
    let source = "hi";
    let result = byte_to_line_col(source, 100);
    assert!(result.is_none());
}

#[test]
fn test_byte_to_line_col_empty_source() {
    let source = "";
    let result = byte_to_line_col(source, 0);
    assert!(result.is_some(), "Empty source at byte 0 should work");
}

#[test]
fn test_source_line_content() {
    let source = "fn main() {\n    42\n}";
    // Get content of line 2
    let line_content = extract_line(source, 2);
    assert_eq!(line_content, "    42");
}

#[test]
fn test_format_diagnostic_integration() {
    let source = "fn main() {\n    42\n}";
    let output = format_diagnostic(
        Some("test.kzd"),
        source,
        "E-PARSE-0001",
        "expected ';'",
        2,
        5,
    );
    assert!(output.contains("E-PARSE-0001"));
    assert!(output.contains("expected ';'"));
    assert!(output.contains("test.kzd:2:5"));
    assert!(output.contains("42"));
    assert!(output.contains("^"));
}
