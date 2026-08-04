//! High-level Intermediate Representation (HIR).
//! The untyped AST produced by the parser.

use crate::span::Span;
use serde::{Deserialize, Serialize};

/// A literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Str(String),
    RawStr(String),
    Bool(bool),
    Null,
}

/// A parameter in a function or lambda declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_: Option<Type>,
}

/// A field in a record type definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_: Type,
}

/// A variant in a union/record definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub arg: Option<Type>,
}

/// An arm in a match expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pat,
    pub guard: Option<Expr>,
    pub body: Expr,
}

/// A statement inside a block expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Let(Pat, Expr),
    Expr(Expr),
}

// ---- Patterns ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pat {
    Wildcard,
    Literal(LiteralValue),
    Variable(String),
    Variant {
        name: String,
        arg: Option<Box<Pat>>,
    },
    Record {
        fields: Vec<(String, Pat)>,
        rest: bool,
    },
}

// ---- Types ----

/// A constraint on a refined type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RefConstraint {
    /// Range constraint: min..max (inclusive)
    Range { min: i64, max: i64 },
    /// Non-empty constraint: string must not be empty
    NonEmpty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Named(String),
    Record(Vec<(String, Box<Type>)>),
    Union(Vec<Type>),
    Func {
        params: Vec<Type>,
        return_: Box<Type>,
    },
    Generic {
        base: String,
        args: Vec<Type>,
    },
    /// A refinement of a base type with a constraint.
    /// e.g., `Int(0..100)` → Type::Refined { base: Box::new(Type::Named("Int")), constraint: RefConstraint::Range { min: 0, max: 100 } }
    Refined {
        base: Box<Type>,
        constraint: RefConstraint,
    },
    /// keyof T — the union of field names of a record type.
    /// e.g., `keyof Person` → Type::KeyOf(Box::new(Type::Named("Person")))
    KeyOf(Box<Type>),
    /// T["key"] — the type of field "key" in type T.
    /// e.g., `Person["name"]` → Type::IndexedAccess { obj: Box::new(Type::Named("Person")), key: "name" }
    IndexedAccess {
        obj: Box<Type>,
        key: String,
    },
}

