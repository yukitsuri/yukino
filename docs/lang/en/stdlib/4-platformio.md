# Module std.platformio

The `std.platformio` module provides direct low-level access to the input-output system APIs of specific target operating systems: POSIX (Unix), Linux-specific zero-copy data system calls, and the Win32 API for Windows.

---

## 1. Import: load std.platformio

Importing the entire package:

```yukino
load std.platformio
```

Constants, structures, and platform input-output functions are imported into the current scope:

- **Standard handle constants:**
  - *POSIX:* `STDIN_FILENO`, `STDOUT_FILENO`, `STDERR_FILENO`;
  - *Windows:* `STD_INPUT_HANDLE`, `STD_OUTPUT_HANDLE`, `STD_ERROR_HANDLE`.
- **Handle structures:**
  - `PipeFds` — a pair of Unix/Linux pipe file descriptors (`read_fd`, `write_fd`);
  - `WinPipeHandles` — a pair of Windows pipe system handles (`read_handle`, `write_handle`);
  - `IoError` — system input-output error structure (codes `201`–`210`).
- **POSIX (Unix) functions:**
  - `isatty`, `pipe`, `dup`, `dup2`, `fsync`, `fdatasync`.
- **Linux functions:**
  - `pipe2`, `dup3`, `splice`, `tee`.
- **Windows (Win32) functions:**
  - `get_std_handle`, `is_console`, `get_console_mode`, `set_console_mode`, `create_pipe`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

### 2.1 POSIX handle constants

```yukino
public static STDIN_FILENO: int32 = 0
public static STDOUT_FILENO: int32 = 1
public static STDERR_FILENO: int32 = 2
```

- `STDIN_FILENO` (0) — the standard input file descriptor.
- `STDOUT_FILENO` (1) — the standard output file descriptor.
- `STDERR_FILENO` (2) — the standard error output file descriptor.

### 2.2 Windows handle constants

```yukino
public static STD_INPUT_HANDLE: int32 = -10
public static STD_OUTPUT_HANDLE: int32 = -11
public static STD_ERROR_HANDLE: int32 = -12
```

- `STD_INPUT_HANDLE` (-10) — the Win32 console standard input device identifier.
- `STD_OUTPUT_HANDLE` (-11) — the Win32 console standard output device identifier.
- `STD_ERROR_HANDLE` (-12) — the Win32 standard error output device identifier.

---

## 3. Data Structures and Errors

### 3.1 PipeFds (Unix / Linux)

Holds a pair of file descriptors of an anonymous pipe.

```yukino
public struct PipeFds {
    read_fd: int32
    write_fd: int32
}
```

- `read_fd` — the file descriptor for reading data from the channel.
- `write_fd` — the file descriptor for writing data to the channel.

### 3.2 WinPipeHandles (Windows)

Holds a pair of 64-bit handles of a Windows anonymous pipe.

```yukino
public struct WinPipeHandles {
    read_handle: int64
    write_handle: int64
}
```

- `read_handle` — the handle for reading from the pipe.
- `write_handle` — the handle for writing to the pipe.

### 3.3 IoError

The error structure raised on system failures of operating system kernel calls.

```yukino
public struct IoError {
    message: string
    code: int32
}
```

- `code: 201` — `pipe` call error (POSIX);
- `code: 202` — `dup` descriptor duplication error (POSIX);
- `code: 203` — `dup2` descriptor duplication error (POSIX);
- `code: 204` — `fsync` buffer flush to disk failure (POSIX);
- `code: 205` — `fdatasync` data flush to disk failure (POSIX);
- `code: 206` — `pipe2` channel creation error (Linux);
- `code: 207` — `dup3` descriptor duplication error (Linux);
- `code: 208` — `splice` data transfer system error (Linux);
- `code: 209` — `tee` data duplication system error (Linux);
- `code: 210` — `create_pipe` anonymous channel creation failure (Windows).

---

## 4. Function Reference: POSIX (Unix)

### 4.1 isatty

