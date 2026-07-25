# Testing Examples in Dwarf

## Unit Tests

```
import "std/testing"

@test
fn testAdd() {
    assert(add(2, 3) == 5)
    assert(add(-1, 1) == 0)
    assert(add(0, 0) == 0)
}

@test
fn testStringOperations() {
    assert(String.toUpper("hello") == "HELLO")
    assert(String.length("dwarf") == 5)
    assert(String.contains("hello world", "world"))
}
```

## Property-Based Testing

```
@test
fn testReverseReverse() {
    forAll(ls: List<Int>) {
        assert(List.reverse(List.reverse(ls)) == ls)
    }
}

@test
fn testSortIdempotent() {
    forAll(ls: List<Int>) {
        let sorted = List.sort(ls)
        assert(List.sort(sorted) == sorted)
    }
}
```

## Testing Edge Cases

```
@test
fn testDivisionByZero() {
    assert(safeDivide(10, 0) == None)
}

@test
fn testEmptyString() {
    assert(String.length("") == 0)
    assert(String.toUpper("") == "")
}

@test
fn testBoundaryValues() {
    // Testing refinement type boundaries
    assert(isValidAge(0))
    assert(isValidAge(150))
    assert(!isValidAge(-1))
    assert(!isValidAge(151))
}
```
