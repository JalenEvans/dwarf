//! Lexer for the Dwarf compiler.
//! Converts source text into a stream of tokens.

use dwarf_syntax::span::Span;
use dwarf_syntax::token::Token;
use dwarf_syntax::token::TokenKind;

/// An error produced by the lexer when it encounters invalid input.
#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    /// An unexpected character that cannot start any valid token.
    UnexpectedCharacter(char, usize),
    /// An unterminated string literal.
    UnterminatedString(usize),
    /// An invalid integer literal (e.g., empty digits, overflow).
    InvalidIntegerLiteral(usize),
    /// An invalid float literal.
    InvalidFloatLiteral(usize),
}

/// A lexical analyzer that converts source text into a stream of tokens.
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    peeked: Option<Token>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer that will tokenize `input`.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            peeked: None,
        }
    }

    /// Return the next token from the input, advancing the lexer.
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        // If we have a peeked token, return it and clear the cache.
        if let Some(token) = self.peeked.take() {
            return Ok(token);
        }
        self.skip_whitespace();
        if self.position >= self.input.len() {
            return Ok(Token::new(
                TokenKind::Eof,
                Span::synthetic(0, self.position),
            ));
        }
        self.lex_token()
    }

    /// Peek at the next token without consuming it.
    /// This is lazy — it only lexes the token if it hasn't been peeked already.
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            match self.next_token() {
                Ok(token) => self.peeked = Some(token),
                Err(_) => return None,
            }
        }
        self.peeked.as_ref()
    }

    // ---- Internal helpers ----

    /// Advance past any whitespace characters at the current position.
    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            match self.input.as_bytes()[self.position] {
                b' ' | b'\t' | b'\n' | b'\r' => self.position += 1,
                _ => break,
            }
        }
    }

    /// Return the byte at the current position, if any.
    fn curr_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    /// Return the byte immediately after the current position, if any.
    fn next_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position + 1).copied()
    }

    /// Lex the next token starting at the current position.
    /// Assumes there is at least one byte remaining.
    fn lex_token(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        let c = self.curr_byte().unwrap();

        // --- Multi-character operators (must be checked before single-char) ---

        if c == b'=' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(TokenKind::EqEq, Span::new(0, start, self.position)));
        }
        if c == b'!' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(TokenKind::BangEq, Span::new(0, start, self.position)));
        }
        if c == b'<' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(TokenKind::LtEq, Span::new(0, start, self.position)));
        }
        if c == b'>' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(TokenKind::GtEq, Span::new(0, start, self.position)));
        }
        if c == b'-' && self.next_byte() == Some(b'>') {
            self.position += 2;
            return Ok(Token::new(TokenKind::Arrow, Span::new(0, start, self.position)));
        }
        if c == b'|' && self.next_byte() == Some(b'>') {
            self.position += 2;
            return Ok(Token::new(TokenKind::PipeGt, Span::new(0, start, self.position)));
        }
        if c == b'&' && self.next_byte() == Some(b'&') {
            self.position += 2;
            return Ok(Token::new(TokenKind::AmpAmp, Span::new(0, start, self.position)));
        }
        if c == b'|' && self.next_byte() == Some(b'|') {
            self.position += 2;
            return Ok(Token::new(TokenKind::PipePipe, Span::new(0, start, self.position)));
        }

        // --- Single-character tokens ---

        match c {
            b'+' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Plus, Span::new(0, start, self.position)))
            }
            b'-' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Minus, Span::new(0, start, self.position)))
            }
            b'*' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Star, Span::new(0, start, self.position)))
            }
            b'/' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Slash, Span::new(0, start, self.position)))
            }
            b'<' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Lt, Span::new(0, start, self.position)))
            }
            b'>' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Gt, Span::new(0, start, self.position)))
            }
            b'!' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Bang, Span::new(0, start, self.position)))
            }
            b'=' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Eq, Span::new(0, start, self.position)))
            }
            b'|' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Pipe, Span::new(0, start, self.position)))
            }
            b'?' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Question, Span::new(0, start, self.position)))
            }
            b'.' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Dot, Span::new(0, start, self.position)))
            }
            b',' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Comma, Span::new(0, start, self.position)))
            }
            b':' => {
                self.position += 1;
                Ok(Token::new(TokenKind::Colon, Span::new(0, start, self.position)))
            }
            b'@' => {
                self.position += 1;
                Ok(Token::new(TokenKind::At, Span::new(0, start, self.position)))
            }
            b'(' => {
                self.position += 1;
                Ok(Token::new(TokenKind::LParen, Span::new(0, start, self.position)))
            }
            b')' => {
                self.position += 1;
                Ok(Token::new(TokenKind::RParen, Span::new(0, start, self.position)))
            }
            b'{' => {
                self.position += 1;
                Ok(Token::new(TokenKind::LBrace, Span::new(0, start, self.position)))
            }
            b'}' => {
                self.position += 1;
                Ok(Token::new(TokenKind::RBrace, Span::new(0, start, self.position)))
            }
            b'[' => {
                self.position += 1;
                Ok(Token::new(TokenKind::LBracket, Span::new(0, start, self.position)))
            }
            b']' => {
                self.position += 1;
                Ok(Token::new(TokenKind::RBracket, Span::new(0, start, self.position)))
            }
            _ => {
                if c.is_ascii_digit() {
                    self.lex_number(start)
                } else if c == b'_' {
                    // A lone `_` is Underscore; `_` followed by alphanumeric
                    // or more underscores starts an identifier.
                    match self.next_byte() {
                        Some(b) if b.is_ascii_alphanumeric() || b == b'_' => {
                            self.lex_identifier_or_keyword(start)
                        }
                        _ => {
                            self.position += 1;
                            Ok(Token::new(
                                TokenKind::Underscore,
                                Span::new(0, start, self.position),
                            ))
                        }
                    }
                } else if c.is_ascii_alphabetic() {
                    self.lex_identifier_or_keyword(start)
                } else {
                    Err(LexError::UnexpectedCharacter(c as char, start))
                }
            }
        }
    }

    /// Lex an integer literal starting at the current position.
    fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
        while self.position < self.input.len()
            && self.input.as_bytes()[self.position].is_ascii_digit()
        {
            self.position += 1;
        }
        let digits = &self.input[start..self.position];
        match digits.parse::<i64>() {
            Ok(n) => Ok(Token::new(TokenKind::Int(n), Span::new(0, start, self.position))),
            Err(_) => Err(LexError::InvalidIntegerLiteral(start)),
        }
    }

    /// Lex an identifier or keyword starting at the current position.
    /// The first character has already been verified to be a letter or `_`.
    fn lex_identifier_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        while self.position < self.input.len() {
            let c = self.input.as_bytes()[self.position];
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.position += 1;
            } else {
                break;
            }
        }
        let word = &self.input[start..self.position];
        let kind = match word {
            "fn" => TokenKind::Fn,
            "type" => TokenKind::Type,
            "let" => TokenKind::Let,
            "match" => TokenKind::Match,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "module" => TokenKind::Module,
            "pub" => TokenKind::Pub,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(word.to_string()),
        };
        Ok(Token::new(kind, Span::new(0, start, self.position)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_syntax::token::TokenKind;

    // -----------------------------------------------------------------------
    // Helper: lex a single token and assert its kind
    // -----------------------------------------------------------------------
    fn assert_token_kind(input: &str, expected_kind: TokenKind) {
        let mut lexer = Lexer::new(input);
        let token = lexer
            .next_token()
            .unwrap_or_else(|e| panic!("lexer error for input {:?}: {:?}", input, e));
        assert_eq!(
            token.kind, expected_kind,
            "input {:?}: expected {:?}, got {:?}",
            input, expected_kind, token.kind
        );
    }

    // -----------------------------------------------------------------------
    // Helper: lex a sequence of tokens and assert EOF at the end
    // -----------------------------------------------------------------------
    fn assert_token_sequence(input: &str, expected: &[TokenKind]) {
        let mut lexer = Lexer::new(input);
        for (i, expected_kind) in expected.iter().enumerate() {
            let token = lexer.next_token().unwrap_or_else(|e| {
                panic!("lexer error at position {} for input {:?}: {:?}", i, input, e)
            });
            assert_eq!(
                &token.kind, expected_kind,
                "position {} for input {:?}: expected {:?}, got {:?}",
                i, input, expected_kind, token.kind
            );
        }
        // After consuming all expected tokens, assert EOF
        let eof = lexer
            .next_token()
            .expect("lexing should succeed after sequence");
        assert_eq!(
            eof.kind,
            TokenKind::Eof,
            "expected Eof after consuming all tokens for input {:?}",
            input
        );
    }

    // =======================================================================
    // Empty input
    // =======================================================================
    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let token = lexer.next_token().expect("lexing empty input should succeed");
        assert_eq!(token.kind, TokenKind::Eof);
    }

    // =======================================================================
    // Keywords
    // =======================================================================
    #[test]
    fn test_keyword_fn() {
        assert_token_kind("fn", TokenKind::Fn);
    }

    #[test]
    fn test_keyword_let() {
        assert_token_kind("let", TokenKind::Let);
    }

    #[test]
    fn test_keyword_match() {
        assert_token_kind("match", TokenKind::Match);
    }

    #[test]
    fn test_keyword_if() {
        assert_token_kind("if", TokenKind::If);
    }

    #[test]
    fn test_keyword_else() {
        assert_token_kind("else", TokenKind::Else);
    }

    #[test]
    fn test_keyword_for() {
        assert_token_kind("for", TokenKind::For);
    }

    #[test]
    fn test_keyword_import() {
        assert_token_kind("import", TokenKind::Import);
    }

    #[test]
    fn test_keyword_from() {
        assert_token_kind("from", TokenKind::From);
    }

    #[test]
    fn test_keyword_module() {
        assert_token_kind("module", TokenKind::Module);
    }

    #[test]
    fn test_keyword_pub() {
        assert_token_kind("pub", TokenKind::Pub);
    }

    #[test]
    fn test_keyword_type() {
        assert_token_kind("type", TokenKind::Type);
    }

    #[test]
    fn test_keyword_true() {
        assert_token_kind("true", TokenKind::True);
    }

    #[test]
    fn test_keyword_false() {
        assert_token_kind("false", TokenKind::False);
    }

    #[test]
    fn test_keyword_null() {
        assert_token_kind("null", TokenKind::Null);
    }

    // =======================================================================
    // Operators
    // =======================================================================
    #[test]
    fn test_op_plus() {
        assert_token_kind("+", TokenKind::Plus);
    }

    #[test]
    fn test_op_minus() {
        assert_token_kind("-", TokenKind::Minus);
    }

    #[test]
    fn test_op_star() {
        assert_token_kind("*", TokenKind::Star);
    }

    #[test]
    fn test_op_slash() {
        assert_token_kind("/", TokenKind::Slash);
    }

    #[test]
    fn test_op_eq_eq() {
        assert_token_kind("==", TokenKind::EqEq);
    }

    #[test]
    fn test_op_bang_eq() {
        assert_token_kind("!=", TokenKind::BangEq);
    }

    #[test]
    fn test_op_lt() {
        assert_token_kind("<", TokenKind::Lt);
    }

    #[test]
    fn test_op_gt() {
        assert_token_kind(">", TokenKind::Gt);
    }

    #[test]
    fn test_op_lt_eq() {
        assert_token_kind("<=", TokenKind::LtEq);
    }

    #[test]
    fn test_op_gt_eq() {
        assert_token_kind(">=", TokenKind::GtEq);
    }

    #[test]
    fn test_op_amp_amp() {
        assert_token_kind("&&", TokenKind::AmpAmp);
    }

    #[test]
    fn test_op_pipe_pipe() {
        assert_token_kind("||", TokenKind::PipePipe);
    }

    #[test]
    fn test_op_bang() {
        assert_token_kind("!", TokenKind::Bang);
    }

    #[test]
    fn test_op_eq() {
        assert_token_kind("=", TokenKind::Eq);
    }

    #[test]
    fn test_op_colon() {
        assert_token_kind(":", TokenKind::Colon);
    }

    #[test]
    fn test_op_arrow() {
        assert_token_kind("->", TokenKind::Arrow);
    }

    #[test]
    fn test_op_pipe() {
        assert_token_kind("|", TokenKind::Pipe);
    }

    #[test]
    fn test_op_pipe_gt() {
        assert_token_kind("|>", TokenKind::PipeGt);
    }

    #[test]
    fn test_op_question() {
        assert_token_kind("?", TokenKind::Question);
    }

    #[test]
    fn test_op_underscore() {
        assert_token_kind("_", TokenKind::Underscore);
    }

    #[test]
    fn test_op_dot() {
        assert_token_kind(".", TokenKind::Dot);
    }

    #[test]
    fn test_op_comma() {
        assert_token_kind(",", TokenKind::Comma);
    }

    #[test]
    fn test_op_at() {
        assert_token_kind("@", TokenKind::At);
    }

    // =======================================================================
    // Brackets
    // =======================================================================
    #[test]
    fn test_bracket_lparen() {
        assert_token_kind("(", TokenKind::LParen);
    }

    #[test]
    fn test_bracket_rparen() {
        assert_token_kind(")", TokenKind::RParen);
    }

    #[test]
    fn test_bracket_lbrace() {
        assert_token_kind("{", TokenKind::LBrace);
    }

    #[test]
    fn test_bracket_rbrace() {
        assert_token_kind("}", TokenKind::RBrace);
    }

    #[test]
    fn test_bracket_lbracket() {
        assert_token_kind("[", TokenKind::LBracket);
    }

    #[test]
    fn test_bracket_rbracket() {
        assert_token_kind("]", TokenKind::RBracket);
    }

    // =======================================================================
    // Identifiers
    // =======================================================================
    #[test]
    fn test_ident_simple() {
        assert_token_kind("hello", TokenKind::Ident("hello".to_string()));
    }

    #[test]
    fn test_ident_with_leading_underscore() {
        assert_token_kind("_foo", TokenKind::Ident("_foo".to_string()));
    }

    #[test]
    fn test_ident_with_trailing_digits() {
        assert_token_kind("hello123", TokenKind::Ident("hello123".to_string()));
    }

    #[test]
    fn test_ident_single_underscore_is_not_ident() {
        // A lone `_` is the Underscore token, not an identifier.
        // This test documents that distinction.
        assert_token_kind("_", TokenKind::Underscore);
    }

    // =======================================================================
    // Multi-token sequences
    // =======================================================================
    #[test]
    fn test_sequence_fn_declaration() {
        assert_token_sequence(
            "fn add(x: i32) -> i32",
            &[
                TokenKind::Fn,
                TokenKind::Ident("add".to_string()),
                TokenKind::LParen,
                TokenKind::Ident("x".to_string()),
                TokenKind::Colon,
                TokenKind::Ident("i32".to_string()),
                TokenKind::RParen,
                TokenKind::Arrow,
                TokenKind::Ident("i32".to_string()),
            ],
        );
    }

    #[test]
    fn test_sequence_let_binding() {
        assert_token_sequence(
            "let x = 42",
            &[
                TokenKind::Let,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Int(42),
            ],
        );
    }

    #[test]
    fn test_sequence_if_else() {
        assert_token_sequence(
            "if true { x } else { y }",
            &[
                TokenKind::If,
                TokenKind::True,
                TokenKind::LBrace,
                TokenKind::Ident("x".to_string()),
                TokenKind::RBrace,
                TokenKind::Else,
                TokenKind::LBrace,
                TokenKind::Ident("y".to_string()),
                TokenKind::RBrace,
            ],
        );
    }

    #[test]
    fn test_sequence_comparison_chain() {
        assert_token_sequence(
            "a <= b && c >= d",
            &[
                TokenKind::Ident("a".to_string()),
                TokenKind::LtEq,
                TokenKind::Ident("b".to_string()),
                TokenKind::AmpAmp,
                TokenKind::Ident("c".to_string()),
                TokenKind::GtEq,
                TokenKind::Ident("d".to_string()),
            ],
        );
    }

    #[test]
    fn test_sequence_pipe_gt() {
        assert_token_sequence(
            "x |> f",
            &[
                TokenKind::Ident("x".to_string()),
                TokenKind::PipeGt,
                TokenKind::Ident("f".to_string()),
            ],
        );
    }

    // =======================================================================
    // Whitespace handling
    // =======================================================================
    #[test]
    fn test_whitespace_spaces_between_tokens() {
        assert_token_sequence(
            "fn   add (  x  : i32 )",
            &[
                TokenKind::Fn,
                TokenKind::Ident("add".to_string()),
                TokenKind::LParen,
                TokenKind::Ident("x".to_string()),
                TokenKind::Colon,
                TokenKind::Ident("i32".to_string()),
                TokenKind::RParen,
            ],
        );
    }

    #[test]
    fn test_whitespace_tabs_and_newlines() {
        assert_token_sequence(
            "let\tx\n=\n42",
            &[
                TokenKind::Let,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Int(42),
            ],
        );
    }

    #[test]
    fn test_whitespace_leading_and_trailing() {
        assert_token_kind("  fn  ", TokenKind::Fn);
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \t\n  ");
        let token = lexer.next_token().expect("lexing whitespace-only input should succeed");
        assert_eq!(token.kind, TokenKind::Eof);
    }

    // =======================================================================
    // Peek behavior
    // =======================================================================
    #[test]
    fn test_peek_returns_same_token_on_consecutive_calls() {
        let mut lexer = Lexer::new("hello world");

        // First peek should return the first token
        let peeked1 = lexer.peek().cloned();
        assert!(peeked1.is_some(), "first peek should return Some");
        if let Some(ref t) = peeked1 {
            assert_eq!(t.kind, TokenKind::Ident("hello".to_string()));
        }

        // Second peek should return the SAME token (lazy — doesn't advance)
        let peeked2 = lexer.peek().cloned();
        assert_eq!(
            peeked1, peeked2,
            "consecutive peek() calls should return the same token"
        );

        // Now consume the peeked token
        let consumed = lexer.next_token().expect("next_token after peek should succeed");
        assert_eq!(consumed.kind, TokenKind::Ident("hello".to_string()));

        // After advancing, peek should now show the next token
        let peeked3 = lexer.peek().cloned();
        assert!(peeked3.is_some(), "peek after advancing should return next token");
        if let Some(ref t) = peeked3 {
            assert_eq!(t.kind, TokenKind::Ident("world".to_string()));
        }
    }

    #[test]
    fn test_peek_after_eof() {
        let mut lexer = Lexer::new("x");

        // Consume the only token
        let _ = lexer.next_token();
        // EOF
        let eof = lexer.next_token().expect("EOF should succeed");
        assert_eq!(eof.kind, TokenKind::Eof);

        // Peek at EOF should return Some(&Token { kind: Eof }) or None — but
        // the important thing is it doesn't panic and is consistent
        let _peek_result = lexer.peek();
        let next = lexer.next_token().expect("subsequent next_token should succeed");
        assert_eq!(next.kind, TokenKind::Eof);
    }

    // =======================================================================
    // EOF after consuming all tokens
    // =======================================================================
    #[test]
    fn test_eof_after_single_token() {
        let mut lexer = Lexer::new("fn");
        let _first = lexer.next_token();
        let eof1 = lexer.next_token().expect("first EOF should succeed");
        assert_eq!(eof1.kind, TokenKind::Eof);
    }

    #[test]
    fn test_eof_is_sticky() {
        let mut lexer = Lexer::new("fn");
        let _first = lexer.next_token();
        let _eof1 = lexer.next_token();
        let eof2 = lexer.next_token().expect("second EOF should succeed");
        assert_eq!(eof2.kind, TokenKind::Eof);
        let eof3 = lexer.next_token().expect("third EOF should succeed");
        assert_eq!(eof3.kind, TokenKind::Eof);
    }

    #[test]
    fn test_eof_after_multi_token_input() {
        let mut lexer = Lexer::new("a + b");
        let _a = lexer.next_token();
        let _plus = lexer.next_token();
        let _b = lexer.next_token();
        let eof = lexer.next_token().expect("EOF after consuming all tokens should succeed");
        assert_eq!(eof.kind, TokenKind::Eof);
    }

    // =======================================================================
    // Spans
    // =======================================================================
    #[test]
    fn test_span_basic() {
        let mut lexer = Lexer::new("fn");
        let token = lexer.next_token().expect("should lex 'fn'");
        assert_eq!(token.kind, TokenKind::Fn);
        assert_eq!(token.span.start, 0);
        assert_eq!(token.span.end, 2);
    }

    #[test]
    fn test_span_after_whitespace() {
        let mut lexer = Lexer::new("  let");
        let token = lexer.next_token().expect("should lex 'let' after whitespace");
        assert_eq!(token.kind, TokenKind::Let);
        assert_eq!(token.span.start, 2);
        assert_eq!(token.span.end, 5);
    }

    #[test]
    fn test_span_multi_token() {
        let mut lexer = Lexer::new("a + b");
        let a = lexer.next_token().expect("a");
        assert_eq!(a.span.start, 0);
        assert_eq!(a.span.end, 1);

        let plus = lexer.next_token().expect("+");
        assert_eq!(plus.span.start, 2);
        assert_eq!(plus.span.end, 3);

        let b = lexer.next_token().expect("b");
        assert_eq!(b.span.start, 4);
        assert_eq!(b.span.end, 5);
    }
}
