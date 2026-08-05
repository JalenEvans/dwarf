//! Decorator parser — converts raw decorator name + args into typed [`Decorator`] variants.
//!
//! The lexer/parser produce raw decorator names (e.g. `"test"`, `"covers"`) and
//! stringified argument lists. This module resolves those into the strongly-typed
//! [`Decorator`] enum defined in [`crate::hir`].

use crate::hir::Decorator;

/// Strip surrounding double-quotes from a string if present (e.g. `"zero"` -> `zero`).
fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

/// Parse a decorator name and its stringified arguments into a typed [`Decorator`].
///
/// # Arguments
/// * `name` — The decorator name without the `@` prefix (e.g. `"test"`, `"covers"`).
/// * `args` — The stringified arguments as produced by the parser.
///
/// # Returns
/// * `Ok(Decorator)` when the name/args match a known decorator variant.
/// * `Err(String)` when the decorator name is unknown or the argument count/types
///   do not match any known variant.
pub fn parse_decorator_name(name: &str, args: &[String]) -> Result<Decorator, String> {
    match name {
        "test" => Ok(Decorator::Test),
        "before_each" => Ok(Decorator::BeforeEach),
        "after_each" => Ok(Decorator::AfterEach),
        "skip" => Ok(Decorator::Skip),
        "gungnir" => Ok(Decorator::Gungnir),
        "skip_test" => {
            let reason = args.first().map(|s| unquote(s)).unwrap_or_default();
            Ok(Decorator::SkipTest { reason })
        }
        "covers" => {
            if args.len() < 3 {
                return Err("covers requires 3 args: fn_name, param, edge_value".to_string());
            }
            Ok(Decorator::Covers {
                fn_name: unquote(&args[0]),
                param: unquote(&args[1]),
                edge_value: unquote(&args[2]),
            })
        }
        "tested" => {
            let fn_name = args.first().map(|s| unquote(s)).unwrap_or_default();
            Ok(Decorator::Tested { fn_name })
        }
        "requires" => {
            let condition = args.first().map(|s| unquote(s)).unwrap_or_default();
            Ok(Decorator::Requires { condition })
        }
        "ensures" => {
            let condition = args.first().map(|s| unquote(s)).unwrap_or_default();
            Ok(Decorator::Ensures { condition })
        }
        "invariant" => {
            let condition = args.first().map(|s| unquote(s)).unwrap_or_default();
            Ok(Decorator::Invariant { condition })
        }
        _ => Err(format!("unknown decorator: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================================================================
    // Tests 1-5: Unit variants (no args)
    //
    // These decorators take no arguments. Calling parse_decorator_name
    // with the correct name and an empty args slice must produce the
    // corresponding unit variant.
    // ==================================================================

    #[test]
    fn test_parse_decorator_test() {
        let result = parse_decorator_name("test", &[]);
        assert_eq!(result.unwrap(), Decorator::Test);
    }

    #[test]
    fn test_parse_decorator_before_each() {
        let result = parse_decorator_name("before_each", &[]);
        assert_eq!(result.unwrap(), Decorator::BeforeEach);
    }

    #[test]
    fn test_parse_decorator_after_each() {
        let result = parse_decorator_name("after_each", &[]);
        assert_eq!(result.unwrap(), Decorator::AfterEach);
    }

    #[test]
    fn test_parse_decorator_skip() {
        let result = parse_decorator_name("skip", &[]);
        assert_eq!(result.unwrap(), Decorator::Skip);
    }

    #[test]
    fn test_parse_decorator_gungnir() {
        let result = parse_decorator_name("gungnir", &[]);
        assert_eq!(result.unwrap(), Decorator::Gungnir);
    }

    // ==================================================================
    // Test 6: @skip_test with reason
    //
    // @skip_test("not ready") should produce SkipTest { reason: "not ready" }.
    // The parser passes the raw string including quotes, so the decorator
    // parser must strip them.
    // ==================================================================

    #[test]
    fn test_parse_decorator_skip_test_with_reason() {
        let args = vec!["\"not ready\"".to_string()];
        let result = parse_decorator_name("skip_test", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::SkipTest {
                reason: "not ready".to_string(),
            }
        );
    }

    // ==================================================================
    // Test 7: @covers with 3 args
    //
    // @covers(divide, b, zero) should produce Covers { fn_name, param, edge_value }.
    // ==================================================================

    #[test]
    fn test_parse_decorator_covers_with_three_args() {
        let args = vec!["divide".to_string(), "b".to_string(), "zero".to_string()];
        let result = parse_decorator_name("covers", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Covers {
                fn_name: "divide".to_string(),
                param: "b".to_string(),
                edge_value: "zero".to_string(),
            }
        );
    }

    // ==================================================================
    // Test 8: @tested with fn_name
    //
    // @tested(add) should produce Tested { fn_name: "add" }.
    // ==================================================================

    #[test]
    fn test_parse_decorator_tested_with_fn_name() {
        let args = vec!["add".to_string()];
        let result = parse_decorator_name("tested", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Tested {
                fn_name: "add".to_string(),
            }
        );
    }

    // ==================================================================
    // Tests 9-11: Contract variants (requires, ensures, invariant)
    //
    // Each takes a single string argument representing a condition
    // expression.
    // ==================================================================

    #[test]
    fn test_parse_decorator_requires() {
        let args = vec!["a > 0".to_string()];
        let result = parse_decorator_name("requires", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Requires {
                condition: "a > 0".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_decorator_ensures() {
        let args = vec!["result > 0".to_string()];
        let result = parse_decorator_name("ensures", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Ensures {
                condition: "result > 0".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_decorator_invariant() {
        let args = vec!["balance >= 0".to_string()];
        let result = parse_decorator_name("invariant", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Invariant {
                condition: "balance >= 0".to_string(),
            }
        );
    }

    // ==================================================================
    // Test 12: Unknown decorator returns an error
    //
    // An unrecognized decorator name must produce Err, not panic.
    // ==================================================================

    #[test]
    fn test_parse_decorator_unknown_returns_error() {
        let result = parse_decorator_name("unknown_decorator", &[]);
        assert!(result.is_err(), "expected Err for unknown decorator name");
    }

    // ==================================================================
    // Test 13: @covers with quoted args strips surrounding quotes
    //
    // @covers(add, b, "zero") should produce Covers { fn_name: "add",
    // param: "b", edge_value: "zero" } — quotes stripped uniformly.
    // ==================================================================

    #[test]
    fn test_parse_decorator_covers_strips_quotes() {
        let args = vec!["add".into(), "b".into(), "\"zero\"".into()];
        let result = parse_decorator_name("covers", &args);
        assert_eq!(
            result.unwrap(),
            Decorator::Covers {
                fn_name: "add".to_string(),
                param: "b".to_string(),
                edge_value: "zero".to_string(),
            }
        );
    }
}
