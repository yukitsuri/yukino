# Module std.process

The `std.process` module provides facilities for spawning child processes, configuring standard I/O streams (stdin/stdout/stderr), capturing process output, interacting through system shells, and managing the current process state and environment variables.

---

## 1. Import: load std.process

Importing the entire package:

```yukino
load std.process
```

All public components of `std.process` are loaded into the current scope:

- **Standard Stream Constants (stdio):**
  - `STDIO_INHERIT`, `STDIO_PIPED`, `STDIO_NULL`.
- **Data and Error Structures:**
  - `ProcessError` — error structure raised on system call or process spawning failure;
  - `ExitStatus` — child process termination status (exit code, success flag, signal).
- **Classes:**
  - `ProcessOutput` — captured stdout/stderr buffers and exit status of a finished command;
  - `ChildProcess` — handle for managing an asynchronous running child process;
  - `Command` — builder for detailed child process configuration and execution.
- **High-Level Spawning Functions:**
  - `command_new`, `command_shell`, `exec`, `run`, `run_shell`, `read_output`, `read_output_shell`, `default_shell`.
- **Process State and Environment:**
  - `current_pid`, `parent_pid`, `current_exe`, `current_dir`, `set_current_dir`, `env_var`, `set_env_var`, `remove_env_var`, `abort_process`.

All of these items are also accessible when importing the whole standard library: `load std`.

---

## 2. Constants

### 2.1 Standard Stream Constants (stdio)

Constants defining redirection modes for `stdin`, `stdout`, and `stderr` descriptors when configuring child processes via `Command`.

```yukino
public static STDIO_INHERIT: int32 = 0
public static STDIO_PIPED: int32 = 1
public static STDIO_NULL: int32 = 2
```

- `STDIO_INHERIT` (`0`) — the child process inherits the parent process's file descriptor.
- `STDIO_PIPED` (`1`) — an anonymous pipe is created for programmatic redirection and stream capturing.
- `STDIO_NULL` (`2`) — the stream is redirected to the system null device (`/dev/null` on POSIX systems or `NUL` on Windows).

---

## 3. Data and Error Structures

### 3.1 ProcessError

Error structure raised via `panic` upon system process launch, wait, or shell execution failures.

```yukino
public struct ProcessError {
    message: string
    code: int32
}
```

- `message` — human-readable description of the operating system error or launch failure.
- `code` — numerical operating system error code or non-zero termination status.

### 3.2 ExitStatus

Represents the exit termination status of a child process.

```yukino
public struct ExitStatus {
    code: int32
    success: bool
    signal: int32
}
```

- `code` — integer exit code returned by the process (`0` indicates success).
- `success` — boolean flag indicating whether the process exited with code `0`.
- `signal` — signal number that terminated the process, or `0` if it terminated normally.

---

## 4. Classes

### 4.1 ProcessOutput

Encapsulates the outcome of executing a command: the exit status and captured `stdout` and `stderr` buffers.

```yukino
public class ProcessOutput
```

#### stdout_string
```yukino
fun stdout_string() -> string
```
Returns captured standard output as a string.

Returns a `string` value.

This function never panics.

#### stderr_string
```yukino
fun stderr_string() -> string
```
Returns captured standard error as a string.

Returns a `string` value.

This function never panics.

#### success
```yukino
fun success() -> bool
```
Checks whether the process exited successfully with return code `0`.

Returns `true` if exit code equals `0`, otherwise `false`.

This function never panics.

#### code
```yukino
fun code() -> int32
```
Returns the integer exit code of the process.

Returns an `int32` value.

This function never panics.

```yukino
load std.process: run
load std.io: println

set out = run("echo", ["hello"])
if out.success() {
    println("Output: " + out.stdout_string())
}
```

---

### 4.2 ChildProcess

Represents an active child process running asynchronously alongside the host application.

```yukino
public class ChildProcess
```

