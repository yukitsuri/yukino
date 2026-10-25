# Module std.jsonc

The `std.jsonc` module provides full-featured tools for parsing, validating, comment stripping, and serializing documents in the JSON with Comments (JSONC) format. The module fully supports single-line `//` comments, multi-line block `/* ... */` comments, and trailing commas in objects and arrays.

---

## 1. Import: load std.jsonc

Importing the entire package:

```yukino
load std.jsonc
```

This brings the following entities into scope:

- **Node Type Constants:** `JSON_NULL`, `JSON_BOOL`, `JSON_NUMBER`, `JSON_STRING`, `JSON_ARRAY`, `JSON_OBJECT`.
- **Error Structures:** `JsonError`.
- **Classes:** `JsonValue`.
- **Parsing and Validation Functions:** `jsonc_parse`, `jsonc_try_parse`, `jsonc_validate`, `jsonc_strip_comments`.
- **Serialization Functions:** `jsonc_stringify`, `jsonc_pretty`, `jsonc_pretty_indent`.
- **Node Factory Functions:** `json_null`, `json_bool`, `json_number`, `json_int`, `json_string`, `json_array`, `json_object`.

All entities above are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.jsonc` module defines integer constants representing the node kind stored in `JsonValue.kind`:

```yukino
public static JSON_NULL: int32 = 0
public static JSON_BOOL: int32 = 1
public static JSON_NUMBER: int32 = 2
public static JSON_STRING: int32 = 3
public static JSON_ARRAY: int32 = 4
public static JSON_OBJECT: int32 = 5
```

These constants match those in `std.json`.

---

## 3. Interfaces

No interfaces are declared in the `std.jsonc` module.

---

## 4. Data Structures and Errors (Panics)

### 4.1 JsonError

The `JsonError` structure represents a syntax parsing error in a JSONC document. It is thrown via `panic` by `jsonc_parse` on syntax errors such as unclosed strings, unterminated block comments, or missing colons.

```yukino
public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: human-readable description of the error.
- `line`: 1-based line number in the source text.
- `column`: 1-based column number within that line.
- `offset`: 0-based byte offset from the start of the source string.

Example of handling the error using `sus / catch`:

```yukino
load std.jsonc: jsonc_parse, JsonError
load std.io: println

fun main() {
    sus {
        set doc = jsonc_parse("/* unterminated block comment")
    } catch err: JsonError {
        println("JSONC error: ${err.message} at line ${err.line}")
    }
}
```

---

## 5. Classes

### 5.1 JsonValue

The `std.jsonc` module uses the shared `JsonValue` class to represent the parsed document hierarchy:

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

All methods on `JsonValue` (`is_null`, `is_string`, `get`, `get_opt`, `at`, `at_opt`, `as_string`, `as_int`, `as_bool`, `as_float`, `has`, `keys`, `len`, `set`, `push`, `cp`) are identical to those documented in `std.json`.

#### Recommendation: Mapping Configuration Files into Typed Structures

JSONC is widely used for configuration files, build settings, and server parameters. It is strongly recommended to convert parsed `JsonValue` nodes into strongly typed configuration structures:

```yukino
load std.jsonc: jsonc_parse, JsonValue
load std.io: println

struct ServerConfig {
    host: string
    port: int64
    workers: int64
    debug: bool
}

fun server_config_from_json(j: JsonValue) -> ServerConfig {
    return ServerConfig {
        host: j.get("host").as_string_or("127.0.0.1"),
        port: j.get("port").as_int_or(8080),
        workers: j.get("workers").as_int_or(4),
        debug: j.get("debug").as_bool_or(false),
    }
}

fun main() {
    set raw_config = "{\n  // Network settings\n  \"host\": \"0.0.0.0\",\n  \"port\": 9000,\n  /* Enable debug */\n  \"debug\": true,\n}"
    set doc = jsonc_parse(raw_config)
    set cfg = server_config_from_json(doc)

    println("Server running on ${cfg.host}:${cfg.port}")
}
```

---

## 6. Function Reference

### 6.1 jsonc_parse

