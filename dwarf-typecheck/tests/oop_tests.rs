//! Integration tests for OOP type-checking (DWARF-103 — Phase 2: HIR &
//! Type-Checking for OOP).
//!
//! These tests exercise `TypeCheckPass::check` on the full pipeline:
//! source text → lexer → parser → HIR → type registry + errors.
//!
//! # GREEN PHASE
//!
//! The DWARF-103 typechecker is implemented and these tests now pass. They
//! pin the final, grounded behavior for:
//!
//!   1. Method-body type-checking with an implicit `self` binding
//!   2. Member access on `self` (field existence checking)
//!   3. Method signature registration in the `TypeRegistry` (as
//!      `TypeDef::Func` entries whose param list EXCLUDES the implicit
//!      `self`)
//!   4. Structural interface conformance (`implements` clause), made
//!      alias-aware via `compat::check`
//!   5. Self-referential method call resolution (`self.other(...)`)
//!
//! A method without a declared return type (e.g. `fn get(self) { self.count }`
//! or `fn reset(self) { }`) has no return contract to enforce and type-checks
//! cleanly; a declared-but-mismatched return (`-> Str` body `42`) is still an
//! error.
//!
//! # HIR STRATEGY DECISION — NO new HIR nodes
//!
//! The DWARF-102 parser already produces exactly the shapes the typechecker
//! needs; we deliberately keep it unchanged and spec *typecheck-level*
//! behavior instead:
//!
//!   - `self`             -> `Expr::Variable { name: "self" }`
//!   - `self.field`       -> `Expr::Member { obj: self, field, .. }`
//!   - `self.method(args)` → `Expr::Call { func: Member(self, method), args }`
//!   - interface methods  → `Decl::Function` inside `Decl::Interface.methods`
//!   - record methods     → `Decl::Function` inside `Decl::RecordDef.methods`
//!   - `implements X`     → `RecordDef.implements: Vec<String>`
//!
//! The parser accepts all of these end-to-end (verified; no parser blockers).
//!
//! # Behavioral notes locked in by the regression tests
//!
//!   - Interface conformance uses alias-aware `compat::check`, so
//!     `type MyInt = Int` with `fn area(self) -> MyInt` conforms to an
//!     interface `fn area(self) -> Int`, while a real `Str`/`Int` mismatch is
//!     still rejected (on params or on return).
//!   - `MethodSig::return_type` is `Option<TypeId>`; an interface method with
//!     no declared return type does NOT enforce the record method's return.
//!   - Error messages render type NAMES (`expected Str, got Int`) and
//!     conformance errors render `expected fn area() -> Int, got fn area() -> Str`
//!     instead of raw TypeIds.
//!   - `TypeRegistry::method_sigs` uses a `String` key (`"{owner}:{name}"`),
//!     so a registry with populated signatures survives serde JSON.

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
/// GREEN: pins that `self` is bound to the record, method bodies are visited,
/// and the method's signature is registered as a `TypeDef::Func` for `get`.
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
/// GREEN: pins that method bodies are type-checked and member access on
/// `self` validates field existence.
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
/// GREEN: pins that method bodies are visited, so the invalid arithmetic is
/// caught.
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
/// GREEN: pins that declared-but-mismatched annotated returns are caught
/// (see also `test_method_annotated_return_mismatch_reports_named_types`,
/// which pins the `expected Str, got Int` message format).
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
/// GREEN: pins that `Decl::Interface` is handled by
/// `resolve::register_decls` and each method signature is registered.
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
/// GREEN: pins that the `implements` clause is checked against the
/// interface's required method set.
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
/// GREEN: pins that a real `Str`/`Int` return mismatch is still rejected
/// despite the alias-aware conformance check (see also
/// `test_real_return_conformance_mismatch_rejected`, which pins the named
/// `expected fn area() -> Int, got fn area() -> Str` message).
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
/// GREEN: pins the valid conformance path end-to-end (the implemented
/// method's signature `Func([], Int)` is registered by the resolver).
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
/// GREEN: pins that method signatures are registered, so both `get` and
/// `double` exist as `Func([], Int)` entries and `self.get()` resolves.
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
/// GREEN: pins that method signatures exclude the implicit `self`.
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
// DWARF-103 regressions — unannotated methods, alias-aware conformance,
// named-type error messages, undefined interfaces, serde round-trip
// ===========================================================================

