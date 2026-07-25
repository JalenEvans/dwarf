# Dwarf Standard Library Reference

The Dwarf standard library provides built-in types and common functions available in every Dwarf program.

## Built-in Types

| Type    | Description              | Example           |
|---------|--------------------------|-------------------|
| `Bool`  | Boolean values           | `true`, `false`   |
| `Int`   | Integer numbers          | `42`, `-7`        |
| `Float` | Floating-point numbers   | `3.14`, `-0.5`    |
| `String`| Text strings             | `"hello"`         |
| `Char`  | Single characters        | `'a'`, `'\n'`     |
| `List`  | Ordered collections      | `[1, 2, 3]`       |
| `Map`   | Key-value mappings       | `{"a": 1}`        |
| `Option`| Optional values          | `Some(x)`, `None` |
| `Result`| Success/failure results  | `Ok(v)`, `Err(e)` |

## Common Functions

### String Operations

```
String.length(s)       -> Int
String.toUpper(s)      -> String
String.toLower(s)      -> String
String.reverse(s)      -> String
String.contains(s, sub) -> Bool
String.split(s, delim)  -> List<String>
String.trim(s)          -> String
```

### List Operations

```
List.length(ls)        -> Int
List.map(ls, f)        -> List<T>
List.filter(ls, pred)  -> List<T>
List.reduce(ls, f)     -> T
List.sum(ls)           -> Int
List.sort(ls)          -> List<T>
List.reverse(ls)       -> List<T>
```

### I/O Operations

```
print(value)           -> () io
readFile(path)         -> String io
writeFile(path, data)  -> () io
```

### Math Operations

```
abs(x)                 -> Int/ Float
max(a, b)              -> Int/ Float
min(a, b)              -> Int/ Float
```
