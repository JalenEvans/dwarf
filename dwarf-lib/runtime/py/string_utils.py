"""Dwarf String utility functions for Python."""


def split(s: str, delimiter: str) -> list:
    return s.split(delimiter)


def to_upper(s: str) -> str:
    return s.upper()


def to_lower(s: str) -> str:
    return s.lower()


def reverse(s: str) -> str:
    return s[::-1]


def contains(s: str, sub: str) -> bool:
    return sub in s


def trim(s: str) -> str:
    return s.strip()


def string_length(s: str) -> int:
    return len(s)
