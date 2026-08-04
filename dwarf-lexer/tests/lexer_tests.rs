use dwarf_lexer::Lexer;
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
            panic!(
                "lexer error at position {} for input {:?}: {:?}",
                i, input, e
            )
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
    let token = lexer
        .next_token()
        .expect("lexing empty input should succeed");
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
    let token = lexer
        .next_token()
        .expect("lexing whitespace-only input should succeed");
    assert_eq!(token.kind, TokenKind::Eof);
}

// =======================================================================
// Peek behavior
// =======================================================================
#[test]
fn test_peek_returns_same_token_on_consecutive_calls() {
    let mut lexer = Lexer::new("hello world");

    // First peek should return the first token
    let peeked1 = lexer.peek().expect("peek should succeed").cloned();
    assert!(peeked1.is_some(), "first peek should return Some");
    if let Some(ref t) = peeked1 {
        assert_eq!(t.kind, TokenKind::Ident("hello".to_string()));
    }

    // Second peek should return the SAME token (lazy — doesn't advance)
    let peeked2 = lexer.peek().expect("peek should succeed").cloned();
    assert_eq!(
        peeked1, peeked2,
        "consecutive peek() calls should return the same token"
    );

    // Now consume the peeked token
    let consumed = lexer
        .next_token()
        .expect("next_token after peek should succeed");
    assert_eq!(consumed.kind, TokenKind::Ident("hello".to_string()));

    // After advancing, peek should now show the next token
    let peeked3 = lexer.peek().expect("peek should succeed").cloned();
    assert!(
        peeked3.is_some(),
        "peek after advancing should return next token"
    );
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

    // Peek at EOF should return Ok(Some(...)) — but
    // the important thing is it doesn't panic and is consistent
    let _ = lexer.peek();
    let next = lexer
        .next_token()
        .expect("subsequent next_token should succeed");
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
    let eof = lexer
        .next_token()
        .expect("EOF after consuming all tokens should succeed");
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
    let token = lexer
        .next_token()
        .expect("should lex 'let' after whitespace");
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

// =======================================================================
// NEW LITERAL TOKENIZATION TESTS (RED Phase — expected to fail)
// =======================================================================

// -----------------------------------------------------------------------
// Integer literals
// -----------------------------------------------------------------------
#[test]
fn test_int_dec() {
    let mut lexer = Lexer::new("42");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(42));
}

#[test]
fn test_int_hex() {
    let mut lexer = Lexer::new("0xFF");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(255));
}

#[test]
fn test_int_binary() {
    let mut lexer = Lexer::new("0b1010");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(10));
}

#[test]
fn test_int_octal() {
    let mut lexer = Lexer::new("0o77");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(63));
}

#[test]
fn test_int_underscore() {
    let mut lexer = Lexer::new("1_000_000");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(1000000));
}

#[test]
fn test_int_zero() {
    let mut lexer = Lexer::new("0");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(0));
}

// -----------------------------------------------------------------------
// Float literals
// -----------------------------------------------------------------------
#[test]
#[allow(clippy::approx_constant)]
fn test_float_simple() {
    let mut lexer = Lexer::new("3.14");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Float(3.14));
}

#[test]
fn test_float_scientific() {
    let mut lexer = Lexer::new("1e10");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Float(1e10));
}

#[test]
fn test_float_sci_neg() {
    let mut lexer = Lexer::new("0.5e-3");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Float(0.5e-3));
}

// -----------------------------------------------------------------------
// String literals
// -----------------------------------------------------------------------
#[test]
fn test_string_simple() {
    let mut lexer = Lexer::new("\"hello\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("hello".to_string())
    );
}

#[test]
fn test_string_escape_newline() {
    let mut lexer = Lexer::new("\"hello\\nworld\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("hello\nworld".to_string())
    );
}

#[test]
fn test_string_escape_tab() {
    let mut lexer = Lexer::new("\"hello\\tworld\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("hello\tworld".to_string())
    );
}

#[test]
fn test_string_escape_backslash() {
    let mut lexer = Lexer::new("\"path\\\\to\\\\file\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("path\\to\\file".to_string())
    );
}