#### id
```yukino
fun id() -> int32
```
Returns the operating system process identifier (PID) of the child process.

Returns an `int32` PID value.

This function never panics.

#### wait
```yukino
fun wait() -> ExitStatus
```
Blocks the calling thread until the child process terminates and returns its exit status.

Returns an `ExitStatus` structure.

In case of an operating system wait error, panics with structure `ProcessError`.

#### try_wait (nullable)
```yukino
fun try_wait() -> ExitStatus?
```
Checks whether the child process has terminated without blocking the calling thread.

Returns an `ExitStatus` structure if the process has finished, or `null` if the process is still running.

In case of an operating system polling error, panics with structure `ProcessError`.

#### kill
```yukino
fun kill() -> bool
```
Immediately terminates the child process forcibly (`SIGKILL` on POSIX, `TerminateProcess` on Windows).

Returns `true` if the termination request succeeded, otherwise `false`.

This function never panics.

#### terminate
```yukino
fun terminate() -> bool
```
Sends a graceful termination signal to the child process (`SIGTERM` on POSIX).

Returns `true` if the signal was delivered successfully, otherwise `false`.

This function never panics.

#### write_stdin
```yukino
fun write_stdin(data: string) -> bool
```
Writes the provided string `data` into the piped `stdin` descriptor of the child process.

Returns `true` if write succeeded, otherwise `false`.

This function never panics.

#### write_and_close_stdin
```yukino
fun write_and_close_stdin(data: string) -> bool
```
Writes the provided string into `stdin` and immediately closes the stream, transmitting an EOF marker.

Returns `true` if write and stream close succeeded, otherwise `false`.

This function never panics.

#### wait_with_output
```yukino
fun wait_with_output() -> ProcessOutput
```
Waits for the child process to exit, drains all data from piped `stdout` and `stderr` streams, closes descriptors, and constructs the resulting output object.

Returns a `ProcessOutput` object.

In case of an operating system error while waiting or reading streams, panics with structure `ProcessError`.

```yukino
load std.process: command_new, STDIO_PIPED
load std.io: println

set cmd = command_new("cat")
cmd.stdin(STDIO_PIPED)
cmd.stdout(STDIO_PIPED)
set child = cmd.spawn()
child.write_and_close_stdin("Yukino Systems")
set out = child.wait_with_output()
println("Output: " + out.stdout_string())
```

---

### 4.3 Command

Builder for configuring arguments, working directory, environment variables, and stream redirection for process execution.

```yukino
public class Command
```

#### arg
```yukino
fun arg(a: string) -> Command
```
Appends a single command-line argument `a`. Returns `this` for method chaining. This function never panics.

#### args
```yukino
fun args(arr: string[]) -> Command
```
Appends an array of command-line arguments `arr`. Returns `this`. This function never panics.

#### current_dir
```yukino
fun current_dir(dir: string) -> Command
```
Sets the working directory `dir` for the spawned process. Returns `this`. This function never panics.

#### env
```yukino
fun env(key: string, val: string) -> Command
```
Sets or overrides environment variable `key` to `val` for the spawned process. Returns `this`. This function never panics.

#### env_clear
```yukino
fun env_clear() -> Command
```
Clears all inherited environment variables, starting the process with an empty environment (except for variables added via `env`). Returns `this`. This function never panics.

#### stdin / stdout / stderr
```yukino
fun stdin(mode: int32) -> Command
fun stdout(mode: int32) -> Command
fun stderr(mode: int32) -> Command
```
Sets redirection mode for standard streams (`STDIO_INHERIT`, `STDIO_PIPED`, or `STDIO_NULL`). Returns `this`. These functions never panic.

#### output
```yukino
fun output() -> ProcessOutput
```
Executes the command with piped `stdout` and `stderr`, waits for completion, and returns captured output.

Returns a `ProcessOutput` object.

If the executable cannot be found or started, panics with structure `ProcessError`.

