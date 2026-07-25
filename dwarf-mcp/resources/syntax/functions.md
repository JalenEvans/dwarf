# Functions in Dwarf

Functions are first-class citizens in Dwarf. They can be pure, I/O-bound, or async.

## Function Declarations

```
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

Functions return the value of their last expression — no explicit `return` keyword needed.

## Effects Annotations

Functions can declare their effect type:

```
// Pure function (default)
fn compute(x: Int) -> Int pure {
    x * 2
}

// I/O function
fn readFile(path: String) -> String io {
    // implementation
}

// Async function
fn fetch(url: String) -> String async {
    // implementation
}
```

## Parameters

Functions accept typed parameters with standard type annotations:

```
fn greet(name: String, greeting: String) -> String {
    greeting + ", " + name + "!"
}
```

## Anonymous Functions (Lambdas)

```
let double = fn(x: Int) -> Int { x * 2 }
let result = double(5)  // 10
```

## Higher-Order Functions

Functions can accept other functions as parameters:

```
fn apply(f: (Int) -> Int, x: Int) -> Int {
    f(x)
}
```

## Function Composition with Pipe

The `|>` operator chains functions in a natural reading order:

```
"hello"
    |> String.toUpper
    |> String.reverse
    |> print
```
