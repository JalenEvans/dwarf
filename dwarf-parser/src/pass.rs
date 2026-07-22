//! Pass wrappers for the parser.

use crate::{ParseError, Parser};
use dwarf_lexer::Lexer;
use dwarf_syntax::hir::Decl;
use dwarf_syntax::token::TokenKind;

/// A pass that tokenizes and parses source text into HIR declarations.
pub struct ParsePass;

impl ParsePass {
    /// Parse the given source text.
    /// Returns (declarations, errors) — partial HIR even on errors.
    pub fn parse(&self, input: &str) -> Result<(Vec<Decl>, Vec<ParseError>), String> {
        // Tokenize
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Ok(token) => {
                    let is_eof = token.kind == TokenKind::Eof;
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                Err(e) => {
                    return Err(format!("Lexer error: {}", e));
                }
            }
        }

        // Parse
        let mut parser = Parser::new(tokens);
        let (decls, errors) = parser.parse();
        Ok((decls, errors))
    }
}