#### status
```yukino
fun status() -> ExitStatus
```
Executes the command using the standard stream modes configured via the `stdin`/`stdout`/`stderr` methods (inherited by default), waits for completion, and returns the exit status.

Returns an `ExitStatus` structure.

If the process fails to start, panics with structure `ProcessError`.

#### spawn
```yukino
fun spawn() -> ChildProcess
```
Spawns the child process asynchronously according to configuration and returns a `ChildProcess` handle.

Returns a `ChildProcess` object.

If the process fails to spawn, panics with structure `ProcessError`.

```yukino
load std.process: command_new
load std.io: println

set cmd = command_new("echo")
cmd.arg("Hello, World!")
set out = cmd.output()
println(out.stdout_string())
```

---

## 5. Function Reference: High-Level Process Spawning

### 5.1 command_new

Creates a new `Command` builder instance for the specified executable program `prog`.

```yukino
public fun command_new(prog: string) -> Command
```

Returns a newly allocated `Command` instance.

This function never panics.

```yukino
load std.process: command_new
load std.io: println

set cmd = command_new("ls")
cmd.arg("-la")
set st = cmd.status()
println("Exit code: ${st.code}")
```

### 5.2 command_shell

Creates a new `Command` builder instance configured to execute `script` via the specified shell `launcher` (e.g. `"sh -c"`).

```yukino
public fun command_shell(launcher: string, script: string) -> Command
```

Returns a configured `Command` instance.

This function never panics.

```yukino
load std.process: command_shell
load std.io: println

set cmd = command_shell("sh -c", "echo 42")
set out = cmd.output()
println("Result: " + out.stdout_string())
```

### 5.3 exec

Synchronously executes a shell command string via `launcher`, inheriting parent terminal standard I/O streams.

```yukino
public fun exec(launcher: string, script: string) -> int32
```

Returns the integer exit code as `int32`.

In case of a failure to launch the shell process, panics with structure `ProcessError`.

```yukino
load std.process: exec, default_shell
load std.io: println

set code = exec(default_shell(), "echo 'Yukino exec'")
println("Code: ${code}")
```


### 5.4 run

Spawns executable `program` with argument slice `args`, waits for termination, and captures both `stdout` and `stderr` streams.

```yukino
public fun run(program: string, args: string[]) -> ProcessOutput
```

Returns a `ProcessOutput` object with captured streams and exit status.

If launching the program fails, panics with structure `ProcessError`.

```yukino
load std.process: run
load std.io: println

set res = run("uname", ["-s"])
println("OS: " + res.stdout_string())
```

### 5.5 run_shell

Executes `script` within shell `launcher`, capturing all standard stream output.

```yukino
public fun run_shell(launcher: string, script: string) -> ProcessOutput
```

Returns a `ProcessOutput` object containing command output.

If launching the shell fails, panics with structure `ProcessError`.

```yukino
load std.process: run_shell, default_shell
load std.io: println

set out = run_shell(default_shell(), "echo 'Line'")
println("Output: " + out.stdout_string())
```

### 5.6 read_output

Spawns `program` with `args`, verifies successful completion (`code == 0`), and returns trimmed `stdout` string.

```yukino
public fun read_output(program: string, args: string[]) -> string
```

Returns the captured output as `string`.

If execution fails or the process exits with a non-zero code, panics with structure `ProcessError`.

```yukino
load std.process: read_output, ProcessError
load std.io: println

sus {
    set branch = read_output("git", ["branch", "--show-current"])
    println("Branch: " + branch)
} catch err: ProcessError {
    println("Git error: ${err.message}")
}
```

### 5.7 read_output_shell

Executes `script` via shell `launcher`, verifies successful exit (`code == 0`), and returns trimmed `stdout` string.

```yukino
public fun read_output_shell(launcher: string, script: string) -> string
```

Returns the captured output as `string`.

