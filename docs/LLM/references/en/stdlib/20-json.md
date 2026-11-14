# Module std.json

The `std.json` module provides high-performance tools for parsing, safe data extraction, validation, and serialization of documents strictly conforming to the RFC 8259 JSON specification in pure Yukino without external dependencies.

---

## 1. Import: load std.json

Importing the entire package:

```yukino
load std.json
```

This brings the following entities into scope:

- **Node Type Constants:** `JSON_NULL`, `JSON_BOOL`, `JSON_NUMBER`, `JSON_STRING`, `JSON_ARRAY`, `JSON_OBJECT`.
- **Error Structures:** `JsonError`.
- **Classes:** `JsonValue`.
- **Parsing and Validation Functions:** `json_parse`, `json_try_parse`, `json_validate`.
- **Serialization Functions:** `json_stringify`, `json_pretty`, `json_pretty_indent`.
- **Node Factory Functions:** `json_null`, `json_bool`, `json_number`, `json_int`, `json_string`, `json_array`, `json_object`.

All entities above are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.json` module defines integer constants representing the node kind stored in `JsonValue.kind`:

```yukino
public static JSON_NULL: int32 = 0
public static JSON_BOOL: int32 = 1
public static JSON_NUMBER: int32 = 2
public static JSON_STRING: int32 = 3
public static JSON_ARRAY: int32 = 4
public static JSON_OBJECT: int32 = 5
```

These constants can be used for low-level node type checks (`if val.kind == JSON_STRING`), although predicate methods (`val.is_string()`, `val.is_null()`, etc.) are generally preferred.

---

## 3. Interfaces

No interfaces are declared in the `std.json` module.

---

## 4. Data Structures and Errors (Panics)

### 4.1 JsonError

The `JsonError` structure represents a syntax parsing error in a JSON document. It is thrown via `panic` by `json_parse` upon encountering invalid syntax, unclosed literals, illegal characters, or comments and trailing commas prohibited by RFC 8259.

```yukino
public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: human-readable description of the error (e.g. `"expected ':' after property name"`).
- `line`: 1-based line number in the source text where the error occurred.
- `column`: 1-based column number within that line.
- `offset`: 0-based byte offset from the start of the source string.

Example of handling the error using `sus / catch`:

```yukino
load std.json: json_parse, JsonError
load std.io: println

fun main() {
    sus {
        set doc = json_parse("{\"invalid\": }")
    } catch err: JsonError {
        println("Parse error: ${err.message} at line ${err.line}, column ${err.column}")
    }
}
```

---

## 5. Classes

### 5.1 JsonValue

The `JsonValue` class represents an arbitrary node in a JSON tree (null, boolean, number, string, array, or key-value object). Designed with a Fluent API, it guarantees safe property navigation: chained calls never panic when a key is missing or types do not match.

```yukino
public class JsonValue {
    public kind: int32
    public bool_val: bool
    public num_val: float64
    public str_val: string
    public array_val: JsonValue[]
    public obj_keys: string[]
    public obj_vals: JsonValue[]
}
```

#### Type Predicates

- `is_null() -> bool`: returns `true` if the node represents null or a missing value.
- `is_bool() -> bool`: returns `true` if the node is a boolean value.
- `is_number() -> bool`: returns `true` if the node is a number.
- `is_string() -> bool`: returns `true` if the node is a string.
- `is_array() -> bool`: returns `true` if the node is an array.
- `is_object() -> bool`: returns `true` if the node is an object.

#### Value Accessors

- `as_bool() -> bool`: returns the boolean value, or `false` if the node is not a boolean.
- `as_bool_or(fallback: bool) -> bool`: returns the boolean value, or `fallback` if the node is not a boolean.
- `as_int() -> int64`: returns the number truncated to `int64`, or `0` if the node is not a number.
- `as_int_or(fallback: int64) -> int64`: returns the integer value, or `fallback` if the node is not a number.
- `as_float() -> float64`: returns the `float64` number, or `0.0` if the node is not a number.
- `as_float_or(fallback: float64) -> float64`: returns the float value, or `fallback`.
- `as_string() -> string`: returns the string value, or `""` if the node is not a string.
- `as_string_or(fallback: string) -> string`: returns the string value, or `fallback` if the node is not a string.
- `as_array() -> JsonValue[]`: returns the internal array of items `JsonValue[]`, or an empty array if not an array.

#### Object and Array Navigation

- `get(key: string) -> JsonValue`: retrieves a property by name. If missing or not an object, returns a safe null `JsonValue`. This enables chaining: `doc.get("user").get("profile").get("email").as_string()`.
- `get_opt(key: string) -> JsonValue?`: returns a nullable `JsonValue?` containing the property value, or `null` if missing.
- `has(key: string) -> bool`: returns `true` if the object contains the specified key.
- `keys() -> string[]`: returns an array of all property keys in the object.
- `at(index: int64) -> JsonValue`: returns the array element at `index`. If out of bounds or not an array, returns a safe null `JsonValue`.
- `at_opt(index: int64) -> JsonValue?`: returns a nullable `JsonValue?` element, or `null` if the index is out of bounds.
- `len() -> int64`: returns the number of elements for arrays, the number of keys for objects, or the byte length of the string for string nodes. Returns `0` for other types.

#### Mutation and Building

