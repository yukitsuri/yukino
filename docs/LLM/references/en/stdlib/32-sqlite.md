# Module std.sqlite

The `std.sqlite` module provides comprehensive and high-performance tools for working with the SQLite 3 embedded relational database engine. The module includes connection management (in-file and in-memory), prepared statements, parameterized queries with SQL-injection protection, transactions, and safe result navigation via the Fluent API of `SqliteRow` and `SqliteValue` classes, designed in accordance with the `std.json` and `std.jsonc` modules.

---

## 1. Import: load std.sqlite

Importing the entire package:

```yukino
load std.sqlite
```

The following entities become available:

- **Constants:** `SQLITE_OK`, `SQLITE_ERROR`, `SQLITE_ROW`, `SQLITE_DONE`, `SQLITE_INTEGER`, `SQLITE_FLOAT`, `SQLITE_TEXT`, `SQLITE_BLOB`, `SQLITE_NULL`, `SQLITE_OPEN_READONLY`, `SQLITE_OPEN_READWRITE`, `SQLITE_OPEN_CREATE`, `SQLITE_OPEN_MEMORY`.
- **Error Structures:** `SqliteError`.
- **Classes:** `SqliteConnection`, `SqliteStatement`, `SqliteRow`, `SqliteValue`.
- **Connection Management Functions:** `open`, `try_open`, `open_memory`, `open_with_flags`, `try_open_with_flags`, `sqlite_open`, `sqlite_try_open`, `sqlite_open_memory`, `sqlite_version`.
- **Factory Construction Functions:** `sqlite_null`, `sqlite_int`, `sqlite_float`, `sqlite_string`, `sqlite_text`, `sqlite_blob`, `sqlite_bool`.

All listed entities automatically become accessible when loading the whole standard library: `load std`.

---

## 2. Constants

The `std.sqlite` module declares integer constants for result codes, storage classes, and database open flags:

```yukino
public static SQLITE_OK: int32 = 0
public static SQLITE_ERROR: int32 = 1
public static SQLITE_ROW: int32 = 100
public static SQLITE_DONE: int32 = 101

public static SQLITE_INTEGER: int32 = 1
public static SQLITE_FLOAT: int32 = 2
public static SQLITE_TEXT: int32 = 3
public static SQLITE_BLOB: int32 = 4
public static SQLITE_NULL: int32 = 5

public static SQLITE_OPEN_READONLY: int32 = 1
public static SQLITE_OPEN_READWRITE: int32 = 2
public static SQLITE_OPEN_CREATE: int32 = 4
public static SQLITE_OPEN_MEMORY: int32 = 128
```

Data type constants reflect SQLite storage classes and are used to inspect the `kind` property of a `SqliteValue`.

---

## 3. Interfaces

No interfaces are declared in the `std.sqlite` module.

---

## 4. Data and Panic Structures

### 4.1 SqliteError

The `SqliteError` structure represents an execution error during a query, statement preparation failure, integrity constraint violation, or database I/O error.

```yukino
public struct SqliteError {
    message: string
    code: int32
}
```

- `message`: human-readable description of the error returned by the SQLite engine or Yukino runtime.
- `code`: integer error code from SQLite (for example, `1` for syntax error, `19` for `SQLITE_CONSTRAINT`).

Example of catching an error safely using `sus / catch`:

```yukino
load std.sqlite: open, SqliteConnection, SqliteError
load std.io: println

fun main() {
    set db = open(":memory:")
    sus {
        set _ = db.execute("SELECT * FROM non_existent_table;")
    } catch err: SqliteError {
        println("Caught SQLite error: " + err.message + " (code: " + cast(err.code, "string") + ")")
    }
    db.close()
}
```

---

## 5. Classes

### 5.1 SqliteValue

The `SqliteValue` class represents a typed database column value (null, integer, floating-point number, text, or binary blob). Its public API mirrors `JsonValue` from `std.json`, giving developers familiar data extraction and fallback methods.

