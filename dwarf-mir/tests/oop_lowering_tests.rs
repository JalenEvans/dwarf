//! DWARF-104 — Phase 3: MIR & LIR Lowering for OOP.
//!
//! RED-phase integration tests specifying how OOP constructs lower from HIR
//! to MIR. These tests define the *target* MIR shape for:
//!   - record methods desugaring to free functions with a `self` parameter
//!   - `self.field` member access inside method bodies
//!   - interface method signatures desugaring to function skeletons
//!
//! They are written to FAIL against the current lowering, which drops record
//! methods (`methods: _`) and interface declarations entirely. They will turn
//! GREEN once the MIR desugarer emits the desugared functions described in the
//! `// EXPECTED` comments.
//!
//! Design decisions locked in by these tests (see the DWARF-104 report):
//!   1. Naming  : a method `area` on a record `Point` lowers to a function
//!      named `"Point::area"` (record-scoped, `::` separator).
//!   2. `self`  : becomes the FIRST parameter, typed `Type::Named(record)`.
//!   3. self.x  : lowers via existing passthrough to `MirExpr::Member`
//!      where the object is `MirExpr::Variable { name: "self" }`.
//!   4. Method *call sites* (obj.method(...)) are NOT rewritten in MIR —
//!      dispatch is LIR's responsibility (see dwarf-lir tests).
//!
//! The test files follow the crate convention of building HIR `Decl` values
//! directly (no parser dependency) as used by the inline tests in
//! `dwarf_mir::desugar` and `dwarf_mir::lib`.

use dwarf_mir::pass::MirPass;
use dwarf_mir::{MirBinaryOp, MirDecl, MirExpr, MirField, MirParam};
use dwarf_syntax::hir::{BinaryOp, Decl, Expr, Field, LiteralValue, Param, Type};
use dwarf_syntax::span::Span;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Synthetic span shared by all constructed values.
fn span() -> Span {
    Span::new(0, 0, 0)
}

/// Build the HIR for `type Point { x, y; fn area(self) -> Int { self.x * self.y } }`.
///
/// `self` is parsed as a param with no type annotation (the parser yields
/// `type_ = None` for bare `self`); the MIR desugarer must inject the record
/// type, making the desugared self parameter `type_ = Some(Type::Named("Point"))`.
fn point_record_with_area_method() -> Decl {
    Decl::RecordDef {
        name: "Point".into(),
        fields: vec![
            Field {
                name: "x".into(),
                type_: Type::Named("Int".into()),
            },
            Field {
                name: "y".into(),
                type_: Type::Named("Int".into()),
            },
        ],
        methods: vec![Decl::Function {
            name: "area".into(),
            params: vec![Param {
                name: "self".into(),
                type_: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            body: Expr::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::Member {
                    obj: Box::new(Expr::Variable {
                        name: "self".into(),
                        span: span(),
                    }),
                    field: "x".into(),
                    span: span(),
                }),
                rhs: Box::new(Expr::Member {
                    obj: Box::new(Expr::Variable {
                        name: "self".into(),
                        span: span(),
                    }),
                    field: "y".into(),
                    span: span(),
                }),
                span: span(),
            },
            is_pub: false,
            decorators: vec![],
            span: span(),
        }],
        implements: vec![],
        is_pub: true,
        span: span(),
    }
}

/// The exact MIR output expected from lowering `point_record_with_area_method`.
///
/// The record passes through as `MirDecl::RecordDef`, immediately followed by
/// one `MirDecl::Function` per method, named `"Point::area"`, with `self` as
/// the first parameter (typed as the record), the method's return type, and
/// the desugared body (`self.x * self.y`).
fn expected_record_area_lowering() -> Vec<MirDecl> {
    vec![
        MirDecl::RecordDef {
            name: "Point".into(),
            fields: vec![
                MirField {
                    name: "x".into(),
                    type_: Type::Named("Int".into()),
                },
                MirField {
                    name: "y".into(),
                    type_: Type::Named("Int".into()),
                },
            ],
            is_pub: true,
            span: span(),
        },
        MirDecl::Function {
            name: "Point::area".into(),
            params: vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Point".into())),
            }],
            return_type: Some(Type::Named("Int".into())),
            body: MirExpr::Binary {
                op: MirBinaryOp::Mul,
                lhs: Box::new(MirExpr::Member {
                    obj: Box::new(MirExpr::Variable {
                        name: "self".into(),
                        span: span(),
                    }),
                    field: "x".into(),
                    span: span(),
                }),
                rhs: Box::new(MirExpr::Member {
                    obj: Box::new(MirExpr::Variable {
                        name: "self".into(),
                        span: span(),
                    }),
                    field: "y".into(),
                    span: span(),
                }),
                span: span(),
            },
            is_pub: false,
            is_generator: false,
            span: span(),
        },
    ]
}

