//! DWARF-102: self.field member access and instance.method() calls
//! RED Phase tests — expected to fail until parser features are complete.

use dwarf_lexer::Lexer;
use dwarf_parser::Parser;
use dwarf_syntax::hir::*;
use dwarf_syntax::token::TokenKind;

/// Helper: tokenize an input string into a Vec<Token> ending with Eof.
fn tokenize(input: &str) -> Vec<dwarf_syntax::token::Token> {
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

// --- Test 1: self.field member access ---

#[test]
fn test_parse_self_field_member_access() {
    // type Counter {
    //     count: Int
    //     fn get_count(self) -> Int {
    //         self.count
    //     }
    // }
    // → self.count should parse as Member { obj: Variable("self"), field: "count" }
    let source = r#"type Counter {
    count: Int
    fn get_count(self) -> Int {
        self.count
    }
}"#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for self.field access: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one type declaration");

    match &decls[0] {
        Decl::RecordDef {
            name,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Counter");
            assert_eq!(fields.len(), 1, "Should have one field");
            assert_eq!(fields[0].name, "count");
            assert_eq!(methods.len(), 1, "Should have one method");

            // Verify the method body contains self.count
            match &methods[0] {
                Decl::Function {
                    name: method_name,
                    params,
                    body,
                    ..
                } => {
                    assert_eq!(method_name, "get_count");
                    assert_eq!(params.len(), 1, "Method should have one param (self)");
                    assert_eq!(params[0].name, "self");

                    // Extract the body expression
                    if let Expr::Block { stmts, .. } = body {
                        assert!(!stmts.is_empty(), "Method body should not be empty");
                        if let Stmt::Expr(expr) = &stmts[0] {
                            // Should be Member { obj: Variable("self"), field: "count" }
                            match expr {
                                Expr::Member { obj, field, .. } => {
                                    assert_eq!(field, "count", "Field should be \"count\"");
                                    assert!(
                                        matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "self"),
                                        "obj should be Variable(\"self\"), got {:?}",
                                        obj
                                    );
                                }
                                other => panic!(
                                    "Expected Member expression for self.count, got {:?}",
                                    other
                                ),
                            }
                        } else {
                            panic!("Expected Stmt::Expr in method body");
                        }
                    } else {
                        panic!("Expected Block body for method");
                    }
                }
                other => panic!("Expected Function method, got {:?}", other),
            }
        }
        other => panic!("Expected RecordDef, got {:?}", other),
    }
}

// --- Test 2: self.method() call ---

#[test]
fn test_parse_self_method_call() {
    // type Point {
    //     x: Int
    //     fn add(self, other: Point) -> Int {
    //         self.get_x() + other.get_x()
    //     }
    //     fn get_x(self) -> Int {
    //         self.x
    //     }
    // }
    // → self.get_x() should parse as Call { func: Member { obj: Variable("self"), field: "get_x" }, args: [] }
    let source = r#"type Point {
    x: Int
    fn add(self, other: Point) -> Int {
        self.get_x() + other.get_x()
    }
    fn get_x(self) -> Int {
        self.x
    }
}"#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for self.method() call: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one type declaration");

    match &decls[0] {
        Decl::RecordDef { name, methods, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(methods.len(), 2, "Should have two methods");

            // Find the 'add' method
            let add_method = methods
                .iter()
                .find(|m| matches!(m, Decl::Function { name, .. } if name == "add"))
                .expect("Should find 'add' method");

            match add_method {
                Decl::Function { body, .. } => {
                    if let Expr::Block { stmts, .. } = body {
                        assert!(!stmts.is_empty(), "Method body should not be empty");
                        if let Stmt::Expr(expr) = &stmts[0] {
                            // Should be Binary { op: Add, lhs: Call(self.get_x()), rhs: Call(other.get_x()) }
                            match expr {
                                Expr::Binary { lhs, rhs, .. } => {
                                    // Verify lhs is self.get_x()
                                    match lhs.as_ref() {
                                        Expr::Call { func, args, .. } => {
                                            assert!(args.is_empty(), "get_x() should have no args");
                                            match func.as_ref() {
                                                Expr::Member { obj, field, .. } => {
                                                    assert_eq!(field, "get_x");
                                                    assert!(
                                                        matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "self"),
                                                        "obj should be Variable(\"self\")"
                                                    );
                                                }
                                                other => panic!(
                                                    "Expected Member for self.get_x, got {:?}",
                                                    other
                                                ),
                                            }
                                        }
                                        other => panic!(
                                            "Expected Call for self.get_x(), got {:?}",
                                            other
                                        ),
                                    }

                                    // Verify rhs is other.get_x()
                                    match rhs.as_ref() {
                                        Expr::Call { func, args, .. } => {
                                            assert!(args.is_empty(), "get_x() should have no args");
                                            match func.as_ref() {
                                                Expr::Member { obj, field, .. } => {
                                                    assert_eq!(field, "get_x");
                                                    assert!(
                                                        matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "other"),
                                                        "obj should be Variable(\"other\")"
                                                    );
                                                }
                                                other => panic!(
                                                    "Expected Member for other.get_x, got {:?}",
                                                    other
                                                ),
                                            }
                                        }
                                        other => panic!(
                                            "Expected Call for other.get_x(), got {:?}",
                                            other
                                        ),
                                    }
                                }
                                other => panic!("Expected Binary expression, got {:?}", other),
                            }
                        } else {
                            panic!("Expected Stmt::Expr in method body");
                        }
                    } else {
                        panic!("Expected Block body for method");
                    }
                }
                other => panic!("Expected Function method, got {:?}", other),
            }
        }
        other => panic!("Expected RecordDef, got {:?}", other),
    }
}

