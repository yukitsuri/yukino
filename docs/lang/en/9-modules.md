# Modules, Imports, and the Visibility System

Yukino organizes code into a hierarchical module system, where filesystem paths directly determine the addressing of modules and namespaces.

---

## Contents

1. [File structure and module addressing](#1-file-structure-and-module-addressing)  
   1.1 [Path prefixes: root, relative dots, and packages](#11-path-prefixes-root-relative-dots-and-packages)  
   1.2 [Resolution of files and directories (mod.ykn and lib.ykn)](#12-resolution-of-files-and-directories-modykn-and-libykn)  
   1.3 [Forced directory imports via the ! prefix](#13-forced-directory-imports-via-the--prefix)
2. [The load declaration and import forms](#2-the-load-declaration-and-import-forms)  
   2.1 [Targeted symbol imports and aliases (as)](#21-targeted-symbol-imports-and-aliases-as)  
   2.2 [Importing an entire package and prefix or suffix imports](#22-importing-an-entire-package-and-prefix-or-suffix-imports)  
   2.3 [Importing all modules of a directory](#23-importing-all-modules-of-a-directory)  
   2.4 [Namespace imports](#24-namespace-imports)
3. [The alias declaration](#3-the-alias-declaration)
4. [Visibility modifiers and re-export](#4-visibility-modifiers-and-re-export)  
   4.1 [Private by default and the public modifier](#41-private-by-default-and-the-public-modifier)  
   4.2 [Re-exporting symbols: public load](#42-re-exporting-symbols-public-load)
5. [Conditional compilation and module attributes](#5-conditional-compilation-and-module-attributes)

---

## 1. File structure and module addressing

Every `.ykn` file and every directory with an entry point file form a module.

### 1.1 Path prefixes: root, relative dots, and packages

A path in the `load` instruction begins with a prefix of one of three types:

| Prefix | Syntax | Description |
| :--- | :--- | :--- |
| `root` | `load root.submod: ...` | Absolute path from the project source root (`src/`) |
| `.` (1 dot) | `load .fiction: ...` | Relative path from the file's current directory |
| `..` (2 dots) | `load ..parent_mod: ...` | Relative path from the parent directory |
| `...` (3 dots) | `load ...grandparent: ...` | Relative path two levels up |
| Package name | `load std.io: print` | An external library from `yuki.toml` (`[libs.runtime]`) or the standard library |

Example project structure:

```text
my_project/
├── yuki.toml
└── src/
    ├── main.ykn
    ├── helper.ykn
    └── network/
        ├── client.ykn
        └── protocol.ykn
```

From the file `src/network/client.ykn`:
- `load root.helper: calc` — accessing `src/helper.ykn` through the `root` root.
- `load .protocol: Packet` — a sibling module in the same `network/` directory.
- `load ..helper: calc` — accessing a module one level up.
- `load std.io: print` — accessing the registered package `std`.

### 1.2 Resolution of files, packages, and directories (mod.ykn and lib.ykn)

In plain terms, here is how the compiler finds the right file when importing:

1. **Importing an entire library without a submodule (`load mylib` or `load std`):**  
   If a library/package is imported without specifying a particular submodule, the compiler automatically looks for the main library entry point — a module of the form `{libname}.lib` (that is, the `lib.ykn` file at the package root). For example:
   - `load std` is equivalent to `load std.lib` (and equivalent to `load std: *`). In this case, `std/lib.ykn` is loaded automatically, re-exporting the key modules of the standard library.
   - `load mylib` is equivalent to `load mylib.lib` (and `load mylib: *`).
2. **Importing a directory (`load .folder` or `load root.components`):**  
   If a path segment points to a directory (folder), the compiler automatically looks for the directory's entry point file inside it — `mod.ykn`. The `mod.ykn` file gathers and, if necessary, re-exports the contents of its package (similar to `__init__.py` in Python or `mod.rs` in Rust). For example:
   - `load std.platformio` is equivalent to `load std.platformio.mod` (and equivalent to `load std.platformio: *`).
   - `load .subpkg` is equivalent to `load .subpkg.mod` (if `subpkg` is a directory with `mod.ykn`).
   - `load root.components` is equivalent to `load root.components.mod`.
3. **Importing a single file (`load .helper`):**  
   If the directory contains a file with the `.ykn` extension (for example, `helper.ykn`), the compiler loads it directly (`load .helper` is equivalent to `load .helper: *`).

The resolution order for any name `foo`:
1. Single file: `foo.ykn`.
2. Directory module: `foo/mod.ykn`.
3. Directory library: `foo/lib.ykn`.

If both a file `foo.ykn` and a subdirectory `foo/mod.ykn` are present in the same directory, **the file `foo.ykn` always takes priority**.

### 1.3 Forced directory imports via the ! suffix

If you need resolution to go to the directory `foo/mod.ykn` (or `foo/lib.ykn`) specifically, ignoring an existing same-named file `foo.ykn`, an exclamation mark `!` is placed after the import name:

```yukino
// Forces a lookup for subpkg/mod.ykn or subpkg/lib.ykn, even if subpkg.ykn exists
load .subpkg!: helper_fn
load std.platformio!
load fiction.my.slayer.araragi!
load root.mushoku.tensei!: peak
```

---

## 2. The load declaration and import forms

The `load` instruction supports targeted symbol imports with renaming, whole-package imports, as well as imports by prefix or suffix.

### 2.1 Targeted symbol imports and aliases (as)

The identifiers to import are listed after the colon. The `as` keyword assigns a local alias:

```yukino
// Importing a single function
load root.math: calculate

// Importing with local renaming
load root.math: calculate as calc

// Importing multiple symbols with corresponding aliases:
// first the full comma-separated list of symbols, then a single
// `as` keyword followed by the same number of aliases
load root.math: add, multiply as my_add, my_mul
```

> [!WARNING]
> The symbol list and the alias list are separated by a single `as` keyword: the form `load .m: add as a, mult as b` (renaming after each symbol) is not supported. The number of aliases after `as` must match the number of imported symbols, otherwise the compiler emits error E0304.

### 2.2 Importing an entire package and prefix or suffix imports

The compiler recognizes the absence of a colon with a symbol list in a `load` declaration as importing all public entities of the module (equivalent to `: *`):

- `load lib.anymod` is fully equivalent to `load lib.anymod: *`
- `load std.io` is fully equivalent to `load std.io: *`
- `load std` is fully equivalent to `load std: *` (and `load std.lib`)

Alternatively, in the symbol section after the colon, you can explicitly specify `*` or select imported symbols by prefix or suffix:

```yukino
// 1. Whole-package import: omitting the colon is equivalent to explicit ': *'
load .worker           // equivalent to load .worker: *
load .worker: *        // imports all public symbols of the module

// 2. Import by prefix: all symbols starting with 'work' (work_a, work_b, worker_init)
load .worker: work*

// 3. Import by suffix: all symbols ending with 'Handler'
load .handlers: *Handler
```

> [!WARNING]
> The compiler forbids combining whole-package imports, as well as prefix or suffix imports (`*`, `prefix*`, `*suffix`), with `as` aliases. An attempt to write `load .worker: work* as my_work` results in compilation error E0304.

### 2.3 Importing all modules of a directory

Importing everything and selecting by prefix is allowed not only in the symbol list, but also in the module path itself:

```yukino
// Imports all modules of the models directory (user.ykn, order.ykn, item.ykn)
load .models.*

// Imports modules of the suites directory starting with 'test_'
load .suites.test_*
```

The compiler automatically traverses the directory, filters out the service files `mod.ykn` and `lib.ykn`, and connects every matching module.

### 2.4 Whole-package import without colon

If no colon with symbols follows the path, the module is loaded entirely, and all its public entities immediately become available in the current scope:

```yukino
load std.io

fun main() {
    print("Hello, Yukino!\n")
}
```

> [!NOTE]
> If you need to access symbols through a namespace prefix or assign a short name to a module/symbol to avoid naming collisions, use the `alias` declaration (see Section 3).

---

## 3. The alias declaration and :: namespaces

### 3.1 Double colon :: and built-in prefixes

The `::` operator allows referencing symbols via qualified namespace paths, eliminating ambiguity and naming collisions when identifiers coincide:

1. **Module namespaces (`std::io::`):**  
   After loading a module (`load std.io`), its public symbols can be addressed directly via their full path:
   ```yukino
   load std.io

   fun main() {
       std::io::print("Hello via qualified path!\n")
   }
   ```

2. **Compiler intrinsics (`rt::`):**  
   All compiler intrinsics (`cast`, `exit`, `args`, `raw_args`, `panic`, `assert`, `ptr_of`, `typeof!`) can optionally be invoked using the `rt::` prefix:
   ```yukino
   fun main() {
       set text = rt::cast(100, "string")
       set t = rt::typeof!(text)
       rt::exit(0)
   }
   ```

3. **C ABI functions (`extern::`):**  
   Functions declared via `cfun` or `externc` blocks can optionally be called using the `extern::` prefix. This explicitly directs the call to the C ABI function, even if a Yukino function with the same name exists:
   ```yukino
   externc {
       cfun puts(s: ptr<c_char>) -> c_int
   }

   // Yukino function with the same name
   fun puts(msg: string) {
       // Yukino logic
   }

   fun main() {
       // Explicit invocation of external C function:
       extern::puts(null)
       // Invocation of Yukino function:
       puts("Hello")
   }
   ```

### 3.2 The alias declaration

The `alias` declaration allows creating short top-level aliases within a module for namespaces or specific symbols:

```yukino
// 1. An alias for a namespace:
alias std::io as io

// 2. An alias for a symbol from a namespace (via the '#' symbol):
alias std::io#print as println

// 3. An alias for a local identifier:
alias heavy_internal_name as fast_name

fun main() {
    println("Using aliased print\n")
}
```

---

## 4. Visibility modifiers and re-export

### 4.1 Private by default and the public modifier

Yukino enforces a strict rule: **all entities of a module are private by default**.

For a function, structure, class, or constant to become available to other modules via `load`, it must have the `public` modifier:

```yukino
// File src/calc.ykn

// Private function: not visible outside calc.ykn
fun helper() -> int32 {
    return 1
}

// Public function: available to other files
public fun sum(a: int32, b: int32) -> int32 {
    return a + b + helper()
}

// Public structure with public fields
public struct Point {
    public x: float64
    public y: float64
}
```

An attempt to import a private identifier will be rejected by the resolver with the error `E0303: symbol is not public`.

### 4.2 Re-exporting symbols: public load

A package can re-export symbols from internal modules outward using the `public load` combination. This is the key mechanism for constructing facade interfaces of libraries (`lib.ykn`) and subpackages (`mod.ykn`):

```yukino
// File src/network/lib.ykn (the package facade)

// 1. Targeted re-export of specific types:
// Package consumers can write: load network: Client, Packet
public load .client: Client
public load .protocol: Packet

// 2. Full re-export of all public symbols of a child module:
// Both forms are equivalent:
public load .transport      // equivalent to public load .transport: *
public load .transport: *
```

#### How `public load` works

1. **Publicity in the current module:** Symbols imported via `public load` are registered in the current module's symbol table as re-exported (`is_reexported = true`). For any external file importing this module, they appear as if declared directly within it with the `public` modifier.
2. **Difference between `load` and `public load`:**  
   - Regular `load` (private): symbols are accessible only within the current file. If module `B` does `load A`, and module `C` does `load B`, module `C` **has no access** to the symbols of module `A`.
   - `public load`: module `B` does `public load A`, after which module `C` doing `load B` gets direct access to all re-exported symbols of `A`.
3. **Equivalence without colon:**  
   The construct `public load .submod` is fully equivalent to `public load .submod: *` — it re-exports all public symbols of the child submodule.

> [!TIP]
> In every subpackage (directory), create a `mod.ykn` file with `public load .submod` directives re-exporting all public symbols of child modules. This provides a uniform module interface for package consumers.

---

## 5. Conditional compilation and module attributes

Module items can be annotated with conditional compilation attributes that depend on the target architecture, operating system, or `yuki.toml` features. A complete reference of all compiler attributes is provided in the document [13. Compiler Attributes](13-attributes.md).

Conditional attributes accept a string or an array of strings. Note that `#[arch]` allows exactly one value from the list `x86_64`, `arm64`, `arm`, while `#[feature]` with several values requires all of the listed features to be present simultaneously.

```yukino
// Compiled exclusively on the x86_64 architecture
#[arch("x86_64")]
public fun fast_simd() -> int32 {
    return 1
}

// Negation: compiled on all architectures except arm
#[!arch("arm")]
public fun non_arm() {
    space
}

// Condition on the full target (triple): a string or an array of strings
#[target("x86_64-unknown-linux-gnu")]
public fun linux_only() {
    space
}

// Checks a feature from the yuki.toml manifest
#[feature("crypto")]
public fun decrypt_buffer() {
    space
}

// LLVM code generator attributes
#[inline]
public fun inlined_op(x: int32) -> int32 {
    return x * 2
}

struct AbortError {
    message: string
    code: int32
}

#[cold]
#[noreturn]
public fun abort_system() {
    panic(AbortError {
        message: "critical error",
        code: 999,
    })
}
```

---

[← Back: Strings and Bytes](8-strings.md) | [Next: The define Macro System →](10-macros.md)