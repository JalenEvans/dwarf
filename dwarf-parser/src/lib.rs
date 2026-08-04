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
/// Set to 64 (±11 frames per level → ~700 peak frames) so that even
/// expression parsing (~11 frames/level) stays safely within a 2 MiB
/// debug-mode stack without requiring untenably deep test inputs.
const MAX_DEPTH: usize = 64;

/// An error produced by the parser when it encounters invalid syntax.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub code: &'static str,
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
    /// boundary (fn, type, interface, import, extern, const, @, pub, or eof).
    fn sync_to_declaration_boundary(&mut self) {
        while !self.is_at_end() {
            match &self.peek().kind {
                TokenKind::Fn
                | TokenKind::Type
                | TokenKind::Interface
                | TokenKind::Import
                | TokenKind::Extern
                | TokenKind::Const
                | TokenKind::Enum
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
            code: "DWARF-E-PARSE-0003",
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
            TokenKind::Import => self.parse_import(is_pub),
            TokenKind::Fn => self.parse_function(is_pub),
            TokenKind::Extern => self.parse_extern(is_pub),
            TokenKind::Const => self.parse_const(is_pub),
            TokenKind::Type => self.parse_type_decl(is_pub),
            TokenKind::Interface => self.parse_interface(is_pub),
            TokenKind::Enum => self.parse_enum(is_pub),
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
                    is_pub,
                    decorators: Vec::new(),
                    span,
                })
            }
        }
    }

    /// Parse `import names from "module"`.
    fn parse_import(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `import`
        let names = self.parse_import_names()?;
        self.consume(TokenKind::From, "expected 'from' after import names")?;
        let module = self.consume_str("expected module path string")?;
        Ok(Decl::Import {
            module,
            names,
            is_pub,
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

    /// Parse a function declaration: `fn name(params) -> ret { body }`
    /// or `fn name(params) -> ret = expr`.
    fn parse_function(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
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

        // Support both `= expr` and `{ stmts }` function bodies
        let body = if self.match_token(TokenKind::Eq) {
            self.parse_expression()?
        } else {
            self.parse_block()?
        };

        Ok(Decl::Function {
            name,
            params,
            return_type,
            body,
            is_pub,
            decorators: Vec::new(),
            span: Span::new(fn_start.file_id, fn_start.start, self.previous().span.end),
        })
    }

    /// Parse an extern declaration: `extern "source" fn name(params) -> ret`
    fn parse_extern(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let extern_start = self.advance().span; // consume `extern`
        let source = self.consume_str("expected source string after 'extern'")?;
        self.consume(TokenKind::Fn, "expected 'fn' after extern source")?;
        let name = self.consume_ident("expected function name")?;

        self.consume(TokenKind::LParen, "expected '(' after function name")?;
        let params = self.parse_params()?;
        self.consume(TokenKind::RParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        Ok(Decl::Extern {
            source,
            name,
            params,
            return_type,
            is_pub,
            span: Span::new(
                extern_start.file_id,
                extern_start.start,
                self.previous().span.end,
            ),
        })
    }

    /// Parse a const declaration: `const name: Type = value` or `const name = value`
    fn parse_const(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let const_start = self.advance().span; // consume `const`
        let name = self.consume_ident("expected constant name")?;

        // Optional type annotation: `: Type`
        let type_ = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenKind::Eq, "expected '=' after constant name")?;
        let value = self.parse_expression()?;

        Ok(Decl::Const {
            name,
            value: Box::new(value),
            type_,
            is_pub,
            span: Span::new(
                const_start.file_id,
                const_start.start,
                self.previous().span.end,
            ),
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
    fn parse_type_decl(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `type`
        let name = self.consume_ident("expected type name")?;

        // Optional `implements` clause: `type Name implements Foo, Bar { ... }`
        let implements = if self.match_token(TokenKind::Implements) {
            self.parse_implements_list()?
        } else {
            vec![]
        };

        // Check for `type Name { ... }` or `type Name implements ... { ... }` syntax (without `=`)
        if self.check(TokenKind::LBrace) {
            // Parse as a type body with fields and methods
            return self.parse_type_body(name, start, is_pub, implements);
        }

        // If we parsed implements but there's no `{`, that's an error
        if !implements.is_empty() {
            return Err(self.error("expected '{' after implements clause"));
        }

        self.consume(TokenKind::Eq, "expected '=' after type name")?;

        if self.check(TokenKind::LBrace) {
            // Parse as a type alias with a record type
            let type_ = self.parse_record_type()?;
            Ok(Decl::TypeDef {
                name,
                type_,
                is_pub,
                span: Span::new(start.file_id, start.start, self.previous().span.end),
            })
        } else if self.is_at_union_start() {
            self.parse_union_def(name, start, is_pub)
        } else {
            let type_ = self.parse_type()?;
            Ok(Decl::TypeDef {
                name,
                type_,
                is_pub,
                span: Span::new(start.file_id, start.start, self.previous().span.end),
            })
        }
    }

    /// Parse a comma-separated list of interface names after `implements`.
    fn parse_implements_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut names = Vec::new();
        names.push(self.consume_ident("expected interface name after 'implements'")?);
        while self.match_token(TokenKind::Comma) {
            names.push(self.consume_ident("expected interface name after ','")?);
        }
        Ok(names)
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

        // If the first identifier is immediately followed by `{`,
        // it is definitely a union variant (even single-variant unions).
        // If followed by `(`, check whether it could be a refinement type
        // (Int(0..100)) rather than a variant payload (Some(Int)).
        if pos < self.tokens.len() {
            match &self.tokens[pos].kind {
                TokenKind::LParen => {
                    // Could be a refinement: Type(int..int)
                    // Check if the tokens inside parens look like a refinement range.
                    // If so, let parse_type() handle it; otherwise treat as union.
                    if pos + 4 < self.tokens.len() {
                        let is_refinement = matches!(&self.tokens[pos + 1].kind, TokenKind::Int(_))
                            && self.tokens[pos + 2].kind == TokenKind::DotDot
                            && matches!(&self.tokens[pos + 3].kind, TokenKind::Int(_))
                            && self.tokens[pos + 4].kind == TokenKind::RParen;
                        if !is_refinement {
                            return true;
                        }
                        // Looks like a refinement — fall through to check for `|`
                    } else {
                        return true;
                    }
                }
                TokenKind::LBrace => return true,
                _ => {}
            }
        }

        // No paren/brace payload on the first identifier (or it was a refinement).
        // Scan forward to see if there is a `|` (indicating at least a second variant).
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

    /// Parse a union definition: `type Name = Variant(Type) | Variant2 | ...`.
    fn parse_union_def(
        &mut self,
        name: String,
        start: Span,
        is_pub: bool,
    ) -> Result<Decl, ParseError> {
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
                    fields
                        .into_iter()
                        .map(|f| (f.name, Box::new(f.type_)))
                        .collect(),
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
            type_params: vec![],
            is_pub,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse an enum definition: `enum Name<T, U> { Var1, Var2(Type), ... }`.
    /// Desugars to `Decl::UnionDef` with optional `type_params`.
    fn parse_enum(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `enum`
        let name = self.consume_ident("expected enum name")?;

        // Optional generic type params: <T, U, ...>
        let type_params = if self.match_token(TokenKind::Lt) {
            let mut params = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                params.push(self.consume_ident("expected type parameter name")?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::Gt, "expected '>' after enum type parameters")?;
            params
        } else {
            vec![]
        };

        self.consume(TokenKind::LBrace, "expected '{' after enum name")?;

        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let var_name = self.consume_ident("expected variant name")?;
            let arg = if self.match_token(TokenKind::LParen) {
                let arg_type = self.parse_type()?;
                self.consume(TokenKind::RParen, "expected ')' after variant arg")?;
                Some(arg_type)
            } else {
                None
            };
            variants.push(Variant {
                name: var_name,
                arg,
            });
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBrace, "expected '}' after enum variants")?;

        Ok(Decl::UnionDef {
            name,
            variants,
            type_params,
            is_pub,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse an interface declaration: `interface Name { fn method(params) -> RetType ... }`.
    /// Interface methods are signatures only — their body is an empty block.
    fn parse_interface(&mut self, is_pub: bool) -> Result<Decl, ParseError> {
        let start = self.advance().span; // consume `interface`
        let name = self.consume_ident("expected interface name")?;
        self.consume(TokenKind::LBrace, "expected '{' after interface name")?;

        let mut methods = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let method = self.parse_interface_method()?;
            methods.push(method);
            // Allow optional comma/semicolon separator
            self.match_token(TokenKind::Comma);
        }

        self.consume(TokenKind::RBrace, "expected '}' after interface body")?;

        Ok(Decl::Interface {
            name,
            methods,
            is_pub,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse an interface method signature: `fn name(params) -> RetType`.
    /// The body is an empty block (signatures have no implementation).
    fn parse_interface_method(&mut self) -> Result<Decl, ParseError> {
        let fn_start = self.advance().span; // consume `fn`
        let name = self.consume_ident("expected method name")?;

        self.consume(TokenKind::LParen, "expected '(' after method name")?;
        let params = self.parse_method_params()?;
        self.consume(TokenKind::RParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Interface methods have no body — use an empty block as placeholder.
        let body = Expr::Block {
            stmts: Vec::new(),
            span: Span::new(fn_start.file_id, fn_start.start, self.previous().span.end),
        };

        Ok(Decl::Function {
            name,
            params,
            return_type,
            body,
            is_pub: false,
            decorators: Vec::new(),
            span: Span::new(fn_start.file_id, fn_start.start, self.previous().span.end),
        })
    }

    /// Parse a decorator: `@name(args?) decl`.
    ///
    /// When the target is a function, the decorator is attached directly to the
    /// function's `decorators` field. Otherwise, it wraps the target in
    /// `Decl::Decorator`.
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

        // Convert Expr args to their string representations for the decorator resolver.
        let string_args: Vec<String> = args
            .iter()
            .map(|arg| match arg {
                Expr::Variable { name, .. } => name.clone(),
                Expr::Literal { value: LiteralValue::Str(s), .. } => format!("\"{}\"", s),
                Expr::Literal { value: LiteralValue::RawStr(s), .. } => s.clone(),
                Expr::Literal { value: LiteralValue::Int(i), .. } => i.to_string(),
                Expr::Literal { value: LiteralValue::Float(f), .. } => f.to_string(),
                Expr::Literal { value: LiteralValue::Bool(b), .. } => b.to_string(),
                Expr::Literal { value: LiteralValue::Null, .. } => "null".to_string(),
                other => format!("{:?}", other),
            })
            .collect();

        // Note: `pub` before the decorated decl is consumed by the caller
        // (`parse`).  We peek past any `pub` here.
        let target_is_pub = self.check_and_advance(TokenKind::Pub);
        let target = Box::new(self.parse_declaration(target_is_pub)?);
        let end = self.previous().span.end;

        // If the target is a function, attach the decorator directly to it.
        if let Decl::Function {
            name: fn_name,
            params,
            return_type,
            body,
            is_pub,
            mut decorators,
            span: fn_span,
        } = *target
        {
            match dwarf_syntax::decorator::parse_decorator_name(&name, &string_args) {
                Ok(decorator) => {
                    decorators.push(decorator);
                    Ok(Decl::Function {
                        name: fn_name,
                        params,
                        return_type,
                        body,
                        is_pub,
                        decorators,
                        span: fn_span,
                    })
                }
                Err(_) => {
                    // Unknown decorator — fall back to wrapping in Decl::Decorator
                    // so downstream code still sees the raw decorator info.
                    let target = Box::new(Decl::Function {
                        name: fn_name,
                        params,
                        return_type,
                        body,
                        is_pub,
                        decorators,
                        span: fn_span,
                    });
                    Ok(Decl::Decorator {
                        name,
                        args,
                        target,
                        is_pub: false,
                        span: Span::new(start.file_id, start.start, end),
                    })
                }
            }
        } else {
            // Non-function target: wrap in Decl::Decorator as before.
            Ok(Decl::Decorator {
                name,
                args,
                target,
                is_pub: false,
                span: Span::new(start.file_id, start.start, end),
            })
        }
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
                code: "DWARF-E-PARSE-0004",
            });
        }
        let result = self.parse_pipe();
        self.depth -= 1;
        result
    }

    fn parse_pipe(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_assign()?;
        while self.match_token(TokenKind::PipeGt) {
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let rhs = self.parse_assign()?;
            let end = self.previous().span.end;
            expr = Expr::Pipe {
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(file_id, start, end),
            };
        }
        Ok(expr)
    }

    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_logical_or()?;
        if self.match_token(TokenKind::Eq) {
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let value = self.parse_assign()?; // right-recursive = right-assoc
            let end = self.previous().span.end;
            return Ok(Expr::Assign {
                target: Box::new(expr),
                value: Box::new(value),
                span: Span::new(file_id, start, end),
            });
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;
        while self.match_token(TokenKind::PipePipe) {
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let rhs = self.parse_logical_and()?;
            let end = self.previous().span.end;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(file_id, start, end),
            };
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        while self.match_token(TokenKind::AmpAmp) {
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let rhs = self.parse_comparison()?;
            let end = self.previous().span.end;
            expr = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(file_id, start, end),
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

        let lhs_file_id = lhs.span().file_id;
        let lhs_start = lhs.span().start;
        let rhs = self.parse_term()?;
        let span_end = rhs.span().end;
        Ok(Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            span: Span::new(lhs_file_id, lhs_start, span_end),
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
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let rhs = self.parse_factor()?;
            let end = self.previous().span.end;
            expr = Expr::Binary {
                op,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(file_id, start, end),
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
            let start = expr.span().start;
            let file_id = expr.span().file_id;
            let rhs = self.parse_unary()?;
            let end = self.previous().span.end;
            expr = Expr::Binary {
                op,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                span: Span::new(file_id, start, end),
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
        
        // Check for record construction: TypeName { field: value, ... }
        // This must happen before the postfix loop to avoid ambiguity with blocks
        if let Expr::Variable { name: _, span: var_span } = &expr {
            if self.check(TokenKind::LBrace) && self.looks_like_record_construction() {
                let start = var_span.start;
                let file_id = var_span.file_id;
                self.advance(); // consume '{'
                let fields = self.parse_record_fields()?;
                self.consume(TokenKind::RBrace, "expected '}' after record fields")?;
                let end = self.previous().span.end;
                expr = Expr::Record {
                    fields,
                    span: Span::new(file_id, start, end),
                };
                // Continue the loop to handle postfix operations on the record
                // e.g., Point { x: 1 }.get_x()
            }
        }
        
        loop {
            if self.match_token(TokenKind::LParen) {
                let start = expr.span().start;
                let file_id = expr.span().file_id;
                let args = self.parse_expr_list(TokenKind::RParen)?;
                self.consume(TokenKind::RParen, "expected ')' after arguments")?;
                let end = self.previous().span.end;
                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    span: Span::new(file_id, start, end),
                };
            } else if self.match_token(TokenKind::Dot) {
                let start = expr.span().start;
                let file_id = expr.span().file_id;
                let field = self.consume_ident("expected field name after '.'")?;
                expr = Expr::Member {
                    obj: Box::new(expr),
                    field,
                    span: Span::new(file_id, start, self.previous().span.end),
                };
            } else if self.match_token(TokenKind::QuestionDot) {
                let start = expr.span().start;
                let file_id = expr.span().file_id;
                let field = self.consume_ident("expected field name after '?.'")?;
                expr = Expr::OptionalAccess {
                    obj: Box::new(expr),
                    field,
                    span: Span::new(file_id, start, self.previous().span.end),
                };
            } else if self.match_token(TokenKind::Question) {
                let start = expr.span().start;
                let file_id = expr.span().file_id;
                expr = Expr::Propagate {
                    expr: Box::new(expr),
                    span: Span::new(file_id, start, self.previous().span.end),
                };
            } else if self.match_token(TokenKind::Bang) {
                let start = expr.span().start;
                let file_id = expr.span().file_id;
                expr = Expr::NonNullAssert {
                    expr: Box::new(expr),
                    span: Span::new(file_id, start, self.previous().span.end),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Lookahead to determine if `{` starts a record construction vs a block.
    /// Returns true if we see: `{}` (empty record) or `{ ident : ...` (field pattern).
    fn looks_like_record_construction(&self) -> bool {
        // Current token is `{`. Check what follows.
        let pos = self.position;
        if pos + 1 >= self.tokens.len() {
            return false;
        }
        
        // Empty record: `{ }`
        if self.tokens[pos + 1].kind == TokenKind::RBrace {
            return true;
        }
        
        // Record with fields: `{ ident : ...`
        if pos + 2 < self.tokens.len() {
            if let TokenKind::Ident(_) = &self.tokens[pos + 1].kind {
                if self.tokens[pos + 2].kind == TokenKind::Colon {
                    return true;
                }
            }
        }
        
        false
    }

    /// Parse comma-separated `field: expr` pairs for record construction.
    fn parse_record_fields(&mut self) -> Result<Vec<(String, Expr)>, ParseError> {
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.consume_ident("expected field name")?;
            self.consume(TokenKind::Colon, "expected ':' after field name")?;
            let value = self.parse_expression()?;
            fields.push((field_name, value));
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.peek().kind {
            TokenKind::Int(val) => {
                let val = *val;
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Int(val),
                    span,
                })
            }
            TokenKind::Float(val) => {
                let val = *val;
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Float(val),
                    span,
                })
            }
            TokenKind::Str(val) => {
                let val = val.clone();
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Str(val),
                    span,
                })
            }
            TokenKind::RawStr(val) => {
                let val = val.clone();
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::RawStr(val),
                    span,
                })
            }
            TokenKind::True => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Bool(true),
                    span,
                })
            }
            TokenKind::False => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Bool(false),
                    span,
                })
            }
            TokenKind::Null => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LiteralValue::Null,
                    span,
                })
            }
            TokenKind::Ident(name) if name == "assert" => {
                let name = name.clone();
                let start = self.peek().span;
                let saved = self.position;
                // Check for assert.consistent(expr) special form via lookahead.
                // Use self.tokens directly to avoid borrowing through self.peek()
                if saved + 3 < self.tokens.len()
                    && self.tokens[saved + 1].kind == TokenKind::Dot
                    && self.tokens[saved + 2].kind == TokenKind::Ident("consistent".to_string())
                    && self.tokens[saved + 3].kind == TokenKind::LParen
                {
                    self.advance(); // consume 'assert'
                    self.advance(); // consume '.'
                    self.advance(); // consume 'consistent'
                    self.advance(); // consume '('
                    let expr = self.parse_expression()?;
                    self.consume(
                        TokenKind::RParen,
                        "expected ')' after assert.consistent expr",
                    )?;
                    let end = self.previous().span.end;
                    return Ok(Expr::AssertConsistent {
                        expr: Box::new(expr),
                        span: Span::new(start.file_id, start.start, end),
                    });
                }
                // Not assert.consistent( — fall through to normal variable.
                let span = self.advance().span;
                Ok(Expr::Variable { name, span })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = self.advance().span;
                Ok(Expr::Variable { name, span })
            }
            TokenKind::Self_ => {
                let span = self.advance().span;
                Ok(Expr::Variable {
                    name: "self".to_string(),
                    span,
                })
            }
            TokenKind::Underscore => {
                let span = self.advance().span;
                Ok(Expr::Wildcard { span })
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Try => self.parse_try_expr(),
            TokenKind::Throw => self.parse_throw_expr(),
            TokenKind::For => self.parse_for_expr(),
            TokenKind::ForAll => self.parse_forall_expr(),
            TokenKind::Pipe => self.parse_lambda(),
            TokenKind::LBrace => self.parse_block(),
            TokenKind::LBracket => self.parse_array_literal(),
            TokenKind::LParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                self.consume(TokenKind::RParen, "expected ')' after expression")?;
                Ok(expr)
            }
            _ => Err(self.error(&format!("unexpected token: {:?}", self.peek().kind))),
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

    fn parse_try_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'try'
        let body = self.parse_expression()?;
        self.consume(TokenKind::Catch, "expected 'catch' after try body")?;
        let binding = self.parse_catch_binding()?;
        let guard = if self.match_token(TokenKind::If) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        let handler = self.parse_expression()?;
        let body = Self::unwrap_single_try_block(body);
        Ok(Expr::Try {
            body: Box::new(body),
            binding,
            guard,
            handler: Box::new(handler),
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_throw_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'throw'
        let expr = self.parse_expression()?;
        Ok(Expr::Throw {
            expr: Box::new(expr),
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
        // 'in' is now a keyword token
        self.consume(TokenKind::In, "expected 'in'")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Expr::For {
            binding,
            iterable: Box::new(iterable),
            body: Box::new(body),
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    fn parse_forall_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.advance().span; // consume 'forAll'
        let type_ = self.parse_type()?;
        self.consume(TokenKind::LBrace, "expected '{' after forAll type")?;
        let binding = self.parse_pattern()?;
        self.consume(TokenKind::Arrow, "expected '->' after forAll binding")?;
        let property = self.parse_expression()?;
        self.consume(TokenKind::RBrace, "expected '}' after forAll property")?;
        Ok(Expr::ForAll {
            type_,
            binding,
            property: Box::new(property),
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
        let start = self.advance().span; // consume '['
        let mut items = Vec::new();
        while !self.check(TokenKind::RBracket) && !self.is_at_end() {
            items.push(self.parse_expression()?);
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::RBracket, "expected ']'")?;
        Ok(Expr::Array {
            items,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
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
                // Check for variant pattern: Some(val)
                if self.match_token(TokenKind::LParen) {
                    let arg = self.parse_pattern()?;
                    self.consume(TokenKind::RParen, "expected ')' after variant pattern arg")?;
                    Ok(Pat::Variant {
                        name,
                        arg: Some(Box::new(arg)),
                    })
                } else if self.match_token(TokenKind::LBrace) {
                    // Record pattern: { field: pat, ... }
                    let mut fields = Vec::new();
                    while !self.check(TokenKind::RBrace) && !self.is_at_end() {
                        let field_name = self.consume_ident("expected field name")?;
                        self.consume(TokenKind::Colon, "expected ':' after field name")?;
                        let field_pat = self.parse_pattern()?;
                        fields.push((field_name, field_pat));
                        self.match_token(TokenKind::Comma);
                    }
                    self.consume(TokenKind::RBrace, "expected '}' after record pattern")?;
                    Ok(Pat::Record {
                        fields,
                        rest: false,
                    })
                } else if name.starts_with(|c: char| c.is_uppercase()) {
                    // Uppercase identifier without payload → unit variant
                    Ok(Pat::Variant { name, arg: None })
                } else {
                    Ok(Pat::Variable(name))
                }
            }
            _ => Err(self.error("expected pattern")),
        }
    }

    /// Parse a catch binding, stopping before the handler body so that `e { ... }`
    /// is interpreted as a variable binding `e` followed by the handler block
    /// rather than a record pattern.
    /// If `expr` is a block containing a single `try` expression, return that
    /// inner expression directly; otherwise return the original expression.
    /// This normalizes the HIR for nested try/catch without changing the meaning
    /// of the parsed syntax.
    fn unwrap_single_try_block(expr: Expr) -> Expr {
        match expr {
            Expr::Block { stmts, span } => {
                let mut stmts = stmts;
                if stmts.len() == 1 {
                    if let Stmt::Expr(inner) = stmts.remove(0) {
                        if matches!(inner, Expr::Try { .. }) {
                            return inner;
                        }
                    }
                }
                Expr::Block { stmts, span }
            }
            other => other,
        }
    }

    fn parse_catch_binding(&mut self) -> Result<Pat, ParseError> {
        match &self.peek().kind {
            TokenKind::Ident(name) if !name.starts_with(|c: char| c.is_uppercase()) => {
                let name = name.clone();
                // A simple variable binding followed directly by `{` (or `if`)
                // is the catch binding; do not treat `e { ... }` as a record pattern.
                if self.position + 1 < self.tokens.len()
                    && matches!(
                        self.tokens[self.position + 1].kind,
                        TokenKind::LBrace | TokenKind::If
                    )
                {
                    self.advance();
                    Ok(Pat::Variable(name))
                } else {
                    self.parse_pattern()
                }
            }
            _ => self.parse_pattern(),
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
                code: "DWARF-E-PARSE-0004",
            });
        }

        let base = self.parse_primary_type()?;
        let result = self.parse_union_suffix(base);
        self.depth -= 1;
        result
    }

    /// Parse a primary type (no union suffix).
    /// Handles: keyof prefix, indexed access postfix, record, func, named (with refinement/generic).
    fn parse_primary_type(&mut self) -> Result<Type, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.depth -= 1;
            return Err(ParseError {
                message: "recursion depth limit exceeded".to_string(),
                span: self.peek().span,
                code: "DWARF-E-PARSE-0004",
            });
        }

        // keyof binds tighter than indexed access — check prefix before base type parsing
        if self.check(TokenKind::KeyOf) {
            self.advance(); // consume 'keyof'
            let inner = self.parse_primary_type()?;
            let inner = self.parse_indexed_access_suffix(inner)?;
            self.depth -= 1;
            return Ok(Type::KeyOf(Box::new(inner)));
        }

        if self.check(TokenKind::LBrace) {
            let result = self.parse_record_type();
            let result = result?;
            let result = self.parse_indexed_access_suffix(result)?;
            self.depth -= 1;
            return Ok(result);
        }
        if self.check(TokenKind::LParen) {
            let result = self.parse_func_type();
            self.depth -= 1;
            return result;
        }

        // Simple named type, possibly with generics and/or indexed access.
        let name = self.consume_ident("expected type name")?;

        // Try refinement type: Name(min..max)
        if let Some(result) = self.try_parse_refinement(name.clone()) {
            let refined = result?;
            let result = self.parse_indexed_access_suffix(refined)?;
            self.depth -= 1;
            return Ok(result);
        }

        // Generic args: Type<T>
        if self.match_token(TokenKind::Lt) {
            let mut args = Vec::new();
            while !self.check(TokenKind::Gt) && !self.is_at_end() {
                args.push(self.parse_type()?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::Gt, "expected '>' after generic args")?;
            let base_type = Type::Generic { base: name, args };
            let result = self.parse_indexed_access_suffix(base_type)?;
            self.depth -= 1;
            return Ok(result);
        }

        // Plain named type with possible indexed access
        let base = Type::Named(name);
        let result = self.parse_indexed_access_suffix(base)?;
        self.depth -= 1;
        Ok(result)
    }

    /// Parse zero or more indexed access suffixes: T["key1"]["key2"]...
    fn parse_indexed_access_suffix(&mut self, base: Type) -> Result<Type, ParseError> {
        let mut current = base;
        while self.check(TokenKind::LBracket) {
            // Peek ahead to confirm this is ["string"] indexed access
            if self.position + 2 < self.tokens.len()
                && matches!(&self.tokens[self.position + 1].kind, TokenKind::Str(_))
                && matches!(&self.tokens[self.position + 2].kind, TokenKind::RBracket)
            {
                self.advance(); // consume '['
                let key = self.consume_str("expected string key in indexed access")?;
                self.consume(TokenKind::RBracket, "expected ']' after indexed access key")?;
                current = Type::IndexedAccess {
                    obj: Box::new(current),
                    key,
                };
            } else {
                break;
            }
        }
        Ok(current)
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

    /// Try to parse a refinement type: Name(min..max)
    /// Returns None if the next tokens don't form a valid refinement,
    /// allowing the caller to fall through to other type forms.
    fn try_parse_refinement(&mut self, base_name: String) -> Option<Result<Type, ParseError>> {
        let save = self.position;

        if !self.check(TokenKind::LParen) {
            return None;
        }
        self.advance(); // consume (

        // Expect int
        let min = match &self.peek().kind {
            TokenKind::Int(v) => *v,
            _ => {
                self.position = save;
                return None;
            }
        };
        self.advance();

        // Expect ..
        if !self.check(TokenKind::DotDot) {
            self.position = save;
            return None;
        }
        self.advance();

        // Expect int
        let max = match &self.peek().kind {
            TokenKind::Int(v) => *v,
            _ => {
                self.position = save;
                return None;
            }
        };
        self.advance();

        // Expect )
        if !self.check(TokenKind::RParen) {
            self.position = save;
            return None;
        }
        self.advance();

        Some(Ok(Type::Refined {
            base: Box::new(Type::Named(base_name)),
            constraint: RefConstraint::Range { min, max },
        }))
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

    /// Parse a type body with fields and methods: `{ field: Type, fn name(...) { ... }, ... }`.
    /// Used for `type Name { ... }` syntax (without `=`).
    fn parse_type_body(
        &mut self,
        name: String,
        start: Span,
        is_pub: bool,
        implements: Vec<String>,
    ) -> Result<Decl, ParseError> {
        self.advance(); // consume '{'
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            // Check if this is a method declaration
            if self.check(TokenKind::Fn) {
                let method = self.parse_method()?;
                methods.push(method);
            } else {
                // Otherwise, it's a field declaration
                let field_name = self.consume_ident("expected field name or method")?;
                self.consume(TokenKind::Colon, "expected ':' after field name")?;
                let type_ = self.parse_type()?;
                fields.push(Field {
                    name: field_name,
                    type_,
                });
            }
            // Allow optional comma separator
            self.match_token(TokenKind::Comma);
        }

        self.consume(TokenKind::RBrace, "expected '}' after type body")?;

        Ok(Decl::RecordDef {
            name,
            fields,
            methods,
            implements,
            is_pub,
            span: Span::new(start.file_id, start.start, self.previous().span.end),
        })
    }

    /// Parse a method declaration inside a type body: `fn name(params) -> ReturnType { body }`.
    fn parse_method(&mut self) -> Result<Decl, ParseError> {
        let fn_start = self.advance().span; // consume `fn`
        let name = self.consume_ident("expected method name")?;

        self.consume(TokenKind::LParen, "expected '(' after method name")?;
        let params = self.parse_method_params()?;
        self.consume(TokenKind::RParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse method body (block expression)
        let body = self.parse_block()?;

        Ok(Decl::Function {
            name,
            params,
            return_type,
            body,
            is_pub: false,
            decorators: Vec::new(),
            span: Span::new(fn_start.file_id, fn_start.start, self.previous().span.end),
        })
    }

    /// Parse method parameters, allowing `self` as a parameter name.
    fn parse_method_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) && !self.is_at_end() {
            params.push(self.parse_method_param()?);
            self.match_token(TokenKind::Comma);
        }
        Ok(params)
    }

    /// Parse a single method parameter, allowing `self` as a parameter name.
    fn parse_method_param(&mut self) -> Result<Param, ParseError> {
        // Allow `self` as a parameter name
        let name = if self.check(TokenKind::Self_) {
            self.advance();
            "self".to_string()
        } else {
            self.consume_ident("expected parameter name")?
        };

        let type_ = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(Param { name, type_ })
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

    // ------------------------------------------------------------------
    // forAll property-test expression tests (DWARF-37)
    //
    // These tests specify the expected shape of the forAll HIR node once
    // the lexer, HIR, and parser have been extended.  They will fail to
    // compile until Expr::ForAll, TokenKind::ForAll, and the parser branch
    // are implemented (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_forall_expr_basic() {
        // forAll Int { x -> x > 0 }
        //
        // At top level, a bare forAll is wrapped in a synthetic function
        // declaration.  The body should be Expr::ForAll { type_: Int,
        // binding: x, property: x > 0 }.
        let tokens = tokenize("forAll Int { x -> x > 0 }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function { body, .. } => match body {
                Expr::ForAll {
                    type_,
                    binding,
                    property,
                    ..
                } => {
                    assert_eq!(*type_, Type::Named("Int".to_string()));
                    assert_eq!(*binding, Pat::Variable("x".to_string()));
                    assert!(matches!(
                        property.as_ref(),
                        Expr::Binary {
                            op: BinaryOp::Gt,
                            ..
                        }
                    ));
                }
                other => panic!("Expected ForAll expression, got {other:?}"),
            },
            other => panic!("Expected synthetic function, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_forall_expr_string() {
        // forAll String { s -> s.length() >= 0 }
        //
        // Verify that string types and member-access properties work.
        let tokens = tokenize("forAll String { s -> s.length() >= 0 }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function { body, .. } => match body {
                Expr::ForAll {
                    type_,
                    binding,
                    property,
                    ..
                } => {
                    assert_eq!(*type_, Type::Named("String".to_string()));
                    assert_eq!(*binding, Pat::Variable("s".to_string()));
                    assert!(matches!(
                        property.as_ref(),
                        Expr::Binary {
                            op: BinaryOp::Ge,
                            ..
                        }
                    ));
                }
                other => panic!("Expected ForAll expression, got {other:?}"),
            },
            other => panic!("Expected synthetic function, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_forall_expr_in_block() {
        // fn test() { forAll Int { x -> x + 1 > x } }
        //
        // When forAll appears inside a function body, it should parse as
        // an expression statement inside the block.
        let tokens = tokenize("fn test() { forAll Int { x -> x + 1 > x } }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function { name, body, .. } => {
                assert_eq!(name, "test");
                match body {
                    Expr::Block { stmts, .. } => {
                        assert_eq!(stmts.len(), 1);
                        match &stmts[0] {
                            Stmt::Expr(Expr::ForAll { type_, binding, .. }) => {
                                assert_eq!(*type_, Type::Named("Int".to_string()));
                                assert_eq!(*binding, Pat::Variable("x".to_string()));
                            }
                            other => {
                                panic!("Expected ForAll statement, got {other:?}")
                            }
                        }
                    }
                    other => panic!("Expected block body, got {other:?}"),
                }
            }
            other => panic!("Expected function declaration, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_forall_with_decorator() {
        // @QuickCheck forAll Int { x -> x > 0 }
        //
        // A decorator applied to a forAll expression — the decorator wraps
        // the synthetic function that contains the forAll.
        let tokens = tokenize("@QuickCheck forAll Int { x -> x > 0 }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Decorator { name, target, .. } => {
                assert_eq!(name, "QuickCheck");
                match target.as_ref() {
                    Decl::Function { body, .. } => {
                        assert!(matches!(body, Expr::ForAll { .. }));
                    }
                    other => {
                        panic!("Expected synthetic function, got {other:?}")
                    }
                }
            }
            other => panic!("Expected decorator declaration, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // @gen decorator tests (DWARF-37: custom generator derivation)
    //
    // These tests verify that the parser handles `@gen(Type)` decorators
    // that signal custom generator functions for property-based testing.
    // The parser already supports `@name(args)` decorator syntax; these
    // tests confirm that `@gen` specifically parses correctly with both
    // function targets and forAll expression targets.
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_gen_decorator_on_fn() {
        // @gen(Color) fn gen_color() -> Color { pure_red() }
        //
        // A @gen decorator wrapping a function declaration. The decorator
        // name should be "gen", the args should contain a reference to the
        // Color type, and the target should be a function named "gen_color"
        // with return type Color.
        let tokens = tokenize("@gen(Color) fn gen_color() -> Color { pure_red() }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Decorator {
                name, args, target, ..
            } => {
                assert_eq!(name, "gen", "decorator name should be 'gen'");
                assert_eq!(args.len(), 1, "should have one type arg");
                match &args[0] {
                    Expr::Variable {
                        name: type_name, ..
                    } => {
                        assert_eq!(type_name, "Color", "type arg should be 'Color'");
                    }
                    other => panic!("Expected type reference as Variable expr, got {other:?}"),
                }
                match target.as_ref() {
                    Decl::Function {
                        name: fn_name,
                        return_type,
                        ..
                    } => {
                        assert_eq!(fn_name, "gen_color", "function name should be 'gen_color'");
                        assert!(
                            return_type
                                .as_ref()
                                .is_some_and(|t| *t == Type::Named("Color".to_string())),
                            "function should return Color"
                        );
                    }
                    other => panic!("Expected function target, got {other:?}"),
                }
            }
            other => panic!("Expected decorator declaration, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_gen_decorator_on_forall_expr() {
        // @gen(Int) forAll Int { x -> x > 0 }
        //
        // A @gen decorator applied to a forAll expression. The decorator
        // wraps the synthetic function that wraps the forAll. The decorator
        // name should be "gen", args should contain Int, and the target
        // synthetic function should contain a ForAll expression in its body.
        let tokens = tokenize("@gen(Int) forAll Int { x -> x > 0 }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Decorator {
                name, args, target, ..
            } => {
                assert_eq!(name, "gen", "decorator name should be 'gen'");
                assert_eq!(args.len(), 1, "should have one type arg");
                match &args[0] {
                    Expr::Variable {
                        name: type_name, ..
                    } => {
                        assert_eq!(type_name, "Int", "type arg should be 'Int'");
                    }
                    other => panic!("Expected type reference as Variable expr, got {other:?}"),
                }
                match target.as_ref() {
                    Decl::Function { body, .. } => match body {
                        Expr::ForAll {
                            type_,
                            binding,
                            property,
                            ..
                        } => {
                            assert_eq!(*type_, Type::Named("Int".to_string()));
                            assert_eq!(*binding, Pat::Variable("x".to_string()));
                            assert!(matches!(
                                property.as_ref(),
                                Expr::Binary {
                                    op: BinaryOp::Gt,
                                    ..
                                }
                            ));
                        }
                        other => panic!("Expected ForAll expression in target body, got {other:?}"),
                    },
                    other => panic!("Expected synthetic function target, got {other:?}"),
                }
            }
            other => panic!("Expected decorator declaration, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // Refinement type parsing tests (DWARF-38: Edge Case Generator)
    //
    // These tests verify that the parser can parse refined types like
    // Int(0..100). They will fail to compile until TokenKind::DotDot,
    // Type::Refined, and RefConstraint are implemented, and will fail at
    // runtime until the lexer and parser are updated to handle `..` and
    // refinement syntax (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_refined_int_range() {
        // Parsing a type alias with a refined type:
        //   type MyInt = Int(0..100)
        //
        // The type alias value should be Type::Refined { base: Int, constraint: Range { 0, 100 } }
        let tokens = tokenize("type MyInt = Int(0..100)");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::TypeDef { name, type_, .. } => {
                assert_eq!(name, "MyInt");
                assert_eq!(
                    *type_,
                    Type::Refined {
                        base: Box::new(Type::Named("Int".to_string())),
                        constraint: RefConstraint::Range { min: 0, max: 100 },
                    }
                );
            }
            other => panic!("Expected TypeDef declaration, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_refined_type_in_fn() {
        // Parsing a function with refined param and return types:
        //   fn test(x: Int(0..100)) -> Int(0..100) { x }
        //
        // The param type and return type should both be Type::Refined.
        let tokens = tokenize("fn test(x: Int(0..100)) -> Int(0..100) { x }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function {
                name,
                params,
                return_type,
                ..
            } => {
                assert_eq!(name, "test");
                assert_eq!(params.len(), 1);

                // Check param type
                let param_type = params[0]
                    .type_
                    .as_ref()
                    .expect("param should have a type annotation");
                assert_eq!(
                    *param_type,
                    Type::Refined {
                        base: Box::new(Type::Named("Int".to_string())),
                        constraint: RefConstraint::Range { min: 0, max: 100 },
                    }
                );

                // Check return type
                let ret = return_type.as_ref().expect("should have return type");
                assert_eq!(
                    *ret,
                    Type::Refined {
                        base: Box::new(Type::Named("Int".to_string())),
                        constraint: RefConstraint::Range { min: 0, max: 100 },
                    }
                );
            }
            other => panic!("Expected Function declaration, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_refined_type_in_record() {
        // Parsing a record definition with refined field types:
        //   type Person = { age: Int(0..150), name: String(1..100) }
        //
        // Both fields should use refined types.
        let tokens = tokenize("type Person = { age: Int(0..150), name: String(1..100) }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::TypeDef { name, type_, .. } => {
                assert_eq!(name, "Person");
                match type_ {
                    Type::Record(fields) => {
                        assert_eq!(fields.len(), 2);

                        // age field: Int(0..150)
                        assert_eq!(fields[0].0, "age");
                        assert_eq!(
                            *fields[0].1,
                            Type::Refined {
                                base: Box::new(Type::Named("Int".to_string())),
                                constraint: RefConstraint::Range { min: 0, max: 150 },
                            }
                        );

                        // name field: String(1..100)
                        assert_eq!(fields[1].0, "name");
                        assert_eq!(
                            *fields[1].1,
                            Type::Refined {
                                base: Box::new(Type::Named("String".to_string())),
                                constraint: RefConstraint::Range { min: 1, max: 100 },
                            }
                        );
                    }
                    other => panic!("Expected Type::Record, got {other:?}"),
                }
            }
            other => panic!("Expected TypeDef declaration, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_refined_type_disjoint_from_func_type() {
        // Verify that (Int, Int) -> Int still parses as a function type,
        // not a refinement. Function types start with '(' and have '->',
        // while refinements are TypeIdent(literal..literal). These should
        // not interfere with each other.
        let tokens = tokenize("type MyFn = (Int, Int) -> Int");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::TypeDef { name, type_, .. } => {
                assert_eq!(name, "MyFn");
                match type_ {
                    Type::Func { params, return_ } => {
                        assert_eq!(params.len(), 2);
                        assert_eq!(params[0], Type::Named("Int".to_string()));
                        assert_eq!(params[1], Type::Named("Int".to_string()));
                        assert_eq!(*return_.as_ref(), Type::Named("Int".to_string()));
                    }
                    other => panic!("Expected Func type, got {other:?}"),
                }
            }
            other => panic!("Expected TypeDef declaration, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // assert.consistent expression tests (DWARF-41)
    //
    // These tests verify that the parser can handle the
    // `assert.consistent(expr)` construct, which marks an expression
    // for cross-target consistency checking. They will fail to compile
    // until Expr::AssertConsistent and the parser branch are implemented
    // (Red phase).
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_assert_consistent() {
        let tokens = tokenize("fn test() { assert.consistent(42) }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function { name, body, .. } => {
                assert_eq!(name, "test");
                match body {
                    Expr::Block { stmts, .. } => {
                        assert_eq!(stmts.len(), 1);
                        match &stmts[0] {
                            Stmt::Expr(Expr::AssertConsistent { expr, .. }) => {
                                match expr.as_ref() {
                                    Expr::Literal { value, .. } => {
                                        assert_eq!(*value, LiteralValue::Int(42));
                                    }
                                    _ => panic!("Expected literal inside assert.consistent"),
                                }
                            }
                            _ => panic!("Expected AssertConsistent expression"),
                        }
                    }
                    _ => panic!("Expected block body"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_assert_consistent_with_expr() {
        let tokens = tokenize("fn test() { assert.consistent(x + 1) }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        match &program[0] {
            Decl::Function { name, body, .. } => {
                assert_eq!(name, "test");
                match body {
                    Expr::Block { stmts, .. } => {
                        assert_eq!(stmts.len(), 1);
                        match &stmts[0] {
                            Stmt::Expr(Expr::AssertConsistent { expr, .. }) => {
                                // The inner expression should be x + 1 (a Binary Add)
                                match expr.as_ref() {
                                    Expr::Binary {
                                        op: BinaryOp::Add,
                                        lhs,
                                        rhs,
                                        ..
                                    } => {
                                        match lhs.as_ref() {
                                            Expr::Variable { name, .. } => {
                                                assert_eq!(name, "x");
                                            }
                                            _ => panic!("Expected variable 'x' on LHS"),
                                        }
                                        match rhs.as_ref() {
                                            Expr::Literal { value, .. } => {
                                                assert_eq!(*value, LiteralValue::Int(1));
                                            }
                                            _ => panic!("Expected literal 1 on RHS"),
                                        }
                                    }
                                    _ => panic!(
                                        "Expected binary expression inside assert.consistent"
                                    ),
                                }
                            }
                            _ => panic!("Expected AssertConsistent expression"),
                        }
                    }
                    _ => panic!("Expected block body"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    // ------------------------------------------------------------------
    // Type body with method declarations tests (DWARF-102)
    //
    // These tests verify that the parser can accept `fn` declarations
    // inside type bodies, enabling object-oriented style type definitions
    // with both fields and methods. They will fail until the parser is
    // extended to handle:
    // 1. `type Name { ... }` syntax (without `=`)
    // 2. `fn` declarations inside type bodies
    // 3. `self` as a method parameter
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_type_body_with_single_method() {
        // type Counter {
        //     count: Int
        //     fn increment(self) -> Int {
        //         self.count + 1
        //     }
        // }
        //
        // A type body with one field and one method. The parser should
        // accept this syntax and produce a declaration with the name "Counter".
        let tokens = tokenize(
            r#"
type Counter {
    count: Int
    fn increment(self) -> Int {
        self.count + 1
    }
}
"#,
        );
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        assert_eq!(program.len(), 1, "expected one type declaration");
        match &program[0] {
            Decl::TypeDef { name, .. } | Decl::RecordDef { name, .. } => {
                assert_eq!(name, "Counter");
            }
            other => panic!("Expected TypeDef or RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_body_with_multiple_methods() {
        // type Math {
        //     fn add(a: Int, b: Int) -> Int { a + b }
        //     fn sub(a: Int, b: Int) -> Int { a - b }
        // }
        //
        // A type body with only methods (no fields). The parser should
        // accept multiple method declarations inside the type body.
        let tokens = tokenize(
            r#"
type Math {
    fn add(a: Int, b: Int) -> Int { a + b }
    fn sub(a: Int, b: Int) -> Int { a - b }
}
"#,
        );
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        assert_eq!(program.len(), 1, "expected one type declaration");
        match &program[0] {
            Decl::TypeDef { name, .. } | Decl::RecordDef { name, .. } => {
                assert_eq!(name, "Math");
            }
            other => panic!("Expected TypeDef or RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_body_with_method_no_return_type() {
        // type Logger {
        //     fn log(msg: Str) {
        //         print(msg)
        //     }
        // }
        //
        // A method with no return type annotation. The parser should
        // accept methods without explicit return types.
        let tokens = tokenize(
            r#"
type Logger {
    fn log(msg: Str) {
        print(msg)
    }
}
"#,
        );
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        assert_eq!(program.len(), 1, "expected one type declaration");
        match &program[0] {
            Decl::TypeDef { name, .. } | Decl::RecordDef { name, .. } => {
                assert_eq!(name, "Logger");
            }
            other => panic!("Expected TypeDef or RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_body_with_only_methods() {
        // type PureCalc {
        //     fn square(x: Int) -> Int { x * x }
        // }
        //
        // A type body with only a single method and no fields. This tests
        // the minimal case of a type with methods.
        let tokens = tokenize(
            r#"
type PureCalc {
    fn square(x: Int) -> Int { x * x }
}
"#,
        );
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        assert_eq!(program.len(), 1, "expected one type declaration");
        match &program[0] {
            Decl::TypeDef { name, .. } | Decl::RecordDef { name, .. } => {
                assert_eq!(name, "PureCalc");
            }
            other => panic!("Expected TypeDef or RecordDef, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_assert_consistent_in_diff_suite() {
        let tokens = tokenize("@Diff(\"ts\") fn test() { assert.consistent(result) }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty());
        assert_eq!(program.len(), 1);
        // The @Diff decorator wraps the function declaration
        match &program[0] {
            Decl::Decorator { name, target, .. } => {
                assert_eq!(name, "Diff");
                match target.as_ref() {
                    Decl::Function {
                        name: fn_name,
                        body,
                        ..
                    } => {
                        assert_eq!(fn_name, "test");
                        match body {
                            Expr::Block { stmts, .. } => {
                                assert_eq!(stmts.len(), 1);
                                match &stmts[0] {
                                    Stmt::Expr(Expr::AssertConsistent { expr, .. }) => {
                                        match expr.as_ref() {
                                            Expr::Variable { name, .. } => {
                                                assert_eq!(name, "result");
                                            }
                                            _ => panic!("Expected variable reference inside assert.consistent"),
                                        }
                                    }
                                    _ => panic!("Expected AssertConsistent expression"),
                                }
                            }
                            _ => panic!("Expected block body"),
                        }
                    }
                    _ => panic!("Expected function target"),
                }
            }
            _ => panic!("Expected decorator declaration"),
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
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

    fn parse_single_body(input: &str) -> Expr {
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(errors.is_empty(), "parse errors: {errors:?}");
        assert_eq!(program.len(), 1, "expected one synthetic top-level decl");
        match &program[0] {
            Decl::Function { body, .. } => body.clone(),
            other => panic!("expected synthetic function, got {other:?}"),
        }
    }

    #[test]
    fn parser_try_catch_string() {
        let body = parse_single_body("try { \"ok\" } catch e { \"fallback\" }");
        match body {
            Expr::Try {
                body,
                binding,
                guard,
                handler,
                ..
            } => {
                assert!(matches!(body.as_ref(), Expr::Block { .. }));
                assert_eq!(binding, Pat::Variable("e".to_string()));
                assert!(guard.is_none());
                assert!(matches!(handler.as_ref(), Expr::Block { .. }));
            }
            other => panic!("expected Expr::Try, got {other:?}"),
        }
    }

    #[test]
    fn parser_try_catch_int() {
        let body = parse_single_body("try { 42 } catch e { \"oops\" }");
        match body {
            Expr::Try { body, .. } => {
                assert!(matches!(body.as_ref(), Expr::Block { .. }));
            }
            other => panic!("expected Expr::Try, got {other:?}"),
        }
    }

    #[test]
    fn parser_throw_call() {
        let body = parse_single_body("throw Error(\"msg\")");
        match body {
            Expr::Throw { expr, .. } => match expr.as_ref() {
                Expr::Call { func, args, .. } => {
                    assert!(
                        matches!(func.as_ref(), Expr::Variable { name, .. } if name == "Error"),
                        "expected Error constructor call"
                    );
                    assert_eq!(args.len(), 1);
                    assert!(
                        matches!(args[0], Expr::Literal { value: LiteralValue::Str(ref s), .. } if s == "msg"),
                        "expected string argument"
                    );
                }
                other => panic!("expected call inside throw, got {other:?}"),
            },
            other => panic!("expected Expr::Throw, got {other:?}"),
        }
    }

    #[test]
    fn parser_guarded_catch() {
        let body = parse_single_body("try { body } catch e if e.code == 1 { handler }");
        match body {
            Expr::Try { guard, .. } => {
                assert!(guard.is_some(), "expected guard expression");
                assert!(
                    matches!(
                        guard.as_ref().unwrap().as_ref(),
                        Expr::Binary {
                            op: BinaryOp::Eq,
                            ..
                        }
                    ),
                    "expected equality guard"
                );
            }
            other => panic!("expected Expr::Try, got {other:?}"),
        }
    }

    #[test]
    fn parser_nested_try_catch() {
        let body = parse_single_body("try { try { inner } catch e { mid } } catch e { outer }");
        match body {
            Expr::Try { body, .. } => {
                assert!(
                    matches!(body.as_ref(), Expr::Try { .. }),
                    "expected nested try/catch in body"
                );
            }
            other => panic!("expected Expr::Try, got {other:?}"),
        }
    }

    #[test]
    fn parser_try_without_catch_reports_error() {
        let tokens = tokenize("try { 42 }");
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse();
        assert!(!errors.is_empty(), "expected parse error for malformed try");
        assert!(program.is_empty(), "expected no declarations on error");
    }
}
