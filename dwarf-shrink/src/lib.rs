//! Shrinking engine — reduces failing counterexamples to minimal reproducing inputs.

/// A shrinker that takes a failing value and produces a minimal counterexample.
pub trait Shrinker<T> {
    fn shrink(&self, value: &T, is_failing: &mut dyn FnMut(&T) -> bool) -> T;
}

/// Shrink integers using binary search.
pub struct IntShrinker;

impl Shrinker<i64> for IntShrinker {
    fn shrink(&self, value: &i64, is_failing: &mut dyn FnMut(&i64) -> bool) -> i64 {
        let failing = *value;
        if failing == 0 {
            return 0;
        }

        // Binary search helper.
        // `pass` is a value where is_failing returns false.
        // `fail` is a value where is_failing returns true.
        // Returns the smallest (closest to pass) value that still fails.
        let between = |mut pass: i64, mut fail: i64, f: &mut dyn FnMut(&i64) -> bool| -> i64 {
            assert!(pass != fail);
            // Ensure we search between them regardless of sign
            while pass.abs_diff(fail) > 1 {
                let mid = pass + (fail - pass) / 2;
                if f(&mid) {
                    fail = mid;
                } else {
                    pass = mid;
                }
            }
            fail
        };

        // If the initial value doesn't fail, walk toward 0 to find a failing value
        if !is_failing(&failing) {
            if failing < 0 {
                for candidate in failing + 1..=0 {
                    if is_failing(&candidate) {
                        return between(failing, candidate, is_failing);
                    }
                }
            } else {
                for candidate in (0..failing).rev() {
                    if is_failing(&candidate) {
                        return between(candidate, failing, is_failing);
                    }
                }
            }
            return failing;
        }

        // Initial value fails. Check if 0 passes.
        if !is_failing(&0) {
            // 0 passes. Search between 0 (pass) and failing (fail).
            if failing > 0 {
                return between(0, failing, is_failing);
            } else {
                // failing < 0: 0 is the pass, failing is the fail
                return between(0, failing, is_failing);
            }
        }

        // Both 0 and failing value fail. Try halving toward 0 to find a passing point.
        let mut current = failing;
        loop {
            let next = current / 2;
            if next == current {
                break;
            }
            if !is_failing(&next) {
                return if failing > 0 {
                    between(next, current, is_failing)
                } else {
                    // failing < 0: next > current (closer to 0), so next passes, current fails
                    between(next, current, is_failing)
                };
            }
            current = next;
            if current == 0 {
                break;
            }
        }
        current
    }
}

/// Shrink strings by removing characters.
pub struct StringShrinker;

impl Shrinker<String> for StringShrinker {
    fn shrink(&self, value: &String, is_failing: &mut dyn FnMut(&String) -> bool) -> String {
        let mut current = value.clone();
        if current.is_empty() {
            return current;
        }

        // Phase 1: Remove halves
        loop {
            let chars: Vec<char> = current.chars().collect();
            let len = chars.len();
            if len <= 1 {
                break;
            }
            let mid = len / 2;

            let first_half: String = chars.iter().take(mid).collect();
            if is_failing(&first_half) {
                current = first_half;
                continue;
            }

            let second_half: String = chars.iter().skip(mid).collect();
            if is_failing(&second_half) {
                current = second_half;
                continue;
            }
            break;
        }

        // Phase 2: Remove individual characters
        loop {
            let chars: Vec<char> = current.chars().collect();
            let len = chars.len();
            if len <= 1 {
                break;
            }

            let mut found = false;
            for i in 0..len {
                let candidate: String = chars
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, c)| c)
                    .collect();
                if is_failing(&candidate) && !candidate.is_empty() {
                    current = candidate;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        current
    }
}

/// Shrink lists by removing elements.
pub struct ListShrinker;

impl<T: Clone + PartialEq> Shrinker<Vec<T>> for ListShrinker {
    fn shrink(&self, value: &Vec<T>, is_failing: &mut dyn FnMut(&Vec<T>) -> bool) -> Vec<T> {
        let mut current = value.clone();
        if current.is_empty() {
            return current;
        }

        // Phase 1: Remove halves
        loop {
            let len = current.len();
            if len <= 1 {
                break;
            }
            let mid = len / 2;

            let first_half: Vec<T> = current.iter().take(mid).cloned().collect();
            if is_failing(&first_half) {
                current = first_half;
                continue;
            }

            let second_half: Vec<T> = current.iter().skip(mid).cloned().collect();
            if is_failing(&second_half) {
                current = second_half;
                continue;
            }
            break;
        }

        // Phase 2: Remove individual elements
        loop {
            let len = current.len();
            if len <= 1 {
                break;
            }

            let mut found = false;
            for i in 0..len {
                let candidate: Vec<T> = current
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, v)| v.clone())
                    .collect();
                if is_failing(&candidate) && !candidate.is_empty() {
                    current = candidate;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        current
    }
}
