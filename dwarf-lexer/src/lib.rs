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

        // Check for raw string `r"` before checking identifiers.
        if c == b'r' && self.next_byte() == Some(b'"') {
            self.position += 2; // skip `r` and `"`
            return self.lex_raw_string(start);
        }

        // Check for string literal.
        if c == b'"' {
            self.position += 1; // skip opening `"`
            return self.lex_string(start);
        }

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
                if self.next_byte().is_some_and(|b| b.is_ascii_digit()) {
                    self.lex_number(start)
                } else {
                    self.position += 1;
                    Ok(Token::new(TokenKind::Dot, Span::new(0, start, self.position)))
                }
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

    /// Lex a number literal (integer or float) starting at the current position.
    ///
    /// Supports:
    /// - Decimal integers: `42`, `1_000_000`
    /// - Hex: `0xFF`
    /// - Binary: `0b1010`
    /// - Octal: `0o77`
    /// - Floats: `3.14`, `1e10`, `0.5e-3`
    /// - Floats starting with `.`: `.5`
    fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
        let first_byte = self.input.as_bytes()[start];

        // Check for prefixed integers (0x, 0b, 0o).
        if first_byte == b'0' {
            match self.input.as_bytes().get(start + 1) {
                Some(b'x') | Some(b'X') => return self.lex_radix_int(start, 16),
                Some(b'b') | Some(b'B') => return self.lex_radix_int(start, 2),
                Some(b'o') | Some(b'O') => return self.lex_radix_int(start, 8),
                _ => {}
            }
        }

        // Decimal number (potentially a float).
        self.position = start;
        let mut is_float = false;

        // Parse integer part — skip if number starts with '.' (e.g. `.5`).
        if first_byte != b'.' {
            self.skip_digits_and_underscores();
        }

        // Check for fractional part ('.' followed by a digit).
        if self.curr_byte() == Some(b'.')
            && self.next_byte().is_some_and(|b| b.is_ascii_digit())
        {
            is_float = true;
            self.position += 1; // consume '.'
            self.skip_digits_and_underscores();
        }

        // Check for exponent (e/E optionally followed by +/- and digits).
        if matches!(self.curr_byte(), Some(b'e') | Some(b'E')) {
            let exp_pos = self.position;
            self.position += 1; // consume 'e'/'E'
            if matches!(self.curr_byte(), Some(b'+') | Some(b'-')) {
                self.position += 1;
            }
            if self.curr_byte().is_some_and(|b| b.is_ascii_digit()) {
                is_float = true;
                self.skip_digits_and_underscores();
            } else {
                self.position = exp_pos; // backtrack — not a valid exponent
            }
        }

        let num_text = &self.input[start..self.position];
        let clean: String = num_text.chars().filter(|c| *c != '_').collect();

        if is_float {
            match clean.parse::<f64>() {
                Ok(n) => Ok(Token::new(
                    TokenKind::Float(n),
                    Span::new(0, start, self.position),
                )),
                Err(_) => Err(LexError::InvalidFloatLiteral(start)),
            }
        } else {
            match clean.parse::<i64>() {
                Ok(n) => Ok(Token::new(
                    TokenKind::Int(n),
                    Span::new(0, start, self.position),
                )),
                Err(_) => Err(LexError::InvalidIntegerLiteral(start)),
            }
        }
    }

    /// Lex a prefixed integer literal (hex, binary, octal) starting at the
    /// current position.  `start` points to the leading `0`, and the prefix
    /// (e.g. `0x`) is `prefix_len` bytes.  `base` is 2, 8, or 16.
    fn lex_radix_int(&mut self, start: usize, base: u32) -> Result<Token, LexError> {
        self.position = start + 2; // skip "0x", "0b", or "0o"
        let mut digits = String::new();
        while self.position < self.input.len() {
            let b = self.input.as_bytes()[self.position];
            let is_valid = match base {
                16 => {
                    b.is_ascii_digit()
                        || (b'a'..=b'f').contains(&b)
                        || (b'A'..=b'F').contains(&b)
                }
                2 => b == b'0' || b == b'1',
                8 => (b'0'..=b'7').contains(&b),
                _ => unreachable!(),
            };
            if is_valid {
                digits.push(b as char);
                self.position += 1;
            } else if b == b'_' {
                self.position += 1;
            } else {
                break;
            }
        }
        if digits.is_empty() {
            return Err(LexError::InvalidIntegerLiteral(start));
        }
        match i64::from_str_radix(&digits, base) {
            Ok(n) => Ok(Token::new(
                TokenKind::Int(n),
                Span::new(0, start, self.position),
            )),
            Err(_) => Err(LexError::InvalidIntegerLiteral(start)),
        }
    }

    /// Advance past any sequence of ASCII digits and underscores.
    fn skip_digits_and_underscores(&mut self) {
        while self.position < self.input.len() {
            let b = self.input.as_bytes()[self.position];
            if b.is_ascii_digit() || b == b'_' {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    /// Lex a string literal (escape-processed) starting at the current
    /// position.  The opening `"` has already been consumed.
    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        let mut value = String::new();
        loop {
            match self.curr_byte() {
                None => return Err(LexError::UnterminatedString(start)),
                Some(b'"') => {
                    self.position += 1; // consume closing `"`
                    return Ok(Token::new(
                        TokenKind::Str(value),
                        Span::new(0, start, self.position),
                    ));
                }
                Some(b'\\') => {
                    self.position += 1; // consume backslash
                    match self.curr_byte() {
                        None => return Err(LexError::UnterminatedString(start)),
                        Some(b'n') => {
                            self.position += 1;
                            value.push('\n');
                        }
                        Some(b't') => {
                            self.position += 1;
                            value.push('\t');
                        }
                        Some(b'\\') => {
                            self.position += 1;
                            value.push('\\');
                        }
                        Some(b'"') => {
                            self.position += 1;
                            value.push('"');
                        }
                        Some(b'{') => {
                            self.position += 1;
                            value.push('{');
                        }
                        Some(b'x') => {
                            self.position += 1; // consume 'x'
                            let mut hex = String::with_capacity(2);
                            for _ in 0..2 {
                                match self.curr_byte() {
                                    Some(b)
                                        if b.is_ascii_digit()
                                            || (b'a'..=b'f').contains(&b)
                                            || (b'A'..=b'F').contains(&b) =>
                                    {
                                        hex.push(b as char);
                                        self.position += 1;
                                    }
                                    _ => {
                                        return Err(LexError::UnterminatedString(start));
                                    }
                                }
                            }
                            match u8::from_str_radix(&hex, 16) {
                                Ok(byte) => value.push(byte as char),
                                Err(_) => {
                                    return Err(LexError::UnterminatedString(start));
                                }
                            }
                        }
                        Some(c) => {
                            self.position += 1;
                            value.push(c as char);
                        }
                    }
                }
                Some(b) => {
                    value.push(b as char);
                    self.position += 1;
                }
            }
        }
    }

    /// Lex a raw string literal starting at the current position.
    /// The `r"` has already been consumed; no escape processing is done.
    fn lex_raw_string(&mut self, start: usize) -> Result<Token, LexError> {
        let content_start = self.position;
        loop {
            match self.curr_byte() {
                None => return Err(LexError::UnterminatedString(start)),
                Some(b'"') => {
                    let value = self.input[content_start..self.position].to_string();
                    self.position += 1; // consume closing `"`
                    return Ok(Token::new(
                        TokenKind::RawStr(value),
                        Span::new(0, start, self.position),
                    ));
                }
                Some(_) => {
                    self.position += 1;
                }
            }
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