// ---- Expressions ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Literal value (int, float, string, bool, null)
    Literal { value: LiteralValue, span: Span },
    /// Variable reference
    Variable { name: String, span: Span },
    /// Function call
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// Member access (obj.field)
    Member {
        obj: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Optional member access (obj?.field)
    OptionalAccess {
        obj: Box<Expr>,
        field: String,
        span: Span,
    },
    /// If expression with optional else branch
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Option<Box<Expr>>,
        span: Span,
    },
    /// Match expression with arms
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// Block expression (sequence of statements)
    Block { stmts: Vec<Stmt>, span: Span },
    /// Pipe operator (|>)
    Pipe {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Propagate operator (?)
    Propagate { expr: Box<Expr>, span: Span },
    /// Try expression with catch handler
    Try {
        body: Box<Expr>,
        binding: Pat,
        guard: Option<Box<Expr>>,
        handler: Box<Expr>,
        span: Span,
    },
    /// Throw expression
    Throw { expr: Box<Expr>, span: Span },
    /// For loop
    For {
        binding: Pat,
        iterable: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    /// Assignment
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// Lambda expression
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },
    /// Record literal
    Record {
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    /// Variant literal
    Variant {
        name: String,
        arg: Option<Box<Expr>>,
        span: Span,
    },
    /// Array literal
    Array { items: Vec<Expr>, span: Span },
    /// Wildcard expression
    Wildcard { span: Span },
    /// Binary operation
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Unary operation
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    /// Property-based testing: forAll Type { var -> property }
    ForAll {
        type_: Type,
        binding: Pat,
        property: Box<Expr>,
        span: Span,
    },
    /// Assert that an expression produces consistent results across all targets.
    AssertConsistent { expr: Box<Expr>, span: Span },
    /// Non-null assertion operator (expr!)
    NonNullAssert { expr: Box<Expr>, span: Span },
}

impl Expr {
    /// Get the source span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Variable { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Member { span, .. } => *span,
            Expr::OptionalAccess { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Pipe { span, .. } => *span,
            Expr::Propagate { span, .. } => *span,
            Expr::Try { span, .. } => *span,
            Expr::Throw { span, .. } => *span,
            Expr::For { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Record { span, .. } => *span,
            Expr::Variant { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Wildcard { span } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::ForAll { span, .. } => *span,
            Expr::AssertConsistent { span, .. } => *span,
            Expr::NonNullAssert { span, .. } => *span,
        }
    }
}

// ---- Binary & Unary Operators ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

// ---- Declarations (top-level) ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decl {
    Import {
        module: String,
        names: Vec<String>,
        is_pub: bool,
        span: Span,
    },
    Function {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Expr,
        is_pub: bool,
        span: Span,
    },
    TypeDef {
        name: String,
        type_: Type,
        is_pub: bool,
        span: Span,
    },
    RecordDef {
        name: String,
        fields: Vec<Field>,
        is_pub: bool,
        span: Span,
    },
    UnionDef {
        name: String,
        variants: Vec<Variant>,
        type_params: Vec<String>,
        is_pub: bool,
        span: Span,
    },
    Decorator {
        name: String,
        args: Vec<Expr>,
        target: Box<Decl>,
        is_pub: bool,
        span: Span,
    },
    Extern {
        source: String,
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        is_pub: bool,
        span: Span,
    },
    Const {
        name: String,
        value: Box<Expr>,
        type_: Option<Type>,
        is_pub: bool,
        span: Span,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Refinement type tests (DWARF-38: Edge Case Generator)
    //
    // These tests specify the expected shape of Type::Refined and
    // RefConstraint once they are implemented. They will fail to compile
    // until both are added (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_hir_refined_type_construction() {
        let refined = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        assert_eq!(
            refined,
            Type::Refined {
                base: Box::new(Type::Named("Int".to_string())),
                constraint: RefConstraint::Range { min: 0, max: 100 },
            }
        );
    }

    #[test]
    fn test_hir_refined_type_partial_eq() {
        let a = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        let b = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        let c = Type::Refined {
            base: Box::new(Type::Named("String".to_string())),
            constraint: RefConstraint::Range { min: 1, max: 50 },
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(
            a,
            Type::Refined {
                base: Box::new(Type::Named("Int".to_string())),
                constraint: RefConstraint::Range { min: 0, max: 200 },
            }
        );
    }

    #[test]
    fn test_ref_constraint_range_values() {
        let constraint = RefConstraint::Range { min: 0, max: 100 };
        assert_eq!(constraint, RefConstraint::Range { min: 0, max: 100 });
        assert_ne!(constraint, RefConstraint::Range { min: 0, max: 50 });
        assert_ne!(constraint, RefConstraint::Range { min: 1, max: 100 });
    }

    #[test]
    fn test_ref_constraint_negative_range() {
        let constraint = RefConstraint::Range { min: -10, max: 10 };
        assert_eq!(constraint, RefConstraint::Range { min: -10, max: 10 });
    }

    #[test]
    fn test_refined_type_clone() {
        let refined = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        assert_eq!(refined, refined.clone());
    }

    #[test]
    fn test_refined_type_debug() {
        let refined = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        let s = format!("{refined:?}");
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    #[test]
    fn test_refined_type_serde_roundtrip() {
        let original = Type::Refined {
            base: Box::new(Type::Named("Int".to_string())),
            constraint: RefConstraint::Range { min: 0, max: 100 },
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: Type = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_refined_type_in_record() {
        // Type::Record containing a Type::Refined field
        let record = Type::Record(vec![(
            "age".to_string(),
            Box::new(Type::Refined {
                base: Box::new(Type::Named("Int".to_string())),
                constraint: RefConstraint::Range { min: 0, max: 150 },
            }),
        )]);
        if let Type::Record(fields) = &record {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "age");
            assert_eq!(
                *fields[0].1,
                Type::Refined {
                    base: Box::new(Type::Named("Int".to_string())),
                    constraint: RefConstraint::Range { min: 0, max: 150 },
                }
            );
        } else {
            panic!("Expected Record variant");
        }
    }

    // ------------------------------------------------------------------
    // AssertConsistent expression tests (DWARF-41)
    //
    // These tests specify the expected shape of Expr::AssertConsistent
    // once the HIR variant is added. They will fail to compile until
    // the variant is implemented (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_hir_assert_consistent_construction() {
        let expr = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Int(42),
                span: Span::new(0, 0, 0),
            }),
            span: Span::new(0, 0, 0),
        };
        match &expr {
            Expr::AssertConsistent { expr: inner, .. } => match inner.as_ref() {
                Expr::Literal { value, .. } => {
                    assert_eq!(*value, LiteralValue::Int(42));
                }
                _ => panic!("Expected literal inside AssertConsistent"),
            },
            _ => panic!("Expected AssertConsistent variant"),
        }
    }

    #[test]
    fn test_hir_assert_consistent_span() {
        let span = Span::new(0, 5, 20);
        let expr = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Null,
                span: Span::new(0, 15, 18),
            }),
            span,
        };
        assert_eq!(expr.span(), span);
    }

    #[test]
    fn test_hir_assert_consistent_partial_eq() {
        let span = Span::new(0, 0, 0);
        let a = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Int(42),
                span,
            }),
            span,
        };
        let b = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Int(42),
                span,
            }),
            span,
        };
        let c = Expr::AssertConsistent {
            expr: Box::new(Expr::Literal {
                value: LiteralValue::Int(99),
                span,
            }),
            span,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
