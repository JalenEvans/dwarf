use dwarf_syntax::hir::*;

#[test]
fn test_literal_value_variants() {
    let i = LiteralValue::Int(42);
    let f = LiteralValue::Float(2.5);
    let s = LiteralValue::Str("hello".to_string());
    let r = LiteralValue::RawStr("raw".to_string());
    let b = LiteralValue::Bool(true);
    let n = LiteralValue::Null;

    // Just verify they construct — this is a type-check test
    assert!(matches!(i, LiteralValue::Int(42)));
    assert!(matches!(f, LiteralValue::Float(v) if (v - 2.5).abs() < f64::EPSILON));
    assert!(matches!(s, LiteralValue::Str(ref v) if v == "hello"));
    assert!(matches!(r, LiteralValue::RawStr(ref v) if v == "raw"));
    assert!(matches!(b, LiteralValue::Bool(true)));
    assert!(matches!(n, LiteralValue::Null));
}

#[test]
fn test_function_decl_construction() {
    let decl = Decl::Function {
        name: "add".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                type_: Some(Type::Named("i32".to_string())),
            },
            Param {
                name: "b".to_string(),
                type_: Some(Type::Named("i32".to_string())),
            },
        ],
        return_type: Some(Type::Named("i32".to_string())),
        body: Expr::Literal(LiteralValue::Int(0)),
        span: Default::default(),
    };

    if let Decl::Function { name, params, return_type, .. } = &decl {
        assert_eq!(name, "add");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
        assert!(return_type.is_some());
    } else {
        panic!("Expected Function decl");
    }
}

#[test]
fn test_import_decl_construction() {
    let decl = Decl::Import {
        module: "std.io".to_string(),
        names: vec!["println".to_string(), "readln".to_string()],
        span: Default::default(),
    };

    if let Decl::Import { module, names, .. } = &decl {
        assert_eq!(module, "std.io");
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"println".to_string()));
    } else {
        panic!("Expected Import decl");
    }
}

#[test]
fn test_type_def_decl_construction() {
    let decl = Decl::TypeDef {
        name: "MyInt".to_string(),
        type_: Type::Named("i64".to_string()),
        span: Default::default(),
    };

    if let Decl::TypeDef { name, type_, .. } = &decl {
        assert_eq!(name, "MyInt");
        assert!(matches!(type_, Type::Named(t) if t == "i64"));
    } else {
        panic!("Expected TypeDef decl");
    }
}

#[test]
fn test_record_def_decl_construction() {
    let decl = Decl::RecordDef {
        name: "Point".to_string(),
        fields: vec![
            Field {
                name: "x".to_string(),
                type_: Type::Named("f64".to_string()),
            },
            Field {
                name: "y".to_string(),
                type_: Type::Named("f64".to_string()),
            },
        ],
        span: Default::default(),
    };

    if let Decl::RecordDef { name, fields, .. } = &decl {
        assert_eq!(name, "Point");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "x");
        assert_eq!(fields[1].name, "y");
    } else {
        panic!("Expected RecordDef decl");
    }
}

#[test]
fn test_union_def_decl_construction() {
    let decl = Decl::UnionDef {
        name: "Option".to_string(),
        variants: vec![
            Variant {
                name: "Some".to_string(),
                arg: Some(Type::Named("i32".to_string())),
            },
            Variant {
                name: "None".to_string(),
                arg: None,
            },
        ],
        span: Default::default(),
    };

    if let Decl::UnionDef { name, variants, .. } = &decl {
        assert_eq!(name, "Option");
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].name, "Some");
        assert!(variants[0].arg.is_some());
        assert_eq!(variants[1].name, "None");
        assert!(variants[1].arg.is_none());
    } else {
        panic!("Expected UnionDef decl");
    }
}

#[test]
fn test_decorator_decl_construction() {
    let inner = Decl::Function {
        name: "hello".to_string(),
        params: vec![],
        return_type: None,
        body: Expr::Literal(LiteralValue::Null),
        span: Default::default(),
    };

    let decl = Decl::Decorator {
        name: "route".to_string(),
        args: vec![Expr::Literal(LiteralValue::Str("/api".to_string()))],
        target: Box::new(inner),
        span: Default::default(),
    };

    if let Decl::Decorator { name, args, target, .. } = &decl {
        assert_eq!(name, "route");
        assert_eq!(args.len(), 1);
        assert!(matches!(target.as_ref(), Decl::Function { name, .. } if name == "hello"));
    } else {
        panic!("Expected Decorator decl");
    }
}

#[test]
fn test_if_expr_construction() {
    let expr = Expr::If {
        cond: Box::new(Expr::Literal(LiteralValue::Bool(true))),
        then: Box::new(Expr::Literal(LiteralValue::Int(1))),
        else_: Some(Box::new(Expr::Literal(LiteralValue::Int(0)))),
        span: Default::default(),
    };
    assert!(matches!(expr, Expr::If { .. }));
}

