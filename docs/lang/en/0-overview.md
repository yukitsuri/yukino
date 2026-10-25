# Yukino Documentation

This documentation is intended for developers who are already familiar with functional programming, object-oriented programming, typed languages, and common development tools. It is not aimed at absolute beginners: the primary focus is on Yukino semantics, memory safety model, and practical code examples.

For learning from the beginning, read the documents in numeric order: start with this overview, then `1-setup.md`, followed by later sections in ascending order.

> [!WARNING]
> **Documentation and compiler are under active development:** specifications, language semantics, and tooling may evolve. If you encounter any documentation discrepancies, unexpected behaviors, or compiler bugs, please do not hesitate to open an [Issue](https://github.com/yukitsuri/yukino/issues) or submit a [Pull Request](https://github.com/yukitsuri/yukino/pulls).

---

## About Yukino

**Yukino** is a statically typed, general-purpose application programming language that pairs maximum native execution performance with strict memory discipline and an elegant, concise syntax.

### Scope: Not for Systems Programming or Bare-Metal

It is important to understand the intended scope of the language:
- **Not aimed at low-level systems programming**: Yukino is not designed for kernel development, hardware device drivers, bootloaders, or bare-metal embedded targets running directly on raw silicon. The language strictly requires a working operating system environment (system calls, platform subsystem APIs, threading, I/O facilities).
- **No manual memory management**: The language deliberately does not burden the developer with manual memory management rituals such as `malloc`/`free` or pointer manipulation in everyday application code. Resource deallocation and memory safety are completely automated through the regional memory model and affine ownership semantics.

### Technology Stack and Architecture

- **LLVM 22.1 Backend**: Native machine code generation with modern optimization pipelines (including parallel codegen units CGU, Link-Time Optimization LTO, and Dead Code Elimination DCE).
- **Regional Memory Management**: Hierarchical resource allocation across stack frames, local lexical scopes, and explicit `region` blocks, eliminating tracing garbage collection (GC) overhead and manual deallocation pitfalls.
- **Copy / Affine Semantics**: Value types are trivially copied (`Copy`), while non-trivial managed resources adhere to strict affine ownership semantics (`Affine`) with explicit move mechanics and disciplined borrowing (`ref` and mutable `ref ~`).

### Goals and Philosophy

1. **Maximum Simplicity and Peak Performance**: Zero heavy runtime baggage, no garbage collection pauses, and execution efficiency matching C and C++.
2. **Conciseness and Elegant Syntax**: Minimal visual noise, intuitive and predictable constructs (`set`, `rewrite`, `switch`, `sus/catch/finally`), freeing code from boilerplate ceremonial clutter.
3. **Readability, Transparency, and Explicitness over Magic**: No hidden control flow, unexpected allocations, or invisible mutations. The code does exactly what is written. Yukino firmly prioritizes explicitness over implicit magic; consequently, certain language decisions (such as requiring the `fun` keyword for lambdas or explicit `rewrite` for mutations) may intentionally seem slightly more verbose or boilerplate-like at first glance, but this is a deliberate design choice in favor of zero ambiguity, crystal-clear readability, and easy long-term maintenance.
4. **Minimal Compiler Footprint**: The `yukic` compiler and `yukino` toolchain are built to be lightweight, self-contained, and blazingly fast, without bloated dependency graphs.
5. **Beginner-Friendly Experience and Quality Documentation**: Approachable type system, descriptive compiler diagnostics, and comprehensive documentation for every language feature.

> [!TIP]
> **A plea to library authors:**
> We have poured tremendous effort into making Yukino's documentation crystal-clear and thorough. When you build libraries for the Yukino ecosystem, please do not turn your documentation into cryptic scrolls of ancestors, or into runes from the Six-Faced World! Write clear examples, explain function behavior in the language of the Human World, and do not make fellow developers divine types from the stars. Quality documentation saves thousands of sanity points!

---

## Important Note: Environment Requirements

Requirements for external compilers depend on the installation method and target platform:

- **Installing a prebuilt binary (compiling for host):** if you install a prebuilt compiler binary and compile projects for your current host system, **neither C nor Rust are needed**. The runtime archive (`libyukino_rt.a`) and standard library core (`libyukistd_core.a`) are already precompiled for the host target and embedded directly into the compiler binary.
- **Building the compiler from source:** **Rust** (Cargo / `rustc`) and **C** (Clang / GCC / MSVC) compilers are required **only when building the compiler binary itself**. After that, compiling projects for the host target does not require C or Rust.
- **Compiling for other targets (cross-compilation):** if you compile a project for a target other than the current host, both **C** and **Rust** compilers **are still required** — they are used by the compiler to automatically build the runtime and standard library core from source for the selected target platform.

Please keep these requirements in mind when developing locally, testing, or contributing to the project.

---

## Contents (please read till the end)

0. [Documentation Overview](0-overview.md) — navigation, reading order, and stability notice.
1. [Installing the Compiler](1-setup.md) — installing Yukino on Linux and Windows with Nix or Cargo, plus cross-compiling the compiler and runtime.
2. [Toolchain and Project Structure](2-cli-project.md) — `yukino` CLI commands, `yuki.toml` manifest, dependencies (path, git, tag, branch, commit, link), build profiles, `yuki-work/` workspace, `yuki.lock` lockfile, and `build.ykn` build scripts.
3. [Basic Syntax, Operators, and Control Flow](3-basics.md) — `set` variables, strict `rewrite` mutation rule, `static` constants, arithmetic and bitwise operators, Pratt precedence table, branching, loops, and `space` stub.
4. [Type System and Pattern Matching](4-types.md) — primitive and compound types, nullable `T?`, null safety (`??`, `?.`), tagged unions `T1 | T2`, `type` aliases, and `switch` matching.
5. [Functions and Closures](5-functions-closures.md) — `fun` declarations, default parameters, reference passing, `cfun` function pointers, and closures.
6. [Memory Model, Ownership, and Regions](6-memory.md) — affine ownership, move semantics, borrowing (`ref`, `ref ~`), regional memory hierarchy (stack frames, loop scopes, explicit `region {}`), `defer`, and `sus/catch/finally`.
7. [Structures, Classes, and Interfaces](7-oop.md) — value structs, reference classes, inheritance prevention (`locked class`), `super`, property accessors, operator overloading, `interface`, and `enum`.
8. [Strings and Bytes](8-strings.md) — memory layout, interpolation `${...}`, byte indexing, `bytes` literals, built-in methods, and C-string conversions.
9. [Modules, Imports, and Visibility](9-modules.md) — module hierarchy, path prefixes (`root`, `.`, `..`, `...`, libraries), forced folder resolution (`!folder`), pattern matching (`*`, `prefix*`, `*suffix`), aliases (`as`, `alias`), `public` visibility, and conditional compilation.
10. [Macro System, Procedural Macros, and Built-ins](10-macros.md) — syntactic macros (`define`), procedural macros (runners, attributes `#[macro::...]`, invocations `macro!(...)`, `MacroContext`, `track_file`, CGU caching), built-in macros (`file!`, `line!`, `col!`, `hostenv!`, `get_target!`, `include_str!`, etc.), and standard `macrosdk`.
11. [System C ABI and C FFI](11-cabi.md) — `externc` blocks, `cfun` declarations, calling conventions and attributes (`|cdecl|`, `|packed, align="N"|`, `|cold, noreturn|`), `cstruct`, raw pointers `ptr<T>`, `ptr_of`, and C object file integration.
12. [Compiler Intrinsics](12-intrinsics.md) — complete reference for built-in compiler functions (`typeof`, `cast`, `args`, `raw_args`, `exit`, `panic`, `assert`, slice methods `.len()`, `.cap()`, `.as_ptr()`, `.cp()`, `from_raw_parts`).
13. [Compiler Attributes](13-attributes.md) — complete reference of language attributes (`#[...]`, `!#[...]`), conditional compilation, function behavior, testing, and lints.
14. [Error Handling and Exceptions](14-errors.md) — `sus/catch/finally` construction, `catch` block variable (`errkind`), catch clause forms `catch TempName: Struct`, `catch Struct`, and `catch`, error struct payloads, `panic` exception generation with struct enforcement, and error struct design standards (best practices and anti-patterns).
15. [Workspaces and Multi-Package Projects](15-workspaces.md) — monorepo organization, root manifest `yukispace.toml`, native C build inheritance, inline virtual packages, and the `package` dependency selector.
16. [Testing and Test Runner](16-tests.md) — built-in testing system, `yukino test` command, `#[test]`, `#[should_panic]`, `#[ignore]`, `#[timeout_ms]` attributes, worker isolation, watchdog timeouts, and summary reports.
17. [Standard Library: Overview](stdlib/0-overview.md) — architecture of the embedded standard library (Rust core + Yukino std), documentation.
18. [Third-Party Libraries and Ecosystem](../libs-en.md) — official packages, community libraries, and Pull Request submission guidelines.

Developer documentation regarding compiler internals is located in a separate section: [Developer Overview](../../dev/0-overview.md).

---

[Next: Installing the Compiler →](1-setup.md)
