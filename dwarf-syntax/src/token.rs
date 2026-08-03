//! Token types for the Dwarf lexer.

use crate::span::Span;
use serde::{Deserialize, Serialize};

/// The kind of a token — categorizes what the token represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenKind {
    // ---- Keywords (13 reserved words + true/false/null) ----
    Fn,
    Type,
    Let,
    Match,
    If,
    Else,
    For,
    ForAll,
    Import,
    From,
    Module,
    Pub,
    True,
    False,
    Null,
    Try,
    Catch,
    Throw,
    Extern,
    Const,
    In,
    KeyOf,

    // ---- Arithmetic Operators ----
    Plus,  // +
    Minus, // -
    Star,  // *
    Slash, // /

    // ---- Comparison Operators ----
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=

    // ---- Logical Operators ----
    AmpAmp,   // &&
    PipePipe, // ||
    Bang,     // !

    // ---- Assignment ----
    Eq, // =

    // ---- Delimiters & Punctuation ----
    Colon,       // :
    Arrow,       // ->
    Pipe,        // | (union type, lambda)
    PipeGt,      // |>
    Question,    // ?
    QuestionDot, // ?.
    Underscore,  // _
    DotDot,      // ..
    Dot,         // .
    Comma,       // ,
    At,          // @

    // ---- Brackets ----
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]

    // ---- Literals ----
    Int(i64),
    Float(f64),
    Str(String),
    RawStr(String),

    // ---- Identifiers & Special ----
    Ident(String),
    DocComment,

    // ---- End of File ----
    Eof,
}

impl TokenKind {
    /// Returns a human-readable description of this token kind.
    pub fn description(&self) -> &'static str {
        match self {
            // Keywords
            Self::Fn => "'fn'",
            Self::Type => "'type'",
            Self::Let => "'let'",
            Self::Match => "'match'",
            Self::If => "'if'",
            Self::Else => "'else'",
            Self::For => "'for'",
            Self::ForAll => "'forAll'",
            Self::Import => "'import'",
            Self::From => "'from'",
            Self::Module => "'module'",
            Self::Pub => "'pub'",
            Self::True => "'true'",
            Self::False => "'false'",
            Self::Null => "'null'",
            Self::Try => "'try'",
            Self::Catch => "'catch'",
            Self::Throw => "'throw'",
            Self::Extern => "'extern'",
            Self::Const => "'const'",
            Self::In => "'in'",
            Self::KeyOf => "'keyof'",

            // Arithmetic
            Self::Plus => "'+'",
            Self::Minus => "'-'",
            Self::Star => "'*'",
            Self::Slash => "'/'",

            // Comparison
            Self::EqEq => "'=='",
            Self::BangEq => "'!='",
            Self::Lt => "'<'",
            Self::Gt => "'>'",
            Self::LtEq => "'<='",
            Self::GtEq => "'>='",

            // Logical
            Self::AmpAmp => "'&&'",
            Self::PipePipe => "'||'",
            Self::Bang => "'!'",

            // Assignment
            Self::Eq => "'='",

            // Punctuation
            Self::Colon => "':'",
            Self::Arrow => "'->'",
            Self::Pipe => "'|'",
            Self::PipeGt => "'|>'",
            Self::Question => "'?'",
            Self::QuestionDot => "'?.'",
            Self::Underscore => "'_'",
            Self::DotDot => "'..'",
            Self::Dot => "'.'",
            Self::Comma => "','",
            Self::At => "'@'",

            // Brackets
            Self::LParen => "'('",
            Self::RParen => "')'",
            Self::LBrace => "'{'",
            Self::RBrace => "'}'",
            Self::LBracket => "'['",
            Self::RBracket => "']'",

            // Literals
            Self::Int(_) => "integer literal",
            Self::Float(_) => "float literal",
            Self::Str(_) => "string literal",
            Self::RawStr(_) => "raw string literal",

            // Identifiers
            Self::Ident(_) => "identifier",
            Self::DocComment => "doc comment",

            Self::Eof => "end of file",
        }
    }
}

/// A token produced by the lexer, consisting of a kind and source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_dotdot_description() {
        assert_eq!(TokenKind::DotDot.description(), "'..'");
    }

    #[test]
    fn test_token_dotdot_equality() {
        assert_eq!(TokenKind::DotDot, TokenKind::DotDot);
        assert_ne!(TokenKind::DotDot, TokenKind::Dot);
        assert_ne!(TokenKind::DotDot, TokenKind::Arrow);
    }

    #[test]
    fn test_token_dotdot_debug() {
        let s = format!("{:?}", TokenKind::DotDot);
        assert!(!s.is_empty(), "Debug output should not be empty");
    }

    // ------------------------------------------------------------------
    // Extern keyword tests (RED Phase — expected to fail)
    //
    // TokenKind::Extern does not exist yet. These tests specify the
    // expected token for Phase 1 extern/FFI support.
    // ------------------------------------------------------------------

    #[test]
    fn test_extern_token_kind_exists() {
        // TokenKind::Extern should exist and have the correct description
        assert_eq!(TokenKind::Extern.description(), "'extern'");
    }

    #[test]
    fn test_extern_token_kind_equality() {
        assert_eq!(TokenKind::Extern, TokenKind::Extern);
        assert_ne!(TokenKind::Extern, TokenKind::Fn);
        assert_ne!(TokenKind::Extern, TokenKind::Import);
    }

    #[test]
    fn test_extern_token_kind_debug() {
        let s = format!("{:?}", TokenKind::Extern);
        assert_eq!(s, "Extern");
    }
}