#[test]
fn test_if_expr_no_else() {
    let expr = Expr::If {
        cond: Box::new(Expr::Literal(LiteralValue::Bool(true))),
        then: Box::new(Expr::Literal(LiteralValue::Int(1))),
        else_: None,
        span: Default::default(),
    };

    if let Expr::If { cond, then, else_, .. } = &expr {
        assert!(matches!(cond.as_ref(), Expr::Literal(LiteralValue::Bool(true))));
        assert!(matches!(then.as_ref(), Expr::Literal(LiteralValue::Int(1))));
        assert!(else_.is_none());
    } else {
        panic!("Expected If expr");
    }
}

#[test]
fn test_match_expr_construction() {
    let expr = Expr::Match {
        expr: Box::new(Expr::Variable("x".to_string())),
        arms: vec![MatchArm {
            pattern: Pat::Literal(LiteralValue::Int(1)),
            guard: None,
            body: Expr::Literal(LiteralValue::Str("one".to_string())),
        }],
        span: Default::default(),
    };
    assert!(matches!(expr, Expr::Match { .. }));
}

#[test]
fn test_match_expr_with_guard() {
    let expr = Expr::Match {
        expr: Box::new(Expr::Variable("x".to_string())),
        arms: vec![MatchArm {
            pattern: Pat::Variable("n".to_string()),
            guard: Some(Expr::Binary {
                lhs: Box::new(Expr::Variable("n".to_string())),
                op: BinaryOp::Gt,
                rhs: Box::new(Expr::Literal(LiteralValue::Int(0))),
                span: Default::default(),
            }),
            body: Expr::Literal(LiteralValue::Str("positive".to_string())),
        }],
        span: Default::default(),
    };

    if let Expr::Match { arms, .. } = &expr {
        assert_eq!(arms.len(), 1);
        assert!(arms[0].guard.is_some());
        assert!(matches!(&arms[0].pattern, Pat::Variable(n) if n == "n"));
    } else {
        panic!("Expected Match expr");
    }
}

#[test]
fn test_block_expr_with_let_and_expr_stmts() {
    let expr = Expr::Block {
        stmts: vec![
            Stmt::Let(
                Pat::Variable("x".to_string()),
                Expr::Literal(LiteralValue::Int(10)),
            ),
            Stmt::Expr(Expr::Variable("x".to_string())),
        ],
        span: Default::default(),
    };

    if let Expr::Block { stmts, .. } = &expr {
        assert_eq!(stmts.len(), 2);
        assert!(matches!(&stmts[0], Stmt::Let(Pat::Variable(n), _) if n == "x"));
        assert!(matches!(&stmts[1], Stmt::Expr(_)));
    } else {
        panic!("Expected Block expr");
    }
}

#[test]
fn test_pipe_expr() {
    let expr = Expr::Pipe {
        lhs: Box::new(Expr::Literal(LiteralValue::Int(1))),
        rhs: Box::new(Expr::Literal(LiteralValue::Int(2))),
        span: Default::default(),
    };
    assert!(matches!(expr, Expr::Pipe { .. }));
}

#[test]
fn test_propagate_expr() {
    let expr = Expr::Propagate {
        expr: Box::new(Expr::Call {
            func: Box::new(Expr::Variable("read_file".to_string())),
            args: vec![Expr::Literal(LiteralValue::Str("foo.txt".to_string()))],
            span: Default::default(),
        }),
        span: Default::default(),
    };
    assert!(matches!(expr, Expr::Propagate { .. }));
}

#[test]
fn test_for_expr_construction() {
    let expr = Expr::For {
        binding: Pat::Variable("item".to_string()),
        iterable: Box::new(Expr::Variable("items".to_string())),
        body: Box::new(Expr::Literal(LiteralValue::Null)),
        span: Default::default(),
    };

    if let Expr::For { binding, .. } = &expr {
        assert!(matches!(binding, Pat::Variable(n) if n == "item"));
    } else {
        panic!("Expected For expr");
    }
}

#[test]
fn test_assign_expr() {
    let expr = Expr::Assign {
        target: Box::new(Expr::Variable("x".to_string())),
        value: Box::new(Expr::Literal(LiteralValue::Int(42))),
        span: Default::default(),
    };

    if let Expr::Assign { target, value, .. } = &expr {
        assert!(matches!(target.as_ref(), Expr::Variable(name) if name == "x"));
        assert!(matches!(value.as_ref(), Expr::Literal(LiteralValue::Int(42))));
    } else {
        panic!("Expected Assign expr");
    }
}

