# Module std.toml

The `std.toml` module provides high-performance tools for parsing, safe data extraction, validation, and serialization of configuration documents in the TOML (Tom's Obvious Minimal Language) v1.0.0 format in pure Yukino without external dependencies.

---

## 1. Import: load std.toml

Importing the entire package:

```yukino
load std.toml
```

This makes the following entities available:

- **Node type constants:** `TOML_NULL`, `TOML_BOOL`, `TOML_INT`, `TOML_FLOAT`, `TOML_STRING`, `TOML_DATETIME`, `TOML_ARRAY`, `TOML_TABLE`.
- **Error structures:** `TomlError`.
- **Classes:** `TomlValue`.
- **Parsing and validation functions:** `parse`, `try_parse`, `validate`, `toml_parse`, `toml_try_parse`, `toml_validate`.
- **Serialization functions:** `stringify`, `pretty`, `pretty_indent`, `toml_stringify`, `toml_pretty`, `toml_pretty_indent`.
- **Factory constructor functions:** `toml_null`, `toml_bool`, `toml_int`, `toml_float`, `toml_number`, `toml_string`, `toml_datetime`, `toml_array`, `toml_table`, `toml_object`.

All listed entities are also automatically available when importing the entire standard library: `load std`.

---

## 2. Constants

The `std.toml` module defines integer constants specifying the node type in the `kind` property of the `TomlValue` class:

```yukino
public static TOML_NULL: int32 = 0
public static TOML_BOOL: int32 = 1
public static TOML_INT: int32 = 2
public static TOML_FLOAT: int32 = 3
public static TOML_STRING: int32 = 4
public static TOML_DATETIME: int32 = 5
public static TOML_ARRAY: int32 = 6
public static TOML_TABLE: int32 = 7
```

These constants are used for direct node type checking `if val.kind == TOML_TABLE`, although in most cases built-in predicate methods (`val.is_table()`, `val.is_string()`, `val.is_int()`, `val.is_datetime()`, etc.) are preferred.

---

## 3. Interfaces

No interfaces are declared in the `std.toml` module.

---

## 4. Data and Error Structures (Panic)

### 4.1 TomlError

The `TomlError` structure represents a TOML document parsing error. It is generated via `panic` by `parse` and `toml_parse` when syntax errors, unterminated strings, invalid table headers, or unsupported tokens are encountered.

```yukino
public struct TomlError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: human-readable description of the error (e.g. `"expected '=' after TOML key"`).
- `line`: 1-based line number in the source text where the error occurred.
- `column`: 1-based column number in the line.
- `offset`: 0-based byte offset from the beginning of the input string.

Example of handling errors with the `sus / catch` construct:

```yukino
load std.toml: parse, TomlError
load std.io: println

fun main() {
    sus {
        set doc = parse("[broken table")
    } catch err: TomlError {
        println("TOML error: ${err.message} on line ${err.line}, col ${err.column}")
    }
}
```

---

## 5. Classes

### 5.1 TomlValue

The `TomlValue` class represents an arbitrary node in a TOML document tree (null, boolean, integer, float, string, datetime, array, or table). Designed with a Fluent API, it ensures safe traversal through nested sections and keys: navigation calls never panic when a key is absent or when types do not match.

```yukino
public class TomlValue {
    public kind: int32
    public bool_val: bool
    public int_val: int64
    public float_val: float64
    public str_val: string
    public array_val: TomlValue[]
    public obj_keys: string[]
    public obj_vals: TomlValue[]
}
```

#### Type Checking Methods (Predicates)

- `is_null() -> bool`: returns `true` if the node is `null` (or represents a missing value).
- `is_bool() -> bool`: returns `true` if the node is a boolean value.
- `is_number() -> bool`: returns `true` if the node is an integer or float.
- `is_int() -> bool`: returns `true` if the node is an integer.
- `is_float() -> bool`: returns `true` if the node is a floating-point number.
- `is_string() -> bool`: returns `true` if the node is a string.
- `is_datetime() -> bool`: returns `true` if the node contains an RFC 3339 / ISO 8601 date-time string.
- `is_array() -> bool`: returns `true` if the node is an array.
- `is_table() -> bool`: returns `true` if the node is a TOML table.
- `is_object() -> bool`: synonym for `is_table()`, returns `true` if the node is a table.
- `is_table_array() -> bool`: returns `true` if the node is an array of tables `[[table]]`.

#### Value Extraction Methods

- `as_bool() -> bool`: returns the boolean value, or `false` if the node is not a boolean.
- `as_bool_or(fallback: bool) -> bool`: returns the boolean value, or `fallback` if the node is not a boolean.
- `as_int() -> int64`: returns an `int64` integer. Converts floats to integers; returns `0` if the node is not numeric.
- `as_int_or(fallback: int64) -> int64`: returns an integer, or `fallback` if the node is not numeric.
- `as_float() -> float64`: returns a `float64` number. Converts integers to floats; returns `0.0` if the node is not numeric.
- `as_float_or(fallback: float64) -> float64`: returns a float, or `fallback` if the node is not numeric.
- `as_string() -> string`: returns the string value. For date-time nodes, returns their ISO representation. Returns `""` if the node is not a string.
- `as_string_or(fallback: string) -> string`: returns a string, or `fallback` if the node is not a string.
- `as_array() -> TomlValue[]`: returns the internal array of `TomlValue[]` items, or an empty array if the node is not an array.

#### Table and Array Navigation Methods

- `get(key: string) -> TomlValue`: returns the property value with the specified `key`. If the key is missing or the node is not a table, returns a safe null `TomlValue` node. This enables fluent chains like `doc.get("database").get("ports").at(0).as_int()`.
- `get_opt(key: string) -> TomlValue?`: returns a nullable `TomlValue?`, equal to `null` if the key is missing or the node is not a table.
- `has(key: string) -> bool`: returns `true` if the table contains the given key.
- `keys() -> string[]`: returns an array of all keys in the table.
- `at(index: int64) -> TomlValue`: returns the array element at `index`. If the index is out of bounds or the node is not an array, returns a safe null `TomlValue` node.
- `at_opt(index: int64) -> TomlValue?`: returns a nullable element `TomlValue?` at `index`, or `null` if out of bounds.
- `len() -> int64`: returns the number of elements in an array, keys in a table, or byte length of a string. Returns `0` for all other types.

#### Mutation and Construction Methods

- `set(key: string, val: TomlValue) -> TomlValue`: sets or overwrites a table key and returns `this` for chaining.
- `push(item: TomlValue) -> TomlValue`: appends an item to the array and returns `this`.
- `set_path(path: string[], val: TomlValue) -> TomlValue`: recursively creates intermediate tables along `path` and sets the target property.
- `cp() -> TomlValue`: creates a deep clone of the entire `TomlValue` tree.

---

## 6. Function Reference

### 6.1 parse

Parses an input string in TOML v1.0.0 format and returns the root `TomlValue` table. Supports standard tables `[table]`, nested dotted sections `[a.b.c]`, arrays of tables `[[items]]`, inline tables, heterogeneous arrays, comments, and all number and datetime formats.

```yukino
public fun parse(text: string) -> TomlValue
```

Returns the parsed root table node of type `TomlValue`. The return value is non-nullable.

Panics with a `TomlError` structure upon encountering a syntax error, unterminated literal, or invalid structure.

```yukino
load std.toml: parse
load std.io: println

fun main() {
    set doc = parse("title = \"Yukino Project\"\n\n[owner]\nname = \"Alice\"\n")
    println("Title: " + doc.get("title").as_string())
    println("Owner: " + doc.get("owner").get("name").as_string())
}
```

### 6.2 try_parse

Attempts to parse a TOML string safely without panicking on syntax errors.

```yukino
public fun try_parse(text: string) -> TomlValue?
```

Returns a `TomlValue` node on success, or `null` if the input text contains syntax errors. The return value is nullable.

This function never panics.

```yukino
load std.toml: try_parse
load std.io: println

fun main() {
    set res = try_parse("broken = [1, 2")
    if res == null {
        println("String is not valid TOML")
    }
}
```

### 6.3 validate

Checks whether the given string is a syntactically valid TOML document.

```yukino
public fun validate(text: string) -> bool
```

Returns `true` if the string is valid TOML, or `false` otherwise. The return value is a non-nullable `bool`.

This function never panics.

```yukino
load std.toml: validate
load std.io: println

fun main() {
    if validate("key = \"value\"\n[section]\nid = 100\n") {
        println("TOML document is valid")
    }
}
```

### 6.4 stringify

Serializes a `TomlValue` node into a canonical TOML string, properly formatted with tables and array of table headers.

```yukino
public fun stringify(val: TomlValue) -> string
```

Returns a serialized TOML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.toml: toml_table, toml_string, toml_int, stringify
load std.io: println

fun main() {
    set doc = toml_table()
    doc.set("app", toml_string("server"))
    doc.set("port", toml_int(8080))
    println(stringify(doc))
}
```

### 6.5 pretty

Serializes a `TomlValue` node into a formatted TOML string with standard two-space indentation for inline structures.

```yukino
public fun pretty(val: TomlValue) -> string
```

Returns a formatted TOML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.toml: parse, pretty
load std.io: println

fun main() {
    set doc = parse("name = \"demo\"\n[server]\nhost = \"127.0.0.1\"\n")
    println(pretty(doc))
}
```

### 6.6 pretty_indent

Serializes a `TomlValue` node into a formatted TOML string with custom indentation.

```yukino
public fun pretty_indent(val: TomlValue, indent: string) -> string
```

Returns a formatted TOML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.toml: parse, pretty_indent
load std.io: println

fun main() {
    set doc = parse("name = \"demo\"\n")
    println(pretty_indent(doc, "    "))
}
```

### 6.7 Prefixed Function Aliases

For developer convenience and complete parity with the `std.json` naming convention, the module provides prefixed function aliases:

- `toml_parse(text: string) -> TomlValue`: alias for `parse`.
- `toml_try_parse(text: string) -> TomlValue?`: alias for `try_parse`.
- `toml_validate(text: string) -> bool`: alias for `validate`.
- `toml_stringify(val: TomlValue) -> string`: alias for `stringify`.
- `toml_pretty(val: TomlValue) -> string`: alias for `pretty`.
- `toml_pretty_indent(val: TomlValue, indent: string) -> string`: alias for `pretty_indent`.

Behavior, return types, panic contracts, and nullability are identical to the base functions.

```yukino
load std.toml: toml_parse, toml_stringify
load std.io: println

fun main() {
    set doc = toml_parse("version = 1\n")
    println(toml_stringify(doc))
}
```

### 6.8 Factory Constructor Functions

The module provides factory functions for building `TomlValue` node trees:

- `toml_null() -> TomlValue`: creates an empty null node.
- `toml_bool(b: bool) -> TomlValue`: creates a boolean node with value `b`.
- `toml_int(n: int64) -> TomlValue`: creates an integer node with value `n`.
- `toml_float(f: float64) -> TomlValue`: creates a float node with value `f`.
- `toml_number(f: float64) -> TomlValue`: creates a numeric node with value `f`.
- `toml_string(s: string) -> TomlValue`: creates a string node with value `s`.
- `toml_datetime(dt: string) -> TomlValue`: creates a datetime node with an ISO 8601 / RFC 3339 string.
- `toml_array(items: TomlValue[]) -> TomlValue`: creates an array node containing `items`.
- `toml_table() -> TomlValue`: creates an empty TOML table.
- `toml_object() -> TomlValue`: alias for `toml_table()`, creates an empty table.

All factory functions return a non-nullable `TomlValue` and never panic.

```yukino
load std.toml: toml_table, toml_string, toml_array, toml_int, toml_stringify
load std.io: println

fun main() {
    set root = toml_table()
    root.set("title", toml_string("Config"))
    set ports = toml_array([])
    ports.push(toml_int(80))
    ports.push(toml_int(443))
    root.set("ports", ports)
    println(toml_stringify(root))
}
```

---

[← Previous: Module std.regex](29-regex.md) | [Next: Module std.yaml →](31-yaml.md)
