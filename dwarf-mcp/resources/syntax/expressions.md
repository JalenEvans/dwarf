# Expressions in Dwarf

In Dwarf, everything is an expression. Every construct produces a value that can be bound, passed, or composed.

## If Expressions

```
let status = if score >= 60 {
    "pass"
} else {
    "fail"
}
```

## Match Expressions

Match expressions enable powerful pattern matching:

```
let description = match shape {
    Circle { radius } -> "Circle with radius " + radius
    Rectangle { width, height } ->
        width + "x" + height + " rectangle"
    Triangle { base, height } ->
        "Triangle with base " + base
}
```

## Block Expressions

Blocks group multiple expressions and return the last one:

```
let result = {
    let x = 10
    let y = 20
    x + y
}
// result is 30
```

## Loop Expressions

Loops can return values using `break` with a value:

```
let first = for x in list {
    if condition(x) {
        break x
    }
}
```

## Pipe Operator

Functional pipelines with the `|>` operator:

```
let result = numbers
    |> List.filter(fn(x) -> x > 0)
    |> List.map(fn(x) -> x * 2)
    |> List.sum
```

## String Interpolation

```
let name = "Dwarf"
let msg = "Hello, ${name}!"
```