```yukino
public class SqliteValue {
    public kind: int32
    public int_val: int64
    public float_val: float64
    public str_val: string
    public blob_val: uint8[]
}
```

#### Predicate Methods

- `is_null() -> bool`: returns `true` if the value is `NULL`.
- `is_int() -> bool`: returns `true` if the value is an integer.
- `is_float() -> bool`: returns `true` if the value is a floating-point number.
- `is_number() -> bool`: returns `true` if the value is either integer or float.
- `is_string() -> bool` (also `is_text() -> bool`): returns `true` if the value is a text string.
- `is_blob() -> bool`: returns `true` if the value is a binary byte array.

#### Extraction Methods

- `as_int() -> int64`: returns an integer. Floats are cast to `int64`. Returns `0` for other types.
- `as_int_or(fallback: int64) -> int64`: returns an integer or `fallback` if not numeric.
- `as_float() -> float64`: returns a float. Integers are cast to `float64`. Returns `0.0` for other types.
- `as_float_or(fallback: float64) -> float64`: returns a float or `fallback`.
- `as_string() -> string`: returns a string representation. Automatically formats numbers as strings; returns `""` for other non-text types.
- `as_string_or(fallback: string) -> string`: returns a string or `fallback`.
- `as_bool() -> bool`: returns a boolean. Zero numbers and empty strings return `false`; non-zero and non-empty values return `true`. Returns `false` for `NULL`.
- `as_bool_or(fallback: bool) -> bool`: returns a boolean or `fallback`.
- `as_blob() -> uint8[]`: returns a byte array. Returns an empty array if not a blob.

---

### 5.2 SqliteRow

The `SqliteRow` class encapsulates a single row from a query result set. Column access follows the Fluent API pattern: accessing an invalid index or missing column name returns a safe `SqliteValue` of kind `NULL` without panicking.

```yukino
public class SqliteRow {
    public col_names: string[]
    public col_vals: SqliteValue[]
}
```

#### Row Access Methods

- `get(key: string) -> SqliteValue`: returns the column value by its name `key`. Returns a `NULL` node if missing, enabling chains like `row.get("age").as_int_or(18)`.
- `get_opt(key: string) -> SqliteValue?`: returns a nullable `SqliteValue?` or `null` if the column does not exist.
- `at(index: int64) -> SqliteValue`: returns the column value by zero-based index. Returns a `NULL` node if the index is out of bounds.
- `at_opt(index: int64) -> SqliteValue?`: returns a nullable `SqliteValue?` or `null` if the index is out of bounds.
- `has(key: string) -> bool`: returns `true` if the row contains a column named `key`.
- `keys() -> string[]`: returns an array of all column names in the row.
- `values() -> SqliteValue[]`: returns an array of all column values in order.
- `len() -> int64`: returns the number of columns in the row.

---

### 5.3 SqliteStatement

The `SqliteStatement` class encapsulates a compiled SQLite prepared statement. It allows binding parameters, stepping through result rows, and reusing queries.

```yukino
public class SqliteStatement {
    public db_handle: ptr<void>
    public handle: ptr<void>
    public is_finalized: bool
}
```

#### Statement Methods

- `step() -> bool`: advances to the next result row. Returns `true` if a row is available, or `false` when finished. Panics with `SqliteError` upon failure.
- `current_row() -> SqliteRow`: extracts column data for the current row into a `SqliteRow` instance. Not nullable.
- `reset()`: resets the statement execution state for reuse while keeping bound parameters.
- `finalize()`: frees the statement handle in memory.
- `bind_null(index: int32)`: binds `NULL` to the 1-based parameter index.
- `bind_int(index: int32, val: int64)`: binds a 64-bit integer to parameter index.
- `bind_float(index: int32, val: float64)`: binds a floating-point number to parameter index.
- `bind_string(index: int32, val: string)`: binds a string to parameter index.
- `bind_blob(index: int32, val: uint8[])`: binds a byte array to parameter index.
- `bind_value(index: int32, val: SqliteValue)`: dynamically binds a `SqliteValue` according to its kind.
- `bind_params(params: SqliteValue[])`: sequentially binds an array of parameters to positional `?` placeholders (starting at index 1).

