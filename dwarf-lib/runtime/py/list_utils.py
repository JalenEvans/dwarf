"""Dwarf List utility functions for Python."""
from typing import Callable, List, TypeVar

T = TypeVar("T")
U = TypeVar("U")


def map_list(ls: List[T], fn: Callable[[T], U]) -> List[U]:
    return [fn(x) for x in ls]


def filter_list(ls: List[T], pred: Callable[[T], bool]) -> List[T]:
    return [x for x in ls if pred(x)]


def reduce_list(ls: List[T], fn: Callable[[U, T], U], initial: U) -> U:
    result = initial
    for x in ls:
        result = fn(result, x)
    return result


def sum_list(ls: List[float]) -> float:
    return sum(ls)


def sort_list(ls: List[T]) -> List[T]:
    return sorted(ls)


def reverse_list(ls: List[T]) -> List[T]:
    return list(reversed(ls))


def list_length(ls: List[T]) -> int:
    return len(ls)
