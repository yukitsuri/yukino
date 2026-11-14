---
name: yukino-lang
description: >-
  Expert guide and workflow for writing, understanding, and debugging code in the Yukino
  programming language. Use when creating, refactoring, or analyzing Yukino (.ykn) code,
  working with memory regions, affine types, error handling (panic vs null), manifests,
  and the standard library.
---

# Yukino Language and Standard Library Skill

This guide provides an authoritative, parser-accurate reference for the Yukino programming language.
Yukino combines deterministic affine memory management with regional bump allocators, static borrow verification, strict typing, and zero runtime garbage collection.

---

## 1. Quick Syntax Reference (For Rust / Kotlin Engineers)

Yukino shares high-level ergonomics with Rust, Kotlin, and TypeScript, but contains distinct grammatical and semantic rules enforced strictly by the parser and type checker.

### Common Language Traps & Parser Rules

| Concept | Rust / Kotlin Trap | Correct Yukino Syntax | Parser & Semantic Rule |
| :--- | :--- | :--- | :--- |
| **Variable Binding** | `let x = 10` / `val x = 10` | `set x: int32 = 10` | Variables are declared with `set`. |
| **Variable Mutation** | `x = 20` or `x += 1` | `rewrite x = 20`<br/>`rewrite x += 1` | In-place mutation **requires** the `rewrite` keyword. Bare assignment `x = 20` is a syntax error. |
| **Function Declaration** | `fun foo(): int32` | `fun foo() -> int32` | Function return types are declared with `->`, not `:`. |
| **Empty Blocks / Stubs** | `{}` | `space` or `{ space }` | Empty function or block bodies trigger warnings. Use `space` (no-op keyword). |
| **Lambdas / Closures** | `\|x\| x + 1` or `{ x -> x + 1 }` | `fun(x: int32) => x + 1`<br/>`fun(x: int32) -> int32 { return x + 1 }` | Anonymous functions and closures **always** begin with `fun`. |
| **Pattern Matching** | `match x { 1 => ... }` | `switch x { 1 -> ... }` | `switch` uses `->`, not `=>`. Guards use `pat if cond -> ...`. Default arm is `else -> ...` (`_ ->` is rejected). |
| **Loops** | `for item in array` | `for i in 0..array.len()`<br/>`set item = array[i]` | `for` loops iterate exclusively over numeric ranges (`0..n` or `0..=n`). Slices are indexed manually. |
| **Integer Types** | Literals default to `i32` or `int` | `0` is `int64` | Numeric literals default to `int64`. Yukino forbids implicit widening; cast with `cast(val, "int32")`. |
| **Borrowing** | `&T` / `&mut T` | `ref T` / `ref ~T` | Immutable borrow is `ref T`. Exclusive mutable borrow is `ref ~T`. |
| **Error Handling** | `Result<T, E>` / `try/catch` | `sus { ... } catch e: Err { ... }` | Protected blocks use `sus`, followed by typed `catch` and `finally`. |

### Code Mapping Examples

```yukino
load std.io: println
load core.utils: cast

// Function with typed parameters and return type
fun calculate_total(ref prices: int32[], tax_rate: float64) -> float64 {
    set sum: int32 = 0
    for i in 0..prices.len() {
        rewrite sum += prices[i]
    }
    return cast(sum, "float64") * (1.0 + tax_rate)
}

// Pattern matching with switch and else default
fun categorize_status(code: int32) -> string {
    return switch code {
        200 -> "OK"
        201 | 204 -> "Success (No Content)"
        c if c >= 400 && c < 500 -> "Client Error"
        c if c >= 500 -> "Server Error"
        else -> "Unknown Status"
    }
}
```

---

## 2. Memory Model, Affine Types, and Arenas

Yukino has no garbage collector, no reference counting pauses, and no manual `malloc`/`free`.

### 2.1 Copy Types vs Affine Types

1. **Copy Types**:
   - Primitive numbers (`int8`..`int64`, `uint8`..`uint64`, `float32`, `float64`), `bool`, `char`.
   - Raw pointers: `ptr<T>`.
   - String descriptors: `string` (16-byte fat pointer `{ ptr, len }`). Assigning `set s2 = s1` copies the descriptor; both variables remain valid.
2. **Affine Types**:
   - `struct`, `class`, file handles, network sockets, mutexes, raw buffers.
   - Assignment transfers ownership (**move semantics**). The source variable is statically invalidated:
     ```yukino
     set a = MyStruct { id: 1 }
     set b = a // 'a' is moved to 'b'; accessing 'a' subsequently is a compiler error
     ```

