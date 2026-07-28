/**
 * Dwarf List operations — wraps native JavaScript arrays.
 *
 * `List<T>` is simply a type alias over `T[]`, so all native array
 * methods are available.  This module provides Dwarf-standard
 * utility functions on top of native arrays.
 *
 * @module
 */

/**
 * Dwarf List type — an alias for native arrays.
 */
export type List<T> = T[];

/**
 * Creates a list from the given items.
 */
export function list<T>(...items: T[]): List<T> {
  return items;
}

/**
 * Returns the number of elements in the list.
 */
export function length<T>(ls: List<T>): number {
  return ls.length;
}

/**
 * Transforms each element of the list with `fn`.
 */
export function map<T, U>(ls: List<T>, fn: (item: T) => U): List<U> {
  return ls.map(fn);
}

/**
 * Keeps only elements for which `pred` returns `true`.
 */
export function filter<T>(ls: List<T>, pred: (item: T) => boolean): List<T> {
  return ls.filter(pred);
}

/**
 * Reduces the list to a single value using `fn` and an `initial` accumulator.
 */
export function reduce<T, U>(ls: List<T>, fn: (acc: U, item: T) => U, initial: U): U {
  return ls.reduce(fn, initial);
}

/**
 * Sums a list of numbers.
 */
export function sum(ls: List<number>): number {
  return ls.reduce((a, b) => a + b, 0);
}

/**
 * Returns a sorted copy of the list (lexicographic order for strings,
 * numeric for numbers via default `.sort()`).
 */
export function sort<T>(ls: List<T>): List<T> {
  const copy = [...ls];
  copy.sort();
  return copy;
}

/**
 * Returns a reversed copy of the list.
 */
export function reverse<T>(ls: List<T>): List<T> {
  return [...ls].reverse();
}
