# Module std.io

The `std.io` module provides standard stream and console input-output functionality: output to stdout and stderr, reading lines and bytes from stdin, thread lock management, interactive terminal detection, cursor control, and raw mode management.

---

## 1. Import: load std.io

Importing the entire package:

```yukino
load std.io
```

All public elements of the `std.io` module are imported into the current scope:

- **Stream interfaces:**
  - `Reader<T>` — contract for reading items of type `T` via raw buffer `ptr<T>`;
  - `Writer<T>` — contract for writing items of type `T` and flushing the buffer;
  - `Stream<T>` — combined contract for reading, writing, flushing, and closing a resource;
  - `Seekable` — contract for stream positioning.
- **Structures:**
  - `IoError` — input-output error structure (codes `101`–`108`).
- **Input-output functions:**
  - *Output to stdout / stderr:* `print`, `println`, `eprint`, `eprintln`, `flush`, `eflush`, `write_string`, `ewrite_string`;
  - *Reading from stdin:* `read_line`, `read_all`, `read_char`, `discard_line`, `read_exact`;
  - *Asynchronous and background polling:* `poll_readable`, `enable_background_reader`, `read_char_async`;
  - *Stream locks:* `lock_stdout`, `unlock_stdout`, `lock_stdin`, `unlock_stdin`;
  - *Terminal management:* `is_terminal`, `is_stdin_terminal`, `is_stdout_terminal`, `is_stderr_terminal`, `terminal_size`, `terminal_tuple`, `terminal_cols`, `terminal_rows`, `set_raw_mode`, `clear_screen`, `cursor_move_to`, `hide_cursor`, `show_cursor`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.io` module does not declare global constants.

---

## 3. Interfaces

### 3.1 Reader<T>

A generic contract for objects that support low-level reading of type `T` elements into memory by pointer.

```yukino
public interface Reader<T> {
    fun read(buf: ptr<T>, max_len: int64) -> int64
}
```

### 3.2 Writer<T>

A generic contract for objects that support writing type `T` elements and forcibly flushing the buffer.

```yukino
public interface Writer<T> {
    fun write(data: T) -> int64
    fun flush() -> int32
}
```

### 3.3 Stream<T>

A combined generic interface for stream resources that support reading type `T` elements, writing type `T` elements, and closing.

```yukino
public interface Stream<T> {
    fun read(buf: ptr<T>, max_len: int64) -> int64
    fun write(data: T) -> int64
    fun flush() -> int32
    fun close() -> int32
}
```

### 3.4 Seekable

A contract for resources that support repositioning the read/write pointer.

```yukino
public interface Seekable {
    fun seek(offset: int64, whence: int32) -> int64
}
```

---

## 4. Data Structures and Errors

### 4.1 IoError

The error structure raised on system failures of input-output operations.

```yukino
public struct IoError {
    message: string
    code: int32
}
```

- `code: 101` — line reading error from stdin (`read_line`);
- `code: 102` — full stdin read error (`read_all`);
- `code: 103` — failure writing data to stdout (`write_string`);
- `code: 104` — failure writing data to stderr (`ewrite_string`);
- `code: 105` — stdout buffer flush failure (`flush`);
- `code: 106` — stderr buffer flush failure (`eflush`);
- `code: 107` — a null buffer pointer was passed to the function with a positive length (`read_exact`);
- `code: 108` — system error reading an exact number of bytes from stdin (`read_exact`).

---

## 5. Function Reference

### 5.1 print

Writes the string `msg` to the standard output `stdout` without appending a trailing newline character and without forcing a buffer flush; use the `flush` function when immediate output is required.

```yukino
public fun print(msg: string)
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: print

print("Loading: ")
print("100%\n")
```

### 5.2 println

Writes the string `msg` to the standard output `stdout` and appends a trailing newline character `\n`.

```yukino
public fun println(msg: string)
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: println