/// Run the full MIR desugaring pipeline over the given HIR declarations.
fn lower_to_mir(decls: Vec<Decl>) -> Vec<MirDecl> {
    MirPass::new().run(&decls)
}

// ---------------------------------------------------------------------------
// Record methods → desugared functions
// ---------------------------------------------------------------------------

/// A record with a method lowers to BOTH its `MirDecl::RecordDef` AND a
/// `MirDecl::Function` for the method, named `Point::area`, with `self` as the
/// first parameter typed as the record.
///
/// Now GREEN: the desugarer emits both the `MirDecl::RecordDef` and the
/// desugared `Point::area` function, with `self` as the first parameter.
#[test]
fn record_method_desugars_to_function_with_self_param() {
    let result = lower_to_mir(vec![point_record_with_area_method()]);

    assert_eq!(result, expected_record_area_lowering());
}

/// The record's field definition must still pass through unchanged even when
/// the record has methods (the RecordDef node should NOT be dropped).
#[test]
fn record_def_still_passes_through_alongside_methods() {
    let result = lower_to_mir(vec![point_record_with_area_method()]);

    let record_defs: Vec<&MirDecl> = result
        .iter()
        .filter(|d| matches!(d, MirDecl::RecordDef { .. }))
        .collect();
    assert_eq!(
        record_defs.len(),
        1,
        "the record type definition must still be present in MIR"
    );
    if let MirDecl::RecordDef { name, .. } = record_defs[0] {
        assert_eq!(name, "Point");
    } else {
        unreachable!();
    }
}

/// The desugared method function must have `self` as its FIRST parameter,
/// typed with the record type (`Type::Named("Point")`).
///
/// Now GREEN: the desugared `Point::area` function is emitted with `self` as
/// its first parameter, typed as the record (`Type::Named("Point")`).
#[test]
fn desugared_method_self_param_is_first_and_typed_as_record() {
    let result = lower_to_mir(vec![point_record_with_area_method()]);

    let func = result
        .iter()
        .find(|d| matches!(d, MirDecl::Function { name, .. } if name == "Point::area"))
        .unwrap_or_else(|| panic!("expected a desugared Point::area function in MIR"));

    if let MirDecl::Function { params, .. } = func {
        assert_eq!(
            params,
            &vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Point".into())),
            }],
            "method's own params are desugared as [self: Point]"
        );
    } else {
        unreachable!();
    }
}

// ---------------------------------------------------------------------------
// self.field access inside method bodies
// ---------------------------------------------------------------------------

/// `self.x * self.y` inside a method body lowers to `MirExpr::Binary` of two
/// `MirExpr::Member` expressions whose object is `Variable { name: "self" }`.
///
/// Now GREEN: the desugared method body is produced, lowering `self.x * self.y`
/// to member accesses on the `self` local.
#[test]
fn self_field_access_lowers_to_member_on_self_local() {
    let result = lower_to_mir(vec![point_record_with_area_method()]);

    let body = result
        .iter()
        .find_map(|d| match d {
            MirDecl::Function { name, body, .. } if name == "Point::area" => Some(body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a Point::area function with a body"));

    assert_eq!(
        body,
        &MirExpr::Binary {
            op: MirBinaryOp::Mul,
            lhs: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "self".into(),
                    span: span(),
                }),
                field: "x".into(),
                span: span(),
            }),
            rhs: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "self".into(),
                    span: span(),
                }),
                field: "y".into(),
                span: span(),
            }),
            span: span(),
        }
    );
}

// ---------------------------------------------------------------------------
// Multiple methods
// ---------------------------------------------------------------------------