#[test]
fn test_lambda_expr() {
    let expr = Expr::Lambda {
        params: vec![Param {
            name: "x".to_string(),
            type_: Some(Type::Named("i32".to_string())),
        }],
        body: Box::new(Expr::Binary {
            lhs: Box::new(Expr::Variable("x".to_string())),
            op: BinaryOp::Add,
            rhs: Box::new(Expr::Literal(LiteralValue::Int(1))),
            span: Default::default(),
        }),
        span: Default::default(),
    };

    if let Expr::Lambda { params, body, .. } = &expr {
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "x");
        assert!(matches!(body.as_ref(), Expr::Binary { op: BinaryOp::Add, .. }));
    } else {
        panic!("Expected Lambda expr");
    }
}

#[test]
fn test_record_literal_expr() {
    let expr = Expr::Record {
        fields: vec![
            ("x".to_string(), Expr::Literal(LiteralValue::Int(10))),
            ("y".to_string(), Expr::Literal(LiteralValue::Int(20))),
        ],
        span: Default::default(),
    };

    if let Expr::Record { fields, .. } = &expr {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "x");
        assert!(matches!(fields[0].1, Expr::Literal(LiteralValue::Int(10))));
        assert_eq!(fields[1].0, "y");
    } else {
        panic!("Expected Record expr");
    }
}

#[test]
fn test_variant_literal_expr() {
    let expr = Expr::Variant {
        name: "Some".to_string(),
        arg: Some(Box::new(Expr::Literal(LiteralValue::Int(42)))),
        span: Default::default(),
    };

    if let Expr::Variant { name, arg, .. } = &expr {
        assert_eq!(name, "Some");
        assert!(arg.is_some());
    } else {
        panic!("Expected Variant expr");
    }
}

#[test]
fn test_variant_literal_no_arg() {
    let expr = Expr::Variant {
        name: "None".to_string(),
        arg: None,
        span: Default::default(),
    };

    if let Expr::Variant { name, arg, .. } = &expr {
        assert_eq!(name, "None");
        assert!(arg.is_none());
    } else {
        panic!("Expected Variant expr");
    }
}

#[test]
fn test_array_literal_expr() {
    let expr = Expr::Array(vec![
        Expr::Literal(LiteralValue::Int(1)),
        Expr::Literal(LiteralValue::Int(2)),
        Expr::Literal(LiteralValue::Int(3)),
    ]);

    if let Expr::Array(elements) = &expr {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("Expected Array expr");
    }
}

#[test]
fn test_wildcard_expr() {
    let expr = Expr::Wildcard;
    assert!(matches!(expr, Expr::Wildcard));
}

#[test]
fn test_call_expr() {
    let expr = Expr::Call {
        func: Box::new(Expr::Variable("add".to_string())),
        args: vec![
            Expr::Literal(LiteralValue::Int(1)),
            Expr::Literal(LiteralValue::Int(2)),
        ],
        span: Default::default(),
    };

    if let Expr::Call { func, args, .. } = &expr {
        assert!(matches!(func.as_ref(), Expr::Variable(name) if name == "add"));
        assert_eq!(args.len(), 2);
    } else {
        panic!("Expected Call expr");
    }
}

#[test]
fn test_member_expr() {
    let expr = Expr::Member {
        obj: Box::new(Expr::Variable("point".to_string())),
        field: "x".to_string(),
        span: Default::default(),
    };

    if let Expr::Member { obj, field, .. } = &expr {
        assert!(matches!(obj.as_ref(), Expr::Variable(name) if name == "point"));
        assert_eq!(field, "x");
    } else {
        panic!("Expected Member expr");
    }
}

#[test]
fn test_variable_expr() {
    let expr = Expr::Variable("foo".to_string());
    assert!(matches!(&expr, Expr::Variable(name) if name == "foo"));
}

#[test]
fn test_type_union() {
    let type_ = Type::Union(vec![
        Type::Named("i32".to_string()),
        Type::Named("string".to_string()),
    ]);
    assert!(matches!(type_, Type::Union(_)));
}

#[test]
fn test_type_record() {
    let type_ = Type::Record(vec![
        ("x".to_string(), Box::new(Type::Named("f64".to_string()))),
        ("y".to_string(), Box::new(Type::Named("f64".to_string()))),
    ]);

    if let Type::Record(fields) = &type_ {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "x");
        assert!(matches!(&fields[0].1.as_ref(), Type::Named(n) if n == "f64"));
    } else {
        panic!("Expected Type::Record");
    }
}

#[test]
fn test_type_func() {
    let type_ = Type::Func {
        params: vec![Type::Named("i32".to_string()), Type::Named("i32".to_string())],
        return_: Box::new(Type::Named("i32".to_string())),
    };

    if let Type::Func { params, return_ } = &type_ {
        assert_eq!(params.len(), 2);
        assert!(matches!(return_.as_ref(), Type::Named(n) if n == "i32"));
    } else {
        panic!("Expected Type::Func");
    }
}

