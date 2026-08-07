//! DWARF-130 — Draupnir deep-behavioral implementation: RED-phase integration
//! tests for the Rust-native PBT engine.
//!
//! The DWARF-119 surface (runtime/*.dwarf stubs + `compile_draupnir()`) is
//! pinned by `draupnir_tests.rs` and `draupnir_module_tests.rs`. This file pins
//! the DEEP-BEHAVIORAL contract: a real Rust-native generation/shrinking engine
//! behind `dwarf_lib::draupnir::engine`.
//!
//! # RED phase (today)
//!
//! `dwarf-lib/src/draupnir.rs` is a FILE module; there is no
//! `dwarf-lib/src/draupnir/` directory and no `engine` submodule. The `use`
//! below therefore fails to compile — the deliberate RED signal. The
//! implementation must:
//!
//! 1. Convert `src/draupnir.rs` into `src/draupnir/mod.rs` (keeping
//!    `pub fn compile_draupnir()` so `draupnir_module_tests.rs` stays green).
//! 2. Add `src/draupnir/engine.rs` with the public API contract below, declared
//!    as `pub mod engine;` in `mod.rs`.
//! 3. Add `dwarf-shrink` to `dwarf-lib/Cargo.toml` and route shrinking through
//!    `IntShrinker` / `StringShrinker` / `ListShrinker`.
//!
//! # Public API contract the implementation MUST provide
//!
//! ```ignore
//! // Module path: dwarf_lib::draupnir::engine
//!
//! pub const DEFAULT_ITERATIONS: usize = 100;
//!
//! pub trait Generator<T> {
//!     /// Next value in the deterministic generation sequence.
//!     fn draw(&mut self) -> T;
//! }
//!
//! pub struct IntGen { /* min, max, seed */ }
//! impl IntGen {
//!     pub fn new(min: i64, max: i64) -> Self; // deterministic (fixed default seed)
//!     pub fn full() -> Self;                  // i64::MIN..=i64::MAX
//!     pub fn seeded(seed: u64) -> Self;       // explicit seed -> reproducible
//! }
//! impl Generator<i64> for IntGen; // edge cases (0, min, max) FIRST, then deterministic draws
//!
//! pub struct NatGen;
//! impl NatGen { pub fn new(max: i64) -> Self; }
//! impl Generator<i64> for NatGen;            // yields 0..=max, never negative
//!
//! pub struct FloatGen;                        // unit struct
//! impl Generator<f64> for FloatGen;           // yields finite values
//!
//! pub struct StringGen;
//! impl StringGen { pub fn new(max_len: usize) -> Self; }
//! impl Generator<String> for StringGen;       // yields len <= max_len
//!
//! pub struct BoolGen;                         // unit struct
//! impl Generator<bool> for BoolGen;
//!
//! pub struct ListGen<G, T>;                   // elem generator + max_len
//! impl ListGen<G, T> { pub fn new(elem: G, max_len: usize) -> Self; }
//! impl<G: Generator<T>, T: Clone> Generator<Vec<T>> for ListGen<G, T>;
//!
//! pub struct OptionGen<G, T>;
//! impl OptionGen<G, T> { pub fn new(inner: G) -> Self; }
//! impl<G: Generator<T>, T: Clone> Generator<Option<T>> for OptionGen<G, T>;
//!
//! pub struct ResultGen<G, E>;
//! impl ResultGen<G, E> { pub fn new(ok: G, err: E) -> Self; }
//! impl<G, E, T, U> Generator<Result<T, U>> for ResultGen<G, E>
//! where G: Generator<T>, E: Generator<U>;
//!
//! pub struct RefineGen<G, T>;
//! impl<G, T> RefineGen<G, T> {
//!     pub fn new(inner: G, pred: impl Fn(&T) -> bool + 'static) -> Self;
//! }
//! impl<G: Generator<T>, T: Clone> Generator<T> for RefineGen<G, T>;
//! //   draws from inner until pred(value) == true; only passing values escape.
//!
//! pub struct MapGen<G, S, T>;
//! impl<G, S, T> MapGen<G, S, T> {
//!     pub fn new(inner: G, f: impl Fn(S) -> T + 'static) -> Self;
//! }
//! impl<G: Generator<S>, S, T> Generator<T> for MapGen<G, S, T>;
//!
//! // Tuple of generators = generator of tuples (2-ary properties).
//! impl<G1, G2, T1, T2> Generator<(T1, T2)> for (G1, G2)
//! where G1: Generator<T1>, G2: Generator<T2>;
//!
//! /// Shrinking capability the engine uses to minimize counterexamples.
//! pub trait Shrinkable: Clone + std::fmt::Debug {}
//! // Required impls: i64 (IntShrinker), String (StringShrinker),
//! // Vec<i64> (ListShrinker), and (A, B) where A: Shrinkable, B: Shrinkable.
//!
//! pub enum PropertyResult<T> {
//!     Passed { iterations: usize },
//!     Failed { value: T, shrunk: T, iterations: usize },
//! }
//! impl<T> PropertyResult<T> {
//!     pub fn passed(&self) -> bool;
//!     pub fn iterations(&self) -> usize;
//!     pub fn counterexample(&self) -> Option<&T>; // Some(_) iff Failed
//!     pub fn shrunk(&self) -> Option<&T>;         // Some(_) iff Failed
//! }
//!
//! pub fn for_all<G, T, P>(gen: G, iterations: usize, property: P) -> PropertyResult<T>
//! where
//!     G: Generator<T>,
//!     T: Clone + std::fmt::Debug + Shrinkable,
//!     P: FnMut(&T) -> bool;
//! //   Draws `iterations` values (edge cases first), invoking `property` on each.
//! //   First failure stops the loop and the counterexample is SHRUNK to a minimal
//! //   one before being returned.
//!
//! pub fn check<G, T, P>(gen: G, property: P) -> PropertyResult<T>
//! where
//!     G: Generator<T>,
//!     T: Clone + std::fmt::Debug + Shrinkable,
//!     P: FnMut(&T) -> bool;
//! //   = for_all(gen, DEFAULT_ITERATIONS, property)
//! ```

