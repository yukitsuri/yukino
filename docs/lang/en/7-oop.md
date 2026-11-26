# Structures, Classes, and Interfaces

Yukino provides two models of object data types: **structures (`struct`)** for high-performance value types without overhead, and **classes (`class`)** for polymorphic object-oriented programming with virtual tables (`vtable`).

## Contents

1. [Structures (struct) — value types](#1-structures-struct--value-types)  
   1.1 [Declaration and creation](#11-declaration-and-creation)  
2. [Classes (class) — reference types](#2-classes-class--reference-types)  
   2.1 [Declaration and constructors](#21-declaration-and-constructors)  
   2.2 [Locked classes: locked class](#22-locked-classes-locked-class)  
   2.3 [Inheritance and the super keyword](#23-inheritance-and-the-super-keyword)  
   2.4 [Reference parameters in methods](#24-reference-parameters-in-methods)  
   2.5 [The this keyword, ref this, and return this](#25-the-this-keyword-ref-this-and-return-this)  
   2.6 [Class fields: set, static, and default values](#26-class-fields-set-static-and-default-values)
3. [Properties: get and set accessors](#3-properties-get-and-set-accessors)
4. [Operator overloading](#4-operator-overloading)
5. [Interfaces (interface) and itable](#5-interfaces-interface-and-itable)
6. [Enumerations (enum)](#6-enumerations-enum)  
   6.1 [Declaration and variant types](#61-declaration-and-variant-types)  
   6.2 [Creating and using values](#62-creating-and-using-values)  
   6.3 [Pattern matching with switch](#63-pattern-matching-with-switch)  
   6.4 [Memory layout and comparison](#64-memory-layout-and-comparison)

---

## 1. Structures (struct) — value types

Structures in Yukino are lightweight value types with a direct in-memory representation (C-compatible layout). They are placed directly on the stack or embedded into the memory of enclosing structures, without object headers or vtables. Structures in Yukino are intended solely for storing and transferring data (including error structures in `panic`, function arguments, and return values).

### 1.1 Declaration and creation

```yukino
struct Point {
    x: float64,
    y: float64
}

// Creating an instance via a structure literal
set pt = Point { x: 10.0, y: 20.0 }

// Reading fields (specifying the type for explicitness is recommended)
set px: float64 = pt.x
set py: float64 = pt.y
```

#### Structure declaration and validation rules:

1. **Unique field names in declarations**: field names inside a structure must be unique. Declaring duplicate field names produces compiler error `E0304` (duplicate field).
2. **Structure type existence**: attempting to instantiate an undeclared structure produces compiler error `E0305` (unknown type/struct).
3. **Mandatory field initialization**: every declared field must be initialized in the structure literal. Omitting any field produces compiler error `E0401` (missing required field).
4. **No unknown fields**: specifying fields in the literal that were not declared in the structure produces compiler error `E0401` (unknown field).
5. **No duplicate fields in literals**: specifying the same field more than once in an initializer produces compiler error `E0401` (duplicate field in struct initializer).
6. **Field type checking**: the type of each expression provided for a field is strictly verified against the declared field type. A mismatch produces compiler error `E0401` (type mismatch) with an explanatory note.

> [!NOTE]
> In Yukino, structures (`struct`) are used solely for storing and transferring pure data without overhead (for example, typed errors in `panic(Error { ... })`, passing composite arguments, and returning multiple values). They do not contain methods, constructors, or behavioral logic. For an object-oriented model with methods, inheritance, and polymorphism, classes (`class`) are used.

---

## 2. Classes (class) — reference types

Classes are polymorphic entities managed by the regional memory model. They support constructors, single inheritance, method overriding, and dynamic dispatch through virtual method tables (vtable).

### 2.1 Declaration and constructors

Class instances are created with the `init` keyword: `init ClassName(...)`. Attempting to construct an instance as `ClassName(...)` without `init` produces compiler error `E0401`:

```yukino
class Animal {
    name: string

    fun init(name: string) {
        rewrite this.name = name
    }

    fun speak() -> string {
        return "..."
    }

    fun describe() -> string {
        return this.name
    }
}

// Creating an instance using init
set pet = init Animal("Barsik")
set voice = pet.speak()
```

#### Initialization and `init` constructor subtleties:

1. **Instantiation order**: `init ClassName(...)` first allocates memory for the object (in the active region or on the stack when elided), **deterministically zero-initializes all instance fields** (including inherited ones), stores the virtual method table (`vtable`) pointer, applies **literal default field values** (`field: T = <literal>`) from ancestors down to the subclass, and only then calls `init`.
2. **Default field values**: any regular class field may specify a literal default initializer, e.g. `port: int32 = 8080` or `set port: int32 = 8080` (the `set` keyword before a field is optional and equivalent to a bare field declaration). Only literals (including unary `-`/`!`/`~` on literals) are allowed as defaults; non-literal expressions produce `E0401`.
3. **Static class fields (`static`)**: `static FIELD: T = <literal>` declares an immutable class-level constant without per-instance memory overhead (emitted as a read-only global constant and respecting `inline_statics`). A literal initializer is **mandatory** for `static` fields. Reading a `static` field uses ordinary field access (`this.FIELD`, `super.FIELD`, `obj.FIELD`), except mutating it with `rewrite` is forbidden (`E0503`).
4. **Constructor inheritance and missing `init`**: if a subclass does not declare its own `init`, it automatically inherits the `init` constructor of the nearest ancestor that defines one. If no class in the hierarchy defines `init`, `init ClassName()` takes 0 arguments (passing arguments fails with `E0401`).
5. **Return value inside `init`**: `init` initializes an already-allocated `this` instance and defaults to a `void` return type. An empty `return` inside `init` is valid for early exit, whereas `return this` without an explicit `-> Type` causes type error `E0401` (`init ClassName(...)` returns the created instance automatically).
6. **Visibility**: use `public` to export fields and methods from a module (e.g., `public static MAX: int32 = 16`).

### 2.2 Locked classes: locked class

The `locked` modifier forbids inheriting from a class. Attempting to create a subclass results in a compilation error (`E0409`):

```yukino
locked class GuardService {
    active: bool

    fun init(active: bool) {
        rewrite this.active = active
    }
}

// ERROR: E0409 (cannot inherit from locked class)
// class CustomGuard extends GuardService {}
```

### 2.3 Inheritance and the super keyword

Classes extend a base class via `extends` (single inheritance is supported). Ancestor fields are laid out before the subclass's own fields. The `super` keyword has several distinct forms:

```yukino
class Dog extends Animal {
    breed: string

    fun init(name: string, breed: string) {
        // Static call to the base class constructor
        super(name)
        rewrite this.breed = breed
    }

    // Overriding a virtual method
    fun speak() -> string {
        // Direct call to ancestor implementation bypassing vtable, plus base field access
        return super.speak() + " Woof (" + super.name + ")"
    }

    fun as_animal() -> Animal {
        // Upcast view of the current instance this as Animal
        return super
    }
}

set dog = init Dog("Rex", "Collie")
set sound = dog.speak() // "... Woof (Rex)"
```

#### All forms of `super` and their exact semantics:

1. **`super(args...)` — calling the ancestor constructor**:
   - Statically calls the nearest ancestor's `init` method passing the current `this` pointer (no new object is allocated).
   - **No implicit call**: if a subclass declares its own `init`, the compiler does not invoke `super(...)` automatically. Without an explicit `super(...)`, inherited fields remain zero-initialized.
   - If no ancestor defines an `init` method, `super(...)` fails with error `E0416`.

2. **`super.method(args...)` — statically calling an ancestor method**:
   - Resolves the method in the nearest base class and calls it **directly, bypassing the `vtable`**, passing `this`. This lets an overriding method extend the parent implementation without infinite recursion. Fails with `E0416` if the method does not exist in an ancestor.

3. **`super.field` and `rewrite super.field = value` — accessing ancestor fields**:
   - Reads or writes an inherited field or accessor on the current `this` instance (`E0416` if the field is not found in the ancestor hierarchy).

4. **Standalone (empty) `super` without parentheses or a dot**:
   - Used as an expression (`set base = super`, `return super`, or passed as a function argument), standalone `super` evaluates to the **exact same `this` instance pointer**, typed as the immediate base class (an upcast view).
   - **Important subtlety**: bare `super` written as a standalone statement (without `()`) **does NOT call the ancestor constructor**! It parses as an expression statement whose value is discarded (a runtime no-op), even though the compiler still checks that it is inside a method of a class with a superclass (`E0416`). To call a zero-argument base constructor, always write parentheses: `super()`.

> [!NOTE]
> Using `super` outside a class method or in a class without a superclass (`extends`) always produces compiler error `E0416`.

### 2.4 Reference parameters in methods

Class methods accept `ref T` and `ref ~T` parameters with the same semantics as plain functions (see the "Memory, ownership and regions" chapter): `ref` gives shared read access, `ref ~` gives exclusive in-place write access. The reference is passed explicitly at the call site; ownership is not moved.

```yukino
struct Counter {
    value: int64
}

class Accumulator {
    step: int64

    fun init(step: int64) {
        rewrite this.step = step
    }

    fun add(ref ~c: Counter) {
        rewrite c.value = c.value + this.step
    }

    fun peek(ref c: Counter) -> int64 {
        return c.value
    }
}

set acc = init Accumulator(5)
set c = Counter { value: 10 }
acc.add(ref ~c)
set v: int64 = acc.peek(ref c) // 15
```

Through `ref ~` a method can modify not only its own fields but also an object owned by the caller; `ref` guarantees to the caller that the object stays unchanged.

### 2.5 The this keyword, ref this, and return this

Inside class methods and accessors, `this` is always available as a reference to the current instance with the static type of the enclosing class.

#### Implicit and explicit `this` in parameters:

- If `this` is omitted from the parameter list (`fun bump(step: int64)`), the compiler implicitly inserts it as the method receiver.
- The receiver can also be written explicitly: `fun bump(this, step: int64)` or as a shared borrow `fun peek(ref this) -> int64`.
- `this` itself cannot be rebound (`rewrite this = ...`), while fields of the instance can be mutated freely via `rewrite this.field = ...` (calling `obj.method()` does not move ownership of `obj`).

#### What `return this` does:

`return this` returns the pointer to the **exact same object instance** without copying the object or allocating new memory. This is the standard pattern for fluent APIs and method chaining:

```yukino
class RequestBuilder {
    host: string
    port: int64

    fun init() {
        rewrite this.host = "localhost"
        rewrite this.port = 80
    }

    fun with_host(host: string) -> RequestBuilder {
        rewrite this.host = host
        return this
    }

    fun with_port(port: int64) -> RequestBuilder {
        rewrite this.port = port
        return this
    }

    fun get_port(ref this) -> int64 {
        return this.port
    }
}

set req = init RequestBuilder()
    .with_host("example.com")
    .with_port(443)
set p: int64 = req.get_port() // 443
```

Key rules for `return this`:
- The method must declare a compatible return type (`-> RequestBuilder`, a base class `-> BaseClass`, or an implemented interface `-> InterfaceName`).
- `return this` does not move or invalidate `this` inside the method body before the return point.
- Regular class method calls are dispatched virtually through `vtable` (and interface-typed calls through `itable`), unlike `super(...)` and `super.method(...)`, which are always statically dispatched.

### 2.6 Class fields: set, static, and default values

Class fields support optional `set` and `static` modifiers as well as literal default initializers:

```yukino
class Config {
    // Equivalent declarations of regular mutable instance fields
    host: string = "localhost"
    set port: int32 = 8080

    // Fields without a default are deterministically zeroed ("" / 0 / false / null)
    label: string

    // Static class field (literal initializer is mandatory)
    public static MAX_RETRIES: int32 = 5

    fun get_retries(ref this) -> int32 {
        return this.MAX_RETRIES
    }
}

set cfg = init Config()
println(cfg.host)         // "localhost"
println(cfg.port)         // 8080
println(cfg.MAX_RETRIES)  // 5

// Regular and set fields can be mutated with rewrite
rewrite cfg.port = 9090

// ERROR E0503: static class fields cannot be mutated with rewrite
// rewrite cfg.MAX_RETRIES = 10
```

Rules for class fields and default values:

1. **`field: T` and `set field: T` are equivalent**: both declare an ordinary instance field that can be reassigned via `rewrite obj.field = ...` (or `rewrite this.field = ...`). Combining `set` and `static` on the same field is forbidden.
2. **Default field values (`field: T = <literal>`)**:
   - Only **literals** (including unary `-`, `!`, `~` on literals) are accepted as default values. Non-literal expressions (`x: int32 = 1 + 2`) fail with `E0401`.
   - When `init ClassName(...)` runs, instance fields are first zero-initialized, then literal defaults are applied from ancestors to the subclass, and finally the `init` constructor body executes (and may overwrite any defaults).
3. **Static class fields (`static field: T = <literal>`)**:
   - A literal initializer is **mandatory** for `static` fields (omitting `=` is a compile error).
   - Static fields add zero per-instance storage overhead (emitted as global constants and honoring `inline_statics`), are inherited by subclasses (`this.FIELD`, `super.FIELD`, `obj.FIELD`), and **cannot be reassigned** via `rewrite` (`E0503`).

---

## 3. Properties: get and set accessors

Classes support computed properties with the `get` and `set` syntax:

```yukino
class Rectangle {
    w: float64
    h: float64

    fun init(w: float64, h: float64) {
        rewrite this.w = w
        rewrite this.h = h
    }

    // Getter
    get area() -> float64 {
        return this.w * this.h
    }

    // Setter
    set size(val: float64) {
        rewrite this.w = val
        rewrite this.h = val
    }
}

set rect = init Rectangle(10.0, 5.0)
set a: float64 = rect.area // Getter call

// Writing to the setter requires rewrite
rewrite rect.size = 20.0
```

> [!NOTE]
> If a property defines only `get` (with neither `set` nor a matching underlying field), attempting to write via `rewrite obj.prop = ...` fails with compiler error `E0401` (read-only property).

---

## 4. Operator overloading

Classes can overload operators via the `operator` keyword. Available operators:

- arithmetic: `+`, `-`, `*`, `/`, `%`, `**`;
- bitwise operations and shifts: `&`, `|`, `^`, `~`, `<<`, `>>`;
- comparisons: `==`, `!=`, `<`, `<=`, `>`, `>=`;
- unary: `!`, `~`, and unary minus (an `operator -` method without parameters);
- indexing: `[]` and `[]=`.

If only `==` is defined, the compiler automatically derives `!=` as the negation of the `==` result. Dispatch of overloaded operators is implemented for class operands.

```yukino
class Complex {
    re: float64
    im: float64

    fun init(re: float64, im: float64) {
        rewrite this.re = re
        rewrite this.im = im
    }

    operator +(other: Complex) -> Complex {
        return init Complex(this.re + other.re, this.im + other.im)
    }

    operator ==(other: Complex) -> bool {
        return this.re == other.re && this.im == other.im
    }
}

set c1 = init Complex(1.0, 2.0)
set c2 = init Complex(3.0, 4.0)
set c3 = c1 + c2 // Invokes operator +
```

---

## 5. Interfaces (interface)

Interfaces declare method contracts that a class undertakes to implement. Classes declare implementation with the `with` keyword:

```yukino
interface Describable {
    fun describe() -> string
}

class Item with Describable {
    title: string

    fun init(title: string) {
        rewrite this.title = title
    }

    fun describe() -> string {
        return this.title
    }
}

fun print_description(d: Describable) {
    println(d.describe())
}
```

> [!NOTE]
> The typechecker guarantees that a class implementing an interface (`with InterfaceName`) contains all the required methods with correct signatures. When methods are called through an interface variable, the compiler performs dynamic dispatch through the interface method table `itable`.

### 5.1 Generic Interfaces

Interfaces can declare type parameters (`interface Name<T>` or `interface Name<T, R>`):

```yukino
interface Box<T> {
    fun set_val(v: T) -> int32
    fun get_val() -> T
}

class StringBox with Box<string> {
    item: string

    fun set_val(v: string) -> int32 {
        rewrite this.item = v
        return 0
    }

    fun get_val() -> string {
        return this.item
    }
}

fun print_box(b: Box<string>) {
    println(b.get_val())
}
```

A class implements a generic interface by supplying concrete type arguments (`with Box<string>`). When calling methods through an interface variable, the compiler generates dynamic dispatch through the `itable` with compile-time type verification.

---

## 6. Enumerations (enum)

Enumerations (`enum`) in Yukino are value types with a discriminated tag. They define a closed set of possible states, combining simple numeric labels with rich algebraic data types (ADT / Tagged Unions).

### 6.1 Declaration and variant types

Yukino supports three types of variants within an enumeration:

1. **Simple variants (Simple)**:
   Named constants without a payload. The compiler automatically assigns each variant a sequential integer tag starting from zero:

   ```yukino
   enum Color {
       Red,
       Green,
       Blue
   }
   ```

2. **Variants with values (WithValue)**:
   Variants with explicitly specified integer constants. Useful for protocol status codes, system error numbers, and bit flags:

   ```yukino
   enum HttpStatus {
       Ok = 200,
       Created = 201,
       BadRequest = 400,
       NotFound = 404,
       ServerError = 500
   }
   ```

3. **ADT variants with payload (Adt)**:
   Variants carrying typed data fields in parentheses. Different variants of the same enumeration can contain different numbers of fields of various types or none at all:

   ```yukino
   enum WebEvent {
       PageLoad,
       KeyPress(key: char),
       Click(x: int64, y: int64),
       Message(text: string)
   }
   ```

### 6.2 Creating and using values

Instances of an enumeration are created using qualified path syntax `EnumName::Variant`:

```yukino
// Simple variants
set c: Color = Color::Red

// Variants with numeric values
set status: HttpStatus = HttpStatus::Ok

// Variants with payloads (ADT)
set ev_load: WebEvent = WebEvent::PageLoad
set ev_key: WebEvent = WebEvent::KeyPress('Q')
set ev_click: WebEvent = WebEvent::Click(100, 200)
set ev_msg: WebEvent = WebEvent::Message("connection established")
```

### 6.3 Pattern matching with switch

Enumerations are most powerful when combined with the `switch` construct. In Yukino, `switch` arms do not require the `case` keyword and are written as `Pattern -> { body }`.

#### Exhaustiveness checking

The compiler strictly checks matching exhaustiveness. If a `switch` without an `else ->` arm does not cover all variants of the enumeration, the compiler halts the build with error `E0405`:

```yukino
set color = Color::Green

// All Color variants are covered — check succeeds
switch color {
    Color::Red -> {
        println("Stop")
    }
    Color::Green -> {
        println("Go")
    }
    Color::Blue -> {
        println("Caution")
    }
}
```

#### Default arm (else ->) and combining variants (|)

When only a subset of variants needs explicit handling, the remaining variants can be handled with an `else ->` arm. Multiple variants with identical handling can be combined with the `|` operator:

```yukino
set status = HttpStatus::NotFound

switch status {
    HttpStatus::Ok | HttpStatus::Created -> {
        println("Successful status")
    }
    HttpStatus::NotFound -> {
        println("Resource not found")
    }
    else -> {
        println("Other status code")
    }
}
```

> [!NOTE]
> Any pattern placed after an `else ->` arm or a wildcard pattern `_` is considered unreachable and produces compilation error `E0406`.

#### Destructuring payload fields (ADT)

When matching variants with a payload, fields are automatically unpacked into local variables inside the corresponding arm:

```yukino
set event = WebEvent::Click(1024, 768)

switch event {
    WebEvent::PageLoad -> {
        println("Page loaded completely")
    }
    WebEvent::KeyPress(k) -> {
        println("User pressed key: " + cast(k, "string"))
    }
    WebEvent::Click(x, y) -> {
        println("Mouse clicked at: (${x}, ${y})")
    }
    WebEvent::Message(msg) -> {
        println("Incoming message: " + msg)
    }
}
```

#### Guard expressions (guards)

An additional boolean condition can be attached to a variant pattern using the `if` keyword:

```yukino
switch event {
    WebEvent::Click(x, y) if x < 0 || y < 0 -> {
        println("Click outside active area")
    }
    WebEvent::Click(x, y) -> {
        println("Click in active area: (${x}, ${y})")
    }
    else -> {}
}
```

### 6.4 Memory layout and comparison

An `enum` in Yukino is a value type with a tagged union layout:
- A discriminant tag field (`i32`, 4 bytes) identifying the active variant.
- A byte buffer `[N x i8]` for payload data, aligned to the maximum alignment of the payload fields. The size `N` equals the maximum payload size among all variants of the enum (for simple enums `N = 0`).

Enumerations are placed directly on the stack or embedded into structures without heap allocations. Instances of the same enumeration type support equality comparisons with `==` and `!=`:

```yukino
set current_state = Color::Red
set target_state = Color::Green

if current_state == target_state {
    println("States match")
} else if current_state != Color::Blue {
    println("Current state is not Blue")
}
```

---

[← Back: Memory Model, Ownership, and Regions](6-memory.md) | [Next: Strings and Bytes →](8-strings.md)