#[test]
fn test_type_generic() {
    let type_ = Type::Generic {
        base: "Array".to_string(),
        args: vec![Type::Named("i32".to_string())],
    };

    if let Type::Generic { base, args } = &type_ {
        assert_eq!(base, "Array");
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0], Type::Named(n) if n == "i32"));
    } else {
        panic!("Expected Type::Generic");
    }
}

#[test]
fn test_pattern_variants() {
    let w = Pat::Wildcard;
    let l = Pat::Literal(LiteralValue::Int(42));
    let v = Pat::Variable("x".to_string());
    let vr = Pat::Variant {
        name: "Some".to_string(),
        arg: Some(Box::new(Pat::Variable("inner".to_string()))),
    };
    let vr_no_arg = Pat::Variant {
        name: "None".to_string(),
        arg: None,
    };
    let pr = Pat::Record {
        fields: vec![
            ("x".to_string(), Pat::Variable("a".to_string())),
            ("y".to_string(), Pat::Variable("b".to_string())),
        ],
        rest: false,
    };
    let pr_rest = Pat::Record {
        fields: vec![("x".to_string(), Pat::Variable("a".to_string()))],
        rest: true,
    };

    assert!(matches!(w, Pat::Wildcard));
    assert!(matches!(l, Pat::Literal(LiteralValue::Int(42))));
    assert!(matches!(v, Pat::Variable(name) if name == "x"));
    assert!(matches!(vr, Pat::Variant { name, .. } if name == "Some"));
    assert!(matches!(vr_no_arg, Pat::Variant { arg: None, .. }));
    assert!(matches!(pr, Pat::Record { fields, rest: false } if fields.len() == 2));
    assert!(matches!(pr_rest, Pat::Record { rest: true, .. }));
}

#[test]
fn test_binary_op_variants() {
    let _ops = vec![
        (BinaryOp::Add, "+"),
        (BinaryOp::Sub, "-"),
        (BinaryOp::Mul, "*"),
        (BinaryOp::Div, "/"),
        (BinaryOp::Eq, "=="),
        (BinaryOp::Ne, "!="),
        (BinaryOp::Lt, "<"),
        (BinaryOp::Gt, ">"),
        (BinaryOp::Le, "<="),
        (BinaryOp::Ge, ">="),
        (BinaryOp::And, "&&"),
        (BinaryOp::Or, "||"),
    ];

    // Binary operations used in expressions
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Literal(LiteralValue::Int(1))),
        op: BinaryOp::Add,
        rhs: Box::new(Expr::Literal(LiteralValue::Int(2))),
        span: Default::default(),
    };

    if let Expr::Binary { op, .. } = &expr {
        assert!(matches!(op, BinaryOp::Add));
    } else {
        panic!("Expected Binary expr");
    }
}

#[test]
fn test_unary_op_variants() {
    let expr = Expr::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(Expr::Literal(LiteralValue::Int(42))),
        span: Default::default(),
    };

    if let Expr::Unary { op, expr, .. } = &expr {
        assert!(matches!(op, UnaryOp::Neg));
        assert!(matches!(expr.as_ref(), Expr::Literal(LiteralValue::Int(42))));
    } else {
        panic!("Expected Unary expr");
    }
}

#[test]
fn test_binary_expr_comparison() {
    let expr = Expr::Binary {
        lhs: Box::new(Expr::Literal(LiteralValue::Int(5))),
        op: BinaryOp::Gt,
        rhs: Box::new(Expr::Literal(LiteralValue::Int(3))),
        span: Default::default(),
    };

    assert!(matches!(expr, Expr::Binary { op: BinaryOp::Gt, .. }));
}

#[test]
fn test_deeply_nested_expr() {
    // Build: add(1, mul(2, 3))
    let expr = Expr::Call {
        func: Box::new(Expr::Variable("add".to_string())),
        args: vec![
            Expr::Literal(LiteralValue::Int(1)),
            Expr::Call {
                func: Box::new(Expr::Variable("mul".to_string())),
                args: vec![
                    Expr::Literal(LiteralValue::Int(2)),
                    Expr::Literal(LiteralValue::Int(3)),
                ],
                span: Default::default(),
            },
        ],
        span: Default::default(),
    };

    if let Expr::Call { func, args, .. } = &expr {
        assert!(matches!(func.as_ref(), Expr::Variable(name) if name == "add"));
        assert_eq!(args.len(), 2);
        // Second arg should be a nested Call
        assert!(matches!(&args[1], Expr::Call { .. }));
        if let Expr::Call { func: nested_func, args: nested_args, .. } = &args[1] {
            assert!(matches!(nested_func.as_ref(), Expr::Variable(name) if name == "mul"));
            assert_eq!(nested_args.len(), 2);
        } else {
            panic!("Expected nested Call");
        }
    } else {
        panic!("Expected Call expr");
    }
}
