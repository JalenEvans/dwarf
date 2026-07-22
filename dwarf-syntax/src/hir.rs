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
}

// ---- Expressions ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Literal value (int, float, string, bool, null)
    Literal {
        value: LiteralValue,
        span: Span,
    },
    /// Variable reference
    Variable {
        name: String,
        span: Span,
    },
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
    Block {
        stmts: Vec<Stmt>,
        span: Span,
    },
    /// Pipe operator (|>)
    Pipe {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Propagate operator (?)
    Propagate {
        expr: Box<Expr>,
        span: Span,
    },
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
    Array {
        items: Vec<Expr>,
        span: Span,
    },
    /// Wildcard expression
    Wildcard {
        span: Span,
    },
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
}

impl Expr {
    /// Get the source span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Variable { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Member { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Pipe { span, .. } => *span,
            Expr::Propagate { span, .. } => *span,
            Expr::For { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Record { span, .. } => *span,
            Expr::Variant { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Wildcard { span } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
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
}