/// A record with several methods emits one `MirDecl::Function` per method, in
/// declaration order, after its `MirDecl::RecordDef`.
///
/// Now GREEN: each method desugars to a `MirDecl::Function` in declaration
/// order after the record's `MirDecl::RecordDef`.
#[test]
fn record_with_multiple_methods_desugars_each_to_function() {
    let area = Decl::Function {
        name: "area".into(),
        params: vec![Param {
            name: "self".into(),
            type_: None,
        }],
        return_type: Some(Type::Named("Int".into())),
        body: Expr::Literal {
            value: LiteralValue::Int(0),
            span: span(),
        },
        is_pub: false,
        decorators: vec![],
        span: span(),
    };
    let translate = Decl::Function {
        name: "translate".into(),
        params: vec![
            Param {
                name: "self".into(),
                type_: None,
            },
            Param {
                name: "dx".into(),
                type_: Some(Type::Named("Int".into())),
            },
        ],
        return_type: Some(Type::Named("Point".into())),
        body: Expr::Literal {
            value: LiteralValue::Int(0),
            span: span(),
        },
        is_pub: false,
        decorators: vec![],
        span: span(),
    };

    let result = lower_to_mir(vec![Decl::RecordDef {
        name: "Point".into(),
        fields: vec![],
        methods: vec![area, translate],
        implements: vec![],
        is_pub: true,
        span: span(),
    }]);

    let method_names: Vec<&String> = result
        .iter()
        .filter_map(|d| match d {
            MirDecl::Function { name, .. } => Some(name),
            _ => None,
        })
        .collect();

    assert_eq!(
        method_names,
        vec!["Point::area", "Point::translate"],
        "each method lowers to a desugared function, in order"
    );

    // The record definition precedes the desugared methods.
    assert!(matches!(&result[0], MirDecl::RecordDef { name, .. } if name == "Point"));
}

// ---------------------------------------------------------------------------
// Interfaces → function skeletons (still type-level)
// ---------------------------------------------------------------------------

/// An interface declaration stays type-level (no dedicated MIR node), but its
/// method SIGNATURES desugar to bodyless `MirDecl::Function` skeletons named
/// `"{Interface}::{method}"` so that interface method call sites can be
/// resolved to a direct dispatch target in LIR.
///
/// Now GREEN: interface method signatures desugar to bodyless `Shape::area`
/// function skeletons (named `"{Interface}::{method}"`) that reach MIR.
#[test]
fn interface_methods_desugar_to_function_skeletons() {
    let interface = Decl::Interface {
        name: "Shape".into(),
        methods: vec![Decl::Function {
            name: "area".into(),
            params: vec![Param {
                name: "self".into(),
                type_: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            // Parser renders interface method bodies as an empty block.
            body: Expr::Block {
                stmts: vec![],
                span: span(),
            },
            is_pub: false,
            decorators: vec![],
            span: span(),
        }],
        is_pub: true,
        span: span(),
    };

    let result = lower_to_mir(vec![interface]);

    assert_eq!(
        result,
        vec![MirDecl::Function {
            name: "Shape::area".into(),
            params: vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Shape".into())),
            }],
            return_type: Some(Type::Named("Int".into())),
            body: MirExpr::Block {
                stmts: vec![],
                span: span(),
            },
            is_pub: false,
            is_generator: false,
            span: span(),
        }],
        "interface method signatures are lowered to Shape::area function skeletons"
    );
}

// ---------------------------------------------------------------------------
// Method call sites stay as member calls in MIR
// ---------------------------------------------------------------------------