use dwarf_lib::draupnir::engine::{
    BoolGen, DEFAULT_ITERATIONS, FloatGen, Generator, IntGen, ListGen, MapGen, NatGen,
    OptionGen, PropertyResult, RefineGen, ResultGen, StringGen, check, for_all,
};

// ===========================================================================
// AC a — for_all runs the property 100 times and passes a commutative property
// ===========================================================================

/// AC (a): `for_all` must invoke the property against generated Int values and
/// hold. Addition is commutative, so the property must pass; `check` must have
/// run the default 100 iterations.
#[test]
fn test_for_all_passes_commutative_property() {
    let result = check(
        (IntGen::new(-100, 100), IntGen::new(-100, 100)),
        |pair: &(i64, i64)| pair.0 + pair.1 == pair.1 + pair.0,
    );

    assert!(
        result.passed(),
        "addition is commutative — property must hold for every generated pair: {result:?}"
    );
    assert_eq!(
        result.iterations(),
        DEFAULT_ITERATIONS,
        "check() must run the default iteration count (100)"
    );
}

// ===========================================================================
// AC b — a failing property returns the counterexample value
// ===========================================================================

/// AC (b): a property that fails must return the offending input as the
/// counterexample. With `IntGen::full()` the edge case `i64::MAX` is drawn
/// within the first three draws and fails `n < 100`; the raw counterexample
/// must be that exact value.
#[test]
fn test_for_all_failing_property_returns_counterexample() {
    let result = for_all(IntGen::full(), 100, |n: &i64| *n < 100);

    assert!(!result.passed(), "n < 100 cannot hold for the full-range generator");
    assert_eq!(
        result.counterexample(),
        Some(&i64::MAX),
        "the first failing edge case (max) is the raw counterexample"
    );
    assert_eq!(
        result.shrunk(),
        Some(&100),
        "shrinking n < 100 to a minimum must land on the boundary 100"
    );
    assert!(
        result.iterations() <= 3,
        "edge cases are explored first — the failure must surface within the first few draws, \
         got {} iterations",
        result.iterations()
    );
}

// ===========================================================================
// AC c — shrinking reduces a failing counterexample to a minimal one
// ===========================================================================

/// AC (c) via IntShrinker semantics: a generator that only ever draws 1000
/// fails `n <= 5`, and the engine must shrink the counterexample to the minimal
/// failing value 6 (the `IntShrinker.shrink(1000, n > 5) == 6` contract).
#[test]
fn test_for_all_shrinks_int_counterexample_to_minimal() {
    let result = for_all(IntGen::new(0, 1000), 100, |n: &i64| *n <= 5);

    match result {
        PropertyResult::Failed { value, shrunk, .. } => {
            assert_eq!(value, 1000, "raw counterexample must be the 1000 draw");
            assert_eq!(shrunk, 6, "minimal value failing n <= 5 is 6, not {shrunk}");
        }
        other => panic!("property n <= 5 must fail for range [0, 1000], got {other:?}"),
    }
}