// --- Test 3: instance.method() on non-self ---

#[test]
fn test_parse_instance_method_call() {
    // fn run() {
    //     let p = Point { x: 1 }
    //     let result = p.get_x()
    // }
    // → p.get_x() should parse as Call { func: Member { obj: Variable("p"), field: "get_x" }, args: [] }
    //
    // NOTE: This test will FAIL because record construction syntax `Point { x: 1 }`
    // is not yet supported by the parser.
    let source = r#"fn run() {
    let p = Point { x: 1 }
    let result = p.get_x()
}"#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for instance.method() call: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one function declaration");

    match &decls[0] {
        Decl::Function { name, body, .. } => {
            assert_eq!(name, "run");

            if let Expr::Block { stmts, .. } = body {
                assert_eq!(stmts.len(), 2, "Should have two statements");

                // Second statement: let result = p.get_x()
                match &stmts[1] {
                    Stmt::Let(_, expr) => {
                        // Should be Call { func: Member { obj: Variable("p"), field: "get_x" }, args: [] }
                        match expr {
                            Expr::Call { func, args, .. } => {
                                assert!(args.is_empty(), "get_x() should have no args");
                                match func.as_ref() {
                                    Expr::Member { obj, field, .. } => {
                                        assert_eq!(field, "get_x", "Field should be \"get_x\"");
                                        assert!(
                                            matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "p"),
                                            "obj should be Variable(\"p\"), got {:?}",
                                            obj
                                        );
                                    }
                                    other => panic!("Expected Member for p.get_x, got {:?}", other),
                                }
                            }
                            other => panic!("Expected Call expression, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Stmt::Let, got {:?}", other),
                }
            } else {
                panic!("Expected Block body for function");
            }
        }
        other => panic!("Expected Function, got {:?}", other),
    }
}

// --- Test 4: Chained method calls ---

