# Type System and Pattern Matching

Yukino uses a strict static type system with bidirectional type inference, compile-time `null` control, and powerful pattern matching.

## Contents

1. [Primitive data types](#1-primitive-data-types)
2. [Compound types](#2-compound-types)  
   2.1 [Tuples](#21-tuples)  
   2.2 [Arrays](#22-arrays)  
   2.3 [Maps (map)](#23-maps-map)
3. [Optional types (T?) and null safety](#3-optional-types-t-and-null-safety)  
   3.1 [The null problem and Yukino's safety philosophy](#31-the-null-problem-and-yukinos-safety-philosophy)  
   3.2 [Optional type syntax T?](#32-optional-type-syntax-t)  
   3.3 [Returning T? from functions](#33-returning-t-from-functions)  
   3.4 [Why you cannot simply assign T? to T](#34-why-you-cannot-simply-assign-t-to-t)  
   3.5 [Ways to extract a value (??, ?., checks)](#35-ways-to-extract-a-value--checks)  
   3.6 [Automatic widening of T to T?](#36-automatic-widening-of-t-to-t)
4. [Type unions and the is operator](#4-type-unions-and-the-is-operator)
5. [Type aliases (type)](#5-type-aliases-type)
6. [Pattern matching: switch](#6-pattern-matching-switch)
7. [Generic types](#7-generic-types)

---

## 1. Primitive data types

The compiler strictly separates type sizes and signedness. Implicit widening or narrowing is not allowed (use the `cast` intrinsic for explicit conversion):

| Category | Types | Note |
| :--- | :--- | :--- |
| **Signed integers** | `int8`, `int16`, `int32`, `int64` | Integer literals without a suffix are inferred as `int64` by default |
| **Unsigned integers** | `uint8`, `uint16`, `uint32`, `uint64` | `uint8` also represents a byte. Literals with a `u` suffix (`u8`..`u64`) are unsigned and inferred as `uint64` |
| **Floating-point** | `float32`, `float64` | IEEE 754. Fractional literals are `float64` by default |
| **Boolean** | `bool` | The literals `true` and `false` |
| **Character** | `char` | 32-bit Unicode scalar code point (for example, `'A'`, `'ゆ'`) |
| **String** | `string` | Immutable 16-byte byte-slice descriptor `{ ptr, len }` (Copy type) |
| **Utility** | `void`, `never`, `null` | `void` — emptiness; `never` — never returns (e.g. `exit()`); `null` — absence of a value |

A numeric literal is coerced to the annotated type if the value fits into its range: `set small: int8 = 100` is allowed, while `set big: int8 = 300` is a type mismatch error. The `i8`..`i64` and `f32`/`f64` suffixes are recognized by the compiler but do not affect the default inferred type.

Strings (`string`) are immutable sequences of bytes wrapped in a 16-byte descriptor `{ ptr, len }`. They are **Copy types**: assigning or passing a string as an argument copies only the descriptor, leaving both variables valid. Passing strings by reference `ref` is not required for reading (see details in the [Strings and Text](8-strings.md) chapter).

---

## 2. Compound types

### 2.1 Tuples

Group a fixed number of heterogeneous values:

```yukino
set pair: (string, int32) = ("port", 8080)

// Access by positional index .0, .1 (explicit type annotation is recommended)
set key: string = pair.0
set port: int32 = pair.1

// Tuple destructuring
set (name, code) = pair
```

### 2.2 Arrays

Arrays come in dynamic `T[]` and fixed `T[N]` forms:

```yukino
// Dynamic array
set list: int32[] = [10, 20, 30]

// Fixed array with an explicit list of values
set buffer: uint8[4] = [0, 0, 0, 0]

// Allocating an array of a given length without a literal (memory is automatically zeroed):
set ints: int32[100]

// Indexing
set first = list[0]
```

### 2.3 Maps (map)

Associative key-value arrays:

```yukino
set user_ages: map<string, int32> = {
    "Alice": 28,
    "Bob": 34
}

set alice_age = user_ages["Alice"]
```

---

## 3. Optional types (T?) and null safety

### 3.1 The null problem and Yukino's safety philosophy

The inventor of `null`, Sir Tony Hoare, called his own invention the "billion-dollar mistake". In most classic languages (C, Java, C#, Python, JavaScript), any reference to an object or string can unexpectedly turn out to be empty (`null` or `None`). If the programmer forgot to manually check the value before use, the program crashes directly for the user:
- in Java — the notorious `NullPointerException`;
- in C and C++ — a fatal `Segmentation fault`;
- in Python — `AttributeError: 'NoneType' object has no attribute ...`.

Yukino solves this problem at the root: **in Yukino, all data types are by default guaranteed to NEVER be null**!

If a variable or field is declared as `string`, `int32`, or `User`, the compiler gives a 100% static guarantee that it always holds a real, valid value:

```yukino
// COMPILATION ERROR: E0401 (expected `string`, found `null`)
// set name: string = null

// COMPILATION ERROR: E0401 (expected `int32`, found `null`)
// set age: int32 = null
```

If the program compiled, you can be absolutely sure: accessing the fields and methods of such variables will never crash due to `null`.

### 3.2 Optional type syntax T?

But in real programs, a value may be absent:
- the user did not provide a phone number during registration;
- no record with the specified ID was found in the database;
- the configuration file does not contain an optional parameter.

To explicitly express that a variable may either hold a value or be empty, a question mark `?` is appended to the type name:

```yukino
set user_phone: string? = null // A string? variable can hold null
set score: int32? = 100        // Or a real integer
```

Semantically, `T?` is the type union `T | null`.

### 3.3 Returning T? from functions

The most common and correct use case for optional types is returning a result from lookup or data retrieval functions:

```yukino
struct User {
    id: int32
    name: string
    email: string? // email may also be missing
}

fun find_user_by_id(id: int32) -> User? {
    if id == 42 {
        return User {
            id: 42,
            name: "Alice",
            email: "alice@example.com",
        }
    }
    // If there is no user with this id in the database:
    return null
}
```

The `-> User?` signature honestly and directly warns anyone calling this function: *"Be prepared that the user may not exist, and the function will return null"*.

### 3.4 Why you cannot simply assign T? to T

What happens if we try to store the result of `find_user_by_id` in an ordinary (non-nullable) variable?

```yukino
set maybe_user: User? = find_user_by_id(10)

// COMPILATION ERROR: E0401: expected `User`, found `User?`
// set user: User = maybe_user
```

The compiler will categorically refuse to build such a program! And this is a deliberate decision: if the compiler allowed such an assignment and the variable turned out to hold `null`, any subsequent attempt to access `user.name` would crash.

Yukino forces the developer to **explicitly decide what to do if the value is absent**.

### 3.5 Ways to extract a value (??, ?., checks)

The language provides four convenient and safe ways to work with `T?` values:

#### Way 1: The null-coalescing operator (?? — Null-Coalescing)
Perfect when a reasonable default can be substituted for a missing value. The `??` operator unwraps `T?`: if there is a value, it takes it; if it holds `null`, it substitutes the value on the right:

```yukino
set maybe_id: int64? = null

// If maybe_id is null, 0 is taken.
// The result safe_id is GUARANTEED to have the non-nullable type int64:
set safe_id: int64 = maybe_id ?? 0

set raw_nickname: string? = null
// display_name is guaranteed to have the non-nullable type string:
set display_name: string = raw_nickname ?? "Anonymous guest"
```

#### Way 2: Safe navigation (?.)
Prevents crashes when accessing fields or methods of nullable objects. If the left side of the expression is `null`, the compiler does not panic but simply returns `null` for the whole chain:

```yukino
struct Profile {
    avatar_url: string
}

struct Account {
    profile: Profile?
}

set account: Account? = null

// If account == null or profile == null, null is returned without a runtime error:
set avatar: string? = account?.profile?.avatar_url
```

You can combine `?.` and `??` into elegant safe expressions:

```yukino
// If account == null or profile == null, the default picture is substituted:
set safe_avatar: string = account?.profile?.avatar_url ?? "/default_avatar.png"
```

#### Way 3: Explicit equality check `!= null`
If a default value makes no sense and missing data is an error, a check is performed via a condition:

```yukino
struct NotFoundError {
    message: string
    code: int32
}

set user: User? = find_user_by_id(10)

if user == null {
    panic(NotFoundError {
        message: "user with this ID not found",
        code: 404,
    })
}
```

#### Way 4: Check with the `is` operator
The `is` operator checks type membership:

```yukino
set val: int32? = 42

if val is null {
    println("Value is missing")
} else {
    println("Value is present")
}
```

### 3.6 Automatic widening of T to T?

In the other direction (from non-nullable to nullable), assignment is absolutely safe and is performed by the compiler automatically:

```yukino
set real_number: int32 = 42

// A non-nullable int32 value freely fits into the int32 | null union:
set optional_number: int32? = real_number // OK: requires no operators or casts
```

---

## 4. Type unions and the is operator

The `T1 | T2` syntax allows declaring variables that can hold one of several allowed types:

```yukino
set payload: int32 | bool = 42

// Type check with the is operator
if payload is int32 {
    // payload is an int32
}
```

The `is` operator is also used to check for equality with `null` (`val is null`) and for dynamic checks of classes and interfaces under inheritance (`obj is MyInterface`).

---

## 5. Type aliases (type)

The `type` keyword allows declaring named aliases for data types:

```yukino
type UserId = int64
type ResponsePair = (int32, string)
type ResultHandler = fun(int32) -> void
```

---

## 6. Pattern matching: switch

The `switch` construct matches a value against patterns. Unlike some other languages, there is **no** `case` keyword here, and the default branch uses `else ->`:

```yukino
set code = 200

switch code {
    200 -> {
        // Success
    }
    201 | 204 -> {
        // Created or empty response
    }
    404 -> {
        // Not found
    }
    else -> {
        // All other cases
    }
}
```

### 6.1 Variable binding and guard expressions

A pattern can bind a variable and impose a condition via `if`:

```yukino
set score = 85

switch score {
    100 -> {
        // Perfect result
    }
    s if s >= 80 -> {
        // Excellent, s is available inside the block
    }
    s if s >= 50 -> {
        // Satisfactory
    }
    else -> {
        // Failure
    }
}
```

---

## 7. Generic types

Yukino supports generic type syntax (parametric polymorphism) for classes, interfaces, and collections, allowing you to create reusable data structures with compile-time type checking.

Type parameters are listed in angle brackets `<...>` after the type name:

```yukino
// Declaring a generic class with parameters K and V
public class HashMap<K, V> {
    handle: ptr<uint8>

    public fun init() {
        rewrite this.handle = yukistd_map_new()
    }

    public fun set(key: K, val: V) {
        // implementation of the store operation
    }

    public fun get(key: K) -> V? {
        // implementation of the value lookup
    }
}
```

### 7.1 Instantiation and usage

When creating an instance of a generic type, the concrete type arguments are specified explicitly:

```yukino
// Creating a map instance with string key and value types
set map = init HashMap<string, string>()
map.set("HOST", "127.0.0.1")
set host = map.get("HOST")
```

Generic types can be used as variable types, function arguments, and return values:

```yukino
public fun all() -> HashMap<string, string> {
    set map = init HashMap<string, string>()
    // filling with data...
    return map
}
```

### 7.2 Integration with the memory system and regions

Instances of generic classes follow the general rules of Yukino's region-based memory system. If a function returns an instance of a generic class, the compiler applies the destination-passing style mechanism, guaranteeing that the returned object is allocated in the caller's region without premature data destruction when the function exits.

---

[← Back: Basic Syntax, Operators, and Control Flow](3-basics.md) | [Next: Functions and Closures →](5-functions-closures.md)
