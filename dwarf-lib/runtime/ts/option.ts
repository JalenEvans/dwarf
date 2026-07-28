/**
 * Dwarf Option type — represents an optional value that may be Some(T) or None.
 *
 * Provides monadic operations (map, flatMap) as well as standard
 * unwrapping and type-narrowing helpers.
 *
 * @module
 */

export type Option<T> = Some<T> | None;

export class Some<T> {
  readonly tag = "Some" as const;
  constructor(public readonly value: T) {}

  isSome(): this is Some<T> { return true; }
  isNone(): this is None { return false; }

  unwrap(): T { return this.value; }
  unwrapOr(_defaultValue: T): T { return this.value; }

  map<U>(fn: (value: T) => U): Option<U> { return some(fn(this.value)); }
  flatMap<U>(fn: (value: T) => Option<U>): Option<U> { return fn(this.value); }
}

export class None {
  readonly tag = "None" as const;

  isSome(): this is Some<never> { return false; }
  isNone(): this is None { return true; }

  unwrap(): never { throw new Error("Called unwrap on None"); }
  unwrapOr<T>(defaultValue: T): T { return defaultValue; }

  map<U>(_fn: (value: never) => U): Option<U> { return none(); }
  flatMap<U>(_fn: (value: never) => Option<U>): Option<U> { return none(); }
}

/**
 * Wraps a value in `Some`.
 */
export function some<T>(value: T): Some<T> {
  return new Some(value);
}

/**
 * Returns `None`.
 */
export function none<T>(): Option<T> {
  return new None();
}

/**
 * Type guard — returns `true` if the option is `Some`.
 */
export function isSome<T>(option: Option<T>): option is Some<T> {
  return option.isSome();
}

/**
 * Type guard — returns `true` if the option is `None`.
 */
export function isNone<T>(option: Option<T>): option is None {
  return option.isNone();
}
