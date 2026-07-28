"""Dwarf Math utility functions for Python."""
import math


def abs(x: float) -> float:
    return math.fabs(x)


def max(a: float, b: float) -> float:
    return a if a > b else b


def min(a: float, b: float) -> float:
    return a if a < b else b