println("Hello, world!")
```

### 5.3 eprint

Writes the string `msg` to the standard error stream `stderr` without appending a trailing newline character.

```yukino
public fun eprint(msg: string)
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: eprint

eprint("Warning: low memory level")
```

### 5.4 eprintln

Writes the string `msg` to the standard error stream `stderr` with automatic appending of a trailing newline character `\n`.

```yukino
public fun eprintln(msg: string)
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: eprintln

eprintln("Critical configuration error")
```

### 5.5 flush

Forcibly flushes the internal `stdout` output buffer to the terminal or the associated file descriptor.

```yukino
public fun flush() -> int32
```

Returns `0` on successful buffer flush.

On a system buffer flush failure, panics with the `IoError` structure with code `105`.

```yukino
load std.io: print, flush

print("Processing data...")
flush()
```

### 5.6 eflush

Forcibly flushes the internal buffer of the standard error stream `stderr`.

```yukino
public fun eflush() -> int32
```

Returns `0` on successful buffer flush.

On a system buffer flush failure, panics with the `IoError` structure with code `106`.

```yukino
load std.io: eprint, eflush

eprint("Node failure!")
eflush()
```

### 5.7 write_string

Writes the string `msg` to the standard output `stdout` and returns the number of bytes actually transferred.

```yukino
public fun write_string(msg: string) -> int64
```

Returns the number of successfully written bytes as `int64`.

On a system data write failure, panics with the `IoError` structure with code `103`.

```yukino
load std.io: write_string, println

set written = write_string("data packet\n")
println("Bytes written: ${written}")
```

### 5.8 ewrite_string

Writes the string `msg` to the standard error stream `stderr` and returns the number of bytes transferred.

```yukino
public fun ewrite_string(msg: string) -> int64
```

Returns the number of bytes successfully written to the error stream as `int64`.

On a system write failure, panics with the `IoError` structure with code `104`.

```yukino
load std.io: ewrite_string

set written = ewrite_string("descriptor error\n")
```

### 5.9 read_line

Reads a single line from the standard input `stdin` up to the first newline character `\n` encountered. Returns the line read without newline characters. If the end of the input stream (EOF) is reached, returns an empty string `""`.

```yukino
public fun read_line() -> string
```

Returns the line read as `string`.

On a system read error from `stdin`, panics with the `IoError` structure with code `101`.

```yukino
load std.io: print, println, read_line

print("Enter your name: ")
set name = read_line()
println("Hello, ${name}")
```

### 5.10 read_all

Reads all available data from the standard input `stdin` until the end-of-file marker (EOF) is reached and returns it as a single string.

```yukino
public fun read_all() -> string
```

Returns all data read as a `string`.

On a system stream read error, panics with the `IoError` structure with code `102`.

```yukino
load std.io: read_all, println

set content = read_all()
println("Bytes received: ${content.len()}")
```

### 5.11 discard_line

Reads and discards all characters from the `stdin` input buffer up to the first newline character `\n` or the end-of-stream marker (EOF) encountered. Used to clear leftover user input after a partial read.

```yukino
public fun discard_line()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: read_char, discard_line

set choice = read_char(false)
discard_line()
```

### 5.12 read_char

Reads a single byte or character from the standard input `stdin`. If the `clean_buffer` flag is set to `true`, the remainder of the current line is read and discarded.

```yukino
public fun read_char(clean_buffer: bool = false) -> int32
```

Returns the numeric character code as `int32`, or `-1` when the end-of-file marker (EOF) is reached.

This function never panics.

```yukino
load std.io: read_char, println

set code = read_char(true)
if code != -1 {
    set ch: char = cast(code, "char")
    println("Character read: ${ch} (code: ${code})")
}
```

### 5.13 read_exact

Reads up to `n` bytes from the standard input `stdin` in a single read operation directly into a memory buffer via the raw pointer `buf`.

```yukino
public fun read_exact(buf: ptr<uint8>, n: int64) -> int64
```

Returns the actual number of bytes read as `int64`.

Panics with the `IoError` structure with code `107` if a null pointer `buf == null` is passed with `n > 0`. On a system read failure, panics with the `IoError` structure with code `108`.

```yukino
load std.io: read_exact

