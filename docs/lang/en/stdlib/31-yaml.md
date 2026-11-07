# Module std.yaml

The `std.yaml` module provides high-performance tools for parsing, safe data extraction, validation, and serialization of structured documents in the YAML (YAML Ain't Markup Language) v1.2 format in pure Yukino without external dependencies.

---

## 1. Import: load std.yaml

Importing the entire package:

```yukino
load std.yaml
```

This makes the following entities available:

- **Node type constants:** `YAML_NULL`, `YAML_BOOL`, `YAML_NUMBER`, `YAML_STRING`, `YAML_ARRAY`, `YAML_OBJECT`.
- **Error structures:** `YamlError`.
- **Classes:** `YamlValue`.
- **Parsing and validation functions:** `parse`, `try_parse`, `validate`, `yaml_parse`, `yaml_try_parse`, `yaml_validate`.
- **Serialization functions:** `stringify`, `pretty`, `pretty_indent`, `yaml_stringify`, `yaml_pretty`, `yaml_pretty_indent`.
- **Factory constructor functions:** `yaml_null`, `yaml_bool`, `yaml_number`, `yaml_int`, `yaml_float`, `yaml_string`, `yaml_array`, `yaml_object`, `yaml_mapping`.

All listed entities are also automatically available when importing the entire standard library: `load std`.

---

## 2. Constants

The `std.yaml` module defines integer constants specifying the node type in the `kind` property of the `YamlValue` class:

```yukino
public static YAML_NULL: int32 = 0
public static YAML_BOOL: int32 = 1
public static YAML_NUMBER: int32 = 2
public static YAML_STRING: int32 = 3
public static YAML_ARRAY: int32 = 4
public static YAML_OBJECT: int32 = 5
```

These constants are used for direct node type checking `if val.kind == YAML_OBJECT`, although in practical application code built-in predicate methods (`val.is_object()`, `val.is_array()`, `val.is_string()`, `val.is_null()`, etc.) are preferred.

---

## 3. Interfaces

No interfaces are declared in the `std.yaml` module.

---

## 4. Data and Error Structures (Panic)

### 4.1 YamlError

The `YamlError` structure represents a YAML document parsing error. It is generated via `panic` by `parse` and `yaml_parse` when indentation violations, unclosed flow brackets, invalid scalars, or unsupported delimiters are encountered.

```yukino
public struct YamlError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: human-readable description of the error (e.g. `"expected ':' after mapping key"`).
- `line`: 1-based line number in the source text where the error occurred.
- `column`: 1-based column number in the line.
- `offset`: 0-based byte offset from the beginning of the input string.

Example of handling errors with the `sus / catch` construct:

```yukino
load std.yaml: parse, YamlError
load std.io: println

fun main() {
    sus {
        set doc = parse("unclosed: [1, 2")
    } catch err: YamlError {
        println("YAML error: ${err.message} on line ${err.line}, col ${err.column}")
    }
}
```

---

## 5. Classes

### 5.1 YamlValue

The `YamlValue` class represents an arbitrary node in a YAML document tree (null, boolean, number, string, sequence/array, or mapping/object). Designed with a Fluent API, it ensures safe traversal through nested properties: navigation calls never panic when a key is absent or when types do not match.

```yukino
public class YamlValue {
    public kind: int32
    public bool_val: bool
    public num_val: float64
    public str_val: string
    public array_val: YamlValue[]
    public obj_keys: string[]
    public obj_vals: YamlValue[]
}
```

#### Type Checking Methods (Predicates)

- `is_null() -> bool`: returns `true` if the node is `null` (or represents a missing value).
- `is_bool() -> bool`: returns `true` if the node is a boolean value.
- `is_number() -> bool`: returns `true` if the node is a number (integer or float).
- `is_string() -> bool`: returns `true` if the node is a string.
- `is_array() -> bool`: returns `true` if the node is a YAML sequence (list).
- `is_object() -> bool`: returns `true` if the node is a YAML mapping (object).
- `is_map() -> bool`: synonym for `is_object()`, returns `true` for mappings.

#### Value Extraction Methods

- `as_bool() -> bool`: returns the boolean value, or `false` if the node is not a boolean.
- `as_bool_or(fallback: bool) -> bool`: returns the boolean value, or `fallback` if the node is not a boolean.
- `as_int() -> int64`: returns an `int64` integer (casts numeric float to int). Returns `0` if the node is not numeric.
- `as_int_or(fallback: int64) -> int64`: returns an integer, or `fallback` if the node is not numeric.
- `as_float() -> float64`: returns a `float64` float. Returns `0.0` if the node is not numeric.
- `as_float_or(fallback: float64) -> float64`: returns a float, or `fallback` if the node is not numeric.
- `as_string() -> string`: returns the string value, or an empty string `""` if the node is not a string.
- `as_string_or(fallback: string) -> string`: returns a string, or `fallback` if the node is not a string.
- `as_array() -> YamlValue[]`: returns the internal array of `YamlValue[]` elements, or an empty list if the node is not a sequence.

#### Mapping and Sequence Navigation Methods

- `get(key: string) -> YamlValue`: returns the property value with the specified `key`. If the key is missing or the node is not a mapping, returns a safe null `YamlValue` node. This enables fluent chains like `doc.get("server").get("host").as_string()`.
- `get_opt(key: string) -> YamlValue?`: returns a nullable `YamlValue?`, equal to `null` if the key is missing.
- `has(key: string) -> bool`: returns `true` if the mapping contains the given key.
- `keys() -> string[]`: returns an array of all keys in the mapping.
- `at(index: int64) -> YamlValue`: returns the sequence element at `index`. If the index is out of bounds or the node is not a sequence, returns a safe null `YamlValue` node.
- `at_opt(index: int64) -> YamlValue?`: returns a nullable element `YamlValue?` at `index`, or `null` if out of bounds.
- `len() -> int64`: returns the number of elements in a sequence, keys in a mapping, or byte length of a string. Returns `0` for all other types.

#### Mutation and Construction Methods

- `set(key: string, val: YamlValue) -> YamlValue`: sets or overwrites a mapping key and returns `this` for chaining.
- `push(item: YamlValue) -> YamlValue`: appends an item to the sequence and returns `this`.
- `cp() -> YamlValue`: creates a deep clone of the entire `YamlValue` tree.

---

## 6. Function Reference

### 6.1 parse

Parses an input string in YAML v1.2 format and returns the root `YamlValue` node. Supports indentation-based blocks, lists (`- item`), key-value maps (`key: value`), flow collections (`[a, b]`, `{x: 1, y: 2}`), quoted and unquoted strings, comments (`# comment`), and explicit literals (`null`, `true`, `false`).

```yukino
public fun parse(text: string) -> YamlValue
```

Returns the parsed root node of type `YamlValue`. The return value is non-nullable.

Panics with a `YamlError` structure upon encountering an indentation error, unclosed brackets, or syntax error.

```yukino
load std.yaml: parse
load std.io: println

fun main() {
    set raw = "app: my_service\nport: 8080\nservices:\n  - web\n  - db\n"
    set doc = parse(raw)
    println("App: " + doc.get("app").as_string())
    println("Port: " + cast(doc.get("port").as_int(), "string"))
    println("First service: " + doc.get("services").at(0).as_string())
}
```

### 6.2 try_parse

Attempts to parse a YAML string safely without panicking on syntax errors.

```yukino
public fun try_parse(text: string) -> YamlValue?
```

Returns a `YamlValue` node on success, or `null` if the input text contains syntax errors. The return value is nullable.

This function never panics.

```yukino
load std.yaml: try_parse
load std.io: println

fun main() {
    set res = try_parse("broken: [1, 2")
    if res == null {
        println("String is not valid YAML")
    }
}
```

### 6.3 validate

Checks whether the given string is a syntactically valid YAML document.

```yukino
public fun validate(text: string) -> bool
```

Returns `true` if the string is valid YAML, or `false` otherwise. The return value is a non-nullable `bool`.

This function never panics.

```yukino
load std.yaml: validate
load std.io: println

fun main() {
    if validate("key: value\nflag: true\n") {
        println("YAML document is valid")
    }
}
```

### 6.4 stringify

Serializes a `YamlValue` node into a canonical YAML string with standard 2-space block indentation.

```yukino
public fun stringify(val: YamlValue) -> string
```

Returns a serialized YAML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.yaml: yaml_object, yaml_string, yaml_int, stringify
load std.io: println

fun main() {
    set doc = yaml_object()
    doc.set("name", yaml_string("production"))
    doc.set("replicas", yaml_int(3))
    println(stringify(doc))
}
```

### 6.5 pretty

Serializes a `YamlValue` node into a formatted YAML string with a standard indentation step of 2 spaces per level.

```yukino
public fun pretty(val: YamlValue) -> string
```

Returns a formatted YAML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.yaml: parse, pretty
load std.io: println

fun main() {
    set doc = parse("server:\n  host: localhost\n  port: 80\n")
    println(pretty(doc))
}
```

### 6.6 pretty_indent

Serializes a `YamlValue` node into a formatted YAML string with custom indentation.

```yukino
public fun pretty_indent(val: YamlValue, indent: string) -> string
```

Returns a formatted YAML string of type `string`. The return value is non-nullable.

This function never panics.

```yukino
load std.yaml: parse, pretty_indent
load std.io: println

fun main() {
    set doc = parse("key: val\n")
    println(pretty_indent(doc, "    "))
}
```

### 6.7 Prefixed Function Aliases

For developer convenience and complete parity with the `std.json` naming convention, the module provides prefixed function aliases:

- `yaml_parse(text: string) -> YamlValue`: alias for `parse`.
- `yaml_try_parse(text: string) -> YamlValue?`: alias for `try_parse`.
- `yaml_validate(text: string) -> bool`: alias for `validate`.
- `yaml_stringify(val: YamlValue) -> string`: alias for `stringify`.
- `yaml_pretty(val: YamlValue) -> string`: alias for `pretty`.
- `yaml_pretty_indent(val: YamlValue, indent: string) -> string`: alias for `pretty_indent`.

Behavior, return types, panic contracts, and nullability are identical to the base functions.

```yukino
load std.yaml: yaml_parse, yaml_stringify
load std.io: println

fun main() {
    set doc = yaml_parse("debug: true\n")
    println(yaml_stringify(doc))
}
```

### 6.8 Factory Constructor Functions

The module provides factory functions for building `YamlValue` node trees:

- `yaml_null() -> YamlValue`: creates an empty null node.
- `yaml_bool(b: bool) -> YamlValue`: creates a boolean node with value `b`.
- `yaml_number(n: float64) -> YamlValue`: creates a numeric node with value `n`.
- `yaml_int(n: int64) -> YamlValue`: creates an integer numeric node with value `n`.
- `yaml_float(f: float64) -> YamlValue`: creates a floating-point numeric node with value `f`.
- `yaml_string(s: string) -> YamlValue`: creates a string node with value `s`.
- `yaml_array(items: YamlValue[]) -> YamlValue`: creates a sequence node containing `items`.
- `yaml_object() -> YamlValue`: creates an empty mapping node.
- `yaml_mapping() -> YamlValue`: alias for `yaml_object()`, creates an empty mapping node.

All factory functions return a non-nullable `YamlValue` and never panic.

```yukino
load std.yaml: yaml_object, yaml_string, yaml_array, yaml_int, yaml_stringify
load std.io: println

fun main() {
    set root = yaml_object()
    root.set("cluster", yaml_string("primary"))
    set nodes = yaml_array([])
    nodes.push(yaml_string("node-1"))
    nodes.push(yaml_string("node-2"))
    root.set("nodes", nodes)
    println(yaml_stringify(root))
}
```

---

[← Previous: Module std.toml](30-toml.md) | [Next: Module std.sqlite →](32-sqlite.md)