Parses an input string with support for comments (`//` and `/* */`) and trailing commas in objects and arrays, returning the root `JsonValue`.

```yukino
public fun jsonc_parse(text: string) -> JsonValue
```

Returns the parsed root `JsonValue`. Non-nullable.

Throws a panic with a `JsonError` structure on syntax errors, unclosed comments, or unexpected characters.

```yukino
load std.jsonc: jsonc_parse
load std.io: println

fun main() {
    set text = "{\n  // comment\n  \"env\": \"production\",\n}"
    set doc = jsonc_parse(text)
    println("Environment: " + doc.get("env").as_string())
}
```

### 6.2 jsonc_try_parse

Attempts to parse a JSONC string safely without panicking on errors.

```yukino
public fun jsonc_try_parse(text: string) -> JsonValue?
```

Returns the root `JsonValue` on success, or `null` if the input is malformed. Nullable.

The function never panics.

```yukino
load std.jsonc: jsonc_try_parse
load std.io: println

fun main() {
    set res = jsonc_try_parse("{\n  // broken json\n  \"a\": \n}")
    if res == null {
        println("Could not parse configuration")
    }
}
```

### 6.3 jsonc_validate

Validates whether a string is a syntactically valid JSONC document.

```yukino
public fun jsonc_validate(text: string) -> bool
```

Returns `true` if syntax is valid, or `false` otherwise. Non-nullable `bool`.

The function never panics.

```yukino
load std.jsonc: jsonc_validate
load std.io: println

fun main() {
    set valid = jsonc_validate("// config\n{\"port\": 3000,}")
    if valid {
        println("Configuration is valid")
    }
}
```

### 6.4 jsonc_strip_comments

Removes all single-line (`//`) and multi-line (`/* */`) comments from a JSONC source string, replacing them with spaces and preserving newline characters `\n`. This allows passing the cleaned text to strict standard JSON parsers while preserving exact line and column diagnostic locations.

```yukino
public fun jsonc_strip_comments(text: string) -> string
```

Returns the cleaned text string. Non-nullable.

The function never panics.

```yukino
load std.jsonc: jsonc_strip_comments
load std.json: json_parse
load std.io: println

fun main() {
    set text_with_comments = "{\n  // User setting\n  \"key\": \"value\"\n}"
    set clean_json = jsonc_strip_comments(text_with_comments)
    set doc = json_parse(clean_json)
    println("Key: " + doc.get("key").as_string())
}
```

### 6.5 jsonc_stringify

Serializes a `JsonValue` into a compact standard JSON string without comments or formatting.

```yukino
public fun jsonc_stringify(val: JsonValue) -> string
```

Returns a compact JSON string. Non-nullable.

The function never panics.

```yukino
load std.jsonc: jsonc_parse, jsonc_stringify
load std.io: println

fun main() {
    set doc = jsonc_parse("// settings\n{\"a\": 1, \"b\": 2,}")
    println(jsonc_stringify(doc))
}
```

### 6.6 jsonc_pretty

Serializes a `JsonValue` into an indented JSON string using 2 spaces per indentation level.

```yukino
public fun jsonc_pretty(val: JsonValue) -> string
```

Returns an indented JSON string. Non-nullable.

The function never panics.

```yukino
load std.jsonc: jsonc_parse, jsonc_pretty
load std.io: println

fun main() {
    set doc = jsonc_parse("{\"enabled\": true,}")
    println(jsonc_pretty(doc))
}
```

### 6.7 jsonc_pretty_indent

Serializes a `JsonValue` into a formatted JSON string with a custom indentation unit (e.g. 4 spaces or `"\t"`).

```yukino
public fun jsonc_pretty_indent(val: JsonValue, indent: string) -> string
```

Returns a formatted JSON string. Non-nullable.

The function never panics.

```yukino
load std.jsonc: jsonc_parse, jsonc_pretty_indent
load std.io: println

fun main() {
    set doc = jsonc_parse("{\"name\": \"config\"}")
    println(jsonc_pretty_indent(doc, "    "))
}
```

---

[← Previous: Module std.json](20-json.md) | [Next: Module std.outf →](22-outf.md)