### 2.2 Shared (`ref`) and Exclusive (`ref ~`) Borrowing

- `ref T`: Shared, read-only borrow. Any number of active immutable borrows can coexist.
- `ref ~T`: Exclusive, mutable borrow. While an exclusive mutable borrow exists, no other borrows (mutable or immutable) are permitted.

```yukino
struct Buffer {
    size: int32
}

fun modify(ref ~b: Buffer) {
    rewrite b.size = b.size + 10
}

fun inspect(ref b: Buffer) {
    println("Buffer size: " + cast(b.size, "string"))
}
```

### 2.3 Regional Memory Allocation

Curly braces `{}` define both lexical scope and an implicit memory region with a bump allocator. All dynamic allocations (strings, structs, closures) created within a block are allocated sequentially in that region. When control leaves the block, the entire region is reclaimed in $O(1)$ time.

- `region {}`: Explicit scoped region block.
- `for region i in 0..n`, `while region cond`, `loop region`: Loop-scoped regions resetting each iteration.
- `export<T>(val: T) -> T`: Promotes `val` from an inner region to the outer caller region.
- `persistent<T>(val: T) -> T`: Promotes `val` to the process-level lifetime.
- `defer { ... }`: Registers a deferred block executed in LIFO order upon exiting the enclosing scope.

### 2.4 First-Class Arenas (`class Arena`)

For long-lived pools, game engines, servers, and parsers, use `Arena` from `core.mem`:

```yukino
load core.mem: Arena

fun run_worker() {
    set pool = init Arena(65536)
    defer {
        pool.destroy()
    }

    set initial_bytes = pool.used()
    set total_cap = pool.cap()

    // Route temporary allocations into the arena
    region(pool) {
        set payload = "temporary packet data"
        println(payload)
    }

    // Reset arena memory back to zero without reallocating
    pool.reset()
}
```

---

## 3. Code Quality: Design Decisions (Panic vs Null)

Yukino enforces strict architectural separation between **expected data absence** and **exceptional runtime failures**.

### When to Return `null` (`T?`)

Return `null` (optional type `T?`) when a value can be legitimately absent during normal program execution:
- Looking up a key in a map or cache.
- Parsing an optional CLI parameter or environment variable.
- Non-blocking I/O when no data is currently ready.

**Rules**:
1. Never return sentinel values (such as `-1`, `""`, or `0`) to indicate absence.
2. Use null-coalescing `??` for default fallbacks: `set port = config.port ?? 8080`.
3. Use optional chaining `?.` for safe traversal: `set host = client?.connection?.host`.

### When to Invoke `panic`

Invoke `panic` for unrecoverable or unexpected exceptional conditions:
- Corrupted file format or network protocol violation.
- Missing required system resources, corrupted state invariants.
- Division by zero or out-of-bounds memory access.

**Strict Rules**:
1. **Never panic with raw strings or numbers.** Always create and pass a typed struct.
2. The error struct must clearly convey the failure context:
   ```yukino
   public struct DatabaseError {
       message: string
       code: int32
   }

   if connection == null {
       panic(DatabaseError {
           message: "Database connection pool exhausted",
           code: 500,
       })
   }
   ```
3. **Never write empty catch clauses:**
   ```yukino
   // FORBIDDEN ANTI-PATTERN:
   // catch e { space }

   // MANDATORY PATTERN:
   sus {
       fetch_record(id)
   } catch e: DatabaseError {
       println("Database failure [code " + cast(e.code, "string") + "]: " + e.message)
   } catch {
       println("Unexpected panic: " + errkind)
   } finally {
       cleanup_transaction()
   }
   ```
4. For functions that perform side effects without returning data, return `bool` indicating success so callers can verify the operation.

---

## 4. Manifests and Workspaces Guide

Yukino project builds are driven by three distinct manifest formats:

### 4.1 Executable Application: `yuki.toml`

Used for standalone binary applications. Default entry point is `src/main.ykn`.

```toml
[bin]
name = "my_app"
version = "0.1.0"
source = "src/main.ykn"
includes = ["core", "stdlib"]

[libs.runtime]
# External dependencies (Git, path, or registry)
# logger = { path = "../logger" }

[config.debug]
opt = 0
debug_info = true
panic_strategy = "unwind" # "unwind" (sus/catch enabled) or "abort" (instant exit)

[config.release]
opt = 3
lto = true
panic_strategy = "abort"
```

### 4.2 Library Package: `lib.toml`

Used for libraries and compiler extensions. Default entry point is `<name>/lib.ykn`.