#[test]
fn test_string_escape_quote() {
    let mut lexer = Lexer::new("\"she said \\\"hi\\\"\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("she said \"hi\"".to_string())
    );
}

#[test]
fn test_string_escape_hex() {
    let mut lexer = Lexer::new("\"\\x48\\x69\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("Hi".to_string())
    );
}

#[test]
fn test_string_interpolation_escape() {
    let mut lexer = Lexer::new("\"escape \\{ curly\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Str("escape { curly".to_string())
    );
}

// -----------------------------------------------------------------------
// Raw string tests
// -----------------------------------------------------------------------
#[test]
fn test_raw_string_simple() {
    let mut lexer = Lexer::new("r\"hello\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::RawStr("hello".to_string())
    );
}

#[test]
fn test_raw_string_with_backslash() {
    let mut lexer = Lexer::new("r\"C:\\\\path\"");
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::RawStr("C:\\\\path".to_string())
    );
}

// -----------------------------------------------------------------------
// Error case tests
// -----------------------------------------------------------------------
#[test]
fn test_unterminated_string() {
    let mut lexer = Lexer::new("\"hello");
    let result = lexer.next_token();
    assert!(result.is_err());
}

#[test]
fn test_unterminated_raw_string() {
    let mut lexer = Lexer::new("r\"hello");
    let result = lexer.next_token();
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// Multi-token with literals
// -----------------------------------------------------------------------
#[test]
fn test_sequence_with_literals() {
    let mut lexer = Lexer::new("let x = 42");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Let);
    assert_eq!(
        lexer.next_token().unwrap().kind,
        TokenKind::Ident("x".to_string())
    );
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eq);
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Int(42));
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}

// =======================================================================
// COMMENT HANDLING (RED Phase — expected to fail)
// =======================================================================

