# Basic Syntax, Operators, and Control Flow

In Yukino, the syntax is designed so that state mutations, function calls, and control flow are maximally explicit and deterministic.

---

## Contents

1. [Variables: set and explicit rewrite mutation](#1-variables-set-and-explicit-rewrite-mutation)  
   1.1 [Variable declaration](#11-variable-declaration)  
   1.2 [The rewrite mutation rule](#12-the-rewrite-mutation-rule)  
   1.3 [Tuple destructuring](#13-tuple-destructuring)
2. [Immutable constants: static](#2-immutable-constants-static)
3. [Operators, arithmetic, and precedence](#3-operators-arithmetic-and-precedence)  
   3.1 [Arithmetic operators](#31-arithmetic-operators)  
   3.2 [Bitwise operators](#32-bitwise-operators)  
   3.3 [Comparison and type checking operators](#33-comparison-and-type-checking-operators)  
   3.4 [Logical operators](#34-logical-operators)  
   3.5 [Compound mutation operators (rewrite)](#35-compound-mutation-operators-rewrite)  
   3.6 [Memory and null operators](#36-memory-and-null-operators)  
   3.7 [Precedence table (Binding Power)](#37-precedence-table-binding-power)
4. [Control flow constructs](#4-control-flow-constructs)  
   4.1 [Branching: if, elif, else, and the ternary operator](#41-branching-if-elif-else-and-the-ternary-operator)  
   4.2 [Loops: while, loop, for, and labels](#42-loops-while-loop-for-and-labels)  
   4.3 [The space stub](#43-the-space-stub)

---

## 1. Variables: set and explicit rewrite mutation

### 1.1 Variable declaration

All local variables are declared with the `set` keyword. The type is inferred automatically or specified explicitly after a colon:

```yukino
// Automatic type inference
set count = 0                // int64
set ratio = 3.14             // float64
set active = true            // bool
set title = "Yukino"         // string

// Explicit type specification
set buffer_size: int64 = 4096
set scale: float32 = 1.5
```

An integer literal without a suffix is inferred as `int64` by default, and a fractional one as `float64`. A literal with a `u` suffix (`u8`..`u64`) becomes unsigned and is inferred as `uint64`; the `i8`..`i64` and `f32`/`f64` suffixes are recognized by the compiler but do not affect the default inferred type. With an explicit type annotation, the literal is coerced to it if the value fits into the type's range.

### 1.2 The rewrite mutation rule

> [!IMPORTANT]
> In Yukino, it is **forbidden** to modify variables, fields, or collection elements with the plain `x = 10` syntax. This results in a syntax error from the compiler (E0201: expected expression).

Any state modification must begin with the `rewrite` keyword:

```yukino
set balance = 100

// COMPILATION ERROR:
// balance = 200

// CORRECT:
rewrite balance = 200
rewrite balance += 50
rewrite balance -= 10

// Mutating array elements and struct fields:
set numbers = [1, 2, 3]
rewrite numbers[0] = 10

// User is a struct with the fields name and score
set user = User { name: "Alice", score: 42 }
rewrite user.score += 1
```

The `rewrite` keyword makes the places where side effects occur obvious during code audits.

### 1.3 Tuple destructuring

Yukino supports unpacking tuples directly at declaration with `set`:

```yukino
set coords = (10, 20)
set (x, y) = coords
```

### 1.4 Multiple statements on one line

Using `;`, you can place several statements on one line:

```
set a = 1; set b = 2
```

---

## 2. Immutable constants: static

`static` is used for global immutable values at the module or block level. A module-level initializer with a literal value is computed at compile time and placed into the constants section; a local `static` inside a function is initialized at runtime but is equally immutable:

```yukino
// Module-level constants
static MAX_CONNECTIONS: int32 = 1000
static APP_PREFIX: string = "app_v1"

fun get_limit() -> int32 {
    // Local constant in a function body
    static LOCAL_TIMEOUT: int64 = 5000
    return MAX_CONNECTIONS
}
```

An attempt to apply `rewrite` to a `static` identifier is rejected by the compiler at the type checking stage (error E0503: cannot reassign static constant).

---

## 3. Operators, arithmetic, and precedence

Yukino has strict static typing of expressions: **implicit coercions between numeric types are forbidden**. You cannot add `int32` and `int64` or `float32` and `float64` without an explicit `cast<T>(...)` call.

### 3.1 Arithmetic operators

| Operator | Name | Example | Description |
| :--- | :--- | :--- | :--- |
| `+` | Addition / concatenation | `a + b` | Adds numbers of the same type or concatenates strings (`string + string`) |
| `-` | Subtraction | `a - b` | Subtracts numbers |
| `*` | Multiplication | `a * b` | Multiplies numbers |
| `/` | Division | `a / b` | Integer division truncating toward zero (`sdiv`/`udiv`) or FP division |
| `%` | Remainder | `a % b` | Remainder computation (`srem`/`urem`) |
| `**` | Exponentiation | `a ** b` | Right-associative exponentiation of integers and floating-point numbers |
| `-` (unary) | Sign negation | `-x` | Changes the sign of a number |

Usage example:

```yukino
set sum = 10 + 20            // 30
set diff = 50 - 15           // 35
set prod = 6 * 7             // 42
set quot = 20 / 3            // 6 (integer division)
set rem = 20 % 3             // 2
set pwr = 2 ** 4             // 16
set neg = -pwr               // -16
```

### 3.2 Bitwise operators

Bitwise operators apply only to integer types (`int8..int64`, `uint8..uint64`). Both operands must be of the same type.

| Operator | Name | Example | Description |
| :--- | :--- | :--- | :--- |
| `&` | Bitwise AND | `a & b` | Bitwise logical multiplication |
| `\|` | Bitwise OR | `a \| b` | Bitwise logical addition |
| `^` | Bitwise XOR (exclusive OR) | `a ^ b` | Bitwise addition modulo 2 |
| `~` (unary) | Bitwise NOT | `~x` | Inverts all bits of a number |
| `<<` | Left shift | `a << b` | Shifts bits left, filling with zeros |
| `>>` | Right shift | `a >> b` | Arithmetic (for signed) or logical (for unsigned) right shift |

Example:

```yukino
set mask = 0xFF & 0x0F       // 0x0F (15)
set flags = 1 | 2            // 3
set toggled = flags ^ 1      // 2
set inverted = ~0            // All bits set to 1 (-1)
set shifted_left = 1 << 4    // 16
set shifted_right = 32 >> 2  // 8
```

### 3.3 Comparison and type checking operators

All comparison operators return a `bool` value:

| Operator | Name | Description |
| :--- | :--- | :--- |
| `==` | Equality | Compares primitive values, pointers, or nullable with `null` |
| `!=` | Inequality | Checks inequality of primitives or nullable with `null` |
| `<` | Less than | Numeric comparison |
| `<=` | Less than or equal | Numeric comparison |
| `>` | Greater than | Numeric comparison |
| `>=` | Greater than or equal | Numeric comparison |
| `is` | Type check | Checks whether a value belongs to a tagged union |

Example:

```yukino
set is_equal = (10 == 10)    // true
set is_less = (5 < 10)       // true

set value: int32 | string = 42
if value is int32 {
    // Branch taken when the value is an int32
    space
}
```

### 3.4 Logical operators

The `&&`, `||`, and `!` operators work with `bool` values and can be used in any expression, including the conditions of control flow constructs (`if`, `elif`, `while`):

| Operator | Name | Example | Description |
| :--- | :--- | :--- | :--- |
| `&&` | Logical AND | `cond1 && cond2` | True if both operands are true |
| `\|\|` | Logical OR | `cond1 \|\| cond2` | True if at least one operand is true |
| `!` (unary) | Logical NOT | `!cond` | Inverts a boolean value |

```yukino
set ready = true
set error_count = 0

// Idiomatic syntax without extra parentheses:
if ready && error_count == 0 {
    space
} elif !ready || error_count > 5 {
    space
}

// Parentheses are allowed when you explicitly need to change the evaluation order:
if (ready || error_count == 0) && timeout == 0 {
    space
}
```

> [!NOTE]
> In Yukino, comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`) have higher precedence (Binding Power `12`–`15`) than the logical operators `&&` (`10`) and `||` (`8`). Therefore, expressions like `ready && error_count == 0` are parsed unambiguously by the compiler and do not require wrapping comparisons in parentheses `(error_count == 0)`. Parentheses are used only to change the natural operator precedence.

### 3.5 Compound mutation operators (rewrite)

Compound operators combine computation and assignment and are **always** prefixed with the `rewrite` keyword:

```yukino
set x = 10

rewrite x += 5               // x = x + 5 (15)
rewrite x -= 3               // x = x - 3 (12)
rewrite x *= 2               // x = x * 2 (24)
rewrite x /= 4               // x = x / 4 (6)
rewrite x %= 4               // x = x % 4 (2)
rewrite x **= 3              // x = x ** 3 (8)

rewrite x &= 7               // Bitwise AND with assignment
rewrite x |= 8               // Bitwise OR with assignment
rewrite x ^= 3               // Bitwise XOR with assignment
rewrite x <<= 2              // Bitwise left shift with assignment
rewrite x >>= 1              // Bitwise right shift with assignment
```

### 3.6 Memory and null operators

- `??` (Null Coalescing): safely extracting a value from `T?` with a fallback:
  ```yukino
  set name: string? = null
  set safe_name = name ?? "anonymous" // If name == null, returns "anonymous"
  ```
- `?.` (Safe Call): safe navigation through fields of nullable structs and classes:
  ```yukino
  set email: string? = user?.profile?.email
  ```
- `ref expr` / `ref ~expr`: taking an immutable or mutable reference when passing function arguments:
  ```yukino
  inspect(ref my_struct)
  mutate(ref ~my_struct)
  ```

> [!NOTE]
> Detailed coverage of nullable types, null safety, and the `??` and `?.` operators is provided in [4-types.md](4-types.md). The full memory management model, borrowing with `ref`/`ref ~`, and affine ownership are covered in detail in [6-memory.md](6-memory.md).

### 3.7 Precedence table (Binding Power)

Yukino uses the Pratt expression parsing algorithm. The precedence of each operator is determined by a numeric Binding Power. For binary infix operators, the binding power is given as a pair `(Left BP, Right BP)`:
- The higher the Binding Power value, the more strongly the operator binds neighboring operands.
- **Left-associative operators**: satisfy the rule `Left BP < Right BP`. In a chain of identical operators (`a + b + c`), the left operation binds first: `(a + b) + c`.
- **Right-associative operators**: satisfy the rule `Left BP > Right BP`. In a chain of identical operators (`2 ** 3 ** 2`), the right operation binds first: `2 ** (3 ** 2)`.

Below is the complete table of all Yukino language operators in descending order of precedence (rank 1 is the highest precedence):

| Rank | Operators | Binding Power | Associativity | Description |
| :---: | :--- | :---: | :--- | :--- |
| **1** | `()` `[]` `.` `?.` | `34` (Postfix) | Left to right (Left) | Function call, indexing, field access, safe navigation |
| **2** | `-` `!` `~` `ref` `ref ~` `...` | `32` (Prefix) | Right to left (Prefix) | Unary minus, logical NOT, bitwise NOT, reference taking, spread |
| **3** | `**` | `(31, 30)` | **Right to left (Right)** | Exponentiation (`2 ** 3 ** 2 == 2 ** 9`) |
| **4** | `*` `/` `%` | `(28, 29)` | Left to right (Left) | Multiplication, division, remainder |
| **5** | `+` `-` | `(26, 27)` | Left to right (Left) | Addition (and string concatenation), subtraction |
| **6** | `<<` `>>` | `(24, 25)` | Left to right (Left) | Bitwise left and right shifts |
| **7** | `..` `..=` | `(22, 23)` | Left to right (Left) | Range construction |
| **8** | `&` | `(20, 21)` | Left to right (Left) | Bitwise AND |
| **9** | `^` | `(18, 19)` | Left to right (Left) | Bitwise XOR (exclusive OR) |
| **10** | `\|` | `(16, 17)` | Left to right (Left) | Bitwise OR |
| **11** | `<` `<=` `>` `>=` | `(14, 15)` | Left to right (Left) | Relational comparisons |
| **12** | `==` `!=` `is` | `(12, 13)` | Left to right (Left) | Equality, inequality, type check (`expr is Type`) |
| **13** | `&&` | `(10, 11)` | Left to right (Left) | Logical AND |
| **14** | `\|\|` | `(8, 9)` | Left to right (Left) | Logical OR |
| **15** | `??` | `(6, 7)` | Left to right (Left) | Null coalescing |
| **16** | `\|>` | `(4, 5)` | Left to right (Left) | Pipeline call (pipe operator, `x \|> f` is equivalent to `f(x)`) |
| **17** | `? :` | `(3, 2)` | **Right to left (Right)** | Ternary conditional operator (`cond ? then_val : else_val`) |

---

## 4. Control flow constructs

### 4.1 Branching: if, elif, else, and the ternary operator

Conditions in `if` do not require parentheses. For intermediate checks, the `elif` keyword is used:

```yukino
set score = 85

if score >= 90 {
    // Branch taken when score >= 90
} elif score >= 75 {
    // Branch taken when 75 <= score < 90
} else {
    // Default branch
}

// Ternary operator for compact conditional expressions:
set status = score >= 50 ? "passed" : "failed"
```

### 4.2 Loops: while, loop, for, and labels

Yukino provides three main loop constructs: `while`, `loop`, and `for`. Any loop can be terminated with the `break` statement or advanced to the next iteration with the `continue` statement.

#### 1. Precondition loop: while

A `while` loop runs as long as a boolean condition is true:

```yukino
set i = 0
while i < 5 {
    rewrite i += 1
}
```

#### 2. Infinite loop: loop

The `loop` construct is optimized by the compiler for an infinite loop (in LLVM it generates an unconditional jump `br label %loop.body` without redundant condition checks at entry). It is terminated with `break`, `return`, or `panic`:

```yukino
set attempts = 0
loop {
    rewrite attempts += 1
    if attempts >= 3 {
        break
    }
}
```

#### 3. Iteration loop: for

The `for` loop is used for iterating over numeric ranges. Iterating over arrays and other collections is not supported in the current compiler implementation (compilation error: unsupported iterator in for loop):

- **Half-open range `start..end`**: the range `[start, end)`, excluding the final value `end`:
  ```yukino
  // Prints the numbers from 0 to 4
  for i in 0..5 {
      print("${i} ")
  }
  ```

- **Inclusive range `start..=end`**: the range `[start, end]`, including the final value `end`:
  ```yukino
  // Prints the numbers from 0 to 5
  for i in 0..=5 {
      print("${i} ")
  }
  ```

The loop variable of a range iteration has the type `int64`.

#### 4. Named loop labels

With deeply nested loops, the `break` and `continue` statements apply to the innermost loop by default. Labels are used to break out of an outer loop or jump to its next iteration.

The language supports three label declaration syntax variants:
1. **Prefix label** (recommended style): `outer: while ...`, `outer: for ...`, `outer: loop { ... }`
2. **Prefix label with colon**: `:outer while ...`, `:outer for ...`, `:outer loop { ... }`
3. **Inline label**: `while outer: ...`, `for outer: ...`, `loop outer: { ... }`

##### Example of exiting the outer loop (break label):

```yukino
outer: for row in 0..10 {
    for col in 0..10 {
        if row * col > 50 {
            // Immediate exit from both loops (including the outer one)
            break outer
        }
    }
}
```

##### Example of jumping to the next iteration of the outer loop (continue label):

```yukino
outer: while true {
    while true {
        // Jump to the next iteration of the outer loop
        continue outer
    }
}
```

##### Label semantics and resource cleanup (defer LIFO)

> [!IMPORTANT]
> If `break label` or `continue label` takes control flow out of one or more nested blocks containing deferred `defer` calls, the compiler guarantees that all intermediate `defer` calls are invoked in strict LIFO order (from inner to outer) before jumping to the target LLVM block.

```yukino
outer: loop {
    defer print("cleanup outer\n")
    inner: loop {
        defer print("cleanup inner\n")
        break outer
    }
}
// Program output:
// cleanup inner
// cleanup outer
```

### 4.3 The space stub

If a block or function body is intentionally left empty (for example, an interface stub or an ignored event), the compiler generates a warning. To explicitly confirm the absence of actions, the `space` keyword is used:

```yukino
fun on_tick() {
    space // Explicit no-op, suppresses the empty body warning
}

if should_ignore {
    space
} else {
    process()
}
```

---

[← Back: Toolchain and Project Structure](2-cli-project.md) | [Next: Type System and Pattern Matching →](4-types.md)
