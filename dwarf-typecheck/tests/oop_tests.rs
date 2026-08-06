//! Integration tests for OOP type-checking (DWARF-103 — Phase 2: HIR &
//! Type-Checking for OOP).
//!
//! These tests exercise `TypeCheckPass::check` on the full pipeline:
//! source text → lexer → parser → HIR → type registry + errors.
//!
//! # RED PHASE
//!
//! Every test in this file (except the trailing top-level-function sanity
//! test) is expected to FAIL until the typechecker implements:
//!
//!   1. Method-body type-checking with an implicit `self` binding
//!   2. Member access on `self` (field existence checking)
//!   3. Method signature registration in the `TypeRegistry` (as
//!      `TypeDef::Func` entries whose param list EXCLUDES the implicit
//!      `self`)
//!   4. Structural interface conformance (`implements` clause)
//!   5. Self-referential method call resolution (`self.other(...)`)
//!
//! # HIR STRATEGY DECISION — NO new HIR nodes
//!
//! The DWARF-102 parser already produces exactly the shapes the typechecker
//! needs; we deliberately keep it unchanged and spec *typecheck-level*
//! behavior instead:
//!
//!   - `self`             → `Expr::Variable { name: "self" }`
//!   - `self.field`       → `Expr::Member { obj: self, field, .. }`
//!   - `self.method(args)` → `Expr::Call { func: Member(self, method), args }`
//!   - interface methods  → `Decl::Function` inside `Decl::Interface.methods`
//!   - record methods     → `Decl::Function` inside `Decl::RecordDef.methods`
//!   - `implements X`     → `RecordDef.implements: Vec<String>`
//!
//! The parser accepts all of these end-to-end (verified; no parser blockers).
//!
//! # Why the positive tests fail (Red)
//!
//! The negative tests fail because the checker ignores methods/self/interface
//! entirely and reports NO errors when it should report one. The positive
//! tests also fail today — not because errors are produced, but because the
//! method signatures are never registered: `TypeRegistry` contains no
//! `TypeDef::Func` entry for any method (params excluding `self`, declared
//! return type). Registering those signatures is the Green implementer's
//! contract, and it is what makes `self.get()` / `self.area()` resolvable.

use dwarf_lexer::Lexer;
use dwarf_parser::Parser;
use dwarf_syntax::hir::Decl;
use dwarf_syntax::token::TokenKind;
use dwarf_typecheck::error::TypeCheckError;
use dwarf_typecheck::pass::TypeCheckPass;
use dwarf_typecheck::registry::TypeRegistry;
use dwarf_typecheck::types::{TypeDef, TypeId};

// ---------------------------------------------------------------------------
// Helpers — parse Dwarf source text and run the full typecheck pass
// ---------------------------------------------------------------------------

/// Lex + parse a Dwarf source string into HIR declarations.
///
/// Panics on lexer/parser errors so that a test failing here points at a
/// parser problem (which would itself be a Red finding), not at the
/// typecheck behavior under test.
fn parse_ok(src: &str) -> Vec<Decl> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token().expect("lexer error");
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }
    let mut parser = Parser::new(tokens);
    let (decls, errors) = parser.parse();
    assert!(
        errors.is_empty(),
        "unexpected parse errors for input:\n{}\n\n{errors:?}",
        src
    );
    decls
}

/// Parse `src` and run `TypeCheckPass::check` over the resulting HIR.
fn check_source(src: &str) -> (TypeRegistry, Vec<TypeCheckError>) {
    let decls = parse_ok(src);
    let pass = TypeCheckPass::new();
    pass.check(&decls)
}

/// True if the registry contains at least one `TypeDef::Func` whose parameter
/// list (excluding the implicit `self`) and return type match.
fn registry_has_func(registry: &TypeRegistry, params: &[TypeId], ret: TypeId) -> bool {
    (0..registry.len()).any(|id| {
        matches!(registry.get(id), Some(TypeDef::Func(p, r)) if p.as_slice() == params && *r == ret)
    })
}

