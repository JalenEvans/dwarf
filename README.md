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
