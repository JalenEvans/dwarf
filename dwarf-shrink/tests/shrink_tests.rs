use dwarf_shrink::{IntShrinker, ListShrinker, Shrinker, StringShrinker};

// ---------------------------------------------------------------------------
// Int shrinking tests
// ---------------------------------------------------------------------------

#[test]
fn test_shrink_int_positive_to_minimal() {
    // Shrink 1000 with predicate |n| n > 5 (fails when n > 5)
    // The minimal failing value should be 6
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&1000, &mut |n: &i64| *n > 5);
    assert_eq!(result, 6, "should shrink 1000 to 6");
}

#[test]
fn test_shrink_int_already_minimal() {
    // Shrink 7 with predicate |n| n > 5
    // The minimal failing value should be 6
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&7, &mut |n: &i64| *n > 5);
    assert_eq!(result, 6, "should shrink 7 to 6");
}

#[test]
fn test_shrink_int_negative() {
    // Predicate: n < -5 (fails when n < -5)
    // Shrink -1000 to minimal: -6
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&-1000, &mut |n: &i64| *n < -5);
    assert_eq!(result, -6, "should shrink -1000 to -6");
}

#[test]
fn test_shrink_int_no_failure_at_zero() {
    // If the predicate only fails at very large values (e.g., n > 1000000)
    // and the initial value is i64::MAX, the shrink should find the boundary
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&i64::MAX, &mut |n: &i64| *n > 1000000);
    assert_eq!(result, 1000001, "should find minimal value above threshold");
}

#[test]
fn test_shrink_int_boundary_at_zero() {
    // Predicate: n != 0 (fails for any non-zero)
    // Shrink 100 to minimal: 1 (smallest non-zero failing value)
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&100, &mut |n: &i64| *n != 0);
    assert_eq!(result, 1, "should shrink 100 to 1");
}

#[test]
fn test_shrink_int_exact_boundary_zero() {
    // Predicate: n >= 0 (fails for non-negative)
    // Shrink -50 to minimal: 0
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&-50, &mut |n: &i64| *n >= 0);
    assert_eq!(result, 0, "should shrink -50 to 0");
}

#[test]
fn test_shrink_int_already_at_threshold() {
    // Predicate: n > 10 (fails when n > 10)
    // Start at 11 which is already minimal failing
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&11, &mut |n: &i64| *n > 10);
    assert_eq!(result, 11, "should keep already minimal value 11");
}

// ---------------------------------------------------------------------------
// String shrinking tests
// ---------------------------------------------------------------------------

#[test]
fn test_shrink_string_remove_extra_chars() {
    // Predicate: s.len() > 3 (fails when length > 3)
    // "hello world" should shrink to "hell" (first 4 chars that still fail)
    let shrinker = StringShrinker;
    let result = shrinker.shrink(
        &"hello world".to_string(),
        &mut |s: &String| s.len() > 3,
    );
    assert!(
        result.len() == 4,
        "should shrink to minimal 4-char string, got '{}' (len={})",
        result,
        result.len()
    );
}

#[test]
fn test_shrink_string_contains_substring() {
    // Predicate: s.contains("fail")
    // "this is a failing test" should shrink to "fail" (minimal)
    let shrinker = StringShrinker;
    let result = shrinker.shrink(
        &"this is a failing test".to_string(),
        &mut |s: &String| s.contains("fail"),
    );
    assert_eq!(
        result, "fail",
        "should shrink to minimal failing substring"
    );
}

#[test]
fn test_shrink_string_single_char() {
    // Predicate: s == "a"
    // "a" is already minimal
    let shrinker = StringShrinker;
    let result = shrinker.shrink(&"a".to_string(), &mut |s: &String| s == "a");
    assert_eq!(result, "a", "should keep already minimal string 'a'");
}

#[test]
fn test_shrink_string_empty() {
    // Predicate: s.is_empty() — empty string always fails
    // Already minimal
    let shrinker = StringShrinker;
    let result = shrinker.shrink(&"".to_string(), &mut |s: &String| s.is_empty());
    assert_eq!(result, "", "empty string should stay empty");
}

#[test]
fn test_shrink_string_prefix_preserved() {
    // Predicate: s.starts_with("needle")
    // "needle in a haystack" should shrink to "needle"
    let shrinker = StringShrinker;
    let result = shrinker.shrink(
        &"needle in a haystack".to_string(),
        &mut |s: &String| s.starts_with("needle"),
    );
    assert_eq!(
        result, "needle",
        "should shrink to minimal prefix string 'needle'"
    );
}

