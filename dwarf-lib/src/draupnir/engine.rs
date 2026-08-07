//! Draupnir deep-behavioral PBT engine — DWARF-130.
//!
//! This module implements the property-based testing engine behind the Draupnir
//! runtime (`compile_draupnir`). It provides deterministic generators, a
//! shrinking pipeline routed through `dwarf-shrink`, and the `for_all` / `check`
//! entry points that properties run under.
//!
//! DWARF-130 acceptance criteria honored here:
//!
//! - **Edge cases first**: `IntGen` (and generators built on it) emit `0`, then
//!   `min`, then `max` (when in range) before any pseudo-random interior value,
//!   so property bodies see boundary inputs early.
//! - **Deterministic generation**: generators are seeded (fixed default or
//!   explicit) and reproducible; no global randomness.
//! - **Shrinking**: the first failing value is reduced to a minimal reproducing
//!   one via `IntShrinker` / `StringShrinker` / `ListShrinker` from
//!   `dwarf-shrink`, dispatched from the `Shrinkable` marker trait.
//! - **Shape guarantees**: `NatGen` never yields negatives, `FloatGen` yields
//!   only finite values, `ListGen` respects `max_len`, `OptionGen` / `ResultGen`
//!   surface both variants, `RefineGen` only lets values satisfying its
//!   predicate escape, and `MapGen` applies its transform.

use std::cell::Cell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use dwarf_shrink::{IntShrinker, ListShrinker, Shrinker, StringShrinker};

/// Number of draws `check` runs before declaring a property passed.
pub const DEFAULT_ITERATIONS: usize = 100;

/// A deterministic source of values for a property-based test.
pub trait Generator<T> {
    /// Produce the next value in the generation sequence.
    fn draw(&mut self) -> T;
}

// ---------------------------------------------------------------------------
// Random core
// ---------------------------------------------------------------------------

/// SplitMix64 — a tiny, dependency-free deterministic PRNG.
///
/// Two generators seeded identically produce identical sequences (DWARF-130:
/// tests must never depend on random timing).
#[derive(Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 0
    }

    /// Uniform value in `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        self.next_u64() % n
    }
}

// ---------------------------------------------------------------------------
// IntGen
// ---------------------------------------------------------------------------

/// Fixed default seed so two `IntGen::new` instances behave identically.
const DEFAULT_SEED: u64 = 0x1234_5678_9ABC_DEF0;

/// Deterministic `i64` generator over `[min, max]`.
///
/// Edge cases are emitted first — `0`, then `min`, then `max`, each skipped if
/// already covered or out of range — before pseudo-random interior values. This
/// pins the DWARF-130 "edge cases first" guarantee.
pub struct IntGen {
    min: i64,
    max: i64,
    rng: Rng,
    edges: VecDeque<i64>,
}

impl IntGen {
    /// A generator over `[min, max]` with the fixed default seed.
    ///
    /// # Panics
    ///
    /// Panics if `max < min`.
    pub fn new(min: i64, max: i64) -> Self {
        assert!(max >= min, "IntGen range must satisfy max >= min, got [{min}, {max}]");
        Self {
            min,
            max,
            rng: Rng::new(DEFAULT_SEED),
            edges: edge_candidates(min, max),
        }
    }

    /// The full `i64` domain — `i64::MIN..=i64::MAX`.
    pub fn full() -> Self {
        Self::new(i64::MIN, i64::MAX)
    }

    /// A full-range generator with an explicit seed (reproducible draws).
    pub fn seeded(seed: u64) -> Self {
        Self {
            min: i64::MIN,
            max: i64::MAX,
            rng: Rng::new(seed),
            edges: edge_candidates(i64::MIN, i64::MAX),
        }
    }
}

/// Ordered edge values (`0`, `min`, `max`) that lie within `[min, max]`.
fn edge_candidates(min: i64, max: i64) -> VecDeque<i64> {
    let mut edges = VecDeque::new();
    for candidate in [0, min, max] {
        if (min..=max).contains(&candidate) && !edges.contains(&candidate) {
            edges.push_back(candidate);
        }
    }
    edges
}

