//! Utility functions for converting identifier naming conventions.
//!
//! Supports conversion between camelCase, PascalCase, snake_case, and
//! kebab-case, as well as escaping reserved words.

/// Convert `snake_case` or `kebab-case` to `camelCase`.
///
/// # Examples
///
/// ```
/// # use dwarf_emitter::naming::to_camel_case;
/// assert_eq!(to_camel_case("my_variable_name"), "myVariableName");
/// assert_eq!(to_camel_case("my-variable-name"), "myVariableName");
/// assert_eq!(to_camel_case("alreadyCamel"), "alreadyCamel");
/// ```
pub fn to_camel_case(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    // Count and preserve leading underscores
    let leading_underscores = name.chars().take_while(|c| *c == '_').count();
    let rest = &name[leading_underscores..];

    if rest.is_empty() {
        return name.to_string();
    }

    // If no separators, it's already camelCase or a single word — return as-is
    if !rest.contains('_') && !rest.contains('-') {
        return name.to_string();
    }

    // Split on _ and -, filtering empty segments
    let segments: Vec<&str> = rest.split(['_', '-']).filter(|s| !s.is_empty()).collect();

    if segments.is_empty() {
        return name.to_string();
    }

    let mut result = String::new();
    // First segment: lowercase
    for (i, ch) in segments[0].char_indices() {
        if i == 0 {
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    // Subsequent segments: capitalize first letter, lowercase the rest
    for segment in &segments[1..] {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            for ch in chars {
                result.push(ch.to_ascii_lowercase());
            }
        }
    }

    // Prepend leading underscores
    let mut final_result = "_".repeat(leading_underscores);
    final_result.push_str(&result);
    final_result
}

/// Convert `snake_case`, `kebab-case`, or `camelCase` to `PascalCase`.
///
/// # Examples
///
/// ```
/// # use dwarf_emitter::naming::to_pascal_case;
/// assert_eq!(to_pascal_case("my_variable"), "MyVariable");
/// assert_eq!(to_pascal_case("my-variable"), "MyVariable");
/// assert_eq!(to_pascal_case("myVariable"), "MyVariable");
/// ```
pub fn to_pascal_case(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    // Count and preserve leading underscores
    let leading_underscores = name.chars().take_while(|c| *c == '_').count();
    let rest = &name[leading_underscores..];

    if rest.is_empty() {
        return name.to_string();
    }

    // If no separators, it's already PascalCase or a single word.
    // When leading underscores are present, preserve the original to avoid
    // incorrectly capitalizing what is meant to be private (e.g. "_private").
    if !rest.contains('_') && !rest.contains('-') {
        if leading_underscores > 0 {
            return name.to_string();
        }
        let mut result = String::new();
        let mut chars = rest.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            result.push_str(chars.as_str());
        }
        return result;
    }

    // Split on _ and -, filtering empty segments
    let segments: Vec<&str> = rest.split(['_', '-']).filter(|s| !s.is_empty()).collect();

    if segments.is_empty() {
        return name.to_string();
    }

    let mut result = "_".repeat(leading_underscores);
    for segment in segments {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            result.push(first.to_ascii_uppercase());
            for ch in chars {
                result.push(ch.to_ascii_lowercase());
            }
        }
    }
    result
}

