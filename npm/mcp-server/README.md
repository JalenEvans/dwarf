# @dwarf/mcp-server

Standalone [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server for the **Dwarf** programming language. Exposes the Dwarf compiler's capabilities as MCP tools, resources, and prompts — enabling LLM-powered IDEs and agents to compile, analyze, and transform Dwarf source code directly.

## Installation

### Prerequisites

The `dwarf-mcp` binary must be built from source (requires Rust):

```bash
# Build the MCP server binary
cargo build -p dwarf-mcp --release
```

### via npm

```bash
npm install @dwarf/mcp-server
```

### via npx (no install)

```bash
npx @dwarf/mcp-server --transport stdio
```

## Usage

### Start the MCP server

```bash
# stdio transport (default for AI-agent integration)
dwarf-mcp --transport stdio

# Show help
dwarf-mcp --help
```

### Claude Code configuration

Add to your `claude.json`:

```json
{
  "mcpServers": {
    "dwarf": {
      "command": "npx",
      "args": ["@dwarf/mcp-server"]
    }
  }
}
```

See `claude.json.example` for a standalone config reference.

## Tools

The server exposes these MCP tools:

| Tool                  | Description                                       |
|-----------------------|---------------------------------------------------|
| `dwarf_check`         | Validate Dwarf source and return diagnostics      |
| `dwarf_compile`       | Compile Dwarf source to a target language         |
| `dwarf_format`        | Format Dwarf source code                          |
| `dwarf_generate_tests`| Generate edge-case test values from type defs     |

### dwarf_check

Validates Dwarf source code and returns structured diagnostics (errors, warnings, etc).

**Arguments:**
- `source` (required) — Dwarf source code to check
- `filename` (optional) — Source filename (default: `input.kzd`)

### dwarf_compile

Compiles Dwarf source to a target language.

**Arguments:**
- `source` (required) — Dwarf source code
- `target` (required) — Target language: `ts`, `py`, `java`, or `debug`
- `filename` (optional) — Source filename

### dwarf_format

Formats Dwarf source code with basic whitespace normalization.

**Arguments:**
- `source` (required) — Dwarf source code to format

### dwarf_generate_tests

Parses type definitions from Dwarf source and generates edge-case test values for property-based testing.

**Arguments:**
- `source` (required) — Dwarf source with type definitions

## Resources

The server provides these reference resources (`dwarf://` URIs):

| URI                          | Description                                |
|------------------------------|--------------------------------------------|
| `dwarf://syntax/overview`    | Language philosophy and syntax fundamentals |
| `dwarf://syntax/functions`   | Function declarations and the effects system |
| `dwarf://syntax/types`       | Records, unions, generics, refinement types |
| `dwarf://syntax/modules`     | Module declarations and imports             |
| `dwarf://syntax/expressions` | If/match/block/loop, pipe operator          |
| `dwarf://syntax/testing`     | @test decorator, assertions, property-based |
| `dwarf://stdlib/reference`   | Built-in types and common functions         |
| `dwarf://examples/basic`     | Hello world, arithmetic, I/O examples       |
| `dwarf://examples/types`     | Record, union, and generic type patterns    |
| `dwarf://examples/testing`   | Unit tests, property-based tests            |

## Prompts

The server includes prompt templates for common Dwarf tasks:

| Prompt                 | Description                                  |
|------------------------|----------------------------------------------|
| `write-dwarf-function` | Template for writing idiomatic Dwarf functions |
| `define-dwarf-type`    | Template for defining records, unions, generics |
| `create-dwarf-test`    | Template for @test functions and forAll       |
| `port-to-dwarf`        | Guide for porting TypeScript/Python to Dwarf  |

## Development

```bash
# Build the binary
cargo build -p dwarf-mcp --release

# Test the npm module loads
node -e "const pkg = require('./npm/mcp-server'); console.log('Module loads OK')"

# Test the CLI entry point
node ./npm/mcp-server/bin/cli.js --help
```

## License

MIT