#[test]
fn test_line_comment() {
    let mut lexer = Lexer::new("// this is a comment\nfn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_line_comment_eof() {
    let mut lexer = Lexer::new("// just a comment");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_block_comment() {
    let mut lexer = Lexer::new("/* block comment */fn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_block_comment_multiline() {
    let mut lexer = Lexer::new("/* multi\nline */fn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
}

#[test]
fn test_block_comment_empty() {
    let mut lexer = Lexer::new("/**/fn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
}

#[test]
fn test_doc_comment() {
    let mut lexer = Lexer::new("/// this is a doc comment\nfn");
    let tok = lexer.next_token().unwrap();
    // DocComment tokens should be emitted, not skipped
    assert_eq!(tok.kind, TokenKind::DocComment);
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
}

#[test]
fn test_comment_after_token() {
    let mut lexer = Lexer::new("fn // comment after\nlet");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Let);
}

#[test]
fn test_comments_only() {
    let mut lexer = Lexer::new("// line\n/* block */");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Eof);
}

// =======================================================================
// UTF-8 SUPPORT (RED Phase — expected to fail)
// =======================================================================

#[test]
fn test_utf8_string_content() {
    let mut lexer = Lexer::new("\"hello 世界\"");
    if let TokenKind::Str(s) = lexer.next_token().unwrap().kind {
        assert_eq!(s, "hello 世界");
    } else {
        panic!("Expected Str token");
    }
}

#[test]
fn test_utf8_in_comment() {
    let mut lexer = Lexer::new("// 日本語 comment\nfn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
}

#[test]
fn test_utf8_block_comment() {
    let mut lexer = Lexer::new("/* комментарий */fn");
    assert_eq!(lexer.next_token().unwrap().kind, TokenKind::Fn);
}

#[test]
fn test_non_ascii_identifier_rejected() {
    let mut lexer = Lexer::new("café");
    // Should NOT produce an Ident — non-ASCII in identifiers should error
    let result = lexer.next_token();
    assert!(result.is_err(), "Non-ASCII identifiers should be rejected");
}

// =======================================================================
// SOURCE FILE / SPAN LOCATION TRACKING (RED Phase — expected to fail)
// =======================================================================

#[test]
fn test_span_line_col_tracking() {
    let mut lexer = Lexer::new("fn\nlet\nx");
    // After consuming all tokens, spans should be correct
    let t1 = lexer.next_token().unwrap(); // "fn" at line 1
    let t2 = lexer.next_token().unwrap(); // "let" at line 2
    let t3 = lexer.next_token().unwrap(); // "x" at line 3
                                          // We check spans by comparing byte offsets (easier)
    assert_eq!(t1.span.start, 0);
    assert_eq!(t2.span.start, 3); // "fn\n" = 3 bytes
    assert_eq!(t3.span.start, 7); // "fn\nlet\n" = 7 bytes
}

#[test]
fn test_span_after_utf8() {
    let mut lexer = Lexer::new("\"héllo\"");
    let tok = lexer.next_token().unwrap();
    // String content is "héllo" — the span tracks byte offsets not char offsets
    assert_eq!(tok.span.start, 0);
    // "héllo" is 7 bytes: h=1, é=2, l=1, l=1, o=1, plus 2 quote chars = 8 total
    assert_eq!(tok.span.end, 8);
}

// =======================================================================
// TokenizePass (RED Phase — expected to fail)
// =======================================================================

#[test]
fn test_tokenize_pass_simple_input() {
    use dwarf_lexer::pass::TokenizePass;

    let pass = TokenizePass;
    let result = pass.tokenize("fn main() -> i32");
    assert!(result.is_ok(), "TokenizePass should succeed on valid input");
    let tokens = result.unwrap();
    assert!(!tokens.is_empty(), "Should produce at least one token");
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_tokenize_pass_empty() {
    use dwarf_lexer::pass::TokenizePass;

    let pass = TokenizePass;
    let tokens = pass.tokenize("").unwrap();
    assert_eq!(tokens.len(), 1, "Empty input should produce just Eof");
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

// =======================================================================
// Insta Snapshot Tests
// =======================================================================

#[test]
fn test_snapshot_keywords() {
    let input = "fn type let match if else for import from module pub true false null";
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("keywords", tokens);
}

#[test]
fn test_snapshot_operators() {
    let input = "+ - * / == != < > <= >= && || ! = : -> |> ? _ . , @ ( ) { } [ ]";
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("operators", tokens);
}

#[test]
fn test_snapshot_literals() {
    let input = r#"42 0xFF 0b1010 0o77 1_000_000 3.14 1e10 "hello" r"raw" true false null"#;
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("literals", tokens);
}

#[test]
fn test_snapshot_fn_declaration() {
    let input = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("fn_declaration", tokens);
}

#[test]
fn test_snapshot_comments_and_whitespace() {
    let input = "// line comment\nfn /* block */ let /// doc\nimport";
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("comments_and_whitespace", tokens);
}

#[test]
fn test_snapshot_string_escapes() {
    let input = r#""hello\nworld" "tab\there" "path\\to\\file" "she said \"hi\"" "hex\x48\x69""#;
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().unwrap();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(format!("{:?}", token.kind));
        if is_eof {
            break;
        }
    }
    insta::assert_debug_snapshot!("string_escapes", tokens);
}

// =======================================================================
// EXTERN KEYWORD (RED Phase — expected to fail)
//
// TokenKind::Extern does not exist yet. These tests specify the expected
// lexing behavior for the `extern` keyword in Phase 1 extern/FFI support.
// =======================================================================

#[test]
fn test_keyword_extern() {
    assert_token_kind("extern", TokenKind::Extern);
}

#[test]
fn test_sequence_extern_fn() {
    assert_token_sequence("extern fn", &[TokenKind::Extern, TokenKind::Fn]);
}

#[test]
fn test_sequence_extern_with_source_and_fn() {
    assert_token_sequence(
        r#"extern "npm:express" fn express"#,
        &[
            TokenKind::Extern,
            TokenKind::Str("npm:express".to_string()),
            TokenKind::Fn,
            TokenKind::Ident("express".to_string()),
        ],
    );
}

#[test]
fn test_sequence_extern_full_declaration() {
    assert_token_sequence(
        r#"extern "py:json" fn dumps(obj: Any) -> String"#,
        &[
            TokenKind::Extern,
            TokenKind::Str("py:json".to_string()),
            TokenKind::Fn,
            TokenKind::Ident("dumps".to_string()),
            TokenKind::LParen,
            TokenKind::Ident("obj".to_string()),
            TokenKind::Colon,
            TokenKind::Ident("Any".to_string()),
            TokenKind::RParen,
            TokenKind::Arrow,
            TokenKind::Ident("String".to_string()),
        ],
    );
}

#[test]
fn test_extern_not_an_identifier() {
    // After implementation, `extern` should be a keyword, NOT an Ident
    let mut lexer = Lexer::new("extern");
    let token = lexer.next_token().unwrap();
    assert_ne!(
        token.kind,
        TokenKind::Ident("extern".to_string()),
        "`extern` should be lexed as a keyword, not an identifier"
    );
}

// =======================================================================
// CONST KEYWORD (RED Phase — expected to fail)
//
// TokenKind::Const does not exist yet. These tests specify the expected
// lexing behavior for the `const` keyword in null-safety support.
// =======================================================================

#[test]
fn test_keyword_const() {
    assert_token_kind("const", TokenKind::Const);
}

#[test]
fn test_const_not_an_identifier() {
    // After implementation, `const` should be a keyword, NOT an Ident
    let mut lexer = Lexer::new("const");
    let token = lexer.next_token().unwrap();
    assert_ne!(
        token.kind,
        TokenKind::Ident("const".to_string()),
        "`const` should be lexed as a keyword, not an identifier"
    );
}

#[test]
fn test_sequence_const_binding() {
    assert_token_sequence(
        "const x = 42",
        &[
            TokenKind::Const,
            TokenKind::Ident("x".to_string()),
            TokenKind::Eq,
            TokenKind::Int(42),
        ],
    );
}

#[test]
fn test_sequence_const_with_type_annotation() {
    assert_token_sequence(
        "const x: Int = 42",
        &[
            TokenKind::Const,
            TokenKind::Ident("x".to_string()),
            TokenKind::Colon,
            TokenKind::Ident("Int".to_string()),
            TokenKind::Eq,
            TokenKind::Int(42),
        ],
    );
}

#[test]
fn test_sequence_const_string_value() {
    assert_token_sequence(
        r#"const greeting = "hello""#,
        &[
            TokenKind::Const,
            TokenKind::Ident("greeting".to_string()),
            TokenKind::Eq,
            TokenKind::Str("hello".to_string()),
        ],
    );
}

#[test]
fn test_sequence_pub_const() {
    assert_token_sequence(
        "pub const MAX_SIZE = 100",
        &[
            TokenKind::Pub,
            TokenKind::Const,
            TokenKind::Ident("MAX_SIZE".to_string()),
            TokenKind::Eq,
            TokenKind::Int(100),
        ],
    );
}

// =======================================================================
// OPTIONAL CHAINING `?.` TOKEN (RED Phase — expected to fail)
//
// TokenKind::QuestionDot does not exist yet. These tests specify the
// expected lexing behavior for the `?.` optional chaining operator.
// `?.` must lex as a single compound token, NOT as separate `?` + `.`.
// =======================================================================

#[test]
fn test_op_question_dot() {
    // `?.` should lex as a single QuestionDot token
    assert_token_kind("?.", TokenKind::QuestionDot);
}

#[test]
fn test_question_dot_not_separate_tokens() {
    // `?.` must NOT lex as Question + Dot (two separate tokens)
    let mut lexer = Lexer::new("?.");
    let token = lexer.next_token().unwrap();
    assert_eq!(
        token.kind,
        TokenKind::QuestionDot,
        "`?.` should be a single QuestionDot token, not separate ? and ."
    );
    // After consuming QuestionDot, the next token should be Eof —
    // there should NOT be a leftover Dot token
    let next = lexer.next_token().unwrap();
    assert_eq!(
        next.kind,
        TokenKind::Eof,
        "After QuestionDot, should be Eof (no leftover Dot)"
    );
}

#[test]
fn test_sequence_optional_chain() {
    // `obj?.field` should lex as: Ident("obj"), QuestionDot, Ident("field")
    assert_token_sequence(
        "obj?.field",
        &[
            TokenKind::Ident("obj".to_string()),
            TokenKind::QuestionDot,
            TokenKind::Ident("field".to_string()),
        ],
    );
}

#[test]
fn test_question_dot_span() {
    // `?.` should have a span covering both characters
    let mut lexer = Lexer::new("?.");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, TokenKind::QuestionDot);
    assert_eq!(token.span.start, 0);
    assert_eq!(
        token.span.end, 2,
        "QuestionDot span should cover both '?' and '.'"
    );
}

#[test]
fn test_question_dot_in_expression() {
    // `a?.b?.c` should produce: Ident, QuestionDot, Ident, QuestionDot, Ident
    assert_token_sequence(
        "a?.b?.c",
        &[
            TokenKind::Ident("a".to_string()),
            TokenKind::QuestionDot,
            TokenKind::Ident("b".to_string()),
            TokenKind::QuestionDot,
            TokenKind::Ident("c".to_string()),
        ],
    );
}

#[test]
fn test_question_followed_by_space_then_dot() {
    // `? .` (with space) should still lex as Question + Dot (separate tokens)
    assert_token_sequence("? .", &[TokenKind::Question, TokenKind::Dot]);
}

// =======================================================================
// ENUM KEYWORD (RED Phase — expected to fail)
//
// TokenKind::Enum does not exist yet. These tests specify the expected
// lexing behavior for the `enum` keyword, which is syntactic sugar for
// union types. `enum Color { Red, Green, Blue }` should lex identically
// to the tokens needed for a union definition.
// =======================================================================

#[test]
fn test_keyword_enum() {
    assert_token_kind("enum", TokenKind::Enum);
}

#[test]
fn test_enum_not_an_identifier() {
    // After implementation, `enum` should be a keyword, NOT an Ident
    let mut lexer = Lexer::new("enum");
    let token = lexer.next_token().unwrap();
    assert_ne!(
        token.kind,
        TokenKind::Ident("enum".to_string()),
        "`enum` should be lexed as a keyword, not an identifier"
    );
}

#[test]
fn test_sequence_enum_declaration() {
    // `enum Color { Red, Green, Blue }` should lex as:
    // Enum, Ident("Color"), LBrace, Ident("Red"), Comma, Ident("Green"),
    // Comma, Ident("Blue"), RBrace
    assert_token_sequence(
        "enum Color { Red, Green, Blue }",
        &[
            TokenKind::Enum,
            TokenKind::Ident("Color".to_string()),
            TokenKind::LBrace,
            TokenKind::Ident("Red".to_string()),
            TokenKind::Comma,
            TokenKind::Ident("Green".to_string()),
            TokenKind::Comma,
            TokenKind::Ident("Blue".to_string()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_sequence_pub_enum() {
    assert_token_sequence(
        "pub enum Direction { North, South }",
        &[
            TokenKind::Pub,
            TokenKind::Enum,
            TokenKind::Ident("Direction".to_string()),
            TokenKind::LBrace,
            TokenKind::Ident("North".to_string()),
            TokenKind::Comma,
            TokenKind::Ident("South".to_string()),
            TokenKind::RBrace,
        ],
    );
}

#[test]
fn test_sequence_enum_with_generic() {
    // `enum Option<T> { Some(T), None }` — generic enum with payload variants
    assert_token_sequence(
        "enum Option<T> { Some(T), None }",
        &[
            TokenKind::Enum,
            TokenKind::Ident("Option".to_string()),
            TokenKind::Lt,
            TokenKind::Ident("T".to_string()),
            TokenKind::Gt,
            TokenKind::LBrace,
            TokenKind::Ident("Some".to_string()),
            TokenKind::LParen,
            TokenKind::Ident("T".to_string()),
            TokenKind::RParen,
            TokenKind::Comma,
            TokenKind::Ident("None".to_string()),
            TokenKind::RBrace,
        ],
    );
}