---

### 5.4 SqliteConnection

The `SqliteConnection` class represents an open SQLite database connection. It provides methods for executing DDL/DML statements, parameterized queries, transaction control, and metadata inspection.

```yukino
public class SqliteConnection {
    public handle: ptr<void>
    public path: string
    public is_closed: bool
}
```

#### Connection Methods

- `execute(sql: string) -> int64`: executes a DDL/DML command (such as `CREATE TABLE`, `INSERT`, `UPDATE`, `DELETE`) and returns the number of affected rows. Panics with `SqliteError` on failure.
- `execute_params(sql: string, params: SqliteValue[]) -> int64`: executes a parameterized command with parameter values bound safely.
- `query(sql: string) -> SqliteRow[]`: executes a `SELECT` query and returns all matching rows as an array `SqliteRow[]`. Panics with `SqliteError` on failure.
- `query_params(sql: string, params: SqliteValue[]) -> SqliteRow[]`: executes a parameterized `SELECT` query with bound parameters.
- `query_row(sql: string) -> SqliteRow`: returns the first row of a query. Panics with `SqliteError` (code 101) if the result set is empty.
- `query_row_opt(sql: string) -> SqliteRow?`: returns the first row of a query or `null` if empty. Never panics on empty results.
- `prepare(sql: string) -> SqliteStatement`: compiles an SQL query into a prepared statement.
- `changes() -> int64`: returns the number of rows modified by the most recent `INSERT`, `UPDATE`, or `DELETE`.
- `last_insert_rowid() -> int64`: returns the `ROWID` of the last inserted row.
- `begin()`: begins an explicit transaction (`BEGIN`).
- `commit()`: commits the active transaction (`COMMIT`).
- `rollback()`: rolls back the active transaction (`ROLLBACK`).
- `close()`: closes the database connection and frees associated system resources.

---

## 6. Function Reference

### 6.1 open

Opens a SQLite database at the specified file path or creates a new one if it does not exist. Passing `":memory:"` creates a purely in-memory database.

```yukino
public fun open(path: string) -> SqliteConnection
```

Returns a `SqliteConnection` instance. The return value is not nullable.

Panics with a `SqliteError` structure if the database file cannot be opened.

```yukino
load std.sqlite: open, SqliteConnection
load std.io: println

fun main() {
    set db = open("app.db")
    println("Database opened successfully")
    db.close()
}
```

### 6.2 try_open

Attempts to open a SQLite database safely without panicking on failure.

```yukino
public fun try_open(path: string) -> SqliteConnection?
```

Returns a `SqliteConnection` instance on success or `null` if the database cannot be opened. The return value is nullable.

This function never panics.

```yukino
load std.sqlite: try_open, SqliteConnection
load std.io: println

fun main() {
    set maybe_db = try_open("invalid/path/db.sqlite")
    if maybe_db == null {
        println("Could not open database at path")
    }
}
```

### 6.3 open_memory

Creates and opens an isolated in-memory SQLite database (`:memory:`).

```yukino
public fun open_memory() -> SqliteConnection
```

Returns a `SqliteConnection` instance. The return value is not nullable.

Panics with a `SqliteError` structure in case of memory allocation failure.

```yukino
load std.sqlite: open_memory, SqliteConnection
load std.io: println

fun main() {
    set mem_db = open_memory()
    set _ = mem_db.execute("CREATE TABLE config (key TEXT, val TEXT);")
    println("In-memory table created")
    mem_db.close()
}
```

### 6.4 open_with_flags

