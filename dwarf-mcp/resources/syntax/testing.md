# Testing in Dwarf

Dwarf includes first-class testing support with the `@test` decorator and property-based testing via `forAll`.

## Unit Tests with @test

Define tests using the `@test` decorator on a function:

```
@test
fn testAddition() {
    assert(add(2, 3) == 5)
}

@test
fn testStringConcat() {
    assert("hello" + " " + "world" == "hello world")
}
```

## Assertions

The `assert` function checks that a condition is true:

```
@test
fn testWithMessage() {
    assert(result == expected, "Expected ${expected} but got ${result}")
}
```

## Property-Based Testing

Dwarf supports property-based testing with `forAll`, which auto-generates test cases:

```
@test
fn testSortPreservesLength() {
    forAll(list: List<Int>) {
        assert(List.sort(list).length() == list.length())
    }
}
```

## Testing Edge Cases

```
@test
fn testEmptyList() {
    assert(List.sort([]) == [])
}

@test
fn testSingleElement() {
    assert(List.sort([42]) == [42])
}

@test
fn testNegativeNumbers() {
    assert(List.sum([-1, 0, 1]) == 0)
}
```
