# Dwarf Language Overview

Dwarf is a token-efficient, human-readable code transpiler designed for maximum developer productivity. It compiles to TypeScript, Python, and Java, allowing you to write once and target multiple platforms.

## Key Design Principles

- **Expressions-as-Everything**: Everything in Dwarf is an expression, including `if`, `match`, `block`, and `loop` constructs. This makes code more composable and eliminates the distinction between statements and expressions.

- **Token-Efficient Syntax**: Dwarf's syntax is optimized for readability with minimal boilerplate. Type annotations, function declarations, and control flow are all designed to convey maximum intent with minimum characters.

- **Strong Static Typing**: Dwarf features a robust type system with records, unions, generics, and refinement types. The compiler catches errors at compile time rather than runtime.

- **Multi-Platform Output**: Write your code once in Dwarf (`.kzd` files) and compile to idiomatic TypeScript, Python, or Java code.

## Effects System

Dwarf has an explicit effects system that tracks side effects at the type level:

- `pure` — no side effects (deterministic)
- `io` — can perform I/O operations
- `async` — can perform asynchronous operations

This enables reasoning about code purity and helps prevent common bugs related to side effects.

## Pipe Operator

Dwarf includes the `|>` pipe operator for chaining function calls in a readable, left-to-right fashion:

```
value |> transform |> format |> output
```

## File Extension

Dwarf source files use the `.kzd` extension.
