# Modules and Imports in Dwarf

Dwarf's module system allows organizing code into reusable, namespaced units.

## Module Declaration

A file can declare its module at the top:

```
module myapp.utils
```

If no module declaration is present, the module name is inferred from the file path.

## Importing Modules

Dwarf uses a lightweight import syntax:

```
import "std/io"
import "myapp/utils"
import "myapp/types"
```

## Selective Imports

You can import specific names from a module:

```
import "std/io" { readFile, writeFile }
```

## Path Resolution

Import paths are resolved relative to the project root or standard library paths. The compiler searches configured paths in order.

## Visibility and Exports

By default, all top-level declarations in a module are public and available to importers. The compiler determines the public API surface automatically.