/// A method WITHOUT a declared return type that returns a value must
/// type-check cleanly. Previously the resolver defaulted a missing return
/// annotation to Null, which spuriously rejected `fn get(self) { self.count }`
/// as `expected Null, got Int`.
///
/// GREEN: an unannotated method has no return contract to enforce.
#[test]
fn test_method_without_declared_return_type_passes() {
    let src = r#"
type Counter {
    count: Int
    fn get(self) {
        self.count
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "unannotated method 'get' returning self.count should type-check: {errors:?}"
    );
}

/// A method WITHOUT a declared return type and an EMPTY body must type-check
/// cleanly (the `fn reset(self) { }` shape).
///
/// GREEN: no declared return means no return-mismatch check fires.
#[test]
fn test_method_without_declared_return_type_empty_body_passes() {
    let src = r#"
type Counter {
    count: Int
    fn reset(self) { }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "unannotated empty-bodied method 'reset' should type-check: {errors:?}"
    );
}

/// A method that DECLARES a return type but whose body does not match must
/// still be rejected, and the error must render the types by NAME
/// (`expected Str, got Int`) rather than raw TypeIds.
///
/// GREEN: pins the DWARF-103 `TypeRegistry::type_name` rendering in return
/// mismatch messages.
#[test]
fn test_method_annotated_return_mismatch_reports_named_types() {
    let src = r#"
type Greeter {
    fn hello(self) -> Str {
        42
    }
}
"#;
    let (_registry, errors) = check_source(src);
    let mismatch = errors.iter().find(|e| e.message.contains("return type mismatch"));
    assert!(
        mismatch.is_some(),
        "expected a return type mismatch error for '-> Str' body 42: {errors:?}"
    );
    let msg = &mismatch.unwrap().message;
    assert!(
        msg.contains("expected Str") && msg.contains("got Int"),
        "mismatch message should name both types: {msg}"
    );
    assert!(has_type_code(&errors), "expected a DWARF-E-TYPE- error: {errors:?}");
}

/// Alias-based interface conformance: `type MyInt = Int` with a record method
/// `fn area(self) -> MyInt` conforms to an interface `fn area(self) -> Int`.
///
/// GREEN: pins that conformance uses alias-aware `compat::check` (which
/// resolves aliases via `TypeRegistry::resolve`) instead of raw TypeId
/// equality.
#[test]
fn test_alias_interface_conformance_accepted() {
    let src = r#"
type MyInt = Int
interface Shape {
    fn area(self) -> Int
}
type Square implements Shape {
    side: Int
    fn area(self) -> MyInt {
        self.side * self.side
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "record returning 'MyInt' (an alias of Int) must conform to '-> Int': {errors:?}"
    );
}

/// A REAL conformance mismatch on the RETURN type must still be rejected, and
/// the conformance error must render both signatures by name:
/// `expected fn area() -> Int, got fn area() -> Str`.
///
/// GREEN: pins the `render_method_sig` output in DWARF-E-TYPE-0011 messages.
#[test]
fn test_real_return_conformance_mismatch_rejected() {
    let src = r#"
interface Shape {
    fn area(self) -> Int
}
type Square implements Shape {
    side: Int
    fn area(self) -> Str {
        "hi"
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "real Str/Int return mismatch must be rejected despite alias-aware conformance"
    );
    let conformance = errors.iter().find(|e| e.code == "DWARF-E-TYPE-0011");
    assert!(
        conformance.is_some(),
        "expected a DWARF-E-TYPE-0011 conformance error: {errors:?}"
    );
    let msg = &conformance.unwrap().message;
    assert!(
        msg.contains("expected fn area() -> Int") && msg.contains("got fn area() -> Str"),
        "conformance error should render named expected/got signatures: {msg}"
    );
}

/// A REAL conformance mismatch on a PARAMETER type must also be rejected:
/// interface `fn scale(self, factor: Int) -> Int` vs record
/// `fn scale(self, factor: Str) -> Int`.
///
/// GREEN: pins that the param list is compared with alias-aware compatibility.
#[test]
fn test_real_param_conformance_mismatch_rejected() {
    let src = r#"
interface Scalar {
    fn scale(self, factor: Int) -> Int
}
type Doubler implements Scalar {
    base: Int
    fn scale(self, factor: Str) -> Int {
        self.base
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "param mismatch (Int vs Str) must be rejected by conformance"
    );
    let conformance = errors.iter().find(|e| e.code == "DWARF-E-TYPE-0011");
    assert!(
        conformance.is_some(),
        "expected a DWARF-E-TYPE-0011 conformance error: {errors:?}"
    );
    let msg = &conformance.unwrap().message;
    assert!(
        msg.contains("expected fn scale(Int) -> Int") && msg.contains("got fn scale(Str) -> Int"),
        "conformance error should render the mismatched params by name: {msg}"
    );
}

/// An `implements` clause naming an UNDEFINED interface must be handled
/// gracefully: no panic, and a DWARF-E-TYPE-0002 error naming the interface.
///
/// GREEN: pins the unknown-interface guard in `check_conformance`.
#[test]
fn test_undefined_interface_in_implements_handled_gracefully() {
    let src = r#"
type Shape2D implements MissingShape {
    side: Int
    fn area(self) -> Int {
        self.side
    }
}
"#;
    let (_registry, errors) = check_source(src);
    assert!(
        !errors.is_empty(),
        "implementing an undeclared interface must produce an error (not panic)"
    );
    let unknown = errors.iter().find(|e| e.code == "DWARF-E-TYPE-0002");
    assert!(
        unknown.is_some(),
        "expected a DWARF-E-TYPE-0002 unknown-type error: {errors:?}"
    );
    assert!(
        unknown.map(|e| e.message.contains("MissingShape")).unwrap_or(false),
        "error should name the unknown interface: {errors:?}"
    );
}

/// The registry must survive a serde JSON round-trip even when its
/// `method_sigs` table is POPULATED. The method-sig key is a single `String`
/// (`format!("{owner}:{name}")`); a `(TypeId, String)` tuple key is not a
/// valid JSON object key and would PANIC during serialization. This pins that
/// DWARF-103 key change.
#[test]
fn test_registry_with_populated_method_sigs_json_roundtrip() {
    let src = r#"
type Counter {
    count: Int
    fn get(self) -> Int {
        self.count
    }
    fn reset(self) { }
}
"#;
    let (registry, errors) = check_source(src);
    assert!(
        errors.is_empty(),
        "expected no type errors while building the registry: {errors:?}"
    );

    let json = serde_json::to_string(&registry)
        .expect("registry with a populated method_sigs table must serialize");
    let back: TypeRegistry = serde_json::from_str(&json)
        .expect("serialized registry must deserialize back to a TypeRegistry");
    assert_eq!(back, registry, "registry must survive a serde JSON round-trip");
}

// ===========================================================================
// Sanity — existing top-level function type-checking still works
// ===========================================================================

/// Regression guard: the OOP work must not disturb plain top-level function
/// type-checking. GREEN.
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
