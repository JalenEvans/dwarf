# Type Examples in Dwarf

## Records

```
type Book {
    title: String
    author: String
    year: Int
    isbn: String
}

let book = Book {
    title: "The Pragmatic Programmer",
    author: "Andy Hunt & Dave Thomas",
    year: 1999,
    isbn: "978-0201616224"
}

fn getBookInfo(b: Book) -> String {
    "${b.title} by ${b.author} (${b.year})"
}
```

## Unions with Match

```
type PaymentMethod =
    | CreditCard { number: String, expiry: String }
    | PayPal { email: String }
    | Cash {}

fn processPayment(method: PaymentMethod) -> String {
    match method {
        CreditCard { number, expiry } ->
            "Processing card ending in ${String.slice(-4, number)}"
        PayPal { email } ->
            "Processing PayPal account ${email}"
        Cash -> "Processing cash payment"
    }
}
```

## Generics

```
// A generic pair type
type Pair<A, B> {
    first: A
    second: B
}

fn makePair<A, B>(a: A, b: B) -> Pair<A, B> {
    Pair { first: a, second: b }
}

// Usage
let p1 = makePair(42, "answer")
let p2 = makePair("key", 3.14)

// Generic function
fn swap<A, B>(pair: Pair<A, B>) -> Pair<B, A> {
    Pair { first: pair.second, second: pair.first }
}
```

## Refinement Types

```
type Age = Int(0..150)
type Score = Int(0..100)
type NonEmptyString = String(s != "")

fn canVote(age: Age) -> Bool {
    age >= 18
}

fn grade(score: Score) -> String {
    if score >= 90 { "A" }
    else if score >= 80 { "B" }
    else if score >= 70 { "C" }
    else { "D" }
}
```

## Option Type

```
fn safeDivide(a: Int, b: Int) -> Option<Int> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}
```