/// Method dispatch is LIR's responsibility. A call site `self.translate(2)`
/// inside a method body stays as `MirExpr::Call { func: MirExpr::Member { ... }, args }`
/// in the MIR — NOT rewritten into a `MirExpr::Variable` callee.
///
/// GREEN today (MIR's `Call`/`Member` passthrough). This test pins the
/// MIR↔LIR contract so the LIR dispatch rewrite (DWARF-104) is not pushed
/// backward into MIR.
#[test]
fn method_call_in_body_stays_member_call_in_mir() {
    let method = Decl::Function {
        name: "scale".into(),
        params: vec![
            Param {
                name: "self".into(),
                type_: None,
            },
            Param {
                name: "factor".into(),
                type_: Some(Type::Named("Int".into())),
            },
        ],
        return_type: Some(Type::Named("Point".into())),
        body: Expr::Call {
            func: Box::new(Expr::Member {
                obj: Box::new(Expr::Variable {
                    name: "self".into(),
                    span: span(),
                }),
                field: "area".into(),
                span: span(),
            }),
            args: vec![],
            span: span(),
        },
        is_pub: false,
        decorators: vec![],
        span: span(),
    };

    let result = lower_to_mir(vec![Decl::RecordDef {
        name: "Point".into(),
        fields: vec![],
        methods: vec![method],
        implements: vec![],
        is_pub: true,
        span: span(),
    }]);

    let body = result
        .iter()
        .find_map(|d| match d {
            MirDecl::Function { name, body, .. } if name == "Point::scale" => Some(body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a desugared Point::scale function"));

    // Call(Member(self, "area"), []) is preserved 1:1.
    assert_eq!(
        body,
        &MirExpr::Call {
            func: Box::new(MirExpr::Member {
                obj: Box::new(MirExpr::Variable {
                    name: "self".into(),
                    span: span(),
                }),
                field: "area".into(),
                span: span(),
            }),
            args: vec![],
            span: span(),
        }
    );
}

// ---------------------------------------------------------------------------
// Existing function lowering remains unchanged (regression guards)
// ---------------------------------------------------------------------------

/// Plain free functions are unaffected by method desugaring.
#[test]
fn plain_function_lowering_remains_unchanged() {
    let result = lower_to_mir(vec![Decl::Function {
        name: "foo".into(),
        params: vec![Param {
            name: "x".into(),
            type_: Some(Type::Named("Int".into())),
        }],
        return_type: Some(Type::Named("Int".into())),
        body: Expr::Literal {
            value: LiteralValue::Int(42),
            span: span(),
        },
        is_pub: true,
        decorators: vec![],
        span: span(),
    }]);

    assert_eq!(result.len(), 1);
    match &result[0] {
        MirDecl::Function {
            name,
            params,
            return_type,
            is_pub,
            is_generator,
            ..
        } => {
            assert_eq!(name, "foo");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert_eq!(params[0].type_, Some(Type::Named("Int".into())));
            assert_eq!(return_type, &Some(Type::Named("Int".into())));
            assert!(*is_pub);
            assert!(!*is_generator);
        }
        other => panic!("expected MirDecl::Function, got {other:?}"),
    }
}

/// A record WITHOUT methods continues to lower to a lone `MirDecl::RecordDef`
/// (no spurious function emission).
#[test]
fn record_without_methods_lowers_to_single_record_def() {
    let result = lower_to_mir(vec![Decl::RecordDef {
        name: "Point".into(),
        fields: vec![Field {
            name: "x".into(),
            type_: Type::Named("Int".into()),
        }],
        methods: vec![],
        implements: vec![],
        is_pub: true,
        span: span(),
    }]);

    assert_eq!(result.len(), 1);
    assert!(
        matches!(&result[0], MirDecl::RecordDef { name, .. } if name == "Point"),
        "a record with no methods is a lone RecordDef"
    );
}

/// The MIR output has no interface-level node variant for a type that's only
/// declared via an interface plus its skeletons.
#[test]
fn interface_declaration_has_no_mir_node_besides_method_skeletons() {
    let interface = Decl::Interface {
        name: "Shape".into(),
        methods: vec![Decl::Function {
            name: "area".into(),
            params: vec![Param {
                name: "self".into(),
                type_: None,
            }],
            return_type: Some(Type::Named("Int".into())),
            body: Expr::Block {
                stmts: vec![],
                span: span(),
            },
            is_pub: false,
            decorators: vec![],
            span: span(),
        }],
        is_pub: true,
        span: span(),
    };

    let result = lower_to_mir(vec![interface]);

    assert!(
        result.iter().all(|d| matches!(d, MirDecl::Function { .. })),
        "interfaces themselves stay type-level: only method function skeletons appear in MIR"
    );
    assert_eq!(result.len(), 1, "one skeleton function, nothing else");
}