/// Convert `camelCase` or `PascalCase` to `snake_case`.
///
/// # Examples
///
/// ```
/// # use dwarf_emitter::naming::to_snake_case;
/// assert_eq!(to_snake_case("myVariable"), "my_variable");
/// assert_eq!(to_snake_case("MyVariable"), "my_variable");
/// ```
pub fn to_snake_case(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len() + 4);

    for i in 0..chars.len() {
        let c = chars[i];
        if c.is_ascii_uppercase() {
            // Insert underscore before uppercase in two cases:
            // 1. Previous char is lowercase (camelCase boundary: "helloWorld")
            // 2. Previous char is uppercase but next is lowercase (acronym: "XMLParser")
            if i > 0 {
                let prev = chars[i - 1];
                if (prev.is_ascii_lowercase())
                    || (i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase())
                {
                    result.push('_');
                }
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

/// Escape a name if it matches a reserved word, prefixing with `_`.
///
/// # Examples
///
/// ```
/// # use dwarf_emitter::naming::escape_reserved_word;
/// let reserved = &["class", "if", "else"];
/// assert_eq!(escape_reserved_word("class", reserved), "_class");
/// assert_eq!(escape_reserved_word("foo", reserved), "foo");
/// ```
pub fn escape_reserved_word(name: &str, reserved: &[&str]) -> String {
    if reserved.contains(&name) && !name.starts_with('_') {
        format!("_{}", name)
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // to_camel_case
    // ------------------------------------------------------------------

    #[test]
    fn test_camel_case_snake() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
    }

    #[test]
    fn test_camel_case_kebab() {
        assert_eq!(to_camel_case("hello-world"), "helloWorld");
    }

    #[test]
    fn test_camel_case_already_camel() {
        assert_eq!(to_camel_case("helloWorld"), "helloWorld");
    }

    #[test]
    fn test_camel_case_single_word() {
        assert_eq!(to_camel_case("hello"), "hello");
    }

    #[test]
    fn test_camel_case_empty() {
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn test_camel_case_leading_underscore() {
        assert_eq!(to_camel_case("_private"), "_private");
    }

    #[test]
    fn test_camel_case_multiple_underscores() {
        assert_eq!(to_camel_case("a_b_c_d"), "aBCD");
    }

    // ------------------------------------------------------------------
    // to_pascal_case
    // ------------------------------------------------------------------

    #[test]
    fn test_pascal_case_snake() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
    }

    #[test]
    fn test_pascal_case_kebab() {
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
    }

    #[test]
    fn test_pascal_case_camel() {
        assert_eq!(to_pascal_case("helloWorld"), "HelloWorld");
    }

    #[test]
    fn test_pascal_case_already_pascal() {
        assert_eq!(to_pascal_case("HelloWorld"), "HelloWorld");
    }

    #[test]
    fn test_pascal_case_single_word() {
        assert_eq!(to_pascal_case("hello"), "Hello");
    }

    #[test]
    fn test_pascal_case_empty() {
        assert_eq!(to_pascal_case(""), "");
    }

    #[test]
    fn test_pascal_case_leading_underscore() {
        assert_eq!(to_pascal_case("_private"), "_private");
    }

    // ------------------------------------------------------------------
    // to_snake_case
    // ------------------------------------------------------------------

    #[test]
    fn test_snake_case_camel() {
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
    }

    #[test]
    fn test_snake_case_pascal() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    }

    #[test]
    fn test_snake_case_single_word() {
        assert_eq!(to_snake_case("hello"), "hello");
    }

    #[test]
    fn test_snake_case_already_snake() {
        assert_eq!(to_snake_case("hello_world"), "hello_world");
    }

    #[test]
    fn test_snake_case_empty() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_snake_case_all_caps() {
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
    }

    #[test]
    fn test_snake_case_consecutive_uppercase() {
        assert_eq!(to_snake_case("parseJSONFile"), "parse_json_file");
    }

    // ------------------------------------------------------------------
    // escape_reserved_word
    // ------------------------------------------------------------------

    #[test]
    fn test_escape_reserved_word() {
        let reserved = &["class", "if"];
        assert_eq!(escape_reserved_word("class", reserved), "_class");
    }

    #[test]
    fn test_escape_not_reserved() {
        let reserved = &["class", "if"];
        assert_eq!(escape_reserved_word("foo", reserved), "foo");
    }

    #[test]
    fn test_escape_empty_reserved_list() {
        let reserved: &[&str] = &[];
        assert_eq!(escape_reserved_word("foo", reserved), "foo");
    }

    #[test]
    fn test_escape_empty_word() {
        let reserved = &["class"];
        assert_eq!(escape_reserved_word("", reserved), "");
    }

    #[test]
    fn test_escape_multiple_reserved() {
        let reserved = &["class", "if", "else", "for", "while"];
        assert_eq!(escape_reserved_word("class", reserved), "_class");
        assert_eq!(escape_reserved_word("if", reserved), "_if");
        assert_eq!(escape_reserved_word("else", reserved), "_else");
        assert_eq!(escape_reserved_word("for", reserved), "_for");
        assert_eq!(escape_reserved_word("while", reserved), "_while");
    }

    #[test]
    fn test_escape_already_escaped() {
        let reserved = &["class"];
        assert_eq!(escape_reserved_word("_class", reserved), "_class");
    }
}