```toml
[lib]
name = "my_lib"
version = "0.1.0"
source = "my_lib/lib.ykn"
output = "static" # "static" (.a), "shared" (.so / .dll), or "macro" (procedural macro)
includes = ["core", "stdlib"]

[config.release]
opt = 3
lto = true
```

### 4.3 Monorepo Workspace: `yukispace.toml`

Coordinates multiple interrelated packages in a single repository:

```toml
[workspace]
members = [
    "libs/math_core",
    "libs/net_kit",
    "apps/server"
]

[workspace.build]
# Shared native C libraries linked across all members
# syslibs = ["pthread", "dl", "m"]

# Virtual inline packages (lightweight modules without separate lib.toml)
[members.fast_hash]
source = "libs/fast_hash/lib.ykn"
output = "static"
includes = ["core"]
```

---

## 5. Core Library (`core`) and CLI Reference

### 5.1 The `core` Package

The `core` package provides fundamental low-level runtime capabilities.

- `load core.utils: cast, assert, assert_eq, assert_ne`
  - `cast(val, "type")`: Explicit type conversions. Supported target types: `"int32"`, `"int64"`, `"string"`, `"float64"`, `"bool"`, `"char"`, etc.
  - `assert(condition, "error message")`: Invariant verification.
- `load core.mem: Arena, ptr_of, from_raw_parts, region_used, region_cap, region_reset, export, persistent`
  - `ptr_of(val)`: Obtains raw memory pointer `ptr<T>`.
  - `from_raw_parts(p, len, cap)`: Constructs slice `T[]` without allocation.
- `load core.process: exit, args, raw_args`
  - `exit(code)`: Terminates process with status code.
  - `args()`: Returns `string[]` of CLI arguments (index 0 is executable path).
- `load core.macro: typeof, file, line, col, cfg, include_str, include_bytes`
  - `typeof!(expr)`: Compile-time type introspection.
  - `include_str!("path")`: Embeds static file contents at compile time.

### 5.2 Developer CLI: `yukino`

Use the `yukino` CLI for daily project operations:

- `yukino new <name>`: Create executable application.
- `yukino new <name> --static`: Create static library (`lib.toml`).
- `yukino new <name> --dynamic`: Create shared library (`lib.toml`).
- `yukino new <name> --macro`: Create procedural macro crate.
- `yukino new <name> --workspace`: Initialize universal workspace (`yukispace.toml`).
- `yukino build`: Compile project with `debug` profile.
- `yukino build release`: Compile with `release` optimizations.
- `yukino build -b` / `yukino build --rebuild`: Recompile all packages and dependencies from scratch.
- `yukino build --refresh-deps`: Refresh and re-download external Git dependencies.
- `yukino run`: Build and run binary.
- `yukino test`: Run automated tests decorated with `#[test]`.

---

## 6. RAG Navigation Map: Standard Library and Deep Dive

When detailed API documentation or signatures are needed, use relative paths to consult the mirrored documentation in `./references/en/`.

### 6.1 Language Specification Chapters

| Topic | File Link | Key Concepts |
| :--- | :--- | :--- |
| **Language Overview** | [`0-overview.md`](./references/en/0-overview.md) | Design goals, toolchain requirements, platform ABI |
| **Toolchain & Projects** | [`2-cli-project.md`](./references/en/2-cli-project.md) | CLI commands, `yuki.toml`, `build.ykn` hooks, lockfiles |
| **Basic Syntax & Loops** | [`3-basics.md`](./references/en/3-basics.md) | `set`, `rewrite`, ranges `0..n`, loop labels, `space` |
| **Type System** | [`4-types.md`](./references/en/4-types.md) | Primitives, `T?`, `T1 \| T2` unions, `switch`, `type` alias |
| **Functions & Closures** | [`5-functions-closures.md`](./references/en/5-functions-closures.md) | `fun`, parameters, default values, captures, calling conventions |
| **Memory & Borrowing** | [`6-memory.md`](./references/en/6-memory.md) | Affine ownership, `ref ~`, `region {}`, escape analysis |
| **OOP & Data Types** | [`7-oop.md`](./references/en/7-oop.md) | `struct`, `class`, `interface`, `enum` ADTs, accessors |
| **Strings & Bytes** | [`8-strings.md`](./references/en/8-strings.md) | `string`, `bytes`, interpolation `${expr}`, string slicing |
| **Module System** | [`9-modules.md`](./references/en/9-modules.md) | `load`, relative paths `.sub`, directory overrides `!` |
| **Macros & Metaprogramming** | [`10-macros.md`](./references/en/10-macros.md) | Syntactic `define`, procedural macros, macro SDK |
| **System C ABI & FFI** | [`11-cabi.md`](./references/en/11-cabi.md) | `externc`, `cfun`, `cstruct`, calling conventions |
| **Core Library** | [`12-core-library.md`](./references/en/12-core-library.md) | `core.utils`, `core.mem`, `core.process`, `Arena` |
| **Compiler Attributes** | [`13-attributes.md`](./references/en/13-attributes.md) | `#[cfg(...)]`, optimization attributes, inlining |
| **Error Handling** | [`14-errors.md`](./references/en/14-errors.md) | `sus`, `catch`, `finally`, `panic`, unwinding vs abort |
| **Workspaces** | [`15-workspaces.md`](./references/en/15-workspaces.md) | `yukispace.toml`, multi-package monorepos, shared C libs |
| **Unit Testing** | [`16-tests.md`](./references/en/16-tests.md) | `#[test]`, `#[should_panic]`, `yukino test` harness |

