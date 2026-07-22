//! Source location and diagnostic formatting utilities.

/// Convert a byte offset in source text to (line_number, column_number).
/// Line numbers are 1-based, column numbers are 1-based.
/// Returns `None` if the offset is out of bounds.
pub fn byte_to_line_col(source: &str, offset: usize) -> Option<(usize, usize)> {
    if offset > source.len() {
        return None;
    }

    let mut line = 1;
    let mut last_line_start = 0;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }

    let col = offset - last_line_start + 1;
    Some((line, col))
}

/// Extract the content of a specific line (1-based) from source text.
/// Returns an empty string if the line doesn't exist.
pub fn extract_line(source: &str, line_num: usize) -> String {
    let mut current_line = 1;
    let mut start = 0;

    for (i, ch) in source.char_indices() {
        if current_line == line_num && ch == '\n' {
            return source[start..i].to_string();
        }
        if ch == '\n' {
            current_line += 1;
            start = i + 1;
        }
    }

    // Last line (no trailing newline)
    if current_line == line_num {
        source[start..].to_string()
    } else {
        String::new()
    }
}

/// Format a diagnostic with source context.
/// Returns a formatted string like:
/// ```text
/// error[E-PARSE-0001]: expected ';'
///   --> src/file.kzd:5:10
///    |
///  5 | fn main() { 42 }
///    |          ^
/// ```
pub fn format_diagnostic(
    file: Option<&str>,
    source: &str,
    code: &str,
    message: &str,
    line: usize,
    col: usize,
) -> String {
    let file_info = match file {
        Some(f) => format!(" {}:{}:{}", f, line, col),
        None => format!(" {}:{}", line, col),
    };

    let mut output = String::new();

    if !code.is_empty() {
        output.push_str(&format!("error[{}]: {}\n", code, message));
    } else {
        output.push_str(&format!("error: {}\n", message));
    }
    output.push_str(&format!("  -->{}", file_info));
    output.push('\n');

    let line_content = extract_line(source, line);
    if !line_content.is_empty() {
        output.push_str(&format!("   |\n{:>4} | {}\n", line, line_content));
        output.push_str(&format!("   | {:>width$}", "", width = col.saturating_sub(1)));
        output.push('^');
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_line_col_first_line() {
        let result = byte_to_line_col("hello world", 0);
        assert_eq!(result, Some((1, 1)));
    }

    #[test]
    fn test_byte_to_line_col_second_line() {
        let source = "line1\nline2\nline3";
        // byte 6 is 'l' of "line2"
        let result = byte_to_line_col(source, 6);
        assert_eq!(result, Some((2, 1)));
    }

    #[test]
    fn test_byte_to_line_col_out_of_bounds() {
        assert_eq!(byte_to_line_col("hi", 100), None);
    }

    #[test]
    fn test_byte_to_line_col_empty() {
        assert_eq!(byte_to_line_col("", 0), Some((1, 1)));
    }

    #[test]
    fn test_extract_line_middle() {
        let source = "fn main() {\n    42\n}";
        assert_eq!(extract_line(source, 2), "    42");
    }

    #[test]
    fn test_extract_line_first() {
        let source = "fn main() {\n    42\n}";
        assert_eq!(extract_line(source, 1), "fn main() {");
    }

    #[test]
    fn test_extract_line_last() {
        let source = "fn main() {\n    42\n}";
        assert_eq!(extract_line(source, 3), "}");
    }

    #[test]
    fn test_extract_line_nonexistent() {
        let source = "hi";
        assert_eq!(extract_line(source, 5), "");
    }

    #[test]
    fn test_format_diagnostic() {
        let source = "fn main() {\n    42\n}";
        let output = format_diagnostic(Some("test.kzd"), source, "E-PARSE-0001", "expected ';'", 2, 5);
        assert!(output.contains("E-PARSE-0001"));
        assert!(output.contains("expected ';'"));
        assert!(output.contains("test.kzd:2:5"));
        assert!(output.contains("42"));
        assert!(output.contains("^"));
    }
}
