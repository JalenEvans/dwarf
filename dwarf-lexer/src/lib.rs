//! Lexer for the Dwarf compiler.
//! Converts source text into a stream of tokens.

pub mod pass;

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
    /// An invalid escape sequence in a string literal.
    InvalidEscape(usize),
}

use std::fmt;

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedCharacter(ch, pos) => {
                write!(f, "unexpected character '{}' at position {}", ch, pos)
            }
            LexError::UnterminatedString(pos) => {
                write!(
                    f,
                    "unterminated string literal starting at position {}",
                    pos
                )
            }
            LexError::InvalidIntegerLiteral(pos) => {
                write!(f, "invalid integer literal at position {}", pos)
            }
            LexError::InvalidFloatLiteral(pos) => {
                write!(f, "invalid float literal at position {}", pos)
            }
            LexError::InvalidEscape(pos) => {
                write!(f, "invalid escape sequence at position {}", pos)
            }
        }
    }
}

impl std::error::Error for LexError {}

/// A lexical analyzer that converts source text into a stream of tokens.
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    file_id: usize,
    peeked: Option<Token>,
    peeked_err: Option<LexError>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer that will tokenize `input`.
    /// Defaults `file_id` to 0.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            file_id: 0,
            peeked: None,
            peeked_err: None,
        }
    }

    /// Create a new lexer with an explicit `file_id`.
    pub fn with_file_id(input: &'a str, file_id: usize) -> Self {
        Self {
            input,
            position: 0,
            file_id,
            peeked: None,
            peeked_err: None,
        }
    }

    /// Return the next token from the input, advancing the lexer.
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        // If we have a peeked token or error, return it and clear the cache.
        if let Some(token) = self.peeked.take() {
            self.peeked_err = None;
            return Ok(token);
        }
        if let Some(err) = self.peeked_err.take() {
            return Err(err);
        }
        self.skip_whitespace();
        if self.position >= self.input.len() {
            return Ok(Token::new(
                TokenKind::Eof,
                Span::synthetic(self.file_id, self.position),
            ));
        }
        self.lex_token()
    }

    /// Peek at the next token without consuming it.
    /// This is lazy — it only lexes the token if it hasn't been peeked already.
    pub fn peek(&mut self) -> Result<Option<&Token>, &LexError> {
        if self.peeked.is_none() && self.peeked_err.is_none() {
            match self.next_token() {
                Ok(token) => self.peeked = Some(token),
                Err(err) => self.peeked_err = Some(err),
            }
        }
        if let Some(ref token) = self.peeked {
            return Ok(Some(token));
        }
        if let Some(ref err) = self.peeked_err {
            return Err(err);
        }
        Ok(None)
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

    // ---- UTF-8 aware char helpers ----

    /// Return the Unicode character starting at the current position, if any.
    fn current_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// Advance `position` by the byte length of the character at the current
    /// position.  Does nothing if already at EOF.
    fn advance_char(&mut self) {
        if let Some(c) = self.current_char() {
            self.position += c.len_utf8();
        }
    }

    // ---- Comment helpers ----

    /// Skip a line comment.  Assumes `//` has already been consumed.
    /// Advances position past all content until `\n` or EOF (consuming the
    /// newline if present).
    fn skip_line_comment(&mut self) {
        while self.position < self.input.len() {
            if self.input.as_bytes()[self.position] == b'\n' {
                self.position += 1; // consume newline
                break;
            }
            self.position += 1;
        }
    }

    /// Skip a block comment.  Assumes `/*` has already been consumed.
    /// Advances position past `*/`.  If EOF is reached before `*/`, the
    /// comment is truncated silently.
    fn skip_block_comment(&mut self) {
        while self.position + 1 < self.input.len() {
            if self.input.as_bytes()[self.position] == b'*'
                && self.input.as_bytes()[self.position + 1] == b'/'
            {
                self.position += 2; // consume `*/`
                return;
            }
            self.position += 1;
        }
        // EOF before `*/` — skip to end gracefully
        self.position = self.input.len();
    }

    /// Lex a doc comment starting after `///`.  Assumes `///` has already been
    /// consumed.  Consumes everything up to (but not including) the newline or
    /// EOF, and returns a `DocComment` token whose span covers the entire
    /// `/// comment` range.
    fn lex_doc_comment(&mut self, start: usize) -> Result<Token, LexError> {
        while self.position < self.input.len() {
            if self.input.as_bytes()[self.position] == b'\n' {
                break;
            }
            self.position += 1;
        }
        Ok(Token::new(
            TokenKind::DocComment,
            Span::new(self.file_id, start, self.position),
        ))
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
            return Ok(Token::new(
                TokenKind::EqEq,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'!' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::BangEq,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'<' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::LtEq,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'>' && self.next_byte() == Some(b'=') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::GtEq,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'-' && self.next_byte() == Some(b'>') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::Arrow,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'|' && self.next_byte() == Some(b'>') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::PipeGt,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'&' && self.next_byte() == Some(b'&') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::AmpAmp,
                Span::new(self.file_id, start, self.position),
            ));
        }
        if c == b'|' && self.next_byte() == Some(b'|') {
            self.position += 2;
            return Ok(Token::new(
                TokenKind::PipePipe,
                Span::new(self.file_id, start, self.position),
            ));
        }

        // --- Comments and slash (must be checked before single-char `b'/'`) ---

        if c == b'/' {
            match self.next_byte() {
                Some(b'/') => {
                    // Check for `///` (doc comment)
                    if self.position + 2 < self.input.len()
                        && self.input.as_bytes()[self.position + 2] == b'/'
                    {
                        self.position += 3; // skip `///`
                        return self.lex_doc_comment(start);
                    }
                    // `//` line comment — skip without emitting a token
                    self.position += 2; // skip `//`
                    self.skip_line_comment();
                    return self.next_token();
                }
                Some(b'*') => {
                    // `/*` block comment — skip without emitting a token
                    self.position += 2; // skip `/*`
                    self.skip_block_comment();
                    return self.next_token();
                }
                _ => {
                    // Plain `/` operator
                    self.position += 1;
                    return Ok(Token::new(
                        TokenKind::Slash,
                        Span::new(self.file_id, start, self.position),
                    ));
                }
            }
        }

        // --- Single-character tokens ---

        match c {
            b'+' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Plus,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'-' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Minus,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'*' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Star,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'<' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Lt,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'>' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Gt,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'!' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Bang,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'=' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Eq,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'|' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Pipe,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'?' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Question,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'.' => {
                if self.next_byte() == Some(b'.') {
                    self.position += 2;
                    return Ok(Token::new(
                        TokenKind::DotDot,
                        Span::new(self.file_id, start, self.position),
                    ));
                }
                if self.next_byte().is_some_and(|b| b.is_ascii_digit()) {
                    self.lex_number(start)
                } else {
                    self.position += 1;
                    Ok(Token::new(
                        TokenKind::Dot,
                        Span::new(self.file_id, start, self.position),
                    ))
                }
            }
            b',' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Comma,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b':' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::Colon,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'@' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::At,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'(' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::LParen,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b')' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::RParen,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'{' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::LBrace,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'}' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::RBrace,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b'[' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::LBracket,
                    Span::new(self.file_id, start, self.position),
                ))
            }
            b']' => {
                self.position += 1;
                Ok(Token::new(
                    TokenKind::RBracket,
                    Span::new(self.file_id, start, self.position),
                ))
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
                                Span::new(self.file_id, start, self.position),
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
    /// - Floats: `3.5`, `1e10`, `0.5e-3`
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
        if self.curr_byte() == Some(b'.') && self.next_byte().is_some_and(|b| b.is_ascii_digit()) {
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
                    Span::new(self.file_id, start, self.position),
                )),
                Err(_) => Err(LexError::InvalidFloatLiteral(start)),
            }
        } else {
            match clean.parse::<i64>() {
                Ok(n) => Ok(Token::new(
                    TokenKind::Int(n),
                    Span::new(self.file_id, start, self.position),
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
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
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
                Span::new(self.file_id, start, self.position),
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
    ///
    /// Multi-byte UTF-8 characters in the string body are decoded correctly
    /// rather than being pushed byte-by-byte.
    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        let mut value = String::new();
        loop {
            match self.curr_byte() {
                None => return Err(LexError::UnterminatedString(start)),
                Some(b'"') => {
                    self.position += 1; // consume closing `"`
                    return Ok(Token::new(
                        TokenKind::Str(value),
                        Span::new(self.file_id, start, self.position),
                    ));
                }
                Some(b'\\') => {
                    let escape_start = self.position; // position of the backslash
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
                                        return Err(LexError::InvalidEscape(escape_start));
                                    }
                                }
                            }
                            match u8::from_str_radix(&hex, 16) {
                                Ok(byte) => value.push(byte as char),
                                Err(_) => {
                                    return Err(LexError::InvalidEscape(escape_start));
                                }
                            }
                        }
                        Some(_) => {
                            // Unknown escape — include the literal character
                            // (use UTF-8 aware reading for correctness)
                            let ch = self.current_char().unwrap();
                            value.push(ch);
                            self.advance_char();
                        }
                    }
                }
                Some(_) => {
                    // Regular content character — decode UTF-8 correctly
                    let ch = self.current_char().unwrap();
                    value.push(ch);
                    self.advance_char();
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
                        Span::new(self.file_id, start, self.position),
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
    ///
    /// Only ASCII characters are accepted in identifiers.  If a non-ASCII
    /// character is encountered (e.g. `é` in `café`), an
    /// [`LexError::UnexpectedCharacter`] is returned at its byte offset.
    fn lex_identifier_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        let mut first_non_ascii_pos = None;
        while self.position < self.input.len() {
            let b = self.input.as_bytes()[self.position];
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.position += 1;
            } else if b >= 128 {
                // Non-ASCII byte — remember the first occurrence and advance
                // past the full character so position stays correct.
                if first_non_ascii_pos.is_none() {
                    first_non_ascii_pos = Some(self.position);
                }
                self.advance_char();
            } else {
                break;
            }
        }
        // If any non-ASCII content was embedded in the identifier, error out.
        if let Some(pos) = first_non_ascii_pos {
            let ch = self.input[pos..].chars().next().unwrap();
            return Err(LexError::UnexpectedCharacter(ch, pos));
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
            "forAll" => TokenKind::ForAll,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "module" => TokenKind::Module,
            "pub" => TokenKind::Pub,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(word.to_string()),
        };
        Ok(Token::new(
            kind,
            Span::new(self.file_id, start, self.position),
        ))
    }
}