fun read_header(buffer_ptr: ptr<uint8>) -> int64 {
    return read_exact(buffer_ptr, 16)
}
```

### 5.14 is_stdin_terminal

Checks whether the standard input `stdin` (file descriptor 0) is connected to an interactive terminal (TTY).

```yukino
public fun is_stdin_terminal() -> bool
```

Returns `true` if the input is connected to an interactive terminal, otherwise `false` (when redirected from a file or a pipe).

This function never panics.

```yukino
load std.io: is_stdin_terminal, println

if is_stdin_terminal() {
    println("Interactive terminal input")
}
```

### 5.15 is_stdout_terminal

Checks whether the standard output `stdout` (file descriptor 1) is connected to an interactive terminal.

```yukino
public fun is_stdout_terminal() -> bool
```

Returns `true` if the standard output is directed to a terminal, otherwise `false` (when output goes to a file or a pipe).

This function never panics.

```yukino
load std.io: is_stdout_terminal, println

if is_stdout_terminal() {
    println("Output is directed to the terminal window")
}
```

### 5.16 is_stderr_terminal

Checks whether the standard error stream `stderr` (file descriptor 2) is connected to an interactive terminal.

```yukino
public fun is_stderr_terminal() -> bool
```

Returns `true` if the error stream is directed to an interactive terminal, otherwise `false`.

This function never panics.

```yukino
load std.io: is_stderr_terminal, eprintln

if is_stderr_terminal() {
    eprintln("Terminal supports color error codes")
}
```

### 5.17 is_terminal

Checks whether the program's standard output is a terminal. It is a short synonym for the `is_stdout_terminal()` function.

```yukino
public fun is_terminal() -> bool
```

Returns `true` if the standard output is directed to a terminal, otherwise `false`.

This function never panics.

```yukino
load std.io: is_terminal, println

if is_terminal() {
    println("Interactive mode")
}
```

### 5.18 poll_readable

Polls the standard input `stdin` for data ready to be read within the `timeout_ms` milliseconds timeout.

```yukino
public fun poll_readable(timeout_ms: int64 = 0) -> bool
```

Returns `true` if the input stream has data ready to be read without blocking the thread, otherwise `false`.

This function never panics.

```yukino
load std.io: poll_readable, read_line, println

if poll_readable(500) {
    set input = read_line()
    println("Received: ${input}")
}
```

### 5.19 enable_background_reader

Starts an internal background service thread for asynchronous non-blocking reading of bytes from the standard input.

```yukino
public fun enable_background_reader()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: enable_background_reader

enable_background_reader()
```

### 5.20 read_char_async

Performs a non-blocking read of a single byte from the internal queue of the background input service thread.

```yukino
public fun read_char_async() -> int32
```

Returns the numeric byte code as `int32`, or `-1` if no new data is present in the buffer.

This function never panics.

```yukino
load std.io: enable_background_reader, read_char_async, println

enable_background_reader()
set ch = read_char_async()
if ch != -1 {
    println("Key code: ${ch}")
}
```

### 5.21 lock_stdout

Acquires the internal mutex of the standard output `stdout`. Prevents interleaving of string fragments during parallel output from multiple system threads.

```yukino
public fun lock_stdout()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: lock_stdout, unlock_stdout, print

lock_stdout()
print("Section 1: ")
print("OK\n")
unlock_stdout()
```

### 5.22 unlock_stdout

Releases the internal mutex of the standard output `stdout`, allowing other threads to continue writing to the stream.

```yukino
public fun unlock_stdout()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: lock_stdout, unlock_stdout, println

lock_stdout()
println("Atomic message")
unlock_stdout()
```

### 5.23 lock_stdin

Acquires the internal mutex of the standard input `stdin`, eliminating a race condition during parallel reading of data from multiple threads.

```yukino
public fun lock_stdin()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: lock_stdin, unlock_stdin, read_line