/// Count of `TypeDef::Func` entries with the given (self-excluded) params and
/// return type. Used to pin that EACH method gets its own registered
/// signature.
fn registry_count_funcs(registry: &TypeRegistry, params: &[TypeId], ret: TypeId) -> usize {
    (0..registry.len())
        .filter(|id| {
            matches!(registry.get(*id), Some(TypeDef::Func(p, r)) if p.as_slice() == params && *r == ret)
        })
        .count()
}

/// True if any reported error uses a TYPE error code.
fn has_type_code(errors: &[TypeCheckError]) -> bool {
    errors.iter().any(|e| e.code.starts_with("DWARF-E-TYPE-"))
}

// ===========================================================================
// AC1 — Method bodies type-check correctly with implicit `self`
// ===========================================================================

/// A method body reading an existing field through `self` must type-check
/// cleanly, and the method's signature must be registered in the registry as
/// `Func([], Int)` — zero explicit params (implicit `self` excluded), Int
/// return.
///
/// RED: `self` is not bound to the record and method bodies are never
/// visited, so `TypeRegistry` contains no `TypeDef::Func` for `get`.
#[test]
fn test_method_body_self_field_access_typechecks() {
    let src = r#"
type Counter {
    count: Int
    fn get(self) -> Int {
        self.count
    }
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "method body reading self.count should type-check: {errors:?}"
    );
    assert!(
        registry_has_func(&registry, &[], 0), // params=[], return=Int
        "method 'get' should be registered as Func([], Int) — method \
         signatures (with implicit self excluded) must be registered"
    );
}