- `set(key: string, val: JsonValue) -> JsonValue`: adds or overwrites a property on an object and returns `this` for chaining.
- `push(item: JsonValue) -> JsonValue`: appends an element to an array and returns `this`.
- `cp() -> JsonValue`: creates a deep clone of the entire `JsonValue` tree.

#### Recommendation: Mapping JsonValue into Application Structures

Because Yukino is a statically typed compiled language, the cleanest, safest, and most idiomatic pattern is to convert dynamic `JsonValue` nodes into strongly typed custom `struct` definitions via simple constructor functions:

```yukino
load std.json: json_parse, JsonValue
load std.io: println

struct UserProfile {
    id: int64
    username: string
    is_active: bool
    email: string
}

fun user_profile_from_json(j: JsonValue) -> UserProfile {
    return UserProfile {
        id: j.get("id").as_int(),
        username: j.get("username").as_string(),
        is_active: j.get("is_active").as_bool_or(true),
        email: j.get("email").as_string_or("unknown@example.com"),
    }
}

fun main() {
    set raw = "{\"id\": 42, \"username\": \"yukino\", \"email\": \"yuki@local\"}"
    set doc = json_parse(raw)
    set user = user_profile_from_json(doc)

    println("User: ${user.username}, ID: ${user.id}, Email: ${user.email}")
}
```

This pattern isolates dynamic JSON extraction at the boundary and provides compile-time type safety throughout the rest of your program.

---

## 6. Function Reference

### 6.1 json_parse

Parses an input string according to strict RFC 8259 JSON rules and returns the root `JsonValue`. Comments and trailing commas are strictly rejected.

```yukino
public fun json_parse(text: string) -> JsonValue
```

Returns the parsed root `JsonValue`. Non-nullable.

Throws a panic with a `JsonError` structure on syntax errors, unexpected characters, unclosed strings, or comments.

```yukino
load std.json: json_parse
load std.io: println

fun main() {
    set doc = json_parse("{\"title\": \"Yukino\", \"version\": 1}")
    println("Title: " + doc.get("title").as_string())
}
```

### 6.2 json_try_parse

Attempts to parse a JSON string safely without panicking on syntax errors.

```yukino
public fun json_try_parse(text: string) -> JsonValue?
```

Returns the parsed root `JsonValue` on success, or `null` if the input is malformed. Nullable.

The function never panics.

```yukino
load std.json: json_try_parse
load std.io: println

fun main() {
    set res = json_try_parse("{\"broken\": ")
    if res == null {
        println("String is not valid JSON")
    }
}
```

### 6.3 json_validate

Validates whether a string is syntactically conforming to strict RFC 8259 JSON.

```yukino
public fun json_validate(text: string) -> bool
```

Returns `true` if valid, or `false` otherwise. Non-nullable `bool`.

The function never panics.

```yukino
load std.json: json_validate
load std.io: println

fun main() {
    if json_validate("{\"valid\": true}") {
        println("JSON is valid")
    }
}
```

### 6.4 json_stringify

Serializes a `JsonValue` into a compact JSON string without whitespace. All string escape sequences are properly encoded.

```yukino
public fun json_stringify(val: JsonValue) -> string
```

Returns a compact JSON string. Non-nullable.

The function never panics.

```yukino
load std.json: json_object, json_string, json_stringify
load std.io: println

fun main() {
    set obj = json_object()
    obj.set("status", json_string("ok"))
    println(json_stringify(obj))
}
```

### 6.5 json_pretty

Serializes a `JsonValue` into a human-readable indented JSON string using a default indentation of 2 spaces per nesting level.

```yukino
public fun json_pretty(val: JsonValue) -> string
```

Returns an indented JSON string. Non-nullable.

The function never panics.

```yukino
load std.json: json_parse, json_pretty
load std.io: println

fun main() {
    set doc = json_parse("{\"name\": \"Alice\", \"items\": [1, 2]}")
    println(json_pretty(doc))
}
```

### 6.6 json_pretty_indent

Serializes a `JsonValue` into a human-readable JSON string with a custom indentation unit (e.g. 4 spaces or `"\t"`).

```yukino
public fun json_pretty_indent(val: JsonValue, indent: string) -> string
```

Returns a formatted JSON string. Non-nullable.

The function never panics.

```yukino
load std.json: json_parse, json_pretty_indent
load std.io: println

fun main() {
    set doc = json_parse("{\"key\": \"val\"}")
    println(json_pretty_indent(doc, "    "))
}
```

### 6.7 Node Factory Functions

The module provides convenient constructors for creating `JsonValue` instances:

- `json_null() -> JsonValue`: creates a null node.
- `json_bool(b: bool) -> JsonValue`: creates a boolean node.
- `json_number(n: float64) -> JsonValue`: creates a number node.
- `json_int(n: int64) -> JsonValue`: creates a number node from an integer.
- `json_string(s: string) -> JsonValue`: creates a string node.
- `json_array(items: JsonValue[]) -> JsonValue`: creates an array node.
- `json_object() -> JsonValue`: creates an empty object node.

All factory functions return a `JsonValue` and never panic.

```yukino
load std.json: json_object, json_string, json_int, json_array, json_stringify
load std.io: println

fun main() {
    set root = json_object()
    root.set("service", json_string("gateway"))
    root.set("port", json_int(8080))
    set list = json_array([])
    list.push(json_string("api_v1"))
    root.set("routes", list)
    println(json_stringify(root))
}
```

---

[← Previous: Module std.sharedc](19-sharedc.md) | [Next: Module std.jsonc →](21-jsonc.md)