### 6.2 Standard Library Modules (`std.*`)

| Module | Import Path | Primary Symbols / Capabilities | Reference Document |
| :--- | :--- | :--- | :--- |
| **Stdlib Overview** | `load std` | Architecture, common error models, conventions | [`stdlib/0-overview.md`](./references/en/stdlib/0-overview.md) |
| **Strings** | `std.strings` | `trim`, `split`, `join`, `replace`, `starts_with`, `ends_with`, `to_lower` | [`stdlib/1-strings.md`](./references/en/stdlib/1-strings.md) |
| **I/O** | `std.io` | `print`, `println`, `eprintln`, `read_line`, `flush` | [`stdlib/2-io.md`](./references/en/stdlib/2-io.md) |
| **Threads** | `std.threads` | `spawn`, `Thread`, `Mutex`, `Condvar`, `sleep` | [`stdlib/3-threads.md`](./references/en/stdlib/3-threads.md) |
| **Platform I/O** | `std.platformio` | Terminal attributes, raw console, platform descriptors | [`stdlib/4-platformio.md`](./references/en/stdlib/4-platformio.md) |
| **Math** | `std.math` | `sin`, `cos`, `sqrt`, `pow`, `abs`, `min`, `max`, mathematical constants | [`stdlib/5-math.md`](./references/en/stdlib/5-math.md) |
| **Date & Time** | `std.datetime` | `DateTime`, `Duration`, `now`, `format`, `parse`, UNIX timestamps | [`stdlib/6-datetime.md`](./references/en/stdlib/6-datetime.md) |
| **Random** | `std.random` | `rand_int`, `rand_float`, `seed`, cryptographically secure PRNG | [`stdlib/7-random.md`](./references/en/stdlib/7-random.md) |
| **File System** | `std.fs` | `File`, `open_read`, `open_write`, `read_to_string`, `write`, `remove_file` | [`stdlib/8-fs.md`](./references/en/stdlib/8-fs.md) |
| **Collections** | `std.collections` | `HashMap`, `HashSet`, `VecDeque`, `PriorityQueue` | [`stdlib/9-collections.md`](./references/en/stdlib/9-collections.md) |
| **Process Control** | `std.process` | `Command`, `spawn_process`, `output`, piping stdin/stdout | [`stdlib/10-process.md`](./references/en/stdlib/10-process.md) |
| **Operating System** | `std.os` | Platform queries, host architecture, system uptime | [`stdlib/11-os.md`](./references/en/stdlib/11-os.md) |
| **System Info** | `std.sysinfo` | CPU metrics, memory stats, disk space, network interfaces | [`stdlib/12-sysinfo.md`](./references/en/stdlib/12-sysinfo.md) |
| **Environment** | `std.env` | `get`, `set`, `remove`, `vars`, `current_dir` | [`stdlib/13-env.md`](./references/en/stdlib/13-env.md) |
| **URL Parsing** | `std.simpleurl` | `Url`, `parse_url`, query parameters, percent encoding | [`stdlib/14-simpleurl.md`](./references/en/stdlib/14-simpleurl.md) |
| **Compression** | `std.compress` | `gzip`, `zlib`, `deflate` compression and decompression | [`stdlib/15-compress.md`](./references/en/stdlib/15-compress.md) |
| **Archives** | `std.archives` | `tar`, `zip` archive extraction and creation | [`stdlib/16-archives.md`](./references/en/stdlib/16-archives.md) |
| **Locale** | `std.locale` | System locale, regional string and date formatting | [`stdlib/17-locale.md`](./references/en/stdlib/17-locale.md) |
| **Number Formatting** | `std.numf` | Precision control, scientific notation, binary/hex conversion | [`stdlib/18-numf.md`](./references/en/stdlib/18-numf.md) |
| **Shared Memory** | `std.sharedc` | Inter-process shared memory, memory-mapped files | [`stdlib/19-sharedc.md`](./references/en/stdlib/19-sharedc.md) |
| **JSON** | `std.json` | `parse_json`, `stringify_json`, JSON DOM navigation | [`stdlib/20-json.md`](./references/en/stdlib/20-json.md) |
| **JSON Comments** | `std.jsonc` | JSON with comments parsing and serialization | [`stdlib/21-jsonc.md`](./references/en/stdlib/21-jsonc.md) |
| **Output Formatter** | `std.outf` | ANSI colors, bold text, colored console output | [`stdlib/22-outf.md`](./references/en/stdlib/22-outf.md) |
| **CLI Parser** | `std.line` | Argument parser, options, subcommands, help generation | [`stdlib/23-line.md`](./references/en/stdlib/23-line.md) |
| **Terminal Tools** | `std.itermtool` | Interactive prompts, spinners, progress bars, cursors | [`stdlib/24-itermtool.md`](./references/en/stdlib/24-itermtool.md) |
| **Standard Output Tools**| `std.stdotool` | Table formatting, aligned text columns | [`stdlib/25-stdotool.md`](./references/en/stdlib/25-stdotool.md) |
| **Hashing** | `std.hash` | `sha256`, `sha512`, `md5`, `murmur3`, `xxhash` | [`stdlib/26-hash.md`](./references/en/stdlib/26-hash.md) |
| **Cryptography** | `std.crypto` | AES encryption/decryption, HMAC, secure key derivation | [`stdlib/27-crypto.md`](./references/en/stdlib/27-crypto.md) |
| **Networking** | `std.net` | `TcpListener`, `TcpStream`, `UdpSocket`, IP parsing | [`stdlib/28-net.md`](./references/en/stdlib/28-net.md) |
| **Regular Expressions** | `std.regex` | `Regex`, `is_match`, `find`, `captures`, `replace` | [`stdlib/29-regex.md`](./references/en/stdlib/29-regex.md) |
| **TOML** | `std.toml` | TOML document parsing, serialization, value querying | [`stdlib/30-toml.md`](./references/en/stdlib/30-toml.md) |
| **YAML** | `std.yaml` | YAML parsing and emission | [`stdlib/31-yaml.md`](./references/en/stdlib/31-yaml.md) |
| **SQLite** | `std.sqlite` | SQLite database connectivity, prepared statements, transactions | [`stdlib/32-sqlite.md`](./references/en/stdlib/32-sqlite.md) |
| **Encore Encoding** | `std.encore` | Base64, Hex, URL encoding and decoding utilities | [`stdlib/33-encore.md`](./references/en/stdlib/33-encore.md) |