#[test]
fn test_parse_chained_method_calls() {
    // fn process(p: Point) -> Int {
    //     p.get_x().add(5)
    // }
    // → Should parse as:
    //   Call {
    //     func: Member {
    //       obj: Call { func: Member { obj: Variable("p"), field: "get_x" }, args: [] },
    //       field: "add"
    //     },
    //     args: [Literal(5)]
    //   }
    let source = r#"fn process(p: Point) -> Int {
    p.get_x().add(5)
}"#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for chained method calls: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one function declaration");

    match &decls[0] {
        Decl::Function { name, body, .. } => {
            assert_eq!(name, "process");

            if let Expr::Block { stmts, .. } = body {
                assert!(!stmts.is_empty(), "Function body should not be empty");
                if let Stmt::Expr(expr) = &stmts[0] {
                    // Outer Call: .add(5)
                    match expr {
                        Expr::Call { func, args, .. } => {
                            assert_eq!(args.len(), 1, "add should have one argument");
                            assert!(
                                matches!(
                                    &args[0],
                                    Expr::Literal {
                                        value: LiteralValue::Int(5),
                                        ..
                                    }
                                ),
                                "Argument should be Int(5), got {:?}",
                                args[0]
                            );

                            // func should be Member { obj: Call(...), field: "add" }
                            match func.as_ref() {
                                Expr::Member { obj, field, .. } => {
                                    assert_eq!(field, "add", "Field should be \"add\"");

                                    // obj should be Call { func: Member { obj: Variable("p"), field: "get_x" }, args: [] }
                                    match obj.as_ref() {
                                        Expr::Call {
                                            func: inner_func,
                                            args: inner_args,
                                            ..
                                        } => {
                                            assert!(
                                                inner_args.is_empty(),
                                                "get_x() should have no args"
                                            );
                                            match inner_func.as_ref() {
                                                Expr::Member {
                                                    obj: inner_obj,
                                                    field: inner_field,
                                                    ..
                                                } => {
                                                    assert_eq!(inner_field, "get_x");
                                                    assert!(
                                                        matches!(inner_obj.as_ref(), Expr::Variable { name, .. } if name == "p"),
                                                        "Inner obj should be Variable(\"p\")"
                                                    );
                                                }
                                                other => panic!(
                                                    "Expected inner Member for p.get_x, got {:?}",
                                                    other
                                                ),
                                            }
                                        }
                                        other => panic!(
                                            "Expected inner Call for p.get_x(), got {:?}",
                                            other
                                        ),
                                    }
                                }
                                other => panic!("Expected Member for .add, got {:?}", other),
                            }
                        }
                        other => panic!("Expected outer Call expression, got {:?}", other),
                    }
                } else {
                    panic!("Expected Stmt::Expr in function body");
                }
            } else {
                panic!("Expected Block body for function");
            }
        }
        other => panic!("Expected Function, got {:?}", other),
    }
}

// --- Test 5: self in match expression ---

#[test]
fn test_parse_self_in_match_expression() {
    // type Value {
    //     data: Int
    //     fn check(self) -> Str {
    //         match self.data {
    //             0 => "zero"
    //             n => "other"
    //         }
    //     }
    // }
    // → self.data in match should parse as Member { obj: Variable("self"), field: "data" }
    let source = r#"type Value {
    data: Int
    fn check(self) -> Str {
        match self.data {
            0 => "zero"
            n => "other"
        }
    }
}"#;
    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();

    assert!(
        errors.is_empty(),
        "No errors expected for self in match: {:?}",
        errors
    );
    assert_eq!(decls.len(), 1, "Should parse one type declaration");

    match &decls[0] {
        Decl::RecordDef { name, methods, .. } => {
            assert_eq!(name, "Value");
            assert_eq!(methods.len(), 1, "Should have one method");

            match &methods[0] {
                Decl::Function {
                    name: method_name,
                    body,
                    ..
                } => {
                    assert_eq!(method_name, "check");

                    if let Expr::Block { stmts, .. } = body {
                        assert!(!stmts.is_empty(), "Method body should not be empty");
                        if let Stmt::Expr(expr) = &stmts[0] {
                            // Should be Match expression
                            match expr {
                                Expr::Match {
                                    expr: match_expr,
                                    arms,
                                    ..
                                } => {
                                    // match_expr should be Member { obj: Variable("self"), field: "data" }
                                    match match_expr.as_ref() {
                                        Expr::Member { obj, field, .. } => {
                                            assert_eq!(field, "data", "Field should be \"data\"");
                                            assert!(
                                                matches!(obj.as_ref(), Expr::Variable { name, .. } if name == "self"),
                                                "obj should be Variable(\"self\"), got {:?}",
                                                obj
                                            );
                                        }
                                        other => {
                                            panic!("Expected Member for self.data, got {:?}", other)
                                        }
                                    }

                                    // Verify arms
                                    assert_eq!(arms.len(), 2, "Should have two match arms");
                                }
                                other => panic!("Expected Match expression, got {:?}", other),
                            }
                        } else {
                            panic!("Expected Stmt::Expr in method body");
                        }
                    } else {
                        panic!("Expected Block body for method");
                    }
                }
                other => panic!("Expected Function method, got {:?}", other),
            }
        }
        other => panic!("Expected RecordDef, got {:?}", other),
    }
}
