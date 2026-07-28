/**
 * Dwarf I/O operations (Node.js).
 *
 * Provides basic input/output primitives such as printing and
 * synchronous file read/write.
 *
 * @module
 */

/**
 * Prints a value to stdout (with trailing newline).
 */
export function print(value: unknown): void {
  console.log(value);
}

/**
 * Reads the entire contents of a file as a UTF-8 string.
 */
export function readFile(path: string): string {
  const fs = require('fs');
  return fs.readFileSync(path, 'utf-8');
}

/**
 * Writes a UTF-8 string to a file (synchronous).
 */
export function writeFile(path: string, data: string): void {
  const fs = require('fs');
  fs.writeFileSync(path, data, 'utf-8');
}