---

## 7. Helper Automation Scripts

The skill provides scripts to accelerate agent reasoning and documentation lookup:

1. **Fast Documentation Search (`./scripts/search_docs.py`)**:
   Searches function signatures, classes, or keywords across the references:
   ```sh
   # Search stdlib for a specific function or type:
   ./scripts/search_docs.py "read_to_string" --stdlib-only

   # Full search with regex across all chapters:
   ./scripts/search_docs.py "class Arena"
   ```

2. **Snippet Verification (`./scripts/test_snippet.py`)**:
   Compiles and runs isolated `.ykn` code snippets to verify correctness:
   ```sh
   # Verify compilation:
   ./scripts/test_snippet.py path/to/snippet.ykn

   # Compile and run immediately:
   ./scripts/test_snippet.py path/to/snippet.ykn --run
   ```

---

## 8. Working Examples

Refer to validated sample code in `./examples/`:
- [`examples/01_memory/main.ykn`](./examples/01_memory/main.ykn): Demonstrates affine types, borrowing, `region`, `defer`, and `class Arena`.
- [`examples/02_errors/main.ykn`](./examples/02_errors/main.ykn): Demonstrates `null` handling (`??`) vs typed `panic` with `sus / catch / finally`.
- [`examples/03_syntax/main.ykn`](./examples/03_syntax/main.ykn): Demonstrates `rewrite`, `switch` matching (`->`), ranges, and closures (`fun(...) => ...`).
- [`examples/manifests/`](./examples/manifests/): Complete templates for `yuki.toml`, `lib.toml`, and `yukispace.toml`.
