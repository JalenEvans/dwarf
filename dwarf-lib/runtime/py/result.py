"""Dwarf Result type for Python."""
from typing import Generic, TypeVar, Union

T = TypeVar("T")
E = TypeVar("E")
U = TypeVar("U")
F = TypeVar("F")


class _Ok(Generic[T, E]):
    def __init__(self, value: T) -> None:
        self._value = value

    def is_ok(self) -> bool:
        return True

    def is_err(self) -> bool:
        return False

    def unwrap(self) -> T:
        return self._value

    def unwrap_or(self, default: T) -> T:
        return self._value

    def map(self, fn):
        return ok(fn(self._value))

    def map_err(self, fn):
        return ok(self._value)

    def and_then(self, fn):
        return fn(self._value)

    def __repr__(self) -> str:
        return f"Ok({self._value!r})"


class _Err(Generic[T, E]):
    def __init__(self, error: E) -> None:
        self._error = error

    def is_ok(self) -> bool:
        return False

    def is_err(self) -> bool:
        return True

    def unwrap(self) -> T:
        raise ValueError(f"Called unwrap on Err: {self._error}")

    def unwrap_or(self, default: T) -> T:
        return default

    def map(self, fn):
        return err(self._error)

    def map_err(self, fn):
        return err(fn(self._error))

    def and_then(self, fn):
        return err(self._error)

    def __repr__(self) -> str:
        return f"Err({self._error!r})"


Result = Union[_Ok[T, E], _Err[T, E]]


def ok(value: T) -> _Ok[T, E]:
    return _Ok(value)


def err(error: E) -> _Err[T, E]:
    return _Err(error)


def is_ok(result: Result[T, E]) -> bool:
    return result.is_ok()


def is_err(result: Result[T, E]) -> bool:
    return result.is_err()
