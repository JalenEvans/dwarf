/**
 * Dwarf Result type — represents success (Ok) or failure (Err).
 *
 * Provides monadic operations (map, mapErr, andThen) as well as
 * standard unwrapping and type-narrowing helpers.
 *
 * @module
 */

export type Result<T, E> = Ok<T, E> | Err<T, E>;

export class Ok<T, E> {
  readonly tag = "Ok" as const;
  constructor(public readonly value: T) {}

  isOk(): this is Ok<T, E> { return true; }
  isErr(): this is Err<T, E> { return false; }

  unwrap(): T { return this.value; }
  unwrapOr(_defaultValue: T): T { return this.value; }

  map<U>(fn: (value: T) => U): Result<U, E> { return ok(fn(this.value)); }
  mapErr<F>(_fn: (err: E) => F): Result<T, F> { return ok(this.value) as unknown as Result<T, F>; }

  andThen<U>(fn: (value: T) => Result<U, E>): Result<U, E> { return fn(this.value); }
}

export class Err<T, E> {
  readonly tag = "Err" as const;
  constructor(public readonly error: E) {}

  isOk(): this is Ok<T, E> { return false; }
  isErr(): this is Err<T, E> { return true; }

  unwrap(): never { throw new Error(`Called unwrap on Err: ${this.error}`); }
  unwrapOr<U>(defaultValue: U): U { return defaultValue; }

  map<U>(_fn: (value: T) => U): Result<U, E> { return err(this.error) as unknown as Result<U, E>; }
  mapErr<F>(fn: (err: E) => F): Result<T, F> { return err(fn(this.error)); }

  andThen<U>(_fn: (value: T) => Result<U, E>): Result<U, E> { return err(this.error) as unknown as Result<U, E>; }
}

/**
 * Wraps a value in `Ok`.
 */
export function ok<T, E = never>(value: T): Ok<T, E> {
  return new Ok(value);
}

/**
 * Wraps an error in `Err`.
 */
export function err<T = never, E = unknown>(error: E): Err<T, E> {
  return new Err(error);
}

/**
 * Type guard — returns `true` if the result is `Ok`.
 */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T, E> {
  return result.isOk();
}

/**
 * Type guard — returns `true` if the result is `Err`.
 */
export function isErr<T, E>(result: Result<T, E>): result is Err<T, E> {
  return result.isErr();
}
