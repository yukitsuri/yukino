# Error Handling and Exceptions (sus, catch, finally, panic)

The Yukino language adopts a strict, predictable, and explicit error handling model. It separates the logical absence of data from unexpected runtime failures, providing zero checking overhead on the happy path ("zero-cost exceptions") and fully typed error context during stack unwinding.

---

## Contents

1. [Philosophy and error handling strategy](#1-philosophy-and-error-handling-strategy)
2. [The sus, catch, finally construct](#2-the-sus-catch-finally-construct)
3. [The errkind variable](#3-the-errkind-variable)
4. [Catch clause forms](#4-catch-clause-forms)
5. [Exception generation: the panic intrinsic](#5-exception-generation-the-panic-intrinsic)
6. [Designing error structures: Best Practices and Anti-patterns](#6-designing-error-structures-best-practices-and-anti-patterns)
7. [Interaction with defer and memory regions](#7-interaction-with-defer-and-memory-regions)

---

## 1. Philosophy and error handling strategy

Yukino strictly distinguishes two situations:

1. **The logical absence of an expected value during normal operation**:
   If data is not found in a dictionary, a file is empty on a non-blocking read, or an optional parameter cannot be parsed — the function returns `null` (the optional type `T?`).
   No dummy values (such as an empty string in place of an error or `-1` as a valid identifier) are allowed.

2. **Runtime errors and exceptional situations**:
   If an operation cannot complete successfully (division by zero, a dropped network connection, a missing mandatory handle, data format corruption) — an exception is generated via `panic`.
   The exception must always be an instance of a struct. Passing raw strings or numbers is rejected by the compiler.
   Code must be prepared to catch such an exception with a `sus ... catch` block.

If a function by its semantics returns no useful data (for example, closing a handle or synchronizing a mutex), it should return a result indicator (for example, `bool`) so that the caller can confirm the operation completed.

In the `yuki.toml` project configuration, panic behavior is set by the `panic_strategy` in the profile section `[config.<profile>]` (for example, `[config.debug]` or `[config.release]`):

```toml
[config.debug]
panic_strategy = "unwind" # Default for debug: stack unwinding

[config.release]
panic_strategy = "abort"  # Optional for release: immediate abort
```

### Strategy comparison: unwind vs abort

The Yukino compiler supports two panic handling strategies:

| Characteristic | `unwind` (stack unwinding) | `abort` (immediate termination) |
| :--- | :--- | :--- |
| **Behavior on `panic`** | Stack unwinding via DWARF landing pads | Immediate process abort via `SIGABRT` (`abort()`) |
| **`sus ... catch` blocks** | Fully functional, errors can be caught | Do not catch: process aborts immediately on first panic |
| **`defer` and `finally` execution** | Guaranteed to run during unwinding | Do not run on panic (only run on normal exit) |
| **Exception tables (LSDA)** | Generated in the binary | Not generated (zero table size overhead) |
| **Binary size** | Higher due to landing pads and exception metadata | Minimal (no DWARF landing pads, smaller code size) |
| **Compiler optimizations** | Calls may require landing pad invocations | Maximum (`-O3`, inlining without unwind checks) |

#### When to use which strategy

* **Use `panic_strategy = "unwind"`:**
  * **Network and server applications:** HTTP/RPC request handlers, web services, background workers, where failure in a single client request must not crash the entire service.
  * **Parsers and deserializers:** parsing untrusted external inputs (JSON, configurations, protocols), where format errors should be caught via `catch` and reported gracefully to the caller.
  * **Resource management:** critical scenarios where deferred actions `defer` (closing handles, network sockets, releasing locks) must reliably execute even during abnormal termination of a subprogram.

* **Use `panic_strategy = "abort"`:**
  * **CLI tools and scripts:** one-shot utilities and batch processing scripts where any unexpected failure should instantly halt execution with a non-zero exit code (Fail-Fast principle).
  * **Crash-Only microservices:** cloud containers and workers designed to be restarted by orchestrators (Kubernetes / systemd) upon failure.
  * **Embedded and size-critical applications:** environments with strict memory and binary size constraints where omitting landing pads and DWARF exception tables is essential.

---

## 2. The sus, catch, finally construct

The `sus` block is used to safely execute risky code:

```yukino
sus {
    // Protected block: when a panic occurs, control
    // is transferred to the matching catch block
    do_risky_io()
} catch {
    // Error handling
} finally {
    // A block that always runs (after sus completes successfully or after catch)
    cleanup_resources()
}
```

### Execution rules

- If the `sus` block completes successfully, the `catch` block is skipped and the `finally` block runs (if declared).
- If a `panic` fires inside `sus`, execution is interrupted, all deferred `defer` statements inside the protected region run in reverse order (LIFO), and then control is transferred to the `catch` block whose type matched the exception type (or to the untyped catch-all `catch`).
- The `finally` block is guaranteed to run even when the exception is caught.
- If no `catch` block matched by type (and there is no catch-all `catch`), the exception is re-thrown further up the stack, and the `finally` block of this `sus` does not run.

---

## 3. The errkind variable

Inside any `catch` block the compiler automatically provides a string variable `errkind: string`.

| Variable | Type | Description |
| :--- | :--- | :--- |
| `errkind` | `string` | The type name of the structure that caused the panic (e.g., `"JsonError"`, `"MathError"`, or `"Panic"` for system aborts) |

The `errkind` variable is always available: in a universal `catch` block, in a typed `catch Struct` block, and in a typed `catch TempName: Struct` block. It allows identifying the origin of the error even in a universal handler.

---

## 4. Catch clause forms

Yukino supports three complementary forms of error handling in `catch` blocks:

1. **Typed catch with variable binding (`catch TempName: Struct`)**:
   Matches only exceptions of type `Struct` and introduces a local variable named `TempName` of type `Struct` into the block scope.
   The entire error struct payload is transferred into scope: all fields (`TempName.field`) are directly accessible with their accurate runtime values.

2. **Typed catch without variable (`catch Struct`)**:
   Matches only exceptions of type `Struct`, but does not bind any struct variable into the `catch` block scope. Useful when you only need to react to a specific error type without inspecting its fields.

3. **Universal catch-all (`catch`)**:
   Catches any exceptions that were not caught by preceding `catch` blocks. No struct variable is placed into scope, but `errkind: string` is available.

> [!NOTE]
> **Automatic error struct import in `catch`:**  
> In Yukino, error structures do not need to be explicitly imported via `load` to be used in `catch` (explicit imports remain available if preferred):
> - In a universal `catch {}`, the compiler automatically imports all error structures from the files and modules whose functions or classes are called within the `sus` block.
> - In a typed catch (`catch e: ErrorStruct` or `catch ErrorStruct`), only the specified structure `ErrorStruct` is automatically imported.
>
> ```yukino
> load json: parse_data
>
> sus {
>     parse_data(raw_payload)
> } catch e: JsonError {
>     // JsonError is automatically available without an explicit load json: JsonError
>     set x = e.line
> } catch {
>     // In universal catch, errkind contains the error type name
>     println("Error: " + errkind)
> }
> ```

### Example: Comprehensive handling with JsonError

Consider a JSON parser error structure:

```yukino
public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

Safe parsing and differentiated catching:

```yukino
load std.io: println

public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}

public struct NetworkError {
    code: int32
    endpoint: string
}

fun parse_payload(raw: string) {
    if raw.len() == 0 {
        panic(JsonError {
            message: "unexpected end of file",
            line: 1,
            column: 1,
            offset: 0,
        })
    }
}

fun main() {
    sus {
        parse_payload("")
    } catch err: JsonError {
        // The err variable is available with all fields of JsonError:
        println("JSON parse error: " + err.message)
        println("Line:   " + cast(err.line, "string"))
        println("Column: " + cast(err.column, "string"))
        println("Offset: " + cast(err.offset, "string"))
        println("Error kind: " + errkind) // "JsonError"
    } catch NetworkError {
        // Catches NetworkError without binding a variable to scope
        println("Network failure during document loading (kind: " + errkind + ")")
    } catch {
        // Universal fallback for any other exceptions
        println("Unknown error: " + errkind)
    } finally {
        println("Parsing finished")
    }
}
```

---

## 5. Exception generation: the panic intrinsic

The compiler intrinsic `panic` is used to initiate abnormal termination or stack unwinding:

```yukino
panic(payload: S) -> never
```

- **Parameter:** `payload` — an instance of an error struct (must be a `struct` type).
- **Return type:** `never` — control never returns to the next instruction.

Attempting to pass a primitive value (string, integer, array) or invoking `panic()` without arguments results in a compile-time type error:

```yukino
// Compile error: panic requires an argument of struct type
panic("something went wrong")
```

---

## 6. Designing error structures: Best Practices and Anti-patterns

Yukino enforces strict standards for error code design.

### 6.1 How to create error structures (Best Practices)

1. **Always use typed structures**: create a dedicated error structure with a clear name for each domain (`JsonError`, `MathError`, `IoError`, `ThreadError`).
2. **Define diagnostic fields**: include all fields necessary for troubleshooting: message text (`message: string`), integer codes (`code: int32`), coordinates (`line: int64`, `column: int64`), file paths, or hostnames.
3. **Use precise types**: avoid untyped fields; use precise integer and string types.

```yukino
// High quality error struct definition
struct FileNotFoundError {
    message: string
    code: int32
    path: string
}

if !exists(filename) {
    panic(FileNotFoundError {
        message: "file does not exist",
        code: 404,
        path: filename,
    })
}
```

### 6.2 What you must NOT do (Anti-patterns)

#### Anti-pattern 1: Silencing errors and fake fallbacks
```yukino
// BAD: masking a critical failure by returning a fake zero
fun read_config_int(key: string) -> int32 {
    if !has_key(key) {
        return 0 // The caller cannot tell whether the key equals 0 or is missing
    }
    // ...
}

// GOOD: returning the optional type T?
fun read_config_int(key: string) -> int32? {
    if !has_key(key) {
        return null
    }
    // ...
}
```

#### Anti-pattern 2: Empty catch blocks
```yukino
// BAD: the exception is swallowed with no diagnostic value
sus {
    critical_migration()
} catch {
    // empty
}
```

---

## 7. Interaction with defer and memory regions

The Yukino exception model is fully integrated with the automatic memory management and deferred actions system:

1. **LIFO `defer` calls during unwinding**:
   If `defer` statements were declared inside the protected block, when a `panic` fires they run in reverse order (LIFO) before control transfers to `catch`.

2. **Regional memory cleanup (`region`)**:
   Memory allocated inside `region` blocks on the frame's stack pages is freed during stack unwinding without pointer leaks.

```yukino
load std.io: println

struct IoError {
    message: string
    code: int32
}

fun example() {
    sus {
        set handle = open_resource()
        defer close_resource(handle) // Guaranteed to run even on the panic below

        panic(IoError {
            message: "read failure",
            code: 5,
        })
    } catch err: IoError {
        println("Resource already safely closed, error: " + err.message)
    }
}
```

---

[← Back: Compiler Attributes](13-attributes.md) | [Next: Workspaces and Multi-Package Projects →](15-workspaces.md)
