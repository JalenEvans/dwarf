"""Dwarf I/O operations for Python."""


def print_out(value) -> None:
    print(value)


def read_file(path: str) -> str:
    with open(path, "r") as f:
        return f.read()


def write_file(path: str, data: str) -> None:
    with open(path, "w") as f:
        f.write(data)