Checks whether the specified numeric descriptor `fd` refers to an open terminal device (TTY).

```yukino
public fun isatty(fd: int32) -> bool
```

Returns `true` if the descriptor points to a terminal, otherwise `false`.

The function never panics.

```yukino
load std.platformio.unix: isatty, STDOUT_FILENO
load std.io: println

if isatty(STDOUT_FILENO) {
    println("Output is directed to an interactive terminal")
}
```

### 4.2 pipe

Creates a unidirectional data channel (anonymous pipe) in the operating system kernel space.

```yukino
public fun pipe() -> PipeFds
```

Returns a `PipeFds` structure with a pair of descriptors `read_fd` (reading) and `write_fd` (writing).

On a system call error, panics with the `IoError` structure (`code = 201`).

```yukino
load std.platformio.unix: pipe, PipeFds
load std.io: println

set p = pipe()
println("Pipe opened: read ${p.read_fd}, write ${p.write_fd}")
```

### 4.3 dup

Creates a copy of the descriptor `oldfd`, allocating the lowest available free descriptor number.

```yukino
public fun dup(oldfd: int32) -> int32
```

Returns the new file descriptor as `int32`.

On a duplication error, panics with the `IoError` structure (`code = 202`).

```yukino
load std.platformio.unix: dup, STDOUT_FILENO
load std.io: println

set copy_fd = dup(STDOUT_FILENO)
println("Duplicate of the stdout descriptor: ${copy_fd}")
```

### 4.4 dup2

Atomically redirects the descriptor `newfd` to the same open file as `oldfd`.

```yukino
public fun dup2(oldfd: int32, newfd: int32) -> int32
```

Returns the `newfd` descriptor. If `newfd` was previously open, it is safely closed before the redirection.

On a system failure, panics with the `IoError` structure (`code = 203`).

```yukino
load std.platformio.unix: dup2, STDOUT_FILENO
load std.io: println

set res = dup2(3, STDOUT_FILENO)
println("Descriptor redirected: ${res}")
```

### 4.5 fsync

Forcibly synchronizes all modified file buffers (both data and metadata) of the descriptor `fd` with the persistent physical medium.

```yukino
public fun fsync(fd: int32) -> int32
```

Returns `0` on successful synchronization.

On a buffer flush error to the physical disk, panics with the `IoError` structure (`code = 204`).

```yukino
load std.platformio.unix: fsync
load std.io: println

set status = fsync(1)
println("fsync flush completed with status: ${status}")
```

### 4.6 fdatasync

Synchronizes the data of the descriptor `fd` to the physical disk without the mandatory update of unchanged metadata (for example, access timestamps).

```yukino
public fun fdatasync(fd: int32) -> int32
```

Returns `0` on successful data synchronization.

On a data write error to disk, panics with the `IoError` structure (`code = 205`).

```yukino
load std.platformio.unix: fdatasync
load std.io: println

set status = fdatasync(1)
println("fdatasync flush completed with status: ${status}")
```

---

## 5. Function Reference: Linux

### 5.1 pipe2

Creates a pipe with atomic setting of Linux kernel system flags (for example, `O_CLOEXEC` or `O_NONBLOCK`).

```yukino
public fun pipe2(flags: int32) -> PipeFds
```

Returns a `PipeFds` structure with a pair of file descriptors `read_fd` and `write_fd`.

On a channel creation error, panics with the `IoError` structure (`code = 206`).

```yukino
load std.platformio.linux: pipe2, PipeFds
load std.io: println

set flags: int32 = 0x80000 // O_CLOEXEC
set p = pipe2(flags)
println("pipe2 pipe created")
```

### 5.2 dup3

Duplicates the descriptor `oldfd` into `newfd` with atomic setting of Linux system flags (for example, `O_CLOEXEC`).

```yukino
public fun dup3(oldfd: int32, newfd: int32, flags: int32) -> int32
```

Returns the new `newfd` descriptor.

On a failure, panics with the `IoError` structure (`code = 207`).