#[test]
fn test_shrink_string_suffix_preserved() {
    // Predicate: s.ends_with("end")
    // "the end" should shrink to "end"
    let shrinker = StringShrinker;
    let result = shrinker.shrink(
        &"the end".to_string(),
        &mut |s: &String| s.ends_with("end"),
    );
    assert_eq!(
        result, "end",
        "should shrink to minimal suffix string 'end'"
    );
}

// ---------------------------------------------------------------------------
// List shrinking tests
// ---------------------------------------------------------------------------

#[test]
fn test_shrink_list_remove_elements() {
    // Predicate: list.len() > 2 (fails when length > 2)
    let shrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec![1, 2, 3, 4, 5],
        &mut |list: &Vec<i32>| list.len() > 2,
    );
    assert_eq!(result.len(), 3, "should shrink to 3-element list");
}

#[test]
fn test_shrink_list_precise_elements() {
    // Predicate: list contains 42
    // Only elements matter, not length
    let shrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec![10, 20, 30, 42, 50, 60],
        &mut |list: &Vec<i32>| list.contains(&42),
    );
    assert_eq!(result, vec![42], "should shrink to just [42]");
}

#[test]
fn test_shrink_list_single_element() {
    // Predicate: list contains 1
    // vec![1] is already minimal
    let shrinker = ListShrinker;
    let result = shrinker.shrink(&vec![1], &mut |list: &Vec<i32>| list.contains(&1));
    assert_eq!(result, vec![1], "should keep already minimal list [1]");
}

#[test]
fn test_shrink_list_empty() {
    // Predicate: list.is_empty() — empty list always fails
    let shrinker = ListShrinker;
    let result: Vec<i32> =
        shrinker.shrink(&vec![], &mut |list: &Vec<i32>| list.is_empty());
    assert!(result.is_empty(), "empty list should stay empty");
}

#[test]
fn test_shrink_list_first_element_preserved() {
    // Predicate: list.first() == Some(&99)
    // vec![99, 1, 2, 3, 4] should shrink to vec![99]
    let shrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec![99, 1, 2, 3, 4],
        &mut |list: &Vec<i32>| list.first() == Some(&99),
    );
    assert_eq!(result, vec![99], "should shrink to just [99]");
}

#[test]
fn test_shrink_list_last_element_preserved() {
    // Predicate: list.last() == Some(&77)
    // vec![1, 2, 3, 4, 77] should shrink to vec![77]
    let shrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec![1, 2, 3, 4, 77],
        &mut |list: &Vec<i32>| list.last() == Some(&77),
    );
    assert_eq!(result, vec![77], "should shrink to just [77]");
}

#[test]
fn test_shrink_list_all_greater_than() {
    // Predicate: list.iter().all(|x| x > 10)
    // vec![100, 200, 300] should shrink to the smallest list still all > 10
    // This could be vec![100] (single element) or minimal multi-element list
    let shrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec![100, 200, 300],
        &mut |list: &Vec<i32>| list.iter().all(|x| *x > 10),
    );
    assert!(
        result.len() >= 1,
        "should have at least 1 element, got {:?} (len={})",
        result,
        result.len()
    );
    assert!(
        result.iter().all(|x| *x > 10),
        "all elements should still satisfy predicate, got {:?}",
        result
    );
}

#[test]
fn test_shrink_list_strings() {
    // Predicate: list contains "apple"
    let shrinker: ListShrinker = ListShrinker;
    let result = shrinker.shrink(
        &vec!["banana".to_string(), "apple".to_string(), "cherry".to_string()],
        &mut |list: &Vec<String>| list.contains(&"apple".to_string()),
    );
    assert_eq!(
        result,
        vec!["apple".to_string()],
        "should shrink to just [\"apple\"]"
    );
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn test_shrink_int_zero_initial() {
    // Predicate: n == 0 — zero already fails
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&0, &mut |n: &i64| *n == 0);
    assert_eq!(result, 0, "zero should stay zero");
}

#[test]
fn test_shrink_int_large_positive() {
    // Predicate: n > 500_000
    // Shrink 10_000_000 to 500_001
    let shrinker = IntShrinker;
    let result = shrinker.shrink(&10_000_000, &mut |n: &i64| *n > 500_000);
    assert_eq!(result, 500_001, "should find minimal value above 500,000");
}

#[test]
fn test_shrink_string_no_change_needed() {
    // Predicate: "hello" starts with "h"
    // Already minimal
    let shrinker = StringShrinker;
    let result = shrinker.shrink(
        &"h".to_string(),
        &mut |s: &String| s.starts_with("h"),
    );
    assert_eq!(result, "h", "should keep already minimal string 'h'");
}