/// A method body reading a NONEXISTENT field through `self` must be rejected
/// with a TYPE error naming the field.
///
/// RED: method bodies are ignored, so no error is reported.
#[test]
fn test_method_body_self_nonexistent_field_rejected() {
    let src = r#"
type Counter {
    count: Int
    fn get(self) -> Int {
        self.nonexistent
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "method body reading self.nonexistent should be a type error, but \
         the checker reported none — method bodies are not being type-checked"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("nonexistent")),
        "expected the error to name the missing field 'nonexistent': {errors:?}"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

/// A method body containing a plain type error (Int + Str) must be reported.
///
/// RED: method bodies are ignored, so the invalid arithmetic goes unnoticed.
#[test]
fn test_method_body_type_error_reported() {
    let src = r#"
type Counter {
    count: Int
    fn bad(self) -> Int {
        self.count + "oops"
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "method body 'self.count + \"oops\"' (Int + Str) should be a type \
         error, but the checker reported none — method bodies are not being \
         type-checked"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

/// A method whose body type does not match its declared return type must be
/// rejected.
///
/// RED: method bodies are ignored, so the return-type mismatch goes
/// unnoticed.
#[test]
fn test_method_return_type_mismatch_rejected() {
    let src = r#"
type Greeter {
    fn hello(self) -> Str {
        42
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "method declared '-> Str' but returning Int should be a type error, \
         but the checker reported none"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

// ===========================================================================
// AC2 — `self.field` member access on the owning record
// (nonexistent-field rejection is covered by
//  `test_method_body_self_nonexistent_field_rejected` above)
// ===========================================================================

// ===========================================================================
// AC3/AC4 — Structural interface conformance (implements)
// ===========================================================================

/// An `interface` declaration must be reflected in the type registry: each
/// interface method signature is registered as a `TypeDef::Func` with the
/// implicit `self` excluded.
///
/// RED: `Decl::Interface` is ignored entirely by `resolve::register_decls`,
/// so no `Func` entry for `area` is ever registered.
#[test]
fn test_interface_method_signature_registered() {
    let src = r#"
interface Shape {
    fn area(self) -> Int
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "interface declaration should not produce errors: {errors:?}"
    );
    assert!(
        registry_has_func(&registry, &[], 0),
        "interface method 'area' should be registered as Func([], Int) — \
         interface declarations are currently ignored by the resolver"
    );
}

/// A type claiming to implement an interface but MISSING one of the required
/// methods must be rejected with a conformance error naming the interface
/// and/or the missing method.
///
/// RED: `implements` is ignored, so the missing `area` goes unnoticed.
#[test]
fn test_type_missing_interface_method_rejected() {
    let src = r#"
interface Shape {
    fn area(self) -> Int
}
type Circle implements Shape {
    radius: Int
    fn diameter(self) -> Int {
        self.radius
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "'Circle implements Shape' without an 'area' method should be a \
         conformance error, but the checker reported none — the 'implements' \
         clause is currently ignored"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("area") || e.message.contains("Shape")),
        "expected the conformance error to name the interface or the missing \
         method: {errors:?}"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

/// A type that implements an interface method with the WRONG signature (same
/// name, different return type) must be rejected.
///
/// RED: `implements` is ignored, so the `area -> Str` mismatch goes
/// unnoticed.
#[test]
fn test_type_wrong_interface_signature_rejected() {
    let src = r#"
interface Shape {
    fn area(self) -> Int
}
type Circle implements Shape {
    radius: Int
    fn area(self) -> Str {
        "hi"
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "'Circle implements Shape' with 'fn area(self) -> Str' (expected \
         '-> Int') should be a conformance error, but the checker reported \
         none"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("area") || e.message.contains("Shape")),
        "expected the conformance error to name the interface or the method: {errors:?}"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

/// A type that correctly implements every interface method must type-check
/// with no errors, and the implemented method must be registered in the
/// registry.
///
/// RED: even the valid case fails today because the implemented method's
/// signature (`Func([], Int)`) is never registered.
#[test]
fn test_type_correctly_implements_interface_accepted() {
    let src = r#"
interface Shape {
    fn area(self) -> Int
}
type Circle implements Shape {
    radius: Int
    fn area(self) -> Int {
        self.radius * self.radius
    }
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "Circle correctly implements Shape — should produce no errors: {errors:?}"
    );
    assert!(
        registry_has_func(&registry, &[], 0),
        "the implemented method 'area' should be registered as Func([], Int)"
    );
}

// ===========================================================================
// AC5 — Self-referential method calls resolve correctly
// ===========================================================================

/// A method that calls another method on `self` (e.g. `self.get()`) must
/// resolve, and each method must get its own registered signature.
///
/// RED: no method signatures are registered, so neither `get` nor `double`
/// exists as a `Func([], Int)` entry.
#[test]
fn test_self_referential_method_call_resolves() {
    let src = r#"
type Counter {
    count: Int
    fn get(self) -> Int {
        self.count
    }
    fn double(self) -> Int {
        self.get() + self.get()
    }
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "self-referential method call 'self.get()' inside 'double' should \
         resolve: {errors:?}"
    );
    assert_eq!(
        registry_count_funcs(&registry, &[], 0),
        2,
        "both 'get' and 'double' should be registered as Func([], Int) — one \
         signature per method, implicit self excluded"
    );
}

/// A method with an explicit parameter in addition to `self` must register
/// the parameter WITHOUT the implicit `self` in its callable signature
/// (`Func([Int], Int)`), so calls like `self.scale(2)` type-check.
///
/// RED: no method signatures are registered.
#[test]
fn test_method_with_self_and_extra_param_resolves() {
    let src = r#"
type Counter {
    count: Int
    fn scale(self, factor: Int) -> Int {
        self.count * factor
    }
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "method 'scale(self, factor: Int)' should type-check: {errors:?}"
    );
    assert!(
        registry_has_func(&registry, &[0], 0), // params=[Int], return=Int
        "method 'scale' should be registered as Func([Int], Int) — the \
         implicit self must NOT appear in the callable signature"
    );
}

// ===========================================================================
// Sanity — existing top-level function type-checking still works
// ===========================================================================

/// Regression guard: the OOP work must not disturb plain top-level function
/// type-checking. This test is GREEN today and must stay green.
#[test]
fn test_top_level_function_still_typechecks() {
    let src = r#"
fn add(a: Int, b: Int) -> Int {
    a + b
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "existing top-level function should still type-check: {errors:?}"
    );
}