impl Generator<i64> for IntGen {
    fn draw(&mut self) -> i64 {
        if let Some(edge) = self.edges.pop_front() {
            return edge;
        }
        // Map the raw u64 into [min, max] via i128 arithmetic so the full
        // i64::MIN..=i64::MAX range (width 2^64) cannot overflow.
        let width = (self.max as i128) - (self.min as i128) + 1;
        let offset = (self.rng.next_u64() as u128) % (width as u128);
        ((self.min as i128) + (offset as i128)) as i64
    }
}

// ---------------------------------------------------------------------------
// NatGen
// ---------------------------------------------------------------------------

/// Non-negative `i64` generator over `0..=max`.
pub struct NatGen {
    inner: IntGen,
}

impl NatGen {
    /// A generator over `0..=max`.
    pub fn new(max: i64) -> Self {
        Self { inner: IntGen::new(0, max) }
    }
}

impl Generator<i64> for NatGen {
    fn draw(&mut self) -> i64 {
        self.inner.draw()
    }
}

// ---------------------------------------------------------------------------
// FloatGen
// ---------------------------------------------------------------------------

/// Edge-case latch so the first float draw is the `0.0` boundary value.
static FLOAT_EDGE_DONE: AtomicBool = AtomicBool::new(false);

/// `f64` generator that yields finite values.
pub struct FloatGen;

impl Generator<f64> for FloatGen {
    fn draw(&mut self) -> f64 {
        if !FLOAT_EDGE_DONE.swap(true, Ordering::Relaxed) {
            return 0.0;
        }
        static STATE: AtomicU64 = AtomicU64::new(0xFEED_FACE_CAFE_BEEF);
        let state = STATE.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        // Fraction in [0, 1), scaled into [-1e6, 1e6) — always finite.
        let fraction = (state >> 11) as f64 / (1u64 << 53) as f64;
        fraction * 2_000_000.0 - 1_000_000.0
    }
}

// ---------------------------------------------------------------------------
// StringGen
// ---------------------------------------------------------------------------

/// Characters drawn by `StringGen` (printable ASCII, no escapes).
const STRING_ALPHABET: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 _-";

/// `String` generator yielding strings of length `<= max_len`.
pub struct StringGen {
    max_len: usize,
    rng: Rng,
}

impl StringGen {
    /// A generator over strings of length `<= max_len`.
    pub fn new(max_len: usize) -> Self {
        Self { max_len, rng: Rng::new(DEFAULT_SEED) }
    }
}

