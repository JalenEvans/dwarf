//! Edge case generator — automatically derives edge case test inputs
//! from type definitions and refinements.

use dwarf_syntax::hir::{LiteralValue, RefConstraint, Type};

/// A single edge case test case.
#[derive(Debug, Clone, PartialEq)]
pub struct TestCase {
    pub description: String,
    pub value: LiteralValue,
}

/// Generate edge case test values for a given type.
///
/// Returns a list of test cases, each with a human-readable description
/// and the literal value to test.
pub fn generate_edge_cases(ty: &Type) -> Vec<TestCase> {
    match ty {
        Type::Named(name) => generate_named_edge_cases(name),
        Type::Refined { base, constraint } => generate_refined_edge_cases(base, constraint),
        Type::Generic { base, args } => generate_generic_edge_cases(base, args),
        Type::Record(fields) => generate_record_edge_cases(fields),
        Type::Union(variants) => generate_union_edge_cases(variants),
        Type::Func { .. } => vec![],
    }
}

/// Generate edge cases for a named type.
fn generate_named_edge_cases(name: &str) -> Vec<TestCase> {
    match name {
        "Int" => vec![
            TestCase {
                description: "Int(-1)".into(),
                value: LiteralValue::Int(-1),
            },
            TestCase {
                description: "Int(0)".into(),
                value: LiteralValue::Int(0),
            },
            TestCase {
                description: "Int(1)".into(),
                value: LiteralValue::Int(1),
            },
            TestCase {
                description: "Int(MAX)".into(),
                value: LiteralValue::Int(i64::MAX),
            },
            TestCase {
                description: "Int(MIN)".into(),
                value: LiteralValue::Int(i64::MIN),
            },
        ],
        "String" => {
            let long = "a".repeat(256);
            vec![
                TestCase {
                    description: "String empty".into(),
                    value: LiteralValue::Str("".into()),
                },
                TestCase {
                    description: "String a".into(),
                    value: LiteralValue::Str("a".into()),
                },
                TestCase {
                    description: "String abc".into(),
                    value: LiteralValue::Str("abc".into()),
                },
                TestCase {
                    description: "String null".into(),
                    value: LiteralValue::Str("\0".into()),
                },
                TestCase {
                    description: "String long".into(),
                    value: LiteralValue::Str(long),
                },
            ]
        }
        "Bool" => vec![
            TestCase {
                description: "Bool true".into(),
                value: LiteralValue::Bool(true),
            },
            TestCase {
                description: "Bool false".into(),
                value: LiteralValue::Bool(false),
            },
        ],
        "Float" => vec![
            TestCase {
                description: "Float(-1.0)".into(),
                value: LiteralValue::Float(-1.0),
            },
            TestCase {
                description: "Float(0.0)".into(),
                value: LiteralValue::Float(0.0),
            },
            TestCase {
                description: "Float(1.0)".into(),
                value: LiteralValue::Float(1.0),
            },
        ],
        "Null" => vec![TestCase {
            description: "Null".into(),
            value: LiteralValue::Null,
        }],
        _ => vec![],
    }
}

/// Generate edge cases for a refined type (e.g. `Int(0..100)`, `String(1..50)`).
fn generate_refined_edge_cases(base: &Type, constraint: &RefConstraint) -> Vec<TestCase> {
    match constraint {
        RefConstraint::Range { min, max } => match base {
            Type::Named(name) if name == "Int" => {
                let min_minus_1 = if *min == i64::MIN { *min } else { *min - 1 };
                let max_plus_1 = if *max == i64::MAX { *max } else { *max + 1 };
                let mid = min.saturating_add(*max) / 2;
                vec![
                    TestCase {
                        description: format!("Int({min_minus_1}) (min-1)"),
                        value: LiteralValue::Int(min_minus_1),
                    },
                    TestCase {
                        description: format!("Int({min}) (min)"),
                        value: LiteralValue::Int(*min),
                    },
                    TestCase {
                        description: format!("Int({}) (min+1)", *min + 1),
                        value: LiteralValue::Int(*min + 1),
                    },
                    TestCase {
                        description: format!("Int({mid}) (mid)"),
                        value: LiteralValue::Int(mid),
                    },
                    TestCase {
                        description: format!("Int({}) (max-1)", *max - 1),
                        value: LiteralValue::Int(*max - 1),
                    },
                    TestCase {
                        description: format!("Int({max}) (max)"),
                        value: LiteralValue::Int(*max),
                    },
                    TestCase {
                        description: format!("Int({max_plus_1}) (max+1)"),
                        value: LiteralValue::Int(max_plus_1),
                    },
                ]
            }
            Type::Named(name) if name == "String" => {
                let empty = TestCase {
                    description: "String empty".into(),
                    value: LiteralValue::Str("".into()),
                };
                let min_len = *min as usize;
                let max_len = *max as usize;
                let over_max = max.saturating_add(1) as usize;

                let mut cases = vec![empty];

                // Only add the min-length string if it's non-empty (to avoid duplicating the empty string)
                if min_len > 0 {
                    cases.push(TestCase {
                        description: format!("String len={min_len}"),
                        value: LiteralValue::Str("a".repeat(min_len)),
                    });
                }

                // Max-length string
                if max_len > 0 {
                    cases.push(TestCase {
                        description: format!("String len={max_len}"),
                        value: LiteralValue::Str("a".repeat(max_len)),
                    });
                }

                // Over-max string (if distinct from max)
                if over_max != max_len {
                    cases.push(TestCase {
                        description: format!("String len={over_max}"),
                        value: LiteralValue::Str("a".repeat(over_max)),
                    });
                }

                cases
            }
            _ => vec![],
        },
        RefConstraint::NonEmpty => match base {
            Type::Named(name) if name == "String" => vec![
                TestCase {
                    description: "String empty".into(),
                    value: LiteralValue::Str("".into()),
                },
                TestCase {
                    description: "String len=1".into(),
                    value: LiteralValue::Str("a".into()),
                },
            ],
            _ => vec![],
        },
    }
}

