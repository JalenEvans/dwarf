//! DWARF-104 — Phase 3: MIR & LIR Lowering for OOP.
//!
//! RED-phase integration tests specifying how OOP method DISPATCH lowers from
//! MIR to LIR. These tests define the *target* LIR shape for:
//!   - record method calls   `obj.area()`     → direct call to `Point::area`
//!   - self-referential calls `self.scale(2)`  → direct call to `Point::scale`
//!   - interface method calls `shape.area()`   → direct call to `Shape::area`
//!
//! The MIR input is constructed directly (the crate convention) and mirrors
//! exactly what `dwarf-mir` produces once the MIR side of DWARF-104 lands:
//! desugared method functions named `"{Type}::{method}"` with a `self` first
//! parameter, and call sites expressed as `MirExpr::Call { func: MirExpr::Member { ... } }`.
//!
//! Design decision locked in by these tests:
//!   Method dispatch is resolved in LIR. `lower_to_lir` scans the MIR decl
//!   list for `::`-scoped function names (the desugared methods) and rewrites
//!   `Call(Member(obj, "field"), args)` → `Call(Variable("{Type}::{field}"), [obj, ...args])`
//!   whenever `field` names a known method. Non-method member calls (e.g. the
//!   `__iter.next()` produced by for-loop desugaring) are left untouched.
//!
//! Currently RED: `lower_expr` maps `MirExpr::Call`/`MirExpr::Member` 1:1, so
//! the LIR keeps the member-call form instead of dispatching.

use dwarf_lir::lower::lower_to_lir;
use dwarf_lir::{Effect, LirDecl, LirExpr, LirLiteral, LirParam, TargetHint};
use dwarf_mir::{MirDecl, MirExpr, MirLiteral, MirParam};
use dwarf_syntax::hir::Type;
use dwarf_syntax::span::Span;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Synthetic span shared by all constructed values.
fn span() -> Span {
    Span::new(0, 0, 0)
}

/// Build a MIR function declaration.
fn mir_function(name: &str, params: Vec<MirParam>, body: MirExpr) -> MirDecl {
    MirDecl::Function {
        name: name.into(),
        params,
        return_type: None,
        body,
        is_pub: false,
        is_generator: false,
        span: span(),
    }
}

/// `MirExpr::Variable { name }`.
fn mir_var(name: &str) -> MirExpr {
    MirExpr::Variable {
        name: name.into(),
        span: span(),
    }
}

/// `MirExpr::Literal { Int }`.
fn mir_int(value: i64) -> MirExpr {
    MirExpr::Literal {
        value: MirLiteral::Int(value),
        span: span(),
    }
}

/// `MirExpr::Call { func: Member(obj, field), args }` — an unresolved method call.
fn mir_method_call(obj: &str, field: &str, args: Vec<MirExpr>) -> MirExpr {
    MirExpr::Call {
        func: Box::new(MirExpr::Member {
            obj: Box::new(mir_var(obj)),
            field: field.into(),
            span: span(),
        }),
        args,
        span: span(),
    }
}

/// `LirExpr::Variable { name, hint: None }`.
fn lir_var(name: &str) -> LirExpr {
    LirExpr::Variable {
        name: name.into(),
        hint: TargetHint::None,
        span: span(),
    }
}

/// `LirExpr::Literal { Int, hint: None }`.
fn lir_int(value: i64) -> LirExpr {
    LirExpr::Literal {
        value: LirLiteral::Int(value),
        hint: TargetHint::None,
        span: span(),
    }
}

