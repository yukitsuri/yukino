# Compiler Attributes

Compiler attributes in Yukino are declarative metadata attached to program elements or an entire module. They control conditional compilation, LLVM optimizer behavior, the testing framework, and static warning (lint) filtering.

> [!NOTE]
> This document covers exclusively the internal Yukino language attributes (`#[...]` and `!#[...]`). The low-level C attributes of the system FFI (`|cdecl, packed, align=N|`) are described in the chapter [System C ABI and C FFI](11-cabi.md).

---

## Contents

1. [Attribute syntax](#1-attribute-syntax)  
   1.1 [Per-item attributes: #[attr]](#11-per-item-attributes-attr)  
   1.2 [Namespaces and composite paths: #[namespace::attr]](#12-namespaces-and-composite-paths-namespaceattr)  
   1.3 [Module inner attributes: #![attr] and !#[attr]](#13-module-inner-attributes-attr-and-attr)  
   1.4 [Grouping attributes](#14-grouping-attributes)  
   1.5 [Language attribute classification](#15-language-attribute-classification)
2. [Conditional compilation](#2-conditional-compilation)  
   2.1 [Unified configuration attribute: #[cfg(...)]](#21-unified-configuration-attribute-cfg)  
   2.2 [Conditional attribute application: #[cfg_attr(...)]](#22-conditional-attribute-application-cfg_attr)  
   2.3 [Historical attributes target, arch, feature, and #[!attr]](#23-historical-attributes-target-arch-feature-and-attr)
3. [Optimization and function code generation](#3-optimization-and-function-code-generation)  
   3.1 [Pure functions: #[stable]](#31-pure-functions-stable)  
   3.2 [Inlining control: #[inline], #[inline_always], #[no_inline]](#32-inlining-control-inline-inline_always-no_inline)  
   3.3 [Profiling and flow hints: #[cold], #[noreturn]](#33-profiling-and-flow-hints-cold-noreturn)  
   3.4 [Name mangling control: #[mangle], #[!mangle]](#34-name-mangling-control-mangle-mangle)  
   3.5 [C ABI calling convention (thunk functions): #[thunk]](#35-c-abi-calling-convention-thunk-functions-thunk)
4. [Built-in testing attributes](#4-built-in-testing-attributes)  
   4.1 [Marking tests: #[test]](#41-marking-tests-test)  
   4.2 [Expecting a panic: #[should_panic]](#42-expecting-a-panic-should_panic)  
   4.3 [Skipping tests: #[ignore]](#43-skipping-tests-ignore)  
   4.4 [Individual timeout: #[timeout_ms]](#44-individual-timeout-timeout_ms)
5. [Diagnostics and linter](#5-diagnostics-and-linter)  
   5.1 [Suppressing warnings: #[ignore_warn], #[warns_ignore]](#51-suppressing-warnings-ignore_warn-warns_ignore)  
   5.2 [Deprecated code: #[deprecated]](#52-deprecated-code-deprecated)

---

## 1. Attribute syntax

### 1.1 Per-item attributes: #[attr]

Per-item attributes are placed directly before a function, method, struct, class, enum, field, or constant declaration:

```yukino
#[inline]
fun fast_add(a: int32, b: int32) -> int32 {
    return a + b
}
```

Attributes can take arguments via parentheses or the equals sign:
- `#[target = "x86_64-unknown-linux-gnu"]`
- `#[target("x86_64-unknown-linux-gnu")]`
- `#[cfg(all(target_os = "linux", feature = "ssl"))]`

### 1.2 Namespaces and composite paths: #[namespace::attr]

Attributes support namespaced identifier paths using the `::` separator. This prevents naming collisions between compiler built-ins, metaprogramming libraries, and external tools:

```yukino
struct UserAccount {
    #[json::rename = "user_name"]
    name: string

    #[db::primary_key]
    #[db::column = "account_id"]
    id: int64
}
```

### 1.3 Module inner attributes: #![attr] and !#[attr]

Module inner attributes apply to the entire file or module and are declared at the very top of the file. The compiler supports canonical Rust-like `#![...]` syntax alongside Yukino's historical `!#[...]` prefix:

```yukino
#![cfg(target_os = "linux")]
#![warns_ignore("unused_variable")]

public fun linux_helper() {
    space
}
```

If a module attribute's condition is not met, the compiler strips the entire module during the AST filtering stage before name resolution.

Besides conditional compilation (`target`, `arch`, `feature`) and warning filtering (`ignore_warn`, `warns_ignore`), module attributes can define properties for every function of the file. The supported attributes are `deprecated`, `stable`, `inline`, `no_inline`, `inline_never`, `inline_always`, `cold`, and `noreturn`:

```yukino
!#[cold]

fun helper_a() { space }
fun helper_b() { space }
```

Such an attribute is propagated onto every function, method, static method, and accessor of the file as if it were declared on that declaration itself. An attribute placed on the declaration itself takes priority over the file-level one; in particular, any inlining hint on a declaration (`inline`, `no_inline`, `inline_never`, `inline_always`) fully overrides the file-level inlining hint. The `mangle` attribute and the testing attributes are not allowed at file level.

### 1.4 Grouping attributes

Inside a single `#[...]` bracket pair, multiple attributes can be listed separated by commas:

```yukino
struct CriticalError {
    message: string
    code: int32
}

#[cold, noreturn]
fun abort_critical() {
    panic(CriticalError {
        message: "critical condition violated",
        code: 501,
    })
}
```

### 1.5 Language attribute classification

In Yukino, attributes are categorized into distinct semantic tiers:
1. **Built-in Codegen Attributes (`Built-in`):** `#[inline]`, `#[cold]`, `#[noreturn]`, `#[stable]`, `#[mangle]`. Control LLVM IR generation and C ABI conventions.
2. **Configuration Attributes (`Config`):** `#[cfg(...)]`, `#![cfg(...)]`, `#[cfg_attr(...)]`. Prune inactive AST subtrees prior to symbol resolution.
3. **Type Mutations (`Mutation`):** `#[mutation(...)]`. Generate companion code for types (serialization, constructors).
4. **Tool and Lint Attributes (`Tool / Lint`):** `#[lint::ignore(...)]`, `#[lint::deny(...)]`, `#[lint::allow(...)]`, `#[warns_ignore(...)]`, `#[deprecated]`.
5. **Inert Helper Attributes (`Inert Helper`):** Namespaced path attributes (such as `#[json::rename]`) serving as passive metadata for code generators and ignored by the compiler backend.

---

## 2. Conditional compilation

Conditional compilation attributes are checked during the AST filtering stage (`yukic-resolve::conditional`). Elements whose conditions do not match the current target platform or active features are removed entirely from the program tree.

### 2.1 Unified configuration attribute: #[cfg(...)]

The primary and most expressive mechanism for conditional compilation is the `#[cfg(...)]` attribute (and `#![cfg(...)]` at module scope).

It supports arbitrary nesting of boolean logic using:
- `all(cond1, cond2, ...)` — logical AND (all conditions must hold true).
- `any(cond1, cond2, ...)` — logical OR (at least one condition must hold true).
- `not(cond)` — logical NOT (inversion of a condition).

Available predicate parameters:
- `feature = "name"` — checks if the given project feature is active.
- `target_os = "linux"` — checks the target operating system (`"linux"`, `"windows"`, `"macos"`, `"android"`).
- `target_arch = "x86_64"` — checks the target CPU architecture (`"x86_64"`, `"aarch64"`, `"arm"`).
- `target_env = "gnu"` — checks the environment and libc (`"gnu"`, `"musl"`, `"msvc"`).

Usage examples:

```yukino
// Active only on Linux x86_64 with ssl enabled
#[cfg(all(target_os = "linux", target_arch = "x86_64", feature = "ssl"))]
fun init_fast_ssl() {
    space
}

// Active if either pico_tls is enabled, or openssl on POSIX
#[cfg(any(feature = "pico_tls", all(target_os = "linux", feature = "openssl")))]
fun init_tls() {
    space
}

// Active everywhere except Windows
#[cfg(not(target_os = "windows"))]
fun posix_socket() {
    space
}
```

### 2.2 Conditional attribute application: #[cfg_attr(...)]

The `#[cfg_attr(condition, attribute)]` attribute attaches another attribute (such as inlining, mutations, or optimizations) only when the condition is met:

```yukino
// Force inlining only when compiling for Linux
#[cfg_attr(target_os = "linux", inline_always)]
fun low_level_call() {
    space
}

// Attach mutation only if the "json" feature is active
#[cfg_attr(feature = "json", mutation(JsonSerialize))]
struct ConfigPayload {
    host: string
    port: int32
}
```

If the predicate evaluates to true, the compiler dynamically decorates the item with the target attribute prior to subsequent compiler stages.

### 2.3 Historical attributes target, arch, feature, and #[!attr]

For backward compatibility, single-condition attributes and prefix negation remain supported:

- `#[target = "x86_64-unknown-linux-gnu"]` or target list `#[target = ["...", "..."]]`
- `#[arch = "x86_64"]` (accepts `"x86_64"`, `"arm64"`, `"arm"`)
- `#[feature = "ssl"]`
- Prefix negation `#[!target = "..."]`, `#[!arch = "..."]`, `#[!feature = "..."]`

```yukino
#[target = "x86_64-pc-windows-gnu"]
fun windows_only() {
    space
}

#[!target = "x86_64-pc-windows-gnu"]
fun non_windows_only() {
    space
}
```

### 2.2 CPU architecture: arch

Filters code by processor architecture family. The supported values are `"x86_64"` and `"arm64"` (AArch64); any other value is a compilation error. The attribute accepts exactly one value (a list is not allowed):

```yukino
#[arch = "x86_64"]
fun process_simd() {
    // SSE/AVX instructions
}

#[arch = "arm64"]
fun process_simd() {
    // ARM NEON instructions
}

#[!arch = "arm64"]
fun non_arm_routine() {
    space
}
```

### 2.3 Build flags and features: feature

Matches the name of a feature enabled in the `[features]` section of the `yuki.toml` manifest (or passed via `--features`). If a list of strings is passed, the element is considered active only when all of the listed features are enabled simultaneously (unlike `target`, where a single match is enough):

```yukino
struct FeatureError {
    message: string
    code: int32
}

#[feature = "ssl"]
fun connect_secure() {
    space
}

#[!feature = "ssl"]
fun connect_secure() {
    panic(FeatureError {
        message: "SSL feature is not enabled in this build",
        code: 502,
    })
}
```
---

## 3. Optimization and function code generation

### 3.1 Pure functions: #[stable]

Tells the LLVM optimizer that the function is deterministic, terminating, and free of side effects.

The compiler automatically attaches the following LLVM attributes to the function:
- `nounwind`: the function is guaranteed not to throw exceptions or trigger a panic.
- `willreturn`: the function is guaranteed to finish executing (contains no infinite loops).
- `memory(none)` / `readnone`: the function neither reads nor modifies the program's global memory state; the result depends solely on the passed arguments.

```yukino
#[stable]
fun square(x: int64) -> int64 {
    return x * x
}
```

Thanks to `#[stable]`, LLVM can safely merge duplicate calls, perform aggressive Common Subexpression Elimination (CSE), and compute results at compile time.

### 3.2 Inlining control: #[inline], #[inline_always], #[no_inline]

Controls function inlining at call sites:

| Attribute | LLVM equivalent | Description |
| :--- | :--- | :--- |
| `#[inline]` | `inlinehint` | Gives the optimizer a strong recommendation to inline the function body |
| `#[inline("always")]` / `#[inline_always]` | `alwaysinline` | Forces the function body to be inlined at any optimization level |
| `#[inline("never")]` / `#[inline_never]` / `#[no_inline]` | `noinline` | Strictly forbids inlining the function |

Usage example:

```yukino
#[inline_always]
fun bit_get(mask: uint32, index: uint32) -> bool {
    return (mask & (1 << index)) != 0
}

#[no_inline]
fun heavy_diagnostic_report() {
    // Complex formatting code that should not bloat the instruction cache
}
```

### 3.3 Profiling and flow hints: #[cold], #[noreturn]

| Attribute | LLVM equivalent | Purpose |
| :--- | :--- | :--- |
| `#[cold]` | `cold` | Marks a branch/function as unlikely to execute. The optimizer moves such code out of the hot loop |
| `#[noreturn]` | `noreturn` | Tells that the function never returns control to the caller |

```yukino
#[cold, noreturn]
fun panic_handler(msg: string) {
    print("FATAL: ")
    print(msg)
    print("\n")
    exit(1)
}
```

### 3.4 Name mangling control: #[mangle], #[!mangle]

By default, all `fun` functions are automatically mangled by the compiler into safe symbols of the form `__yukifun_{fn_name}_{arg_types}__{ret_type}` (or `__yukifun_{fn_name}___{ret_type}` when there are no arguments), which prevents name collisions with the C library (libc), system APIs, and third-party libraries. The main entry point `fun main` is never mangled.

| Attribute | Applies to | Purpose |
| :--- | :--- | :--- |
| `#[!mangle]` | `fun` | Disables automatic mangling of the function, keeping the original (raw) symbol name in the binary artifact |
| `#[mangle]` | `cfun` | Forcibly enables mangling for a C ABI function |

Usage example:

```yukino
// By default the symbol is exported as __yukifun_add_int32_int32__int32
fun add(a: int32, b: int32) -> int32 {
    return a + b
}

// Exported with the exact raw name @custom_entry
#[!mangle]
public fun custom_entry() -> int32 {
    return 42
}

// By default a cfun is exported without mangling (@native_call)
cfun native_call() -> int32 {
    return 100
}

// Forced mangling for a cfun (__yukifun_mangled_c_int32__int32)
#[mangle]
cfun mangled_c(val: int32) -> int32 {
    return val * 2
}
```

### 3.5 C ABI calling convention (thunk functions): #[thunk]

The `#[thunk]` attribute instructs the compiler to generate a `fun` function with standard C ABI calling conventions (`LLVMCCallConv` / `cdecl`) and a flat signature without hidden return region destination parameters (`dest_region`).

This allows ordinary Yukino functions to be passed directly as C-compatible function pointers:
- in system threads via `std.threads: spawn(...)`;
- as C callbacks for external FFI libraries;
- when passing functions into dynamically loaded shared libraries via `std.sharedc`.

Unlike `cfun`, a function annotated with `#[thunk]`:
- is a first-class Yukino function with full access to the language syntax and capabilities;
- automatically initializes its own stack-local memory region (`_local_region`) when strings or objects are allocated inside the worker body, properly destroying it on thread termination;
- does not require declaring a dummy `arg: ptr<void>` parameter when no inputs are needed.

```yukino
load std.threads: spawn, join
load std.io: println

#[thunk]
fun worker_routine() {
    println("Background thread running without parameters!")
}

#[thunk]
fun worker_with_arg(arg: ptr<void>) {
    println("Background thread received raw pointer: ${arg}")
}

fun main() {
    // Launch worker without arguments
    set h1 = spawn(cast(worker_routine, "ptr<void>"))
    join(h1)

    // Launch worker with raw argument
    set h2 = spawn(cast(worker_with_arg, "ptr<void>"), null)
    join(h2)
}
```

---

## 4. Built-in testing attributes

> [!NOTE]
> For a comprehensive guide on test organization, test attributes, configuration, and the `yukino test` runner, see [16. Testing and Test Runner](16-tests.md).

### 4.1 Marking tests: #[test]

Marks a top-level function with no arguments, no return value, and no generic parameters as a unit test:

```yukino
#[test]
fun test_addition() {
    assert_eq(2 + 2, 4)
}
```

The `#[should_panic]`, `#[ignore]`, and `#[timeout_ms]` attributes are only allowed together with `#[test]` — applying them to a regular function is compilation error E0429.

### 4.2 Expecting a panic: #[should_panic]

The test is considered passed only if a `panic` was thrown during its execution. Substring matching against the panic message payload is also supported:

```yukino
// The test passes if the divisor == 0 division throws a panic
#[test, should_panic]
fun test_zero_division() {
    set zero = 0
    set res = 10 / zero
}

struct DescriptorError {
    message: string
    code: int32
}

// Expected panic message (matched by the runner)
#[test, should_panic = "invalid handle"]
fun test_bad_descriptor() {
    panic(DescriptorError {
        message: "invalid handle: -1",
        code: 503,
    })
}
```

### 4.3 Skipping tests: #[ignore]

Marks a test as temporarily disabled. The compiler emits a W0423 warning when ignored tests are present:

```yukino
#[test, ignore]
fun test_long_benchmarks() {
    // A long network or stress test
}

#[test, ignore = "Requires running PostgreSQL container"]
fun test_db_integration() {
    // A database integration test
}
```

To run ignored tests, the `yukino test` runner provides `--ignored` (run only ignored tests) and `--include-ignored` (run all tests).

### 4.4 Individual timeout: #[timeout_ms]

Sets the maximum execution time of an individual test in milliseconds. If execution exceeds this duration, the isolated worker process is terminated by the runtime watchdog:

```yukino
#[test, timeout_ms = 500]
fun test_fast_io() {
    // The test fails if execution takes more than 500 ms
}
```

---

## 5. Diagnostics and linter

### 5.1 Controlling the linter: #[lint::ignore], #[lint::deny], #[lint::allow]

The Yukino compiler provides a flexible static analysis and diagnostic control system (`yukic-driver::lint`) through the `lint::` attribute namespace:

- **`#[lint::ignore(...)]` / `#[lint::allow(...)]`** — suppresses the specified compiler warnings.
- **`#[lint::deny(...)]`** — escalates warnings to fatal compilation errors (`DiagnosticSeverity::Error`), immediately halting compilation with a non-zero exit code.
- Backward compatibility: legacy forms `#[ignore_warn(...)]`, `#[warns_ignore(...)]`, `#[deny_warn(...)]`, and `#[warns_deny(...)]` remain supported.

#### Scopes

1. **Module level (`#![...]`):** applies to the entire file and all functions, types, and imports declared inside it:
   ```yukino
   #![lint::deny("unused_variable")]
   #![lint::ignore("unused_import")]
   ```
2. **Item and function level (`#[...]`):** placed directly above a specific item (`fun`, `cfun`, `class`, `struct`, method):
   ```yukino
   #[lint::ignore("unused_variable")]
   fun debug_handler(unused_ctx: int32) -> int32 {
       return 0
   }

   #[lint::deny("unused_symbol")]
   fun critical_task() -> int32 {
       // Any unused variable or symbol inside this function will abort compilation
       return 42
   }
   ```

#### Argument format

Attributes accept rule names as strings or identifiers:
- Specific warning names: `"unused_variable"` (or `unused_variable`), `"unused_import"`, `"dead_function"`.
- Warning codes: `"W0001"`, `"W0002"`, `"W0003"`.
- Warning group names:
  - `"unused_symbol"` — encompasses `unused_variable`, `unused_import`, `dead_function`.
  - `"predictable_code"` — encompasses `predictable_condition`, `unreachable_code`.
  - `"performance_hints"` — encompasses `suggest_switch`, `suggest_stable`.
  - `"code_clarity"` — encompasses `pointless_mutability`, `redundant_parentheses`, `empty_block`, `test_ignored`.

```yukino
// Modern lint namespace
#![lint::deny(unused_symbol)]
#![lint::ignore("predictable_code", "performance_hints")]

// Legacy syntax for module and function level
!#[ignore_warn("unused_variable")]
!#[warns_ignore("empty_block")]
#[warns_ignore("dead_function")]
fun legacy_helper() {
    space
}
```

### 5.2 Deprecated code: #[deprecated]

Used to mark deprecated APIs before their final removal. When referencing or calling a marked entity, the compiler emits a static analysis warning **`W0418`** (`deprecated_usage`, alias `deprecated_item` or `deprecated`). The attribute accepts an optional explanatory string:

```yukino
#[deprecated = "Use String.trim() instead"]
fun strip_whitespace(s: string) -> string {
    return s.trim()
}

fun main() {
    // Triggers warning[W0418]: function 'strip_whitespace' is deprecated: Use String.trim() instead
    set res = strip_whitespace("  hello  ")
}
```

#### Syntax and Arguments

The attribute supports both a bare form and a form with an explanatory reason or migration hint:
- `#[deprecated]` — emits the base warning: `use of deprecated function 'name'`.
- `#[deprecated = "reason"]` or `#[deprecated("reason")]` — appends the message to the diagnostic: `use of deprecated function 'name': reason`.

#### Supported Target Entities

- **Functions (`fun`, `cfun`):** warnings are emitted at call sites.
- **Class and struct methods:** warnings are emitted at instance and static method calls.
- **Structures (`struct`) and classes (`class`):** warnings are emitted on instantiation (`StructInit`, `ClassInit`) and in type annotations.
- **Enums (`enum`) and type aliases (`type`):** warnings are emitted on type usage and pattern matching.

#### Deprecated Context Isolation

If a function or method is **itself** marked with `#[deprecated]`, calling other deprecated entities inside its body **does not trigger** warnings. This allows legacy transitional wrappers to be maintained without compiler noise.

#### Lint Control

Warning `W0420` belongs to the `code_clarity` group and fully supports granular control via `#[lint::ignore]` and `#[lint::deny]`:

```yukino
// Suppress deprecated warnings at module level
#![lint::ignore(deprecated_item)]

// Or escalate deprecated API usage to fatal compilation errors
#![lint::deny(deprecated)]
```

The compiler emits the `W0418` warning (name `deprecated_usage`) at every call site of a deprecated function or method. Functions are marked with the attribute directly, and class or struct methods with the attribute on the method declaration. The warning message includes the optional explanatory text: `function 'strip_whitespace' is deprecated: Use String.trim() instead`.

The warning can be suppressed globally with `ignore_warn`:

```yukino
!#[ignore_warn("deprecated_usage")]
```

A name shared by a deprecated and a regular function (or by methods of different types) is considered ambiguous and produces no warning. The file-level `!#[deprecated]` attribute marks every function and method of the module as deprecated.

---

[← Back: Compiler Intrinsics](12-intrinsics.md) | [Next: Error Handling and Exceptions →](14-errors.md)
