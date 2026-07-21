//! Pass wrappers for the lexer.

use crate::{Lexer, LexError};
use dwarf_syntax::token::Token;

/// A pass that tokenizes source text into a token stream.
pub struct TokenizePass;

impl TokenizePass {
    /// Tokenize the given source text.
    /// Returns a vector of all tokens, ending with Eof.
    pub fn tokenize(&self, input: &str) -> Result<Vec<Token>, LexError> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            let is_eof = token.kind == dwarf_syntax::token::TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }
}