lock_stdin()
set s = read_line()
unlock_stdin()
```

### 5.24 unlock_stdin

Releases the previously acquired mutex of the standard input `stdin`.

```yukino
public fun unlock_stdin()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: lock_stdin, unlock_stdin, read_char

lock_stdin()
set ch = read_char(false)
unlock_stdin()
```

### 5.25 terminal_tuple

Queries the operating system for the current terminal window dimensions and returns a tuple consisting of the number of columns and rows.

```yukino
public fun terminal_tuple() -> (int32, int32)
```

Returns a tuple `(int32, int32)`, where the first value is the width (`cols`) and the second is the height (`rows`). If the dimensions cannot be determined, the default values `(80, 24)` are returned.

This function never panics.

```yukino
load std.io: terminal_tuple, println

set (cols, rows) = terminal_tuple()
println("Window size: ${cols}x${rows}")
```

### 5.26 terminal_size

Returns the current dimensions of the text terminal as an anonymous tuple `(int32, int32)`.

```yukino
public fun terminal_size() -> (int32, int32)
```

Returns a `(cols, rows)` tuple, where `sz.0` is the number of columns (width) and `sz.1` is the number of rows (height). If the operating system did not return dimensions, default values `cols = 80, rows = 24` are used.

This function never panics.

```yukino
load std.io: terminal_size, println

set sz = terminal_size()
println("Terminal resolution: ${sz.0}x${sz.1}")
```

### 5.27 terminal_cols

Returns the width of the current terminal screen in character cells (columns).

```yukino
public fun terminal_cols() -> int32
```

Returns the number of terminal columns as `int32` (`80` by default).

This function never panics.

```yukino
load std.io: terminal_cols, println

set width = terminal_cols()
println("Window width: ${width}")
```

### 5.28 terminal_rows

Returns the height of the current terminal screen in character cells (rows).

```yukino
public fun terminal_rows() -> int32
```

Returns the number of terminal rows as `int32` (`24` by default).

This function never panics.

```yukino
load std.io: terminal_rows, println

set height = terminal_rows()
println("Window height: ${height}")
```

### 5.29 set_raw_mode

Switches the terminal operating mode between canonical and raw mode. In raw mode, line buffering of input and echo printing are disabled, allowing individual key presses to be captured without waiting for the Enter key.

```yukino
public fun set_raw_mode(enable: bool) -> bool
```

Returns `true` if the terminal mode was changed successfully, otherwise `false`.

This function never panics.

```yukino
load std.io: set_raw_mode, read_char

set_raw_mode(true)
set key = read_char(false)
set_raw_mode(false)
```

### 5.30 clear_screen

Clears the terminal screen and moves the text cursor to the top-left corner (coordinates 1;1) by sending the ANSI sequence `\x1b[2J\x1b[H`.

```yukino
public fun clear_screen()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: clear_screen, println

clear_screen()
println("Terminal screen cleared")
```

### 5.31 cursor_move_to

Moves the terminal text cursor to the absolute position at the specified column `col` and row `row` coordinates, using the ANSI sequence `\x1b[row;colH`.

```yukino
public fun cursor_move_to(col: int32, row: int32)
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: cursor_move_to, print

cursor_move_to(10, 5)
print("Text at row 5, column 10")
```

### 5.32 hide_cursor

Hides the blinking text cursor in the terminal window using the ANSI command `\x1b[?25l`.

```yukino
public fun hide_cursor()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: hide_cursor

hide_cursor()
```

### 5.33 show_cursor

Makes the text cursor visible in the terminal window using the ANSI command `\x1b[?25h`.

```yukino
public fun show_cursor()
```

The function returns nothing (`void`).

This function never panics.

```yukino
load std.io: show_cursor

show_cursor()
```

---

[← Back: Module std.strings](1-strings.md) | [Next: Module std.threads →](3-threads.md)