/// AC (c) via StringShrinker: the engine must integrate the string shrinker so a
/// constant-failing string counterexample is reduced to its minimal suffix.
#[test]
fn test_for_all_shrinks_string_counterexample_to_minimal() {
    let result = for_all(
        MapGen::new(BoolGen, |_| "boom!".to_string()),
        100,
        |s: &String| !s.contains("boom"),
    );

    assert_eq!(
        result.counterexample(),
        Some(&"boom!".to_string()),
        "every draw is the constant failing string"
    );
    assert_eq!(
        result.shrunk(),
        Some(&"boom".to_string()),
        "string shrinking must remove the trailing '!' to the minimal failing value"
    );
}

/// AC (c) via ListShrinker: a constant-failing list counterexample must shrink
/// down to just the offending element.
#[test]
fn test_for_all_shrinks_list_counterexample_to_minimal() {
    let result = for_all(
        MapGen::new(BoolGen, |_| vec![42, 7]),
        100,
        |l: &Vec<i64>| !l.contains(&42),
    );

    assert_eq!(
        result.counterexample(),
        Some(&vec![42, 7]),
        "raw counterexample is the constant failing list"
    );
    assert_eq!(
        result.shrunk(),
        Some(&vec![42]),
        "list shrinking must remove the non-essential element 7"
    );
}

// ===========================================================================
// AC d — refine filters correctly
// ===========================================================================

/// AC (d): a refined generator must only ever yield values passing its
/// predicate, even when the underlying generator draws values that fail it.
#[test]
fn test_refine_filters_values() {
    let mut gen = RefineGen::new(IntGen::new(0, 100), |n: &i64| n % 2 == 0);

    for _ in 0..100 {
        let v = gen.draw();
        assert!(v % 2 == 0, "refined generator yielded odd value {v}");
        assert!((0..=100).contains(&v), "refined value {v} out of range");
    }
}

/// AC (d): refine must not silently drop the edge-case guarantee — 0 (an even
/// edge case) must still surface as the first refined draw.
#[test]
fn test_refine_preserves_edge_case_ordering() {
    let mut gen = RefineGen::new(IntGen::new(0, 100), |n: &i64| n % 2 == 0);
    assert_eq!(gen.draw(), 0, "first refined draw must be the zero edge case");
}

// ===========================================================================
// AC e — list / option / result generators produce valid values (shape checks)
// ===========================================================================

/// AC (e): list generator respects max_len and element bounds on every draw.
#[test]
fn test_list_generator_produces_valid_lists() {
    let mut gen = ListGen::new(IntGen::new(0, 10), 5);

    for _ in 0..50 {
        let list: Vec<i64> = gen.draw();
        assert!(
            list.len() <= 5,
            "list length {} exceeds max_len 5",
            list.len()
        );
        assert!(
            list.iter().all(|x| (0..=10).contains(x)),
            "list element out of [0, 10]: {list:?}"
        );
    }
}

/// AC (e): option generator yields Some(in-range) and None, and both variants
/// appear (None is an edge case, so it must surface).
#[test]
fn test_option_generator_produces_valid_options() {
    let mut gen = OptionGen::new(IntGen::new(0, 10));
    let mut saw_some = false;
    let mut saw_none = false;

    for _ in 0..64 {
        match gen.draw() {
            Some(x) => {
                saw_some = true;
                assert!((0..=10).contains(&x), "Some({x}) out of range");
            }
            None => saw_none = true,
        }
    }

    assert!(saw_some, "option generator never produced Some");
    assert!(saw_none, "option generator never produced None (edge case missing)");
}

/// AC (e): result generator yields Ok(in-range) and Err(bounded string), and
/// both variants appear.
#[test]
fn test_result_generator_produces_valid_results() {
    let mut gen = ResultGen::new(IntGen::new(0, 10), StringGen::new(4));
    let mut saw_ok = false;
    let mut saw_err = false;

    for _ in 0..64 {
        match gen.draw() {
            Ok(x) => {
                saw_ok = true;
                assert!((0..=10).contains(&x), "Ok({x}) out of range");
            }
            Err(s) => {
                saw_err = true;
                assert!(s.len() <= 4, "Err string '{s}' exceeds max_len 4");
            }
        }
    }

    assert!(saw_ok, "result generator never produced Ok");
    assert!(saw_err, "result generator never produced Err (edge case missing)");
}