If launching fails or returns non-zero, panics with structure `ProcessError`.

```yukino
load std.process: read_output_shell, default_shell
load std.io: println

set who = read_output_shell(default_shell(), "whoami")
println("User: " + who)
```

### 5.8 default_shell

Returns the default system shell command string for the current operating system platform: `"sh -c"` on POSIX/Linux systems or `"powershell -Command"` on Windows.

```yukino
public fun default_shell() -> string
```

Returns a `string` command line prefix.

This function never panics.

```yukino
load std.process: default_shell
load std.io: println

set shell = default_shell()
println("Default shell: " + shell)
```

---

## 6. Function Reference: Process State and Environment Variables

### 6.1 current_pid

Returns the operating system process identifier (PID) of the current process.

```yukino
public fun current_pid() -> int32
```

Returns an `int32` process identifier.

This function never panics.

```yukino
load std.process: current_pid
load std.io: println

set pid = current_pid()
println("Current PID: ${pid}")
```

### 6.2 parent_pid (nullable)

Returns the parent process identifier (PPID).

```yukino
public fun parent_pid() -> int32?
```

Returns an `int32` parent process identifier, or `null` if unavailable or zero.

This function never panics.

```yukino
load std.process: parent_pid
load std.io: println

set ppid = parent_pid()
if ppid != null {
    println("Parent PPID: ${ppid}")
}
```

### 6.3 current_exe

Returns the canonical absolute file system path to the current executable binary.

```yukino
public fun current_exe() -> string
```

Returns a `string` file path.

This function never panics.

```yukino
load std.process: current_exe
load std.io: println

set exe = current_exe()
println("Executable: " + exe)
```

### 6.4 current_dir

Returns the absolute path to the current working directory.

```yukino
public fun current_dir() -> string
```

Returns a `string` directory path.

This function never panics.

```yukino
load std.process: current_dir
load std.io: println

set cwd = current_dir()
println("Working dir: " + cwd)
```

### 6.5 set_current_dir

Changes the current working directory of the process to `path`.

```yukino
public fun set_current_dir(path: string) -> bool
```

Returns `true` if directory was changed successfully, otherwise `false`.

This function never panics.

```yukino
load std.process: set_current_dir
load std.io: println

set ok = set_current_dir("/tmp")
println("Success: ${ok}")
```

### 6.6 env_var (nullable)

Retrieves the value of environment variable `name`.

```yukino
public fun env_var(name: string) -> string?
```

Returns the variable value as `string`, or `null` if the variable is not set.

This function never panics.

```yukino
load std.process: env_var
load std.io: println

set home = env_var("HOME")
if home != null {
    println("Home directory: " + home)
}
```

### 6.7 set_env_var

Sets or overrides environment variable `name` with `value` for the current process.

```yukino
public fun set_env_var(name: string, value: string) -> bool
```

Returns `true` if variable was set successfully, otherwise `false`.

This function never panics.

```yukino
load std.process: set_env_var, env_var
load std.io: println

set_env_var("YUKINO_MODE", "release")
set mode = env_var("YUKINO_MODE") ?? "debug"
println("Mode: " + mode)
```

### 6.8 remove_env_var

Deletes environment variable `name` from the current process environment.

```yukino
public fun remove_env_var(name: string) -> bool
```

Returns `true` if the variable was successfully removed, otherwise `false`.

This function never panics.

```yukino
load std.process: remove_env_var
load std.io: println

set removed = remove_env_var("TEMP_KEY")
println("Removed: ${removed}")
```

### 6.9 abort_process

Immediately aborts the current process via the operating system `abort` syscall without calling destructors or unwinding.

```yukino
public fun abort_process()
```

This function does not return and terminates execution immediately.

This function never panics.

```yukino
load std.process: abort_process

// Abort execution
// abort_process()
```

---

[← Previous: Module std.collections](9-collections.md) | [Next: Module std.os →](11-os.md)
