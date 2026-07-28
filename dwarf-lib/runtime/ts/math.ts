/**
 * Dwarf Math operations.
 *
 * Thin wrappers over native `Math` methods, exposed under
 * Dwarf-standard names.
 *
 * @module
 */

/**
 * Returns the absolute value of `x`.
 */
export function abs(x: number): number {
  return Math.abs(x);
}

/**
 * Returns the larger of `a` and `b`.
 */
export function max(a: number, b: number): number {
  return Math.max(a, b);
}

/**
 * Returns the smaller of `a` and `b`.
 */
export function min(a: number, b: number): number {
  return Math.min(a, b);
}
