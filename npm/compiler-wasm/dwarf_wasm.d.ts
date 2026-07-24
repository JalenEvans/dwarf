/* tslint:disable */
/* eslint-disable */

/**
 * Compile Dwarf source code and return the result as a JSON string.
 *
 * JS usage:
 * ```js
 * const result = JSON.parse(compile(source, filename, JSON.stringify({ target: "ts" })));
 * // result = { success: bool, output: string, diagnostics: [...], outputExtension: string }
 * ```
 */
export function compile(source: string, filename: string, options_json: string): string;

/**
 * Compile Dwarf source code and return the result as a JSON string.
 * Simplified version with just source and filename (uses defaults).
 */
export function compile_simple(source: string, filename: string): string;

/**
 * Get the version of the compiler.
 */
export function version(): string;
