# vite-plugin-dwarf

Vite plugin for the Dwarf compiler. Compile `.kzd` files in your Vite project with HMR, source maps, and React Fast Refresh support.

## Installation

```bash
npm install --save-dev vite-plugin-dwarf @dwarf/compiler-wasm
```

## Usage

Add the plugin to your `vite.config.ts`:

```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import dwarf from 'vite-plugin-dwarf';

export default defineConfig({
  plugins: [
    react(),
    dwarf(),
  ],
});
```

Now you can import `.kzd` files directly in your React components:

```tsx
import Greeting from './Greeting.kzd';

function App() {
  return <Greeting name="World" />;
}
```

## How it works

1. **`.kzd` files** are Dwarf source components that compile to React components
2. **Vite's transform pipeline** intercepts `.kzd` imports and compiles them using the Dwarf WASM compiler
3. **Source maps** map compiled output back to `.kzd` source for debugging
4. **HMR** updates changed components without full page reloads
5. **React Fast Refresh** preserves component state across edits

## Options

### `pretty`
Enable pretty-printed output (default: `false`).

### `compilerOptions`
Additional options passed to the Dwarf compiler.

## Requirements

- Vite 5+ or 6+
- Node.js 18+
- `@dwarf/compiler-wasm` (or `@dwarf/compiler` for native performance)

## Example

Create `Greeting.kzd`:

```dwarf
@react_component
pub fn Greeting(props: { name: Str }) =
  <div>
    <h1>Hello, {props.name}!</h1>
  </div>
```

Import it in your app:

```tsx
import Greeting from './Greeting.kzd';
```

## License

MIT
