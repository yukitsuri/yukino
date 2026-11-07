# Module std.env

The `std.env` module provides cross-platform utilities for inspecting, setting, validating, and removing environment variables of the current process, loading all environment variables into typed structures or hash maps, and parsing `.env` (dotenv) configuration files with support for quoting, escape sequences, variable interpolation, and default values.

---

## 1. Import: load std.env

Importing the entire package:

```yukino
load std.env
```

When imported, the following entities become available:

- **Data and Error Structures:** `EnvEntry`, `EnvError`.
- **Variable Access and Mutation Functions:** `get_env`, `env`, `get_or`, `get_int`, `get_bool`, `get_float`, `require`, `require_env`, `set_env`, `remove_env`, `remove`, `has_env`, `has`.
- **Batch Retrieval Functions:** `all_entries`, `all`, `all_envs`.
- **Dotenv Functions:** `parse_dotenv`, `load_env`, `load_override`, `dotenv`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in module `std.env`.

---

## 3. Interfaces

No interfaces are declared in module `std.env`.

---

## 4. Data and Error Structures (Panic)

### 4.1 EnvEntry

A key-value pair structure representing an environment variable or a parsed dotenv line.

```yukino
public struct EnvEntry {
    key: string
    value: string
}
```

### 4.2 EnvError

An error structure raised when a required environment variable is not defined.

```yukino
public struct EnvError {
    message: string
    key: string
}
```

---

## 5. Classes

No classes are declared in module `std.env`.

---

## 6. Function Reference

### 6.1 get_env

Retrieves the string value of an environment variable by name. The function `env` is an alias of `get_env`.

```yukino
public fun get_env(key: string) -> string?
public fun env(key: string) -> string?
```

Returns a string value of type `string`, or `null` if the specified variable is not set in the current process.

This function never panics.

```yukino
load std.env: get_env
load std.io: println

set path_var = get_env("PATH")
if path_var != null {
    println("PATH: " + (path_var ?? ""))
}
```

### 6.2 get_or

Retrieves the value of an environment variable, returning the specified fallback default if not found.

```yukino
public fun get_or(key: string, default_val: string) -> string
```

Returns a non-nullable string value of type `string`.

This function never panics.

```yukino
load std.env: get_or
load std.io: println

set host = get_or("APP_HOST", "127.0.0.1")
println("Host: " + host)
```

### 6.3 get_int

Retrieves an environment variable and parses it into a 64-bit integer. If the variable is not set or cannot be parsed as an integer, returns `default_val` (default is `0`).

```yukino
public fun get_int(key: string, default_val: int64 = 0) -> int64
```

Returns a numeric value of type `int64`.

This function never panics.

```yukino
load std.env: get_int
load std.io: println

set port = get_int("APP_PORT", 8080)
println("Port: ${port}")
```

### 6.4 get_bool

Retrieves an environment variable and parses it into a boolean value. Values `"true"`, `"1"`, `"yes"`, `"on"` (case-insensitive) are parsed as `true`; values `"false"`, `"0"`, `"no"`, `"off"` are parsed as `false`. Returns `default_val` (default is `false`) if the variable is missing or invalid.

```yukino
public fun get_bool(key: string, default_val: bool = false) -> bool
```

Returns a boolean value of type `bool`.

This function never panics.

```yukino
load std.env: get_bool
load std.io: println

set is_debug = get_bool("APP_DEBUG", false)
if is_debug {
    println("Debug mode is active")
}
```

### 6.5 get_float

Retrieves an environment variable and parses it into a 64-bit floating point number. Returns `default_val` (default is `0.0`) if the variable is missing or malformed.

```yukino
public fun get_float(key: string, default_val: float64 = 0.0) -> float64
```

Returns a floating point value of type `float64`.

This function never panics.

```yukino
load std.env: get_float
load std.io: println

set rate = get_float("SAMPLE_RATE", 1.0)
println("Sample rate: ${rate}")
```

### 6.6 require

Validates that an environment variable is defined and returns its string value. The function `require_env` is an alias of `require`.

```yukino
public fun require(key: string) -> string
public fun require_env(key: string) -> string
```

Returns a string value of type `string`.

If the variable is not set in the current process, this function panics with the `EnvError` structure.

```yukino
load std.env: require
load std.io: println

set secret = require("DATABASE_URL")
println("Connection initialized")
```

