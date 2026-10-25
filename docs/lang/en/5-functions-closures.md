# Functions and Closures

Functions with the C ABI convention (`cfun`) are first-class entities: they can be assigned to variables, passed as arguments, and called indirectly. The language also recognizes anonymous functions and closures (section 2).

## Contents

1. [Function declarations](#1-function-declarations)  
   1.1 [Basic syntax](#11-basic-syntax)  
   1.2 [Parameters with default values](#12-parameters-with-default-values)  
   1.3 [Argument passing modes](#13-argument-passing-modes)  
   1.4 [Named arguments](#14-named-arguments)  
   1.5 [Variadic parameters (variadics) and the spread operator](#15-variadic-parameters-variadics-and-the-spread-operator)
2. [Anonymous functions and closures (closures)](#2-anonymous-functions-and-closures-closures)  
   2.1 [Arrow syntax](#21-arrow-syntax)  
   2.2 [Lexical environment capture](#22-lexical-environment-capture)  
   2.3 [Internals: the fat pointer](#23-internals-the-fat-pointer)
3. [Function types and function pointers](#3-function-types-and-function-pointers)  
4. [Function name mangling](#4-function-name-mangling)

---

## 1. Function declarations

### 1.1 Basic syntax

Functions are declared with the `fun` keyword. The return type is specified with the arrow `->`. If no type is specified, the function returns `void`:

```yukino
// Function with an explicit return type
fun multiply(a: int32, b: int32) -> int32 {
    return a * b
}

// Procedure without a return value (void)
fun log_event(message: string) {
    // Internal operations
}
```

If the function body temporarily contains no implementation, use the `space` keyword to avoid a compiler warning:

```yukino
fun placeholder_handler() {
    space
}
```

### 1.2 Parameters with default values

Parameters can have default values that the compiler substitutes at the call desugaring stage:

```yukino
fun connect(host: string, port: int32 = 8080, secure: bool = true) -> bool {
    // Connection logic
    return true
}

// Valid call variants:
set r1 = connect("127.0.0.1")                    // port=8080, secure=true
set r2 = connect("127.0.0.1", 9000)              // secure=true
set r3 = connect("127.0.0.1", 9000, false)       // all parameters passed explicitly
```

### 1.3 Argument passing modes

- **By value**: elementary Copy types are duplicated; Affine types transfer ownership.
- **By immutable reference (`ref param: T`)**: for efficiently passing heavy structures without copying and without transferring ownership (at the call site: `ref arg`).
- **By mutable reference (`ref ~param: T`)**: for modifying the state of the passed object in place (at the call site: `ref ~arg`).

```yukino
struct Matrix {
    data: float64[]
}

fun inspect(ref m: Matrix) {
    // Read only
}

fun reset(ref ~m: Matrix) {
    // Modification
}
```

### 1.4 Named arguments

When calling functions, arguments can be passed by name using the `name = value` syntax. This allows passing arguments in any order and flexibly combining them with default parameters:

```yukino
fun configure(host: string, port: int32 = 8080, secure: bool = true) -> string {
    return "${host}:${port}"
}

// Named arguments in any order:
set c1 = configure(host = "localhost", secure = false)

// Combining positional and named arguments:
set c2 = configure("127.0.0.1", secure = true, port = 9000)
```

### 1.5 Variadic parameters (variadics) and the spread operator

A function can accept a variable number of arguments of the same type. To do this, an ellipsis `...` is placed before the name of the last parameter:

```yukino
fun sum(...numbers: int64) -> int64 {
    set total = 0
    for i in 0..numbers.len() {
        rewrite total += numbers[i]
    }
    return total
}
```

Rules and restrictions:
- Inside the function body, the variadic parameter is accessible as an array of the corresponding type (`numbers: int64[]`).
- The variadic parameter **must be the last** one in the function's parameter list.
- The variadic parameter **cannot** have a default value.
- Variadic parameters are forbidden for C ABI functions (`cfun`).

Calling a variadic function:

```yukino
// Passing an arbitrary number of arguments:
set s0 = sum()              // numbers = []
set s1 = sum(5)             // numbers = [5]
set s3 = sum(1, 2, 3)       // numbers = [1, 2, 3]

// Expanding an existing array with the spread operator (...):
set items = [10, 20, 30]
set s_all = sum(...items)   // numbers = [10, 20, 30]
```

---

## 2. Anonymous functions and closures (closures)

In line with Yukino's philosophy of explicitness and readability, anonymous functions and closures always start with the `fun` keyword.

### 2.1 Syntax

Yukino supports both a concise arrow form with an expression body `=>` and a full block form `{ ... }`:

```yukino
// Arrow syntax (expression body):
set add = fun(a: int32, b: int32) => a + b
set inc = fun(x: int32) => x + 1
set zero = fun() => 0

// Full block form with explicit return:
set mul = fun(x: int32) -> int32 {
    return x * 3
}

// Passing a closure as a function argument:
println("${apply(fun(x: int32) => x + 10, 5)}")
```

In the arrow form `fun(...) => expr`, the return type is automatically inferred from the expression. In the block form `fun(...) -> RetType { ... }`, the return type can be annotated explicitly or inferred from the `return` statement.

### 2.2 Lexical environment capture

A closure can reference variables from its enclosing lexical scope:

```yukino
set base: int32 = 100
set add_base = fun(x: int32) => base + x

println("${add_base(5)}") // 105
```

The code generator automatically detects free variables and constructs a compact environment layout (`environment`) storing the captured values. The environment structure is allocated in regional memory via bump allocation.

### 2.3 Internals: the fat pointer

In memory, a closure is represented by a fat pointer of two machine words:

```text
Closure: { fn: pointer to code, env: pointer to environment struct }
```

Invoking a closure is an indirect call via the code pointer using the `fastcall` calling convention; the environment pointer is passed as a hidden argument, and the closure body unpacks the captured values. If a closure captures no variables, the environment pointer is `null`. When a top-level named function is coerced to a closure type, the compiler automatically generates a thunk adapter.

---

## 3. Function types and function pointers

In Yukino, function types are declared with the `fun` and `cfun` keywords:

- `fun(T1, T2, ...) -> RetType` — a function type with the standard Yukino calling convention (`fastcall`).
- `cfun(T1, T2, ...) -> RetType` — a function pointer with the standard C ABI calling convention (`cdecl`).

### 3.1 Passing functions as values

Functions with the `cfun` convention can be assigned to variables and called indirectly:

```yukino
cfun add(a: int32, b: int32) -> int32 {
    return a + b
}

cfun multiply(a: int32, b: int32) -> int32 {
    return a * b
}

// Taking a function as an argument
fun apply_op(f: cfun(int32, int32) -> int32, x: int32, y: int32) -> int32 {
    return f(x, y)
}

// Passing the function
set op: cfun(int32, int32) -> int32 = add
set res = apply_op(op, 10, 20) // 30
```

---

## 4. Function name mangling

All functions declared with the `fun` keyword are automatically mangled by the compiler into deterministic symbols in LLVM IR of the form:

```text
__yukifun_{name}_{argument_types_joined_with_underscores}__{return_type}
```

If a function has no arguments, the separator is a triple underscore `___`:

```text
__yukifun_{name}___{return_type}
```

Examples:
- `fun add(a: int32, b: int32) -> int32` compiles into the symbol `@__yukifun_add_int32_int32__int32`
- `fun get_count() -> int64` compiles into the symbol `@__yukifun_get_count___int64`
- `fun run()` compiles into the symbol `@__yukifun_run___void`

Exceptions and control:
- The main entry point `fun main` is never mangled and is always exported as `@main`.
- To disable mangling for a Yukino function, the `#[!mangle]` attribute is used: such a function is assigned the exact source name as its symbol.
- Functions with the C ABI convention (`cfun`) are not mangled by default, ensuring direct compatibility with C. To enable mangling for a `cfun`, the `#[mangle]` attribute is used.

---

[← Back: Type System and Pattern Matching](4-types.md) | [Next: Memory Model, Ownership, and Regions →](6-memory.md)