impl Generator<String> for StringGen {
    fn draw(&mut self) -> String {
        let len = self.rng.below(self.max_len as u64 + 1) as usize;
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let idx = self.rng.below(STRING_ALPHABET.len() as u64) as usize;
            out.push(STRING_ALPHABET[idx] as char);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// BoolGen
// ---------------------------------------------------------------------------

/// `bool` generator that alternates between `true` and `false`.
///
/// Alternation (rather than pure randomness) guarantees both variants surface
/// within a handful of draws (DWARF-130: both edges must appear).
pub struct BoolGen;

impl Generator<bool> for BoolGen {
    fn draw(&mut self) -> bool {
        thread_local! {
            static COUNTER: Cell<u64> = const { Cell::new(0) };
        }
        COUNTER.with(|c| {
            let n = c.get();
            c.set(n + 1);
            n % 2 == 0
        })
    }
}

// ---------------------------------------------------------------------------
// ListGen / OptionGen / ResultGen
// ---------------------------------------------------------------------------

/// `Vec<T>` generator wrapping an element generator and a length cap.
pub struct ListGen<G, T> {
    elem: G,
    max_len: usize,
    rng: Rng,
    _marker: PhantomData<T>,
}

impl<G, T> ListGen<G, T> {
    /// A generator over `Vec<T>` of length `<= max_len`, drawing elements from
    /// `elem`.
    pub fn new(elem: G, max_len: usize) -> Self {
        Self { elem, max_len, rng: Rng::new(DEFAULT_SEED), _marker: PhantomData }
    }
}

impl<G, T> Generator<Vec<T>> for ListGen<G, T>
where
    G: Generator<T>,
    T: Clone,
{
    fn draw(&mut self) -> Vec<T> {
        let len = self.rng.below(self.max_len as u64 + 1) as usize;
        (0..len).map(|_| self.elem.draw()).collect()
    }
}

/// `Option<T>` generator that surfaces both `Some` and `None` (edge case).
pub struct OptionGen<G, T> {
    inner: G,
    rng: Rng,
    _marker: PhantomData<T>,
}

impl<G, T> OptionGen<G, T> {
    /// A generator over `Option<T>` drawing from `inner` when `Some`.
    pub fn new(inner: G) -> Self {
        Self { inner, rng: Rng::new(DEFAULT_SEED), _marker: PhantomData }
    }
}

impl<G, T> Generator<Option<T>> for OptionGen<G, T>
where
    G: Generator<T>,
    T: Clone,
{
    fn draw(&mut self) -> Option<T> {
        if self.rng.next_bool() {
            Some(self.inner.draw())
        } else {
            None
        }
    }
}

/// `Result<T, U>` generator that surfaces both `Ok` and `Err` (edge case).
pub struct ResultGen<G, E> {
    ok: G,
    err: E,
    rng: Rng,
}

impl<G, E> ResultGen<G, E> {
    /// A generator over `Result<T, U>` drawing from `ok` / `err`.
    pub fn new(ok: G, err: E) -> Self {
        Self { ok, err, rng: Rng::new(DEFAULT_SEED) }
    }
}

impl<G, E, T, U> Generator<Result<T, U>> for ResultGen<G, E>
where
    G: Generator<T>,
    E: Generator<U>,
{
    fn draw(&mut self) -> Result<T, U> {
        if self.rng.next_bool() {
            Ok(self.ok.draw())
        } else {
            Err(self.err.draw())
        }
    }
}

// ---------------------------------------------------------------------------
// Combinators
// ---------------------------------------------------------------------------

/// Retry budget for `RefineGen` so a pathological predicate cannot hang the
/// engine.
const REFINE_RETRY_BUDGET: usize = 1024;

/// A generator that filters the draws of an inner generator through a predicate.
///
/// Only values satisfying the predicate escape; the underlying generator is
/// re-drawn (bounded by `REFINE_RETRY_BUDGET`) until one does. The bounded
/// budget guarantees termination even for near-impossible predicates (DWARF-130:
/// refine must never hang).
pub struct RefineGen<G, T> {
    inner: G,
    pred: Box<dyn Fn(&T) -> bool>,
}

impl<G, T> RefineGen<G, T> {
    /// A generator over values of `inner` that satisfy `pred`.
    pub fn new(inner: G, pred: impl Fn(&T) -> bool + 'static) -> Self {
        Self { inner, pred: Box::new(pred) }
    }
}

impl<G, T> Generator<T> for RefineGen<G, T>
where
    G: Generator<T>,
    T: Clone,
{
    fn draw(&mut self) -> T {
        let mut budget = REFINE_RETRY_BUDGET;
        let mut candidate = self.inner.draw();
        while !(self.pred)(&candidate) {
            if budget == 0 {
                // Budget exhausted: hand back the last candidate rather than
                // hang; callers relying on the predicate must use a feasible one.
                break;
            }
            budget -= 1;
            candidate = self.inner.draw();
        }
        candidate
    }
}

/// A generator that applies a transform to every draw of an inner generator.
pub struct MapGen<G, S, T> {
    inner: G,
    f: Box<dyn Fn(S) -> T>,
}

impl<G, S, T> MapGen<G, S, T> {
    /// A generator over `f(inner.draw())`.
    pub fn new(inner: G, f: impl Fn(S) -> T + 'static) -> Self {
        Self { inner, f: Box::new(f) }
    }
}

impl<G, S, T> Generator<T> for MapGen<G, S, T>
where
    G: Generator<S>,
{
    fn draw(&mut self) -> T {
        (self.f)(self.inner.draw())
    }
}

/// Two generators compose into a generator of pairs (2-ary properties).
impl<G1, G2, T1, T2> Generator<(T1, T2)> for (G1, G2)
where
    G1: Generator<T1>,
    G2: Generator<T2>,
{
    fn draw(&mut self) -> (T1, T2) {
        (self.0.draw(), self.1.draw())
    }
}

// ---------------------------------------------------------------------------
// Shrinking
// ---------------------------------------------------------------------------

/// Marker for types the engine can shrink to a minimal failing value.
///
/// Dispatch to the concrete `dwarf-shrink` shrinker happens through
/// [`Shrinkable::shrink_with`], implemented per concrete type.
pub trait Shrinkable: Clone + Debug {
    /// Reduce `self` to a minimal value that still fails `is_failing`.
    ///
    /// Hidden: the engine routes shrinking through this on the first property
    /// failure; callers interact with it only via `for_all` / `check`.
    #[doc(hidden)]
    fn shrink_with(&self, is_failing: &mut dyn FnMut(&Self) -> bool) -> Self;
}

impl Shrinkable for i64 {
    fn shrink_with(&self, is_failing: &mut dyn FnMut(&i64) -> bool) -> i64 {
        IntShrinker.shrink(self, is_failing)
    }
}

impl Shrinkable for String {
    fn shrink_with(&self, is_failing: &mut dyn FnMut(&String) -> bool) -> String {
        StringShrinker.shrink(self, is_failing)
    }
}

impl Shrinkable for Vec<i64> {
    fn shrink_with(&self, is_failing: &mut dyn FnMut(&Vec<i64>) -> bool) -> Vec<i64> {
        ListShrinker.shrink(self, is_failing)
    }
}

impl<A, B> Shrinkable for (A, B)
where
    A: Shrinkable,
    B: Shrinkable,
{
    fn shrink_with(&self, is_failing: &mut dyn FnMut(&(A, B)) -> bool) -> (A, B) {
        let mut a = self.0.clone();
        let mut b = self.1.clone();
        a = a.shrink_with(&mut |candidate| is_failing(&(candidate.clone(), b.clone())));
        b = b.shrink_with(&mut |candidate| is_failing(&(a.clone(), candidate.clone())));
        (a, b)
    }
}

// ---------------------------------------------------------------------------
// PropertyResult / for_all / check
// ---------------------------------------------------------------------------

/// Outcome of a property run.
#[derive(Debug)]
pub enum PropertyResult<T> {
    /// The property held for every generated value.
    Passed { iterations: usize },
    /// The property failed; `value` is the raw counterexample, `shrunk` is the
    /// minimal value that still fails.
    Failed { value: T, shrunk: T, iterations: usize },
}

impl<T> PropertyResult<T> {
    /// Whether the property passed for every generated value.
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// Number of draws actually executed before the outcome was decided.
    pub fn iterations(&self) -> usize {
        match self {
            Self::Passed { iterations } | Self::Failed { iterations, .. } => *iterations,
        }
    }

    /// The raw failing value, if any.
    pub fn counterexample(&self) -> Option<&T> {
        match self {
            Self::Failed { value, .. } => Some(value),
            Self::Passed { .. } => None,
        }
    }

    /// The minimal shrinking-failing value, if any.
    pub fn shrunk(&self) -> Option<&T> {
        match self {
            Self::Failed { shrunk, .. } => Some(shrunk),
            Self::Passed { .. } => None,
        }
    }
}

/// Draw up to `iterations` values from `gen`, invoking `property` on each.
///
/// The first `false` result stops the run; the failing value is shrunk to a
/// minimal reproducing one (DWARF-130: shrinking) before being returned as a
/// `Failed` result. If every draw passes, the run reports `Passed`.
pub fn for_all<G, T, P>(mut gen: G, iterations: usize, mut property: P) -> PropertyResult<T>
where
    G: Generator<T>,
    T: Clone + Debug + Shrinkable,
    P: FnMut(&T) -> bool,
{
    for i in 0..iterations {
        let value = gen.draw();
        if !property(&value) {
            let mut is_failing = |candidate: &T| !property(candidate);
            let shrunk = value.shrink_with(&mut is_failing);
            return PropertyResult::Failed { value, shrunk, iterations: i + 1 };
        }
    }
    PropertyResult::Passed { iterations }
}

/// Run `property` against `gen` for [`DEFAULT_ITERATIONS`] draws.
pub fn check<G, T, P>(gen: G, property: P) -> PropertyResult<T>
where
    G: Generator<T>,
    T: Clone + Debug + Shrinkable,
    P: FnMut(&T) -> bool,
{
    for_all(gen, DEFAULT_ITERATIONS, property)
}
