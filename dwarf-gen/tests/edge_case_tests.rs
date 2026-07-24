use dwarf_gen::{generate_edge_cases, TestCase};
use dwarf_syntax::hir::{LiteralValue, RefConstraint, Type};

// Helper: create a Named type
fn named(name: &str) -> Type {
    Type::Named(name.to_string())
}

// Helper: create a refined type
fn refined(base: Type, min: i64, max: i64) -> Type {
    Type::Refined {
        base: Box::new(base),
        constraint: RefConstraint::Range { min, max },
    }
}

// Helper: create a generic type like Option<Int>
fn generic(base: &str, args: Vec<Type>) -> Type {
    Type::Generic {
        base: base.to_string(),
        args,
    }
}

// Helper: check that test cases contain a specific literal value
fn has_case(cases: &[TestCase], value: &LiteralValue) -> bool {
    cases.iter().any(|c| &c.value == value)
}

#[test]
fn test_int_edge_cases() {
    let cases = generate_edge_cases(&named("Int"));
    assert!(!cases.is_empty(), "Int should produce edge cases");
    assert!(has_case(&cases, &LiteralValue::Int(-1)), "should include -1");
    assert!(has_case(&cases, &LiteralValue::Int(0)), "should include 0");
    assert!(has_case(&cases, &LiteralValue::Int(1)), "should include 1");
    assert!(has_case(&cases, &LiteralValue::Int(i64::MAX)), "should include MAX");
    assert!(has_case(&cases, &LiteralValue::Int(i64::MIN)), "should include MIN");
    assert_eq!(cases.len(), 5, "unrefined Int should have 5 edge cases");
}

#[test]
fn test_int_range_edge_cases() {
    let cases = generate_edge_cases(&refined(named("Int"), 0, 100));
    assert!(!cases.is_empty(), "Int(0..100) should produce edge cases");
    // Boundary: -1, 0, 1, 50, 99, 100, 101
    assert!(has_case(&cases, &LiteralValue::Int(-1)), "should include -1 (min-1)");
    assert!(has_case(&cases, &LiteralValue::Int(0)), "should include 0 (min)");
    assert!(has_case(&cases, &LiteralValue::Int(1)), "should include 1 (min+1)");
    assert!(has_case(&cases, &LiteralValue::Int(50)), "should include 50 (mid)");
    assert!(has_case(&cases, &LiteralValue::Int(99)), "should include 99 (max-1)");
    assert!(has_case(&cases, &LiteralValue::Int(100)), "should include 100 (max)");
    assert!(has_case(&cases, &LiteralValue::Int(101)), "should include 101 (max+1)");
    assert_eq!(cases.len(), 7, "range Int should have 7 edge cases");
}

#[test]
fn test_bool_edge_cases() {
    let cases = generate_edge_cases(&named("Bool"));
    assert_eq!(cases.len(), 2, "Bool should have 2 edge cases");
    assert!(has_case(&cases, &LiteralValue::Bool(true)));
    assert!(has_case(&cases, &LiteralValue::Bool(false)));
}

#[test]
fn test_string_edge_cases() {
    let cases = generate_edge_cases(&named("String"));
    assert!(!cases.is_empty(), "String should produce edge cases");
    // Should include empty string
    assert!(has_case(&cases, &LiteralValue::Str("".to_string())));
}

#[test]
fn test_string_range_edge_cases() {
    let cases = generate_edge_cases(&refined(named("String"), 1, 50));
    assert!(!cases.is_empty(), "String(1..50) should produce edge cases");
    // Should include empty string (below min)
    assert!(has_case(&cases, &LiteralValue::Str("".to_string())), "should include empty string");
}

#[test]
fn test_option_edge_cases() {
    let cases = generate_edge_cases(&generic("Option", vec![named("Int")]));
    assert!(!cases.is_empty(), "Option<Int> should produce edge cases");
    // Should include None
    assert!(has_case(&cases, &LiteralValue::Null), "should include None");
}

#[test]
fn test_list_edge_cases() {
    let cases = generate_edge_cases(&generic("List", vec![named("Int")]));
    assert!(!cases.is_empty(), "List<Int> should produce edge cases");
}

#[test]
fn test_record_edge_cases() {
    let cases = generate_edge_cases(&Type::Record(vec![
        ("age".to_string(), Box::new(named("Int"))),
        ("active".to_string(), Box::new(named("Bool"))),
    ]));
    assert!(!cases.is_empty(), "Record should produce edge cases");
}

#[test]
fn test_union_edge_cases() {
    let cases = generate_edge_cases(&Type::Union(vec![
        Type::Generic {
            base: "Some".to_string(),
            args: vec![named("Int")],
        },
        named("None"),
    ]));
    assert!(!cases.is_empty(), "Union should produce edge cases");
}