/// Generate edge cases for a generic type (e.g., `Option<Int>`, `List<Int>`).
fn generate_generic_edge_cases(base: &str, args: &[Type]) -> Vec<TestCase> {
    match base {
        "Option" if args.len() == 1 => {
            let inner_cases = generate_edge_cases(&args[0]);
            let mut cases = vec![TestCase {
                description: "None".into(),
                value: LiteralValue::Null,
            }];
            for tc in inner_cases {
                cases.push(TestCase {
                    description: format!("Some({})", tc.description),
                    value: tc.value,
                });
            }
            cases
        }
        "List" if args.len() == 1 => {
            let inner = &args[0];
            let inner_cases = generate_edge_cases(inner);
            let type_name = type_display_name(inner);

            let mut cases = vec![TestCase {
                description: format!("List<{type_name}> []"),
                value: LiteralValue::Null,
            }];

            // 1-element lists
            for tc in &inner_cases {
                cases.push(TestCase {
                    description: format!("List<{type_name}> [{}]", tc.description),
                    value: LiteralValue::Null,
                });
            }

            // 2-element lists (combine first two inner cases, or repeat if only one)
            if !inner_cases.is_empty() {
                let a = &inner_cases[0];
                let b = if inner_cases.len() > 1 {
                    &inner_cases[1]
                } else {
                    &inner_cases[0]
                };
                cases.push(TestCase {
                    description: format!(
                        "List<{type_name}> [{}, {}]",
                        a.description, b.description
                    ),
                    value: LiteralValue::Null,
                });
            }

            cases
        }
        _ => vec![],
    }
}

/// Generate a human-readable display name for a type (used in descriptions).
fn type_display_name(ty: &Type) -> String {
    match ty {
        Type::Named(name) => name.clone(),
        Type::Refined { base, .. } => type_display_name(base),
        Type::Generic { base, args } => {
            if args.is_empty() {
                base.clone()
            } else {
                let inner: Vec<String> = args.iter().map(type_display_name).collect();
                format!("{base}<{}>", inner.join(", "))
            }
        }
        Type::Record(_) => "Record".into(),
        Type::Union(_) => "Union".into(),
        Type::Func { .. } => "Func".into(),
    }
}

/// Generate edge cases for a record type by flattening each field's edge cases.
fn generate_record_edge_cases(fields: &[(String, Box<Type>)]) -> Vec<TestCase> {
    let mut cases = vec![];
    for (field_name, field_type) in fields {
        for tc in generate_edge_cases(field_type) {
            cases.push(TestCase {
                description: format!("Record {field_name}={}", tc.description),
                value: LiteralValue::Null,
            });
        }
    }
    cases
}

/// Generate edge cases for a union type by generating edge cases for each variant.
fn generate_union_edge_cases(variants: &[Type]) -> Vec<TestCase> {
    let mut cases = vec![];
    for variant in variants {
        match variant {
            // A bare named variant (e.g., `None`, `Empty`)
            Type::Named(name) => {
                cases.push(TestCase {
                    description: name.clone(),
                    value: LiteralValue::Null,
                });
            }
            // A generic variant with one type argument (e.g., `Some<Int>`, `Err<String>`)
            Type::Generic { base, args } if args.len() == 1 => {
                let inner_cases = generate_edge_cases(&args[0]);
                for tc in inner_cases {
                    cases.push(TestCase {
                        description: format!("{base}({})", tc.description),
                        value: tc.value,
                    });
                }
            }
            // Fallback: generate edge cases for the variant type directly
            other => cases.extend(generate_edge_cases(other)),
        }
    }
    cases
}