// ===========================================================================
// AC f — edge cases (0, min, max) appear FIRST in the generated sequence
// ===========================================================================

/// AC (f): the first three draws of an IntGen over [-5, 5] must be exactly the
/// edge cases {0, min=-5, max=5} — zero first — before any interior value. Two
/// default-seeded instances must also produce identical sequences (no random
/// timing).
#[test]
fn test_int_generator_edge_cases_first() {
    let mut a = IntGen::new(-5, 5);
    let mut b = IntGen::new(-5, 5);

    let first_a = a.draw();
    let first_b = b.draw();
    assert_eq!(
        first_a, first_b,
        "two default-seeded generators must produce identical sequences"
    );
    assert_eq!(first_a, 0, "edge case 0 must be the very first draw");

    let first_three = vec![first_a, a.draw(), a.draw()];
    for edge in [-5i64, 0, 5] {
        assert!(
            first_three.contains(&edge),
            "edge case {edge} must appear among the first three draws, got {first_three:?}"
        );
    }

    // Every subsequent draw stays in range.
    for _ in 0..100 {
        let v = a.draw();
        assert!((-5..=5).contains(&v), "draw {v} escaped [min, max]");
    }
}

/// AC (f): with a non-negative range [0, 100] the max edge case 100 must also
/// appear among the earliest draws (zero first).
#[test]
fn test_int_generator_max_edge_case_early() {
    let mut gen = IntGen::new(0, 100);

    assert_eq!(gen.draw(), 0, "zero edge case must be first");
    let second = gen.draw();
    let third = gen.draw();
    assert!(
        second == 100 || third == 100,
        "max edge case 100 must appear within the first three draws, got {second}, {third}"
    );
}

/// AC (f) determinism: explicitly seeded generators are fully reproducible, so
/// tests never depend on random timing.
#[test]
fn test_seeded_generators_are_deterministic() {
    let mut a = IntGen::seeded(42);
    let mut b = IntGen::seeded(42);

    let seq_a: Vec<i64> = (0..10).map(|_| a.draw()).collect();
    let seq_b: Vec<i64> = (0..10).map(|_| b.draw()).collect();
    assert_eq!(
        seq_a, seq_b,
        "same seed must reproduce the same generation sequence: {seq_a:?} vs {seq_b:?}"
    );
}

// ===========================================================================
// AC g — map transforms values
// ===========================================================================

/// AC (g): every value drawn from a mapped generator is the transform of the
/// underlying value (here: doubling, so every draw is even).
#[test]
fn test_map_transforms_values() {
    let mut gen = MapGen::new(IntGen::new(0, 10), |n: i64| n * 2);

    for _ in 0..50 {
        let v = gen.draw();
        assert!(v % 2 == 0, "map must transform every value, got odd {v}");
        assert!((0..=20).contains(&v), "mapped value {v} out of [0, 20]");
    }
}

// ===========================================================================
// Planned-shape generator set (int/nat/float/bool) — light shape pins
// ===========================================================================

/// The nat generator must never yield a negative value and must surface 0
/// first (the zero edge case).
#[test]
fn test_nat_generator_never_negative() {
    let mut gen = NatGen::new(100);

    assert_eq!(gen.draw(), 0, "nat edge case 0 must be first");
    for _ in 0..50 {
        let v = gen.draw();
        assert!((0..=100).contains(&v), "nat draw {v} escaped [0, max]");
    }
}

/// The float generator must yield finite values (0.0 edge case included).
#[test]
fn test_float_generator_yields_finite_values() {
    let mut gen = FloatGen;

    for _ in 0..50 {
        let v = gen.draw();
        assert!(v.is_finite(), "float generator yielded non-finite {v}");
    }
}

/// The bool generator must eventually yield both true and false.
#[test]
fn test_bool_generator_yields_both_values() {
    let mut gen = BoolGen;
    let mut saw_true = false;
    let mut saw_false = false;

    for _ in 0..64 {
        if gen.draw() {
            saw_true = true;
        } else {
            saw_false = true;
        }
    }

    assert!(saw_true, "bool generator never yielded true");
    assert!(saw_false, "bool generator never yielded false");
}
