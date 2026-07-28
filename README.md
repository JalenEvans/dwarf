# dwarf

[![CI](https://github.com/JalenEvans/dwarf/actions/workflows/ci.yml/badge.svg)](https://github.com/JalenEvans/dwarf/actions/workflows/ci.yml)

A token efficient, human-readable code transpiler that is designed for modern agentic coding.

## CLI Commands

### `dwarf build <file> --target <lang>`

Transpile Dwarf source files to a target language and write the output to disk.

```
dwarf build input.kzd --target ts
dwarf build input.kzd --target ts --out-dir custom/path
dwarf build input.kzd --target ts --pretty
```

| Option | Description |
|---|---|
| `--target` | Target language (e.g., `ts`, `debug`) |
| `--out-dir` | Output directory (default: `dist/{target}`) |
| `--pretty` | Apply pretty formatting to output |
| `--passes` | Comma-separated list of passes to run |
| `--skip-passes` | Comma-separated list of passes to skip |

### `dwarf check <file>`

Check Dwarf source files for errors without emitting code.

```
dwarf check input.kzd
dwarf check input.kzd --json
```

| Option | Description |
|---|---|
| `--json` | Output diagnostics as JSON |
| `--passes` | Comma-separated list of passes to run |
| `--skip-passes` | Comma-separated list of passes to skip |

### `dwarf emit <file> --target <lang>`

Emit code from Dwarf source files to a target language (stdout).

```
dwarf emit input.kzd --target ts
```

| Option | Description |
|---|---|
| `--target` | Target language (e.g., `ts`, `debug`) |
| `--json` | Output diagnostics as JSON |
| `--passes` | Comma-separated list of passes to run |
| `--skip-passes` | Comma-separated list of passes to skip |

### `dwarf run <file> --target <lang>`

Transpile and execute a Dwarf source file in one step.

```
dwarf run input.kzd --target ts
dwarf run input.kzd --target ts --passes tokenize,parse
```

| Option | Description |
|---|---|
| `--target` | Target language (required) |
| `--passes` | Comma-separated list of passes to run |
| `--skip-passes` | Comma-separated list of passes to skip |

### `dwarf dev <file> --target <lang>`

Watch a Dwarf source file for changes and automatically re-transpile and re-run on each change.

```
dwarf dev input.kzd --target ts
dwarf dev input.kzd --target ts --skip-passes typecheck
```

| Option | Description |
|---|---|
| `--target` | Target language (required) |
| `--passes` | Comma-separated list of passes to run |
| `--skip-passes` | Comma-separated list of passes to skip |

### `dwarf --list-runtimes`

List all available runtime targets and exit.

```
dwarf --list-runtimes
```

### `dwarf install <package>`

Install a package and generate an extern declaration stub for FFI interop.

```
dwarf install npm:express
dwarf install py:json
dwarf install java:java.util.ArrayList
```

Supported prefixes: `npm`, `py`, `java`. For Java, the last dotted segment becomes the function name (e.g. `java:java.util.ArrayList` → `extern "java:java.util" fn ArrayList()`).

## FFI & Host Interop

Dwarf can call external target-language code via **extern declarations**. Each extern binds a Dwarf function name to a package in the host language's ecosystem, so the transpiler emits real imports and calls — no manual glue code.

### Syntax

```
extern "<source>:<package>" fn <name>(<params>) -> <return_type>
```

### Supported Sources

| Prefix | Target Language | Example |
|---|---|---|
| `npm:` | TypeScript | `extern "npm:express" fn express() -> ()` |
| `py:` | Python | `extern "py:json" fn dumps(obj: Any) -> String` |
| `java:` | Java | `extern "java:java.util" fn ArrayList() -> List<any>` |

### Codegen Output

Each backend emits native import statements for its own externs and ignores externs for other targets:

- **TypeScript** → `import { express } from 'express'`
- **Python** → `import json`
- **Java** → `import java.util.ArrayList;` (specific class import)

### The `Any` Type

`Any` is compatible with all types. Use it in extern signatures when the parameter or return type is dynamic or unknown at compile time. It maps to `any` in TypeScript, `Any` in Python, and `Object` in Java.

### Security Note

Extern source strings and function names are injected directly into generated code without sanitization (e.g., `import { name } from 'module'`). This is safe because `.kzd` source files are trusted input — the programmer authored them. As with any compiler, only compile source files from trusted authors.
