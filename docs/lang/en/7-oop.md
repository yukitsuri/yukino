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

Class instances are created with the `init` keyword: `init ClassName(...)`, which activates the `init` method:

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

Classes extend a base class via `extends`. The `super` keyword allows calling the ancestor's constructor and accessing its methods:

```yukino
class Dog extends Animal {
    breed: string

    fun init(name: string, breed: string) {
        // Calling the base class constructor
        super(name)
        rewrite this.breed = breed
    }

    // Overriding a virtual method
    fun speak() -> string {
        return "Woof!"
    }
}

set dog = init Dog("Rex", "Collie")
set sound = dog.speak() // "Woof!"
```

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