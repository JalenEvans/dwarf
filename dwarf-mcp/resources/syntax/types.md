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

Refinement types constrain values with predicates:

```
type Natural = Int(n >= 0)
type Percentage = Int(0..100)
type NonEmptyString = String(s != "")
```

Refinements are checked at compile time, eliminating entire classes of runtime errors.

## Type Inference

Dwarf has powerful type inference — explicit type annotations are often optional:

```
let x = 42              // inferred as Int
let name = "Dwarf"      // inferred as String
let nums = [1, 2, 3]    // inferred as List<Int>
```
