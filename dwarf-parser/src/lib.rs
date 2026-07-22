//! Recursive-descent parser that produces HIR from token streams.

pub mod pass;

use dwarf_syntax::hir::*;
use dwarf_syntax::span::Span;
use dwarf_syntax::token::{Token, TokenKind};

/// A recursive-descent parser that converts a token stream into HIR nodes.
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    depth: usize,
}

/// Maximum allowed recursion depth for parsing.
/// Exceeding this produces a `ParseError` instead of a stack overflow.
///
/// Chosen conservatively at 128 so that worst-case debug-mode frames
/// (~12 frames per expression-nesting level × ~1 KB each) fit within
/// a typical 2 MiB Rust test-thread stack.  This is the same limit
/// rustc uses for its parser.
const MAX_DEPTH: usize = 128;

/// An error produced by the parser when it encounters invalid syntax.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl Parser {
    /// Create a new parser from a vector of tokens (including the final Eof).
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            depth: 0,
        }
    }

    /// Parse the full token stream into a list of declarations.
    /// Returns a tuple of (successful declarations, parse errors).
    /// When a declaration fails to parse, the parser enters panic-mode
    /// recovery: it skips tokens until it finds a declaration boundary
    /// (fn, type, import, @, pub, or eof), then continues.
    pub fn parse(&mut self) -> (Vec<Decl>, Vec<ParseError>) {
        let mut decls = Vec::new();
        let mut errors = Vec::new();

        while !self.is_at_end() {
            // Skip doc comments at declaration level
            while self.check(TokenKind::DocComment) {
                self.advance();
            }
            if self.is_at_end() {
                break;
            }

            // The `pub` modifier is consumed *before* the declaration
            let is_pub = self.check_and_advance(TokenKind::Pub);

            match self.parse_declaration(is_pub) {
                Ok(decl) => decls.push(decl),
                Err(e) => {
                    errors.push(e);
                    // Panic-mode recovery: skip to next declaration boundary
                    self.sync_to_declaration_boundary();
                }
            }
        }

        (decls, errors)
    }

    /// Panic-mode recovery: skip tokens until we reach a declaration
    /// boundary (fn, type, import, @, pub, or eof).
    fn sync_to_declaration_boundary(&mut self) {
        while !self.is_at_end() {
            match &self.peek().kind {
                TokenKind::Fn
                | TokenKind::Type
                | TokenKind::Import
                | TokenKind::At
                | TokenKind::Pub => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ========================================================================
    // Core parsing helpers
    // ========================================================================

    /// Return the current token without consuming it.
    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    /// Return the most recently consumed token.
    fn previous(&self) -> &Token {
        let pos = self.position.saturating_sub(1);
        &self.tokens[pos]
    }

    /// Advance one token and return the token that was just consumed.
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.previous()
    }

    /// Check whether the current token has the given kind.
    fn check(&self, kind: TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().kind == kind
    }

    /// Expect and consume a token of the given kind, or return an error.
    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    /// If the current token matches `kind`, consume it and return true.
    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            return true;
        }
        false
    }

    /// If the current token matches any of `kinds`, consume it and return true.
    fn match_any(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.check(kind.clone()) {
                self.advance();
                return true;
            }
        }
        false
    }

    /// Check for a token *and* consume it in one step. Returns whether it was
    /// found.  Convenient for absorbing an optional keyword before a
    /// declaration.
    fn check_and_advance(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// True when the parser has consumed all real tokens.
    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    /// Build a `ParseError` at the current token's location.
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            span: self.peek().span,
        }
    }

    // ========================================================================
    // Declaration parsing
    // ========================================================================

    /// Parse a single declaration.  `is_pub` is true when a `pub` keyword was
    /// consumed before this declaration.
    fn parse_declaration(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        // Decorator: @name(args) target
        if self.check(TokenKind::At) {
            return self.parse_decorator();
        }

        match &self.peek().kind {
            TokenKind::Import => self.parse_import(),
            TokenKind::Fn => self.parse_function(is_pub),
            TokenKind::Type => self.parse_type_decl(),
            _ => {
                // Bare expression at module level — wrap it in a synthetic
                // function so the top-level parse produces at least one decl.
                let expr = self.parse_expression()?;
                let span = self.previous().span;
                Ok(Decl::Function {
                    name: String::new(),
                    params: Vec::new(),
                    return_type: None,
                    body: expr,
                    span,
                })
            }
        }
    }

    /// Parse `import names from "module"`.
    fn parse_import(&mut self) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `import`
        let names = self.parse_import_names()?;
        self.consume(TokenKind::From, "expected 'from' after import names")?;
        let module = self.consume_str("expected module path string")?;
        Ok(Decl::Import {
            module,
            names,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse a (possibly braced) list of imported names: `foo` or `{ a, b }`.
    fn parse_import_names(&mut self) -> Result<Vec<String>, ParseError> {
        let mut names = Vec::new();
        if self.check(TokenKind::LBrace) {
            self.advance(); // consume '{'
            while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                names.push(self.consume_ident("expected identifier in import")?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::RBrace, "expected '}' after import names")?;
        } else {
            names.push(self.consume_ident("expected identifier in import")?);
        }
        Ok(names)
    }

    /// Parse a function declaration: `fn name(params) -> ret { body }`.
    fn parse_function(&mut self, _is_pub: bool) -> Result<Decl, ParseError> {
        let fn_start = self.advance().span; // consume `fn`
        let name = self.consume_ident("expected function name")?;

        self.consume(TokenKind::LParen, "expected '(' after function name")?;
        let params = self.parse_params()?;
        self.consume(TokenKind::RParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Decl::Function {
            name,
            params,
            return_type,
            body,
            span: Span::new(fn_start.file_id, fn_start.start, self.previous().span.end),
        })
    }

    /// Parse a comma-separated list of parameters inside `(...)`.
    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            params.push(self.parse_param()?);
            self.match_token(TokenKind::Comma);
        }
        Ok(params)
    }

    /// Parse a single parameter: `name` or `name: Type`.
    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let name = self.consume_ident("expected parameter name")?;
        let type_ = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(Param { name, type_ })
    }

    /// Parse a type-level declaration: type alias, record, or union.
    fn parse_type_decl(&mut self) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `type`
        let name = self.consume_ident("expected type name")?;
        self.consume(TokenKind::Eq, "expected '=' after type name")?;

        if self.check(TokenKind::LBrace) {
            self.parse_record_def(name, start)
        } else if self.is_at_union_start() {
            self.parse_union_def(name, start)
        } else {
            let type_ = self.parse_type()?;
            Ok(Decl::TypeDef {
                name,
                type_,
                span: Span::new(start.file_id, start.start, self.previous().span.end),
            })
        }
    }

    /// Returns true when the current position looks like a union definition
    /// (variant names with optional payloads, separated by `|`).
    ///
    /// Heuristic: the first token must be an identifier.  If it is followed
    /// by `(` or `{` it is definitely a union variant (even a single-variant
    /// union like `type Wrapper = Wrapped(i32)`).  Otherwise we scan forward
    /// past any payload and check for `|`, but only treat it as a union
    /// definition when the first identifier starts with an uppercase letter
    /// (union variants, e.g. `Red | Green | Blue`) — lowercase identifiers
    /// like `i32 | string` are type-alias union types, not union definitions.
    fn is_at_union_start(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        let mut pos = self.position;

        // First token must be an identifier (the first variant name)
        if pos >= self.tokens.len() {
            return false;
        }
        if !matches!(&self.tokens[pos].kind, TokenKind::Ident(_)) {
            return false;
        }
        pos += 1;

        // If the first identifier is immediately followed by `(` or `{`,
        // it is definitely a union variant (even single-variant unions).
        if pos < self.tokens.len() {
            match &self.tokens[pos].kind {
                TokenKind::LParen => return true,
                TokenKind::LBrace => return true,
                _ => {}
            }
        }

        // No paren/brace payload on the first identifier.  Scan forward to
        // see if there is a `|` (indicating at least a second variant).
        // But only treat this as a union definition if the first identifier
        // starts with uppercase — union variant names start uppercase,
        // while bare type names in union type aliases are lowercase.
        if pos < self.tokens.len() && self.tokens[pos].kind == TokenKind::Pipe {
            if let TokenKind::Ident(name) = &self.tokens[self.position].kind {
                if name.starts_with(|c: char| c.is_uppercase()) {
                    return true;
                }
            }
        }

        false
    }

    /// Parse a record definition: `type Name = { field: Type, ... }`.
    fn parse_record_def(&mut self, name: String, start: Span) -> Result<Decl, ParseError> {
        self.advance(); // consume '{'
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.consume_ident("expected field name")?;
            self.consume(TokenKind::Colon, "expected ':' after field name")?;
            let field_type = self.parse_type()?;
            fields.push(Field {
                name: field_name,
                type_: field_type,
            });
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBrace, "expected '}' after record fields")?;
        Ok(Decl::RecordDef {
            name,
            fields,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse a union definition: `type Name = Variant(Type) | Variant2 | ...`.
    fn parse_union_def(&mut self, name: String, start: Span) -> Result<Decl, ParseError> {
        let mut variants = Vec::new();
        loop {
            let var_name = self.consume_ident("expected variant name")?;
            let arg = if self.match_token(TokenKind::LParen) {
                let arg_type = self.parse_type()?;
                self.consume(TokenKind::RParen, "expected ')' after variant arg")?;
                Some(arg_type)
            } else if self.match_token(TokenKind::LBrace) {
                // Braced variant: Variant { field: Type, ... }
                let mut fields = Vec::new();
                while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                    let field_name = self.consume_ident("expected field name")?;
                    self.consume(TokenKind::Colon, "expected ':' after field name")?;
                    let field_type = self.parse_type()?;
                    fields.push(Field {
                        name: field_name,
                        type_: field_type,
                    });
                    self.match_token(TokenKind::Comma);
                }
                self.consume(TokenKind::RBrace, "expected '}' after variant fields")?;
                // Store braced variant arg as a Record type
                Some(Type::Record(
                    fields.into_iter().map(|f| (f.name, Box::new(f.type_))).collect(),
                ))
            } else {
                None
            };
            variants.push(Variant {
                name: var_name,
                arg,
            });

            if !self.match_token(TokenKind::Pipe) {
                break;
            }
        }
        Ok(Decl::UnionDef {
            name,
            variants,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse a decorator: `@name(args?) decl`.
    fn parse_decorator(&mut self) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume '@'
        let name = self.consume_ident("expected decorator name")?;

        let args = if self.match_token(TokenKind::LParen) {
            let args = self.parse_expr_list(TokenKind::RParen)?;
            self.consume(TokenKind::RParen, "expected ')' after decorator args")?;
            args
        } else {
            Vec::new()
        };

        // Note: `pub` before the decorated decl is consumed by the caller
        // (`parse`).  We peek past any `pub` here.
        let target_is_pub = self.check_and_advance(TokenKind::Pub);
        let target = Box::new(self.parse_declaration(target_is_pub)?);

        Ok(Decl::Decorator {
            name,
            args,
            target,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    // ========================================================================
    // Expression parsing (precedence climbing)
    // ========================================================================
    //
    // Precedence (lowest → highest):
    //   Pipe (|>)
    //   Assign (=)           — right-associative
    //   Logical OR  (||)
    //   Logical AND (&&)
    //   Comparison (== != < > <= >=)
    //   Term       (+ -)
    //   Factor     (* /)
    //   Unary      (- !)
    //   Call / Primary (f() .field ?)

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(ParseError {
                message: "recursion depth limit exceeded".to_string(),
                span: self.peek().span,
            });
        }
        let result = self.parse_pipe();
        self.depth -= 1;
        result
    }

    fn parse_pipe(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_assign()?;
        while self.match_token(TokenKind::PipeGt) {
            let op_span = self.previous().span;
            let rhs = self.parse_assign()?;
            expr = Expr::Pipe {
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(op_span.file_id, op_span.start, self.previous().span.end),
            };
        }
        Ok(expr)
    }

    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_logical_or()?;
        if self.match_token(TokenKind::Eq) {
            let assign_span = self.previous().span;
            let value = self.parse_assign()?; // right-recursive = right-assoc
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span: Span::new(
                    assign_span.file_id,
                    assign_span.start,
                    self.previous().span.end,
                ),
            });
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(TokenKind::PipePipe) {
            let op_span = self.previous().span;
            let rhs = self.parse_logical_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(op_span.file_id, op_span.start, self.previous().span.end),
            };
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        while self.match_token(TokenKind::AmpAmp) {
            let op_span = self.previous().span;
            let rhs = self.parse_comparison()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(op_span.file_id, op_span.start, self.previous().span.end),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_term()?;

        let op = if self.match_any(&[
            TokenKind::EqEq,
            TokenKind::BangEq,
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
        ]) {
            match self.previous().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::Ne,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::LtEq => BinaryOp::Le,
                TokenKind::GtEq => BinaryOp::Ge,
                _ => unreachable!(),
            }
        } else {
            return Ok(lhs);
        };

        let rhs = self.parse_term()?;
        Ok(Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span: Span::new(
                self.previous().span.file_id,
                // best-effort start from the LHS (which may lack a span)
                0,
                self.previous().span.end,
            ),
        })
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        while self.match_any(&[TokenKind::Plus, TokenKind::Minus]) {
            let op = match self.previous().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let rhs = self.parse_factor()?;
            expr = Expr::Binary {
                op,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(
                    self.previous().span.file_id,
                    self.previous().span.start,
                    self.previous().span.end,
                ),
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        while self.match_any(&[TokenKind::Star, TokenKind::Slash]) {
            let op = match self.previous().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            let rhs = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(
                    self.previous().span.file_id,
                    self.previous().span.start,
                    self.previous().span.end,
                ),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_any(&[TokenKind::Minus, TokenKind::Bang]) {
            let op = match self.previous().kind {
                TokenKind::Minus => UnaryOp::Neg,
                TokenKind::Bang => UnaryOp::Not,
                _ => unreachable!(),
            };
            let op_span = self.previous().span;
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
                span: Span::new(op_span.file_id, op_span.start, self.previous().span.end),
            });
        }
        self.parse_call()
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_token(TokenKind::LParen) {
                let args = self.parse_expr_list(TokenKind::RParen)?;
                self.consume(TokenKind::RParen, "expected ')' after arguments")?;
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span: Span::new(
                        self.previous().span.file_id,
                        self.previous().span.start,
                        self.previous().span.end,
                    ),
                };
            } else if self.match_token(TokenKind::Dot) {
                let field = self.consume_ident("expected field name after '.'")?;
                expr = Expr::Member {
                    obj: Box::new(expr),
                    field,
                    span: Span::new(
                        self.previous().span.file_id,
                        self.previous().span.start,
                        self.previous().span.end,
                    ),
                };
            } else if self.match_token(TokenKind::Question) {
                let q_span = self.previous().span;
                expr = Expr::Propagate {
                    expr: Box::new(expr),
                    span: Span::new(q_span.file_id, q_span.start, self.previous().span.end),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.peek().kind {
            TokenKind::Int(val) => {
                let val = *val;
                self.advance();
                Ok(Expr::Literal(LiteralValue::Int(val)))
            }
            TokenKind::Float(val) => {
                let val = *val;
                self.advance();
                Ok(Expr::Literal(LiteralValue::Float(val)))
            }
            TokenKind::Str(val) => {
                let val = val.clone();
                self.advance();
                Ok(Expr::Literal(LiteralValue::Str(val)))
            }
            TokenKind::RawStr(val) => {
                let val = val.clone();
                self.advance();
                Ok(Expr::Literal(LiteralValue::RawStr(val)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Null))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Variable(name))
            }
            TokenKind::Underscore => {
                self.advance();
                Ok(Expr::Wildcard)
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::For => self.parse_for_expr(),
            TokenKind::Pipe => self.parse_lambda(),
            TokenKind::LBrace => self.parse_block(),
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                self.consume(TokenKind::RParen, "expected ')' after expression")?;
                Ok(expr)
            }
            _ => Err(self.error(&format!(
                "unexpected token: {:?}",
                self.peek().kind
            ))),
        }
    }

    // ---- Compound expressions ----

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'if'
        let cond = self.parse_expression()?;
        let then = self.parse_block()?;
        let else_ = if self.match_token(TokenKind::Else) {
            if self.check(TokenKind::If) {
                // else if — chain as a new if-expression
                Some(Box::new(self.parse_if_expr()?))
            } else {
                Some(Box::new(self.parse_block()?))
            }
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then: Box::new(then),
            else_,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'match'
        let expr = self.parse_expression()?;
        self.consume(TokenKind::LBrace, "expected '{' after match expression")?;
        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let arm = self.parse_match_arm()?;
            arms.push(arm);
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBrace, "expected '}' after match arms")?;
        Ok(Expr::Match {
            expr: Box::new(expr),
            arms,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let pattern = self.parse_pattern()?;
        let guard = if self.match_token(TokenKind::If) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        // `=>` is lexed as two separate tokens: `=` then `>`
        self.consume(TokenKind::Eq, "expected '=>' in match arm")?;
        self.consume(TokenKind::Gt, "expected '=>' in match arm")?;
        let body = self.parse_expression()?;
        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }

    fn parse_for_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'for'
        let binding = self.parse_pattern()?;
        // 'in' is lexed as an identifier, not a keyword
        match &self.peek().kind {
            TokenKind::Ident(name) if name == "in" => {
                self.advance();
            }
            _ => return Err(self.error("expected 'in'")),
        }
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Expr::For {
            binding,
            iterable: Box::new(iterable),
            body: Box::new(body),
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume '|'
        let params = self.parse_lambda_params()?;
        self.consume(TokenKind::Pipe, "expected '|' after lambda params")?;
        let body = self.parse_expression()?;
        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_lambda_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(TokenKind::Pipe) && !self.is_at_end() {
            let name = self.consume_ident("expected lambda parameter name")?;
            let type_ = if self.match_token(TokenKind::Colon) {
                // Use a restricted type parser that does NOT consume `|`
                // as a union operator, since `|` closes the lambda parameter
                // list.
                Some(self.parse_lambda_param_type()?)
            } else {
                None
            };
            params.push(Param { name, type_ });
            self.match_token(TokenKind::Comma);
        }
        Ok(params)
    }

    /// Parse a type annotation for lambda params.
    /// Unlike `parse_type`, this does NOT treat `|` as a union operator
    /// because in this context `|` is the closing lambda delimiter.
    fn parse_lambda_param_type(&mut self) -> Result<Type, ParseError> {
        if self.check(TokenKind::LBrace) {
            return self.parse_record_type();
        }
        if self.check(TokenKind::LParen) {
            return self.parse_func_type();
        }
        let name = self.consume_ident("expected type name")?;
        // Generic args: Type<T>
        if self.match_token(TokenKind::Lt) {
            let mut args = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                args.push(self.parse_type()?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::Gt, "expected '>' after generic args")?;
            Ok(Type::Generic { base: name, args })
        } else {
            Ok(Type::Named(name))
        }
    }

    fn parse_block(&mut self) -> Result<Expr, ParseError> {
        let start = self.consume(TokenKind::LBrace, "expected '{'")?.span;
        let mut stmts = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // Skip doc comments inside blocks
            while self.check(TokenKind::DocComment) {
                self.advance();
            }
            if self.check(TokenKind::RBrace) || self.is_at_end() {
                break;
            }

            if self.match_token(TokenKind::Let) {
                // let binding: let pattern = expr
                let pattern = self.parse_pattern()?;
                self.consume(TokenKind::Eq, "expected '=' in let binding")?;
                let value = self.parse_expression()?;
                stmts.push(Stmt::Let(pattern, value));
            } else {
                let expr = self.parse_expression()?;
                stmts.push(Stmt::Expr(expr));
            }
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBrace, "expected '}'")?;
        Ok(Expr::Block {
            stmts,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // consume '['
        let mut exprs = Vec::new();
        while !self.check(TokenKind::RBracket) && !self.is_at_end() {
            exprs.push(self.parse_expression()?);
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBracket, "expected ']'")?;
        Ok(Expr::Array(exprs))
    }

    // ========================================================================
    // Pattern parsing
    // ========================================================================

    fn parse_pattern(&mut self) -> Result<Pat, ParseError> {
        match &self.peek().kind {
            TokenKind::Underscore => {
                self.advance();
                Ok(Pat::Wildcard)
            }
            TokenKind::Int(val) => {
                let val = *val;
                self.advance();
                Ok(Pat::Literal(LiteralValue::Int(val)))
            }
            TokenKind::Float(val) => {
                let val = *val;
                self.advance();
                Ok(Pat::Literal(LiteralValue::Float(val)))
            }
            TokenKind::Str(val) => {
                let val = val.clone();
                self.advance();
                Ok(Pat::Literal(LiteralValue::Str(val)))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pat::Literal(LiteralValue::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pat::Literal(LiteralValue::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Pat::Literal(LiteralValue::Null))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Pat::Variable(name))
            }
            _ => Err(self.error("expected pattern")),
        }
    }

    // ========================================================================
    // Type parsing
    // ========================================================================

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(ParseError {
                message: "recursion depth limit exceeded".to_string(),
                span: self.peek().span,
            });
        }

        if self.check(TokenKind::LBrace) {
            let result = self.parse_record_type();
            self.depth -= 1;
            return result;
        }
        if self.check(TokenKind::LParen) {
            let result = self.parse_func_type();
            self.depth -= 1;
            return result;
        }

        // Simple named type, possibly with generics and/or union suffix.
        let name = self.consume_ident("expected type name")?;

        // Generic args: Type<T>
        if self.match_token(TokenKind::Lt) {
            let mut args = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                args.push(self.parse_type()?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::Gt, "expected '>' after generic args")?;
            let base_type = Type::Generic { base: name, args };

            // Union suffix: Type<A> | B
            let result = self.parse_union_suffix(base_type);
            self.depth -= 1;
            return result;
        }

        // Union suffix: Type1 | Type2
        let result = self.parse_union_suffix(Type::Named(name));
        self.depth -= 1;
        result
    }

    /// If the next token is `|`, parse additional union type members and
    /// return `Type::Union(...)`.  Otherwise return `base` unchanged.
    fn parse_union_suffix(&mut self, base: Type) -> Result<Type, ParseError> {
        if !self.match_token(TokenKind::Pipe) {
            return Ok(base);
        }
        let mut types = vec![base];
        loop {
            types.push(self.parse_type()?);
            if !self.match_token(TokenKind::Pipe) {
                break;
            }
        }
        Ok(Type::Union(types))
    }

    /// Parse a record type: `{ name: Type, ... }`.
    fn parse_record_type(&mut self) -> Result<Type, ParseError> {
        self.advance(); // consume '{'
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let name = self.consume_ident("expected field name")?;
            self.consume(TokenKind::Colon, "expected ':' after field name")?;
            let type_ = self.parse_type()?;
            fields.push((name, Box::new(type_)));
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBrace, "expected '}' after record type fields")?;
        Ok(Type::Record(fields))
    }

    /// Parse a function type: `(Type, ...) -> Type`.
    fn parse_func_type(&mut self) -> Result<Type, ParseError> {
        self.advance(); // consume '('
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            params.push(self.parse_type()?);
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RParen, "expected ')' after function type params")?;
        self.consume(TokenKind::Arrow, "expected '->' in function type")?;
        let return_ = self.parse_type()?;
        Ok(Type::Func {
            params,
            return_: Box::new(return_),
        })
    }

    // ========================================================================
    // Helper methods for consuming value-carrying tokens
    // ========================================================================

    /// Consume an identifier token and return its string value.
    fn consume_ident(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(message)),
        }
    }

    /// Consume a string literal token and return its value.
    fn consume_str(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().kind {
            TokenKind::Str(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error(message)),
        }
    }

    /// Parse a comma-separated list of expressions up to `end_kind`.
    fn parse_expr_list(&mut self, end_kind: TokenKind) -> Result<Vec<Expr>, ParseError> {
        let mut exprs = Vec::new();
        while !self.check(end_kind.clone()) && !self.is_at_end() {
            exprs.push(self.parse_expression()?);
            self.match_token(TokenKind::Comma);
        }
        Ok(exprs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dwarf_lexer::Lexer;

    fn tokenize(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token().unwrap();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    #[test]
    fn test_parse_empty_input() {
        let tokens = tokenize("");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert!(program.is_empty());
    }

    #[test]
    fn test_parse_single_literal_expr() {
        let tokens = tokenize("42");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert!(!program.is_empty());
    }

    #[test]
    fn test_parse_fn_declaration() {
        let tokens = tokenize("fn add(a: i32, b: i32) -> i32 { a + b }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function {
                name,
                params,
                return_type,
                ..
            } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert!(return_type.is_some());
            }
            _ => panic!("Expected Function declaration"),
        }
    }

    #[test]
    fn test_parse_fn_no_params() {
        let tokens = tokenize("fn main() { 42 }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
        assert!(matches!(&program[0], Decl::Function { name, .. } if name == "main"));
    }

    #[test]
    fn test_parse_import_decl() {
        let tokens = tokenize("import math from \"std\"");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
        assert!(matches!(&program[0], Decl::Import { module, .. } if module == "std"));
    }

    #[test]
    fn test_parse_type_alias() {
        let tokens = tokenize("type Age = i32");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
        assert!(matches!(&program[0], Decl::TypeDef { name, .. } if name == "Age"));
    }

    #[test]
    fn test_parse_record_def() {
        let tokens = tokenize("type Person = { name: string, age: i32 }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_union_def() {
        let tokens = tokenize("type Option = Some(value) | None");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_if_expr() {
        let tokens = tokenize("fn test() { if x > 0 { 1 } else { 0 } }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_match_expr() {
        let tokens = tokenize("fn test() { match x { 1 => \"one\", _ => \"other\" } }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_pipe_expr() {
        let tokens = tokenize("fn test() { x |> transform }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_lambda() {
        let tokens = tokenize("fn test() { |x: i32| x + 1 }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_for_loop() {
        let tokens = tokenize("fn test() { for x in items { process(x) } }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_pub_decl() {
        let tokens = tokenize("pub fn add(a: i32) -> i32 { a }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 1);
    }

    #[test]
    fn test_parse_multiple_decls() {
        let tokens = tokenize("fn a() { 1 } fn b() { 2 }");
        let mut parser = Parser::new(tokens);
        let (program, _errors) = parser.parse();
        assert_eq!(program.len(), 2);
    }
}
