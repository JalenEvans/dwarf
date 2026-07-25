//! Implementation of the `dwarf fmt` subcommand.
//!
//! Reformats `.kzd` source files by normalizing whitespace:
//! - Trims trailing whitespace from lines
//! - Collapses multiple blank lines into one
//! - Ensures exactly one trailing newline
//! - Converts leading tabs to 2 spaces

use std::fs;
use std::path::PathBuf;
use std::process;

pub fn run_fmt(files: Vec<PathBuf>, check: bool, stdout: bool) {
    for file_path in &files {
        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {}: {}", file_path.display(), e);
                process::exit(1);
            }
        };

        let formatted = format_source(&source);

        if source == formatted {
            // No changes needed
            continue;
        }

        if check {
            eprintln!("{}: would reformat", file_path.display());
            process::exit(1);
        } else if stdout {
            print!("{}", formatted);
        } else {
            if let Err(e) = fs::write(file_path, &formatted) {
                eprintln!("Error writing {}: {}", file_path.display(), e);
                process::exit(1);
            }
            println!("Formatted {}", file_path.display());
        }
    }
}

/// Normalize whitespace in Dwarf source code.
pub fn format_source(source: &str) -> String {
    let mut result = String::new();
    let mut blank_count = 0;

    for line in source.lines() {
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            // Normalize leading whitespace: tabs → 2 spaces
            let normalized = normalize_indent(trimmed);
            result.push_str(&normalized);
            result.push('\n');
        }
    }

    // Ensure trailing newline
    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Replace leading tabs with 2 spaces, preserve alignment spaces.
fn normalize_indent(line: &str) -> String {
    let trimmed = line.trim_start();
    let leading = &line[..line.len() - trimmed.len()];
    // Count leading whitespace: each tab → 2 spaces, each space → 1 space
    let spaces: usize = leading.chars().map(|c| if c == '\t' { 2 } else { 1 }).sum();
    format!("{:indent$}{}", "", trimmed, indent = spaces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_source_no_change() {
        let input = "fn main() {\n  return 1;\n}\n";
        let output = format_source(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_format_source_trim_trailing_whitespace() {
        let input = "fn main() {  \n  return 1;   \n}\n";
        let expected = "fn main() {\n  return 1;\n}\n";
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn test_format_source_blank_lines() {
        let input = "fn a() {\n  x();\n\n\n\n  y();\n}\n";
        let expected = "fn a() {\n  x();\n\n  y();\n}\n";
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn test_format_source_trailing_newline() {
        let input = "fn main() {\n  return 1;\n}";
        let expected = "fn main() {\n  return 1;\n}\n";
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn test_format_source_tabs_to_spaces() {
        let input = "fn main() {\n\treturn 1;\n}\n";
        let expected = "fn main() {\n  return 1;\n}\n";
        assert_eq!(format_source(input), expected);
    }

    #[test]
    fn test_format_source_mixed_tabs_and_spaces() {
        let input = "fn main() {\n\t  return 1;\n}\n";
        // \t = 2 spaces, then 2 more spaces = 4 total
        let expected = "fn main() {\n    return 1;\n}\n";
        assert_eq!(format_source(input), expected);
    }
}
