# Memory Model, Ownership, and Regions

Yukino offers a deterministic memory management model: **C- and Rust-level performance without garbage collector (GC) overhead** and **static guarantees against dangling pointers and memory leaks**.

This chapter describes Yukino's memory management architecture: resource lifecycle, the affine type system, the rules of the static borrow verifier (Borrow Checker), and regional bump allocators.

---

## Contents

1. [Foundation: why Yukino has no Garbage Collector and malloc/free](#1-foundation-why-yukino-has-no-garbage-collector-and-mallocfree)
2. [Memory management architecture: ownership and borrowing](#2-memory-management-architecture-ownership-and-borrowing)
3. [How ordinary strings (string) are laid out in memory](#3-how-ordinary-strings-string-are-laid-out-in-memory)
4. [Affine ownership and moves (Ownership & Move)](#4-affine-ownership-and-moves-ownership--move)  
   4.1 [Copy types and Affine types](#41-copy-types-and-affine-types)  
   4.2 [Ownership transfer (move)](#42-ownership-transfer-move)  
   4.3 [Protection against use-after-move (Use-After-Move)](#43-protection-against-use-after-move-use-after-move)
5. [Borrowing: references vs ownership](#5-borrowing-references-vs-ownership)  
   5.1 [Technical comparison of semantics](#51-technical-comparison-of-semantics)  
   5.2 [Immutable references (ref T): many readers](#52-immutable-references-ref-t-many-readers)  
   5.3 [Mutable references (ref ~T): one exclusive writer](#53-mutable-references-ref-t-one-exclusive-writer)  
   5.4 [The iron rule of the Borrow Checker](#54-the-iron-rule-of-the-borrow-checker)  
   5.5 [How to write memory-optimized code (Practical rules)](#55-how-to-write-memory-optimized-code-practical-rules)
6. [The regional memory model (Regions)](#6-the-regional-memory-model-regions)  
   6.1 [Bump allocation and instant O(1) deallocation](#61-bump-allocation-and-instant-o1-deallocation)  
   6.2 [Implicit function regions and stack frames](#62-implicit-function-regions-and-stack-frames)  
   6.3 [Memory reset in loops (region in the loop body)](#63-memory-reset-in-loops-region-in-the-loop-body)  
   6.4 [Explicit region { ... } blocks for heavy tasks](#64-explicit-region--blocks-for-heavy-tasks)  
   6.5 [Escape analysis: prohibiting reference escapes](#65-escape-analysis-prohibiting-reference-escapes)
7. [Deferred execution: defer](#7-deferred-execution-defer)
8. [Advanced Memory Management](#8-advanced-memory-management)  
   8.1 [Regional memory introspection (region_used, region_cap)](#81-regional-memory-introspection-region_used-region_cap)  
   8.2 [Explicit local region reset (region_reset)](#82-explicit-local-region-reset-region_reset)  
   8.3 [Configuring initial region capacity (#[region::capacity])](#83-configuring-initial-region-capacity-regioncapacity)  
   8.4 [Regional loops (for region, while region, loop region)](#84-regional-loops-for-region-while-region-loop-region)  
   8.5 [Value export and expression regions (export, returning from region)](#85-value-export-and-expression-regions-export-returning-from-region)
9. [Cheat sheet: what to choose in practice](#9-cheat-sheet-what-to-choose-in-practice)

---

## 1. Foundation: why Yukino has no Garbage Collector and malloc/free

Existing programming languages have historically settled on two approaches:

1. **Manual memory management (C, C++):**  
   `malloc()` and `free()` calls are the programmer's responsibility. Mistakes lead to memory leaks, double frees, or dereferencing freed pointers (Use-After-Free), which results in crashes (`SIGSEGV`) or security vulnerabilities.
2. **Garbage collector (Java, Go, C#, Python, JavaScript):**  
   A background runtime thread scans the heap object graph. This entails increased memory consumption by metadata and execution pauses (Stop-The-World).

### The Yukino approach

Yukino solves the task deterministically: **full safety without garbage collector overhead**.  
The compiler statically analyzes variable lifetimes, ownership points, and borrow boundaries, generating code that frees memory at the moment of leaving the corresponding scope or region.

A key detail of this model: curly braces `{}` play a dual role. They simultaneously define the visibility scope of variables and serve as the memory allocation area (the region). When program execution leaves a block, the entire region is cleaned up in a single action: all data allocated within it — strings, slices, class instances — is freed together, regardless of where exactly inside the block it was created.

---

## 2. Memory management architecture: ownership and borrowing

Yukino's memory model rests on the following fundamental principles:

- **Affine type system (Affine Ownership):**  
  Every value of a nontrivial type has exactly one owning variable. The owner determines the lifetime and the point where the resource is freed.
- **Move semantics:**  
  When a value is assigned to a variable or passed to a function call, ownership is transferred to the new context. The previous variable is statically invalidated at the move point, ruling out reuse or multiple frees.
- **Shared borrowing (`ref T`):**  
  Passing a pointer to a memory area for reading only, without transferring ownership rights. Any number of parallel immutable borrows is allowed.
- **Exclusive borrowing (`ref ~T`):**  
  Passing a pointer for in-place mutation. At any moment in time, for a given resource, either a single mutable reference or any number of immutable references is allowed (**the Aliasing XOR Mutability invariant**).

---

## 3. How ordinary strings (string) are laid out in memory

Strings are the most illustrative example of the memory model in action.

### The string descriptor (Fat Pointer)

In Yukino, a `string` is not an array of bytes by itself, but a compact **descriptor** only 16 bytes in size on a 64-bit system:

```text
String: { ptr: pointer to the bytes, len: length in bytes }
```

```yukino
set greeting = "Hello, world!"
```

- The text `"Hello, world!"` itself resides in memory as a sequence of bytes.
- The variable `greeting` on the stack stores only `{ ptr, len }`.
- Strings in Yukino are **immutable**. No operation can corrupt the bytes of an existing string.

### What happens on assignment `set s2 = s1`?

A string is a **Copy type**, so assignment copies its 16-byte descriptor:

```yukino
set s1 = "Yukino"
set s2 = s1 // copies 16-byte descriptor: both s1 and s2 remain valid
println(s1) // "Yukino"
println(s2) // "Yukino"
```

At the machine level, only the 16 bytes of the `{ ptr, len }` descriptor are copied (values of CPU registers): no second byte buffer is created, and `s2` points to the exact same bytes in memory. In earlier versions of Yukino, `string` was treated as an affine type, causing the source variable `s1` to be consumed (`use of moved value`) and forcing developers to pass `ref s` everywhere. That artificial friction was removed: since strings are strictly immutable, the descriptor is freely copied, both variables remain valid, and passing `ref` is not required to read them.

### And what if we concatenate strings (`s1 + s2`)?

```yukino
set a = "Hello, "
set b = "Yukino!"
set c = a + b // A new memory buffer is allocated for "Hello, Yukino!"
```

The `+` operator allocates a new contiguous memory area in the **current region** and copies the bytes of both strings there. The descriptor `c` points to this new buffer.

### How to force a full independent copy?

Strings in Yukino are immutable, so deep copying is not required: calling `.cp()` on a string returns its descriptor without reallocating memory. Deep copying is relevant for mutable buffers — `T[]` slices and `bytes`, where `.cp()` allocates a new buffer in the current region and copies the elements:

```yukino
set original: uint8[] = [1, 2, 3]
set clone = original.cp() // physically allocates a new independent buffer
```

---

## 4. Affine ownership and moves (Ownership & Move)

### 4.1 Copy types and Affine types

All data types are divided into two groups:

1. **Copy types (scalars, trivial aggregates, and strings):**  
   Numbers (`int8`..`int64`, `uint8`..`uint64`, `float32`, `float64`), boolean flags (`bool`), characters (`char`), strings `string`, pointers `ptr`, function types, `null`, and tuples and union types consisting entirely of Copy types. An immutable reference (`ref T`) is also copied.  
   They are cheaply and automatically copied bitwise on any assignment or pass to a function:
   ```yukino
   set a: int32 = 42
   set b = a // b = 42, the variable 'a' is still valid

   set s1 = "text"
   set s2 = s1 // descriptor copied, both variables remain valid
   ```
2. **Affine types (types with resources and mutable objects):**  
   Bytes `bytes`, arrays `T[]`, maps `map`, structures `struct`, classes `class`, interfaces, enumerations `enum`. Every such value has **exactly one owner**.

### 4.2 Ownership transfer (move)

When a value of an affine type is passed to another variable or as a function argument by value, ownership is **moved**:

```yukino
struct DatabaseConnection {
    host: string
    port: int32
}

fun close_connection(conn: DatabaseConnection) {
    // conn now belongs to this function and will be destroyed on exit
}

set my_conn = DatabaseConnection { host: "localhost", port: 5432 }

// Transfer ownership to the function:
close_connection(my_conn)

// COMPILATION ERROR: my_conn no longer exists in this scope!
// print(my_conn.host)
```

### 4.3 Protection against use-after-move (Use-After-Move)

In other languages, accessing a moved object can lead to reading garbage or crashing at runtime. In Yukino, the compiler tracks this at the static analysis stage:

```yukino
struct Packet {
    payload: string
}

set p1 = Packet { payload: "secret data" }
set p2 = p1 // Ownership has been moved to p2

// Compilation error E0501: use of moved value 'p1'
// set data = p1.payload
```

---

## 5. Borrowing: references vs ownership

What if we want to pass a structure or data to a function for computation or inspection, **but do not want to give up ownership**?  
**References (borrowing)** are used for this.

### 5.1 Technical comparison of semantics

| Criterion | Ownership semantics (`T`) | Borrowing semantics (`ref T` / `ref ~T`) |
| :--- | :--- | :--- |
| **Lifetime management** | Exclusive right to initialize and destroy the resource | The lifetime is strictly bounded by the owner's context |
| **Argument passing** | Move semantics or bitwise copy (Copy) | Passing a pointer without changing or invalidating the owner |
| **Memory at the machine level** | Placed on the stack or a descriptor in the local region | A direct 64-bit pointer to the object's memory |
| **Access invariant** | A single owner | Many readers (`ref`) XOR one writer (`ref ~`) |

### 5.2 Immutable references (ref T): shared read access

An immutable reference allows accessing structure fields or string bytes while guaranteeing that the called code cannot mutate the target buffer.

In Yukino, a reference is specified with the `ref` keyword both when declaring a function parameter and **explicitly at the call site**:

```yukino
struct UserProfile {
    name: string
    age: int32
}

// The function takes a read-only reference:
fun print_user(ref u: UserProfile) {
    println("User: ${u.name}, age: ${u.age}")
    // rewrite u = ... -- COMPILATION ERROR (E0503): cannot reassign a variable taken by immutable reference!
}

set user = UserProfile { name: "Alice", age: 25 }

// Pass explicitly by reference via ref:
print_user(ref user)

// 'user' is still ours! We can keep using it:
println("Owner's name: ${user.name}")
```

#### When ref is needed and when it is not needed for reading:

Because strings `string` are Copy types, they do not need to be passed by reference `ref`: passing by value `compute_stats(title)` copies only the 16-byte descriptor and leaves the caller's variable fully valid:

```yukino
fun compute_stats(s: string) -> int64 {
    return s.len()
}

set title = "Yukino Yukinoshita > fiction"
set length = compute_stats(title)
// title was not moved and remains fully accessible
```

An immutable reference `ref T` in Yukino is primarily used for **heavy structures (`struct`)**, whose size in memory can be tens or hundreds of bytes (to avoid copying all structure fields to the stack), as well as when working with slices and interfaces:

```yukino
struct LargeData {
    matrix: float64[16]
    label: string
}

fun process_data(ref d: LargeData) {
    // an 8-byte pointer is passed instead of copying the 144-byte struct
}
```

### 5.3 Mutable references (ref ~T): one exclusive writer

If a function must modify the fields of an existing structure or reassign the owner variable's value in place, the mutable reference `ref ~` is used:

```yukino
struct Counter {
    value: int64
}

// Takes a mutable reference:
fun increment(ref ~c: Counter, delta: int64) {
    rewrite c.value = c.value + delta
}

set my_counter = Counter { value: 10 }

// Explicitly pass the right to mutate via ref ~:
increment(ref ~my_counter, 5)

println("New value: ${my_counter.value}") // 15
```

#### Mutating a string variable through a reference:
Because the underlying string byte buffer is immutable, a function can only update the caller's string variable by rewriting its 16-byte descriptor. `ref ~s: string` is used for this:

```yukino
fun append_suffix(ref ~s: string, suffix: string) {
    rewrite s = s + suffix
}

set message = "Processing"
append_suffix(ref ~message, " complete.")
println(message) // "Processing complete."
```

### 5.4 The iron rule of the Borrow Checker

To prevent data races, reading inconsistent data, and memory corruption, the Yukino compiler enforces the fundamental exclusivity rule:

> **At any point of the program, for a single value, either is permitted:**  
> - **Any number of immutable readers (`ref T`)**  
> **OR**  
> - **Exactly one exclusive writer (`ref ~T`)**  
> *(But never readers and a writer at the same time!)*

```yukino
set item = Counter { value: 0 }

// You cannot hold ref and ref ~ at the same time:
// fun bad_caller() {
//     set reader = ref item
//     increment(ref ~item, 1) // COMPILATION ERROR: item is already borrowed for reading!
//     print(reader.value)
// }
```

### 5.5 How to write memory-optimized code (Practical rules)

1. **Pass scalars and strings (`int32`, `bool`, `string`) by value.**  
   Passing 1–16 bytes directly in CPU registers is faster than passing a memory address and performing an extra pointer indirection.
2. **Pass heavy structures (`struct`) by reference `ref T`.**  
   For structures larger than 16 bytes, passing by `ref` passes a single 64-bit pointer and avoids copying all fields onto the stack.
3. **Modify data in place via `ref ~T`.**  
   Mutating an existing object or rewriting a variable through `ref ~` avoids allocating intermediate structures and returning tuples.
4. **Wrap loops in `region { ... }` blocks.**  
   If temporary strings or arrays are created inside a loop, the `region` block resets the arena watermark at the end of each iteration in O(1). RAM usage stays flat regardless of the number of iterations.
5. **Use `bytes` instead of hundreds of `+` concatenations for byte-by-byte building.**  
   Repeatedly concatenating strings with `+` allocates a new buffer on every step. When building a complex document or binary packet, use a mutable `bytes` buffer with `.push()` or `reserve()`.

---

## 6. The regional memory model (Regions)

Most languages allocate memory for every dynamic string or array with a separate call to the system allocator (`malloc`). This leads to memory fragmentation and performance degradation.

In Yukino, the memory of dynamic objects is managed by **regions (arena/region allocators)**.

### 6.1 Bump allocation and instant O(1) deallocation

Inside a region, memory allocation is performed through a bump allocator (linear allocation in a contiguous buffer): the offset pointer is incremented by the size of the requested data structure, taking machine alignment requirements into account:

```text
[ Region: contiguous arena buffer ]
[ Object 1 ][ String 2 ][ Array 3 ][ Free ... ] -> The top pointer advances in a single CPU cycle
```

When the region's scope ends, the runtime resets the arena pointer to the original mark in $O(1)$, without calling destructors element by element.

### 6.2 Implicit function regions and stack frames

Every function in Yukino automatically gets its own regional frame. All temporary strings, concatenations, and intermediate arrays inside a function live in its local region.

When the function finishes execution, all temporary memory is instantly cleared:

```yukino
load core.utils: cast

fun format_report(id: int32) -> string {
    // Temporary strings are created in the function's local region
    set prefix = "Report #"
    set num_str = cast(id, "string")
    set full = prefix + num_str

    // The return value full is safely transferred to the caller's region!
    return full
}
```

### 6.3 Memory reset in loops (region in the loop body)

In other languages, loops that create strings or arrays on every iteration often cause memory leaks or garbage collector overload.

Region marks (`save_mark` / `restore_mark`) are set by explicit `region { ... }` blocks: on block entry the current arena position is saved, and on exit the pointer is reset back to it in O(1). If such a block is placed inside a loop body, each iteration's memory is released as soon as the step completes:

```yukino
load core.utils: cast

fun process_many_items() {
    set i = 0
    while i < 1_000_000 {
        region {
            // A new temporary string is created on every iteration:
            set temp = "Item #" + cast(i, "string")
            do_light_work(temp)
        }
        rewrite i = i + 1
        // temp and all intermediate strings of the iteration are ALREADY cleared!
        // Memory usage does NOT grow regardless of the number of iterations!
    }
}
```

Plain `for`, `while`, and `loop` loops without an explicit `region` block do not set marks: temporary values created in the iterations live in the function's local region until the function finishes.

### 6.4 Explicit region { ... } blocks for heavy tasks

If you need to perform heavy intermediate work (for example, parsing a 50 megabyte JSON or processing a large batch of strings) and free all the memory **immediately**, an explicit `region` block is used:

```yukino
fun handle_batch(records: string[]) {
    // Process a heavy block of data:
    region {
        set buffer: string[] = []
        for i in 0..records.len() {
            buffer.push(records[i] + " [validated]")
        }
        send_to_network(buffer)
    }
    // At this point the region's arena pointer is reset to the entry mark,
    // instantly returning all the buffer and temporary string memory to the pool in O(1).

    continue_other_work()
}
```

### 6.5 Escape analysis: prohibiting reference escapes

The compiler statically verifies that a reference to an object living in a local region cannot be stored into a longer-lived context:

```yukino
// Compilation error: a reference to a local region tries to escape outward
// fun bad_escape() -> ref string {
//     set local_str = "temp"
//     return ref local_str // FORBIDDEN: local_str dies on function exit!
// }
```

---

## 7. Deferred execution: defer

For guaranteed release of system resources (file descriptors, mutexes, sockets), the `defer` construct is used:

```yukino
fun read_config_file(path: string) -> string {
    set file = open_file(path)
    // Guarantee the file is closed on any outcome:
    defer {
        close_file(file)
    }

    if file.is_empty() {
        return "" // defer runs here before exiting
    }

    set content = file.read_all()
    return content // and defer is guaranteed to run here too!
}
```

- `defer` instructions execute in reverse order of declaration (**LIFO** — Last In, First Out).
- `defer` runs on a normal return (`return`), on early interruption (`break`, `continue`), and also during exception-driven stack unwinding (`sus`/`catch`/`finally`).

---

## 8. Advanced Memory Management

> [!NOTE]
> The use of advanced memory management and introspection tools is optional. The baseline regional memory model and automatic stack frame allocation fully address the needs of the vast majority of applications without requiring manual configuration.

In high-throughput scenarios, such as batch data processing, parsing gigabyte-scale files, performance profiling, and building system or graphics loops, the Yukino compiler provides mechanisms for direct control over regional memory: memory consumption introspection and arena initial capacity configuration.

### 8.1 Regional memory introspection (region_used, region_cap)

Functions `region_used` and `region_cap` are available from the `core.mem` module (`load core.mem: region_used, region_cap`):

#### Signatures:
```yukino
load core.mem: region_used, region_cap

region_used() -> int64
region_cap() -> int64
```

- `region_used()`: returns the actual number of bytes currently occupied by dynamic structures in the local region (accounting for machine alignment).
- `region_cap()`: returns the total capacity of all system memory chunks allocated for the current region by the operating system.

#### Profiling allocation volume example:
```yukino
load core.mem: region_used, region_cap

fun benchmark_payload(data: string) {
    set mem_start = region_used()

    set parsed = parse_large_json(data)
    set summary = parsed.build_report()

    set mem_end = region_used()
    set allocated = mem_end - mem_start

    println("Memory consumed: ${allocated} bytes")
    println("Total region capacity: ${region_cap()} bytes")
}
```

### 8.2 Explicit local region reset (region_reset)

In long-running loops or batch task processors within a single function, developers can explicitly reset the region allocation pointer back to the beginning, reclaiming all memory accumulated in the current frame:

#### Signature:
```yukino
load core.mem: region_reset

region_reset() -> void
```

The function resets the current arena bump pointer back to zero in $O(1)$, deallocates any secondary overflow chunks, and resets used bytes to 0.

> [!CAUTION]
> Calling `region_reset()` invalidates all local dynamic objects created in the current function prior to the reset point. Only invoke `region_reset()` when all previously allocated temporary structures are no longer referenced.

#### Batch processing example:
```yukino
load core.mem: region_reset

fun process_batches(batches: string[]) {
    set i = 0
    while i < batches.len() {
        set task = load_task(batches[i])
        task.execute()

        // Reclaim all memory accumulated during this batch step
        region_reset()
        rewrite i = i + 1
    }
}
```

### 8.3 Configuring initial region capacity (#[region::capacity])

By default, every function receives a local region with an initial capacity of 4096 bytes (4 KB). When a function is known to perform heavy allocations (e.g. creating frame buffers or processing 100,000 records), repeatedly growing by 64 KB chunks via system calls introduces unnecessary overhead.

The `#[region::capacity(N)]` attribute instructs the compiler to allocate an initial region chunk of the specified byte size:

```yukino
load core.mem: region_cap

#[region::capacity(1048576)] // Preallocate 1 MB upon entering the function
fun heavy_worker(input_path: string) {
    set cap = region_cap()
    println("Initial arena capacity: ${cap} bytes") // at least 1048576 bytes

    // Thousands of string concatenations and arrays allocate within a single bump buffer without kernel calls
}
```

- The argument must be a strictly positive integer. Passing a non-positive integer or invalid literal produces diagnostic error `E0201`/`E0401`.
- The attribute is applicable to functions (`fun`) and class methods (`method`).
- If omitted, the default economical capacity (4096 bytes) is used.

### 8.4 Regional loops (for region, while region, loop region)

When processing large arrays, text streams, or network messages inside loops, numerous short-lived intermediate objects are created. Without isolation, these objects survive until the enclosing function returns, gradually inflating memory consumption.

Instead of manually wrapping the loop body in an inner `region { ... }` block (which adds an extra indentation level and clutters reading flow), Yukino provides a dedicated concise syntax for **regional loops**:

```yukino
load core.utils: cast

// Memory is rolled back in O(1) after each iteration
for region i in 0..100000 {
    set formatted = "[" + cast(i, "string") + "] payload"
    send_message(formatted)
}

// Supporting two variables (index and element):
for region i, item in items {
    set transformed = transform(item)
    store_output(i, transformed)
}

// Regional while:
while region reader.has_next() {
    set line = reader.read_line()
    process_line(line)
}

// Regional infinite loop:
loop region {
    set event = poll_event()
    if event.is_exit() {
        break
    }
    handle_event(event)
}
```

#### Mechanics and guarantees:
1. **Automatic mark rollback in O(1):** At the beginning of each iteration, the compiler saves the current region bump mark (`save_mark`). At the end of the iteration, the mark is restored (`restore_mark`), freeing all allocations performed during that step without any OS kernel calls.
2. **Proper continue and break handling:** When `continue` or `break` executes (including labeled variants such as `break outer`), the compiler's cleanup stack automatically rewinds the regional mark before transferring control.
3. **Outer state preservation:** Variables declared outside the loop retain their mutations (`rewrite acc = acc + 1`). The Borrow Checker statically guarantees that short-lived regional references from a loop iteration cannot escape into outer variables.
4. **Syntactic compatibility with labels:**
   - Prefix label: `outer: for region i in 0..10 { break outer }`
   - Inline label: `for outer: region i in 0..10 { break outer }`

### 8.5 Value export and expression regions (export, returning from region)

By default, a `region { ... }` block or an iteration of a regional loop isolates all temporary intermediate allocations. When the block finishes or the loop proceeds to the next iteration, the entire allocated volume is instantly reclaimed.

However, practical patterns often require:
1. Computing a result inside an isolated region (such as filtering or transforming a string/array) and **returning it to the outer context**.
2. Accumulating selected elements into an outer collection across regional loop iterations.

To support these workflows, Yukino provides first-class **expression regions** and the value export function `export(val)` (as well as the method call syntax `val.export()`), available from the `core.mem` module (`load core.mem: export`).

#### Returning values from a region { ... } block

A `region { ... }` block acts as a first-class expression and evaluates to the value of its trailing expression:

```yukino
// Returning a string from a region block:
set s = region {
    set a = "hello "
    set b = "world"
    a + b // block tail expression
}
// Memory for 'a' and 'b' is reclaimed; 's' is safely preserved in the parent function's region

// Returning an array from a region block:
set arr = region {
    set a: string[] = []
    a.push("alpha")
    a.push("beta")
    a
}
```

#### Accumulators and the export(val) function

When populating an outer accumulator within a regional loop, created elements must survive the iteration's memory rollback. The `export(val)` function (or `.export()` method call) copies the value from the current temporary region into the parent region:

```yukino
load core.mem: export

set matched: string[] = []

for region i in 0..1000 {
    set item = "payload_${i}"
    if i % 2 == 0 {
        // export preserves the string in the parent accumulator region:
        matched.push(export(item))
        // or using method syntax:
        // matched.push(item.export())
    }
    // all other strings in this iteration are instantly destroyed
}
```

#### Architecture and safety guarantees:
1. **Automatic scratch region:** When a `region` block returns a regional type (string, array, bytes) or invokes `export`, the compiler transparently sets up an isolated scratch region for temporary allocations.
2. **Deep Export for composite structures:** When exporting composite structures (e.g., `string[]`), the compiler recursively clones not only the slice descriptor buffer, but also all nested regional elements into the parent region.
3. **Reclaiming intermediate scratch memory:** After the returned or exported value has been copied to the outer context, the scratch region is destroyed in O(1), freeing all intermediate scratch allocations.
4. **Zero overhead for scalars:** When a `region` block returns a scalar value (`int`, `float`, `bool`) and does not use `export`, the compiler uses lightweight mark rollback (`save_mark` / `restore_mark`) without allocating an additional scratch region.

### 8.6 First-class arenas (class Arena, region(arena)) and custom region(capacity) blocks

For demanding system architectures (network servers, HTTP connection handlers, game engines, parsers, and compilers), the `core.mem` package provides the **first-class arena** `Arena` class and the parameterized block syntax `region(...)`.

#### First-class named arenas (class Arena)

`Arena` (`class Arena` from `core.mem`) is a class providing an independent memory arena with manual lifecycle management. Unlike the implicit local function region, an arena lives as long as desired and can be repeatedly reset and reused:

```yukino
load core.mem: Arena

// Create an arena with initial capacity (defaults to 64 KB if omitted):
set pool = init Arena(128 * 1024)
defer {
    pool.destroy() // guaranteed reclamation
}

// Inspect arena metrics:
set cap = pool.cap()
set used = pool.used()

// Route allocations into the arena:
region(pool) {
    set doc = "large payload data"
    set processed = doc + " [verified]"
    // processed and doc reside in pool memory
}

// Instant O(1) reset of the arena for reuse:
pool.reset()
```

#### Arena class methods:
- `Arena(capacity?: int64) -> Arena` — constructor (defaults to 64 KB if capacity is omitted).
- `pool.used() -> int64` — number of bytes currently allocated in the arena.
- `pool.cap() -> int64` — current total capacity of the arena chunks.
- `pool.reset() -> void` — instant O(1) allocation pointer reset without deallocating system chunks. Reuses allocated memory across iterations with zero syscall overhead.
- `pool.destroy() -> void` — releases all system chunks and pages owned by the arena.
- `pool.as_ptr() -> ptr<void>` — returns the raw pointer to the underlying arena descriptor.

#### Scoped custom capacity scratch block: region(capacity) { ... }

When a specific section of a function performs large batch operations, you can provide an initial capacity directly to `region(size)`:

```yukino
// Temporary scratch block with initial 1 MB capacity:
set result = region(1024 * 1024) {
    set huge_buffer = "bulk temporary calculation"
    huge_buffer // safely exported to the parent region
}
// 1 MB scratch region is destroyed here in O(1)
```

Upon exiting `region(capacity)`, the temporary region is guaranteed to be destroyed in O(1), and any regional return value is transparently exported to the parent region of the caller.

### 8.7 Long-lived objects (persistent)

Occasionally, data needs to outlive not just the current function, but the entire call stack — for example, passing an object to a background worker thread (`std.threads`), long-lived application configurations, or global caches.

The built-in function `persistent(val)` and method `val.persistent()` from the `core.mem` module (`load core.mem: persistent`) solve this by storing data in the root process-level application region (`_app_region`), which persists for the entire lifetime of the process:

```yukino
load core.mem: persistent

fun load_system_config() -> string {
    set cfg = "host=0.0.0.0;port=8080;workers=8"
    // Copies string to the process-level persistent region:
    return persistent(cfg)
    // Or via method syntax: return cfg.persistent()
}

fun worker_thread() {
    set data = load_system_config()
    // data is guaranteed valid even long after load_system_config frame has been destroyed
}
```

#### Key properties of persistent:
- Allocates into the global, thread-safe process `_app_region`.
- Deallocating local function regions, exiting scopes, or unwinding the stack does not affect `persistent` allocations.
- Ideal for sharing data across threads without Use-After-Free risks or garbage collection pauses.

---

## 9. Cheat sheet: what to choose in practice

| Situation | What to use | Example signature / code |
| :--- | :--- | :--- |
| Need to pass a simple number, flag, byte, or string | Scalar / string by value (Copy) | `fun calc(x: int32, s: string)` |
| Just need to read fields of a struct without copying | An immutable reference `ref` | `fun inspect(ref user: User)` |
| Need to update/modify an existing object in place | A mutable reference `ref ~` | `fun update(ref ~user: User, ref ~buf: bytes)` |
| The function must take the object and become its sole owner | Ownership transfer `T` (Move) | `fun register_connection(conn: Connection)` |
| Many temporary strings are created in a loop inside the function | Regional loop `for region` / `while region` | `for region i in 0..list.len() { set s = list[i] + "\n" }` |
| Need to return a result from a region or populate an outer accumulator | Return from `region { ... }` or `export(val)` / `.export()` | `set s = region { a + b }` / `matched.push(export(item))` |
| Isolated pool for requests, sessions, or game engine frames | First-class `Arena` and block `region(arena)` | `set p = init Arena(); defer { p.destroy() }; region(p) { ... }` |
| Large block of scratch allocations with custom initial buffer | Scoped block `region(capacity) { ... }` | `set res = region(65536) { build_temp() }` |
| Object must outlive call stack or be sent to background thread | Function `persistent(val)` or method `val.persistent()` | `return persistent(config)` / `return data.persistent()` |
| Need to profile region memory consumption or reset the frame | Functions `region_used`, `region_cap`, `region_reset` | `set used = region_used()` |
| The function performs intensive batch allocations | The `#[region::capacity(N)]` attribute | `#[region::capacity(1048576)] fun worker()` |
| Need to eliminate local region prologue while mutating external state | The `#[no_region]` attribute | `#[no_region] fun scale(ref ~arr: int32[])` |
| Need to close a file, free a socket, or release a lock | `defer { ... }` | `defer { unlock(mutex) }` |

---

[← Back: Functions and Closures](5-functions-closures.md) | [Next: Structures, Classes, and Interfaces →](7-oop.md)