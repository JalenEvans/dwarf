# Types in Dwarf

Dwarf features a rich, expressive type system.

## Primitive Types

```
Bool
Int
Float
String
Char
```

## Records

Records are structured data types with named fields:

```
type Person {
    name: String
    age: Int
    email: String
}

let alice = Person {
    name: "Alice",
    age: 30,
    email: "alice@example.com"
}

// Field access
alice.name  // "Alice"
```

## Unions (Sum Types)

Unions represent a value that can be one of several variants:

```
type Shape =
    | Circle { radius: Float }
    | Rectangle { width: Float, height: Float }
    | Triangle { base: Float, height: Float }
```

## Generics

Parametric polymorphism allows writing reusable code:

```
type Box<T> {
    value: T
}

fn unwrap<T>(box: Box<T>) -> T {
    box.value
}
```

## Type Aliases

Create meaningful names for existing types:

```
type Age = Int
type Coord = (Int, Int)
```

## Refinement Types

Refinement types constrain values with predicates, checked at compile time.

### Range Constraints

Constrain numeric values to an inclusive range:

```
type Percentage = Int(0..100)
type Age = Int(0..150)
type Temperature = Int(-40..60)
```

### NonEmpty Constraint

The type system supports a `NonEmpty` constraint for strings. When a function parameter has a NonEmpty string type, the compiler rejects empty string literals at compile time:

```
// Given a function with a NonEmpty string parameter:
greet("Alice")  // OK: non-empty string
greet("")       // ERROR: empty string not allowed for NonEmptyString
```

The error code for this violation is `DWARF-E-TYPE-0006`.

### Compile-Time Checking

Refinement constraints are enforced at compile time when literal values are passed:

```
type Score = Int(0..100)

fn setScore(s: Score) { ... }

setScore(85)    // OK: 85 is within 0..100
setScore(150)   // ERROR: value 150 is outside the allowed range 0..100
```

### Error Codes

| Code | Description |
|------|-------------|
| `DWARF-E-TYPE-0006` | Refinement constraint violation |

## Type Inference

Dwarf has powerful type inference — explicit type annotations are often optional:

```
let x = 42              // inferred as Int
let name = "Dwarf"      // inferred as String
let nums = [1, 2, 3]    // inferred as List<Int>
```