/// Find the body of a lowered function by name; panic if absent.
fn lowered_body(decls: &[LirDecl], name: &str) -> LirExpr {
    decls
        .iter()
        .find_map(|d| match d {
            LirDecl::Function {
                name: fn_name,
                body,
                ..
            } if fn_name == name => Some(body.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected lowered function `{name}` in LIR output"))
}

// ---------------------------------------------------------------------------
// Record method dispatch
// ---------------------------------------------------------------------------

/// `p.area()` (a `Call(Member(p, "area"))` with no args) lowers to a direct
/// LIR call of the desugared `Point::area` function with `p` passed in the
/// self position: `Call(Variable("Point::area"), [p])`.
///
/// Currently RED: LIR preserves the member-call form
/// `Call(Member(p, "area"), [])`.
#[test]
fn method_call_dispatch_lowers_to_desugared_function_call() {
    // Mirrors the MIR output of the DWARF-104 MIR phase: the desugared method
    // function plus a caller containing an unresolved member call.
    let mir = vec![
        mir_function(
            "Point::area",
            vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Point".into())),
            }],
            mir_int(0),
        ),
        mir_function("main", vec![], mir_method_call("p", "area", vec![])),
    ];

    let lir = lower_to_lir(&mir);

    assert_eq!(
        lowered_body(&lir, "main"),
        LirExpr::Call {
            func: Box::new(lir_var("Point::area")),
            args: vec![lir_var("p")],
            hint: TargetHint::None,
            span: span(),
        }
    );
}

/// A method call with explicit arguments passes the receiver FIRST, followed
/// by the method's own arguments: `p.translate(1, 2)` →
/// `Call(Variable("Point::translate"), [p, 1, 2])`.
///
/// Currently RED: LIR emits `Call(Member(p, "translate"), [1, 2])` with no
/// receiver hoisting.
#[test]
fn method_call_with_args_prepends_receiver_to_args() {
    let mir = vec![
        mir_function(
            "Point::translate",
            vec![
                MirParam {
                    name: "self".into(),
                    type_: Some(Type::Named("Point".into())),
                },
                MirParam {
                    name: "dx".into(),
                    type_: Some(Type::Named("Int".into())),
                },
                MirParam {
                    name: "dy".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            mir_var("self"),
        ),
        mir_function(
            "main",
            vec![],
            mir_method_call("p", "translate", vec![mir_int(1), mir_int(2)]),
        ),
    ];

    let lir = lower_to_lir(&mir);

    assert_eq!(
        lowered_body(&lir, "main"),
        LirExpr::Call {
            func: Box::new(lir_var("Point::translate")),
            args: vec![lir_var("p"), lir_int(1), lir_int(2)],
            hint: TargetHint::None,
            span: span(),
        }
    );
}

// ---------------------------------------------------------------------------
// self-referential method calls
// ---------------------------------------------------------------------------

/// `self.scale(2)` inside a method lowers to a direct call with `self` hoisted
/// into the receiver position: `Call(Variable("Point::scale"), [self, 2])`.
///
/// Currently RED: LIR keeps `Call(Member(self, "scale"), [2])`.
#[test]
fn self_referential_method_call_lowers_to_desugared_function_call() {
    let mir = vec![
        mir_function(
            "Point::scale",
            vec![
                MirParam {
                    name: "self".into(),
                    type_: Some(Type::Named("Point".into())),
                },
                MirParam {
                    name: "factor".into(),
                    type_: Some(Type::Named("Int".into())),
                },
            ],
            mir_var("self"),
        ),
        mir_function(
            "Point::area",
            vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Point".into())),
            }],
            mir_method_call("self", "scale", vec![mir_int(2)]),
        ),
    ];

    let lir = lower_to_lir(&mir);

    assert_eq!(
        lowered_body(&lir, "Point::area"),
        LirExpr::Call {
            func: Box::new(lir_var("Point::scale")),
            args: vec![lir_var("self"), lir_int(2)],
            hint: TargetHint::None,
            span: span(),
        }
    );
}

// ---------------------------------------------------------------------------
// Interface method calls
// ---------------------------------------------------------------------------

/// Interface method calls lower to the same direct-dispatch structure: given
/// a `Shape::area` skeleton function in the MIR list, `shape.area()` becomes
/// `Call(Variable("Shape::area"), [shape])`.
///
/// Currently RED: LIR keeps the member-call form.
#[test]
fn interface_method_call_lowers_to_desugared_function_call() {
    let mir = vec![
        // Bodyless skeleton produced from `interface Shape { fn area(self) -> Int }`.
        mir_function(
            "Shape::area",
            vec![MirParam {
                name: "self".into(),
                type_: Some(Type::Named("Shape".into())),
            }],
            MirExpr::Block {
                stmts: vec![],
                span: span(),
            },
        ),
        mir_function("main", vec![], mir_method_call("shape", "area", vec![])),
    ];

    let lir = lower_to_lir(&mir);

    assert_eq!(
        lowered_body(&lir, "main"),
        LirExpr::Call {
            func: Box::new(lir_var("Shape::area")),
            args: vec![lir_var("shape")],
            hint: TargetHint::None,
            span: span(),
        }
    );
}

// ---------------------------------------------------------------------------
// Regression guards
// ---------------------------------------------------------------------------

/// Desugared method declarations themselves pass through normal function
/// lowering unchanged (same name, same params, `Effect::Pure`).
#[test]
fn desugared_method_function_passes_through_lowering() {
    let mir = vec![mir_function(
        "Point::area",
        vec![MirParam {
            name: "self".into(),
            type_: Some(Type::Named("Point".into())),
        }],
        mir_int(0),
    )];

    let lir = lower_to_lir(&mir);
    assert_eq!(lir.len(), 1);
    match &lir[0] {
        LirDecl::Function {
            name,
            params,
            effect,
            hint,
            ..
        } => {
            assert_eq!(name, "Point::area");
            assert_eq!(
                params,
                &vec![LirParam {
                    name: "self".into(),
                    type_: Some(Type::Named("Point".into())),
                }]
            );
            assert_eq!(*effect, Effect::Pure);
            assert_eq!(*hint, TargetHint::None);
        }
        other => panic!("expected LirDecl::Function, got {other:?}"),
    }
}

/// Member calls that are NOT method dispatch (e.g. the `__iter.next()` shape
/// produced by for-loop desugaring) must be left untouched when no desugared
/// `X::next` function exists.
///
/// GREEN today: this guards the dispatch rewrite against over-eager matching.
#[test]
fn non_method_member_call_remains_untouched() {
    let mir = vec![mir_function(
        "main",
        vec![],
        MirExpr::Call {
            func: Box::new(MirExpr::Member {
                obj: Box::new(mir_var("__iter")),
                field: "next".into(),
                span: span(),
            }),
            args: vec![],
            span: span(),
        },
    )];

    let lir = lower_to_lir(&mir);

    assert_eq!(
        lowered_body(&lir, "main"),
        LirExpr::Call {
            func: Box::new(LirExpr::Member {
                obj: Box::new(lir_var("__iter")),
                field: "next".into(),
                hint: TargetHint::None,
                span: span(),
            }),
            args: vec![],
            hint: TargetHint::None,
            span: span(),
        }
    );
}