### 6.7 set_env

Sets or overwrites the value of an environment variable for the current process and future child processes.

```yukino
public fun set_env(key: string, val: string) -> bool
```

Returns `true` after the value is set; `false` is returned only when the variable name is empty.

This function never panics.

```yukino
load std.env: set_env
load std.io: println

if set_env("SERVICE_NAME", "gateway") {
    println("SERVICE_NAME configured")
}
```

### 6.8 remove_env

Removes an environment variable from the current process. The function `remove` is an alias of `remove_env`.

```yukino
public fun remove_env(key: string) -> bool
public fun remove(key: string) -> bool
```

Returns `true` when the variable name is non-empty (including the case where the variable was not set), or `false` for an empty name.

This function never panics.

```yukino
load std.env: remove_env
load std.io: println

if remove_env("TEMPORARY_TOKEN") {
    println("Token removed from environment")
}
```

### 6.9 has_env

Checks whether an environment variable with the specified key is present. The function `has` is an alias of `has_env`.

```yukino
public fun has_env(key: string) -> bool
public fun has(key: string) -> bool
```

Returns a boolean value of type `bool`.

This function never panics.

```yukino
load std.env: has_env
load std.io: println

if has_env("CI") {
    println("Running in continuous integration")
}
```

### 6.10 all_entries

Retrieves all environment variables of the current process as an array of `EnvEntry` structures.

```yukino
public fun all_entries() -> EnvEntry[]
```

Returns an array of `EnvEntry[]`.

This function never panics.

```yukino
load std.env: all_entries
load std.io: println

set entries = all_entries()
for i in 0..entries.len() {
    println(entries[i].key + " = " + entries[i].value)
}
```

### 6.11 all

Retrieves all environment variables of the current process into a `HashMap<string, string>`. The function `all_envs` is an alias of `all`.

```yukino
public fun all() -> HashMap<string, string>
public fun all_envs() -> HashMap<string, string>
```

Returns an associative map of type `HashMap<string, string>`.

This function never panics.

```yukino
load std.env: all
load std.io: println

set env_map = all()
set user = env_map.get("USER")
if user != null {
    println("Current user: " + (user ?? ""))
}
```

### 6.12 parse_dotenv

Parses `.env` file content into an array of `EnvEntry` items. Supports single and multiline values with single or double quotes, escape sequences (`\n`, `\r`, `\t`, `\\`, `\"`, `\$`), blank line skipping and comments starting with `#`, `export ` prefix, variable interpolation `${VAR}`, and fallback expressions `${VAR:-default}`.

```yukino
public fun parse_dotenv(content: string) -> EnvEntry[]
```

Returns an array of `EnvEntry[]`.

This function never panics.

```yukino
load std.env: parse_dotenv
load std.io: println

// \x24 — the "$" character as an escape sequence,
// so that Yukino string interpolation does not trigger inside the literal
set raw = "HOST=127.0.0.1\nPORT=9000\nURL=http://\x24{HOST}:\x24{PORT}\n"
set items = parse_dotenv(raw)
for i in 0..items.len() {
    println(items[i].key + " -> " + items[i].value)
}
```

### 6.13 load_env

Loads environment variables from a dotenv file (default `".env"`), setting them in the current process only if they are not already set. The function `dotenv` is an alias of `load_env`.

```yukino
public fun load_env(path: string = ".env") -> bool
public fun dotenv(path: string = ".env") -> bool
```

Returns `true` if the file exists and was read successfully, or `false` if the file does not exist.

This function never panics.

```yukino
load std.env: load_env, get_env
load std.io: println

if load_env(".env.local") {
    println("Config loaded from .env.local")
    println("Database: " + (get_env("DB_NAME") ?? "unset"))
}
```

### 6.14 load_override

Loads environment variables from a dotenv file and unconditionally overrides existing process environment variables with values from the file.

```yukino
public fun load_override(path: string = ".env") -> bool
```

Returns `true` if the file exists and was read successfully, or `false` if the file does not exist.

This function never panics.

```yukino
load std.env: load_override, get_env
load std.io: println

if load_override(".env.test") {
    println("Test environment override applied")
}
```

---

[← Previous: Module std.sysinfo](12-sysinfo.md) | [Next: Module std.simpleurl →](14-simpleurl.md)