Opens a SQLite database with explicit low-level open flags (e.g. `SQLITE_OPEN_READONLY` or `SQLITE_OPEN_READWRITE`).

```yukino
public fun open_with_flags(path: string, flags: int32) -> SqliteConnection
```

Returns a `SqliteConnection` instance. The return value is not nullable.

Panics with a `SqliteError` structure if the database cannot be opened with the specified flags.

```yukino
load std.sqlite: open_with_flags, SQLITE_OPEN_READONLY, SqliteConnection
load std.io: println

fun main() {
    set db = open_with_flags("readonly.db", SQLITE_OPEN_READONLY)
    println("Database opened in read-only mode")
    db.close()
}
```

### 6.5 try_open_with_flags

Safely attempts to open a database with specified flags without panicking.

```yukino
public fun try_open_with_flags(path: string, flags: int32) -> SqliteConnection?
```

Returns a `SqliteConnection` instance on success or `null` on failure. The return value is nullable.

This function never panics.

```yukino
load std.sqlite: try_open_with_flags, SQLITE_OPEN_READONLY, SqliteConnection
load std.io: println

fun main() {
    set db = try_open_with_flags("missing.db", SQLITE_OPEN_READONLY)
    if db == null {
        println("Read-only database not accessible")
    }
}
```

### 6.6 sqlite_open, sqlite_try_open, sqlite_open_memory

Prefixed aliases for connection management functions. They prevent name collisions in scopes where other file I/O or network modules are also imported.

```yukino
public fun sqlite_open(path: string) -> SqliteConnection
public fun sqlite_try_open(path: string) -> SqliteConnection?
public fun sqlite_open_memory() -> SqliteConnection
```

Return values and panic behavior are identical to `open`, `try_open`, and `open_memory`.

```yukino
load std.sqlite: sqlite_open_memory, SqliteConnection
load std.io: println

fun main() {
    set db = sqlite_open_memory()
    println("Database opened via prefixed function")
    db.close()
}
```

### 6.7 sqlite_version

Returns the version string of the embedded SQLite engine (for example, `"3.53.2"`).

```yukino
public fun sqlite_version() -> string
```

Returns a string of type `string`. The return value is not nullable.

This function never panics.

```yukino
load std.sqlite: sqlite_version
load std.io: println

fun main() {
    println("SQLite version: " + sqlite_version())
}
```

### 6.8 Factory Functions for Constructing SqliteValue

The module provides helper functions for creating query parameters:

- `sqlite_null() -> SqliteValue`: creates a `NULL` value.
- `sqlite_int(val: int64) -> SqliteValue`: creates an integer value.
- `sqlite_float(val: float64) -> SqliteValue`: creates a floating-point number value.
- `sqlite_string(val: string) -> SqliteValue` (also `sqlite_text`): creates a text value.
- `sqlite_blob(val: uint8[]) -> SqliteValue`: creates a binary blob value.
- `sqlite_bool(val: bool) -> SqliteValue`: creates an integer `1` (`true`) or `0` (`false`).

All factory functions return a `SqliteValue` instance and never panic.

```yukino
load std.sqlite: open_memory, sqlite_int, sqlite_string, sqlite_float, SqliteValue, SqliteConnection
load std.io: println

fun main() {
    set db = open_memory()
    set _ = db.execute("CREATE TABLE products (id INTEGER, name TEXT, price REAL);")
    
    set params: SqliteValue[] = [sqlite_int(101), sqlite_string("Keyboard"), sqlite_float(49.99)]
    set _ = db.execute_params("INSERT INTO products VALUES (?, ?, ?);", params)
    
    set row = db.query_row("SELECT name, price FROM products WHERE id = 101;")
    println("Product: " + row.get("name").as_string() + ", Price: " + cast(row.get("price").as_float(), "string"))
    db.close()
}
```

---

[← Previous: Module std.yaml](31-yaml.md) | [Next: Module std.encore →](33-encore.md)