```yukino
load std.platformio.linux: dup3
load std.io: println

set res = dup3(3, 4, 0x80000)
println("dup3 descriptor: ${res}")
```

### 5.3 splice

Moves up to `len` bytes of data between two file descriptors (one of which must be a pipe) without copying the data into user space (zero-copy in the Linux kernel).

```yukino
public fun splice(fd_in: int32, fd_out: int32, len: int64, flags: uint32) -> int64
```

Returns the actual number of bytes moved as `int64`.

On a system transfer error, panics with the `IoError` structure (`code = 208`).

```yukino
load std.platformio.linux: splice
load std.io: println

set transferred = splice(0, 1, 4096, 0u32)
println("Bytes transferred via splice: ${transferred}")
```

### 5.4 tee

Duplicates up to `len` bytes of data from one pipe `fd_in` into another pipe `fd_out` without consuming the data from the source channel and without copying into user memory.

```yukino
public fun tee(fd_in: int32, fd_out: int32, len: int64, flags: uint32) -> int64
```

Returns the number of successfully duplicated bytes as `int64`.

On a system call error, panics with the `IoError` structure (`code = 209`).

```yukino
load std.platformio.linux: tee
load std.io: println

set count = tee(3, 4, 1024, 0u32)
println("Bytes duplicated via tee: ${count}")
```

---

## 6. Function Reference: Windows (Win32)

### 6.1 get_std_handle

A wrapper over the Win32 `GetStdHandle` call. Obtains the input, output, or error output device handle.

```yukino
public fun get_std_handle(n: int32) -> int64
```

Returns the 64-bit `HANDLE` descriptor, or `-1` (`INVALID_HANDLE_VALUE`) if there is none.

The function never panics.

```yukino
load std.platformio.windows: get_std_handle, STD_OUTPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_OUTPUT_HANDLE)
println("stdout handle: ${h}")
```

### 6.2 is_console

Checks whether the system handle `handle` points to an active screen buffer or the input buffer of the Windows console.

```yukino
public fun is_console(handle: int64) -> bool
```

Returns `true` if the handle is associated with a Windows console, otherwise `false`.

The function never panics.

```yukino
load std.platformio.windows: get_std_handle, is_console, STD_OUTPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_OUTPUT_HANDLE)
if is_console(h) {
    println("Output to the Windows console")
}
```

### 6.3 get_console_mode

Queries the current bit mask of Windows console mode flags (`GetConsoleMode`).

```yukino
public fun get_console_mode(handle: int64) -> int32
```

Returns the 32-bit console mode flag mask, or `-1` on error.

The function never panics.

```yukino
load std.platformio.windows: get_std_handle, get_console_mode, STD_INPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_INPUT_HANDLE)
set mode = get_console_mode(h)
println("Windows console mode: ${mode}")
```

### 6.4 set_console_mode

Sets a new Windows console mode bit mask (`SetConsoleMode`) for the specified handle.

```yukino
public fun set_console_mode(handle: int64, mode: int32) -> bool
```

Returns `true` if the operation completed successfully, otherwise `false`.

The function never panics.

```yukino
load std.platformio.windows: get_std_handle, set_console_mode, STD_INPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_INPUT_HANDLE)
set ok = set_console_mode(h, 7)
println("Mode updated: ${ok}")
```

### 6.5 create_pipe

Creates an anonymous channel (pipe) in Windows via the `CreatePipe` system call.

```yukino
public fun create_pipe() -> WinPipeHandles
```

Returns a `WinPipeHandles` structure with the `read_handle` and `write_handle` descriptors.

On a channel creation error by the operating system, panics with the `IoError` structure (`code = 210`).

```yukino
load std.platformio.windows: create_pipe, WinPipeHandles
load std.io: println

set p = create_pipe()
println("Read handle: ${p.read_handle}, write handle: ${p.write_handle}")
```

---

[← Back: Module std.threads](3-threads.md) | [Next: Module std.math →](5-math.md)
