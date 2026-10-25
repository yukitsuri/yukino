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
8. [Cheat sheet: what to choose in practice](#8-cheat-sheet-what-to-choose-in-practice)

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

## 8. Cheat sheet: what to choose in practice

| Situation | What to use | Example signature |
| :--- | :--- | :--- |
| Need to pass a simple number, flag, byte, or string | Scalar / string by value (Copy) | `fun calc(x: int32, s: string)` |
| Just need to read fields of a struct without copying | An immutable reference `ref` | `fun inspect(ref user: User)` |
| Need to update/modify an existing object in place | A mutable reference `ref ~` | `fun update(ref ~user: User, ref ~buf: bytes)` |
| The function must take the object and become its sole owner | Ownership transfer `T` (Move) | `fun register_connection(conn: Connection)` |
| Many temporary strings are created in a loop inside the function | A `region { ... }` block in the loop body | `for i in 0..list.len() { region { set s = list[i] + "\n" } }` |
| A large block of intermediate allocations whose memory is needed right now | A `region { ... }` block | `region { set big_tree = build_tree() }` |
| Need to close a file, free a socket, or release a lock | `defer { ... }` | `defer { unlock(mutex) }` |

---

[← Back: Functions and Closures](5-functions-closures.md) | [Next: Structures, Classes, and Interfaces →](7-oop.md)