"""Dwarf Option type for Python."""

from typing import Generic, Optional, TypeVar, Union

T = TypeVar("T")
U = TypeVar("U")


class _Some(Generic[T]):
    """Represents a value that exists."""
    def __init__(self, value: T) -> None:
        self._value = value

    def is_some(self) -> bool:
        return True

    def is_none(self) -> bool:
        return False

    def unwrap(self) -> T:
        return self._value

    def unwrap_or(self, default: T) -> T:
        return self._value

    def map(self, fn):
        return some(fn(self._value))

    def flat_map(self, fn):
        return fn(self._value)

    def __repr__(self) -> str:
        return f"Some({self._value!r})"


class _None(Generic[T]):
    """Represents the absence of a value."""
    def is_some(self) -> bool:
        return False

    def is_none(self) -> bool:
        return True

    def unwrap(self) -> T:
        raise ValueError("Called unwrap on None")

    def unwrap_or(self, default: T) -> T:
        return default

    def map(self, fn):
        return none()

    def flat_map(self, fn):
        return none()

    def __repr__(self) -> str:
        return "None"


Option = Union[_Some[T], _None[T]]


def some(value: T) -> _Some[T]:
    return _Some(value)


def none() -> _None[T]:
    return _None()


def is_some(option: Option[T]) -> bool:
    return option.is_some()


def is_none(option: Option[T]) -> bool:
    return option.is_none()
