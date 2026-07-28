/**
 * Dwarf String utility operations.
 *
 * Wraps native JavaScript string methods under Dwarf-standard names.
 *
 * @module
 */

/**
 * Splits a string by the given delimiter.
 */
export function split(s: string, delimiter: string): string[] {
  return s.split(delimiter);
}

/**
 * Converts the string to uppercase.
 */
export function toUpper(s: string): string {
  return s.toUpperCase();
}

/**
 * Converts the string to lowercase.
 */
export function toLower(s: string): string {
  return s.toLowerCase();
}

/**
 * Reverses the characters of the string.
 */
export function reverse(s: string): string {
  return s.split('').reverse().join('');
}

/**
 * Returns `true` if `s` contains `sub`.
 */
export function contains(s: string, sub: string): boolean {
  return s.includes(sub);
}

/**
 * Removes leading and trailing whitespace.
 */
export function trim(s: string): string {
  return s.trim();
}

/**
 * Returns the length of the string.
 */
export function stringLength(s: string): number {
  return s.length;
}
