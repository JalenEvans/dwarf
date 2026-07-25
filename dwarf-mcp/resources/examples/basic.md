# Basic Dwarf Examples

## Hello World

```
import "std/io"

fn main() -> () io {
    print("Hello, World!")
}
```

## Arithmetic

```
fn arithmetic() {
    let sum = 10 + 20
    let diff = 100 - 30
    let product = 6 * 7
    let quotient = 42 / 2
    let remainder = 17 % 5

    // Expressions compose naturally
    let result = (1 + 2) * (10 - 4) / 3  // 6
}
```

## String Manipulation

```
fn stringExamples() {
    let greeting = "Hello"
    let name = "Dwarf"
    let message = greeting + ", " + name + "!"  // "Hello, Dwarf!"

    let upper = String.toUpper(name)   // "DWARF"
    let length = String.length(message) // 14
    let words = String.split("a,b,c", ",")  // ["a", "b", "c"]
}
```

## I/O Examples

```
import "std/io"

fn readAndPrint(path: String) -> () io {
    let content = readFile(path)
    print(content)
}

fn interactive() -> () io {
    print("What is your name?")
    let name = readLine()
    print("Hello, ${name}!")
}
```

## Using the Pipe Operator

```
import "std/io"

fn processData(nums: List<Int>) -> Int io {
    nums
        |> List.filter(fn(x) -> x > 0)
        |> List.map(fn(x) -> x * 2)
        |> List.sum
}

fn main() -> () io {
    let result = processData([-3, 0, 5, 7, -1, 10])
    print(result)  // 44
}
```
