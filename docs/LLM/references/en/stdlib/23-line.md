# Module std.line

The `std.line` module provides a declarative, object-oriented command-line argument parser designed in 100% pure Yukino, inspired by the popular `clap` library. The module supports hierarchical commands and subcommands of arbitrary nesting depth, named options, short and long flags, short flag bundles (`-abc`), assignment syntax via equals sign (`--opt=val`), the special terminator `--` for trailing raw arguments, automatic generation of formatted help (`-h`, `--help`) and version (`-v`, `--version`), conflicting arguments (`conflicts_with`), and fuzzy typo suggestions ("Did you mean...") based on Levenshtein distance styled via `std.outf`.

### Ergonomic CLI as a First-Class Citizen in Yukino

In Yukino's philosophy, building ergonomic, fast, and resilient command-line interfaces is a primary language goal. Modern systems programmers, compiler developers, DevOps engineers, and backend teams interact with software largely through terminals. When CLI design is unhelpful — lacking helpful error messages, crashing silently on typos, forcing awkward argument order, or requiring hand-crafted help screens — developer productivity suffers.

Parsing low-level string arrays via `args()` or `raw_args()` in non-trivial programs quickly devolves into spaghetti code: manual `i + 1 < args.len()` boundary checks, missing required field validations, lack of flag grouping, and accidental out-of-bounds panics. The `std.line` module eliminates this boilerplate entirely: developers declare the *structure* of application parameters in clean, readable Yukino code, while the parser handles syntactic validation, value extraction, constraints, and ANSI-styled output.

### Architectural Rationale: Why std.line is Designed This Way

1. **Explicit OOP Constructors (`init Line(...)`) Over Hidden Factory Wrappers.**
   Yukino strictly adheres to the principle that explicit code is superior to implicit abstractions (`explicit > implicit`). An expression like `init Line("app", version)` or `init Arg("config")` unambiguously indicates that a concrete heap-allocated object is initialized in the active memory region. Yukino purposefully avoids opaque helper functions or hidden state, ensuring that any developer reading the code immediately understands object lifecycles, memory regions, and available methods.

2. **Builder Pattern for Method Chaining.**
   Configuration methods on `Line` and `Arg` return `this`. This enables developers to declare arbitrarily sophisticated CLI applications in a single expressive statement block, avoiding verbose intermediate temporary variables and keeping code readable.

3. **Strict Separation Between Grammar (`Line`) and Evaluation (`Matches`).**
   The `Line` class specifies *rules* (which flags are valid, which arguments are mandatory, and what options conflict). The `Matches` class stores *extracted data* produced from a specific execution in the shell. This clean separation of concerns prevents mutable state bugs and allows `Matches` to be safely passed throughout application business logic.

4. **Automated Validation, POSIX Exit Code 2, and Levenshtein Suggestions.**
   Upon encountering invalid syntax (such as unknown flags, omitted required values, or unrecognized subcommands), `std.line` does not crash with unexpected panics. Instead, it renders standard UNIX CLI error messages, suggests the closest candidate via Levenshtein edit distance ("Did you mean..."), and terminates gracefully with exit code `2` (the standard POSIX/UNIX code for command-line syntax errors).

### Anatomy of a Command-Line Interface

`std.line` supports all conventional CLI building blocks:

| Element | Purpose | Terminal Syntax | Code Declaration | Code Retrieval |
| :--- | :--- | :--- | :--- | :--- |
| **Flag** | Binary switch (on/off) without parameters. | `-v`, `--verbose`, bundle `-xvf` | `(init Arg("verbose")).short("v").long("verbose").flag()` | `matches.has_flag("verbose")` -> `bool` |
| **Option** | Named parameter expecting an explicit value. | `-c config.json`, `--config=config.json` | `(init Arg("config")).short("c").long("config").value_name("path")` | `matches.get_value("config")` -> `string?` |
| **Positional Argument** | Keyless parameter identified by its sequential position. | `mycli input.txt output.bin` | `(init Arg("input")).positional(true).required(true)` | `matches.get_value("input")` -> `string?` |
| **Subcommand** | Isolated nested action with its own flags and help. | `git commit -m "..."`, `cargo build --release` | `cli.subcommand((init Line("build", "1.0.0"))...)` | `matches.has_subcommand("build")`, `matches.get_subcommand_matches("build")` |
| **Terminator `--` (Rest)** | Trailing arguments passed through unparsed. | `mycli run -- -x -y raw_file` | Automatically supported everywhere | `matches.get_rest()` -> `string[]` |

### Practical Scenarios: "Configure in Code -> How the Shell Accepts Input"

#### Scenario 1: Boolean Flags and Flag Bundling
When declaring multiple single-letter flags:

```yukino
set cli = (init Line("mytool", "1.0.0"))
    .flag("all", "a", "all", "Show hidden entries")
    .flag("long", "l", "long", "Use long listing format")
    .flag("human", "h", "human-readable", "Print human-readable sizes")
```

The terminal user can invoke the application in multiple convenient formats:
- Separate long flags: `mytool --all --long`
- Separate short flags: `mytool -a -l`
- Combined bundle: `mytool -al` or `mytool -la`
- In application code: `if matches.has_flag("all") { ... }`.

#### Scenario 2: Options with Values and Safe Defaults
When an option expects a string value, but a sensible fallback should be used if omitted:

```yukino
set cli = (init Line("server", "1.0.0"))
    .arg((init Arg("port"))
        .short("p")
        .long("port")
        .value_name("number")
        .default_value("8080")
        .about("Port number to listen on"))
```

The terminal user can specify the value via:
- Space-separated: `server -p 3000` or `server --port 3000`
- Equals sign: `server -p=3000` or `server --port=3000`
- Omitted entirely: `server`
- In application code, `matches.get_value("port")` returns `"3000"` when provided, or `"8080"` when omitted.
- Furthermore, `matches.has_flag("port")` returns `true` only if the user explicitly provided the argument on the command line.

#### Scenario 3: Required Positional Arguments
For file processors and compilers where arguments do not require key prefixes:

```yukino
set cli = (init Line("compiler", "1.0.0"))
    .arg((init Arg("source")).positional(true).required(true).about("Source code file path"))
    .arg((init Arg("target")).positional(true).default_value("out.bin").about("Output binary name"))
```

- Invocations like `compiler main.ykn app.bin` assign `"main.ykn"` to `source` and `"app.bin"` to `target`.
- Running `compiler main.ykn` assigns `"main.ykn"` to `source` and uses the default `"out.bin"` for `target`.
- If invoked without arguments (`compiler`), `std.line` automatically halts execution with:
  ```text
  error: the following required argument was not provided: <source>

  For more information, try '--help'.
  ```

#### Scenario 4: Mutually Exclusive Arguments (`conflicts_with`)
When two arguments are incompatible (such as silent mode vs verbose debugging):

```yukino
set cli = (init Line("app", "1.0.0"))
    .arg((init Arg("quiet")).short("q").long("quiet").flag().conflicts_with("verbose").about("Suppress output"))
    .arg((init Arg("verbose")).short("v").long("verbose").flag().about("Enable verbose debugging"))
```

If the user provides both (`app -q -v` or `app --quiet --verbose`), the parser immediately intercepts the conflict:
```text
error: the argument '--quiet' cannot be used with '--verbose'

For more information, try '--help'.
```

#### Scenario 5: Nested Subcommands (Git/Cargo Style)
Each subcommand is an independent `Line` instance with its own flags and isolated argument scope:

```yukino
set cmd_build = (init Line("build", "1.0.0"))
    .about("Build project artifacts")
    .flag("release", "r", "release", "Compile with optimizations")

set cli = (init Line("mytool", "1.0.0"))
    .subcommand(cmd_build)
```

- Invoking `mytool build --release` activates the subcommand branch.
- Running `mytool build --help` displays contextual help specific to `build`, including `USAGE: mytool build [FLAGS]`.
- Reading subcommand state in code:
  ```yukino
  if matches.has_subcommand("build") {
      set sub = matches.get_subcommand_matches("build")
      set is_release = sub.has_flag("release")
  }
  ```

#### Scenario 6: Passthrough Arguments via Terminator `--`
When wrapping other CLI tools or child processes:
- Invocations like `mytool run -- --custom-flag arg1 arg2`
- All arguments following `--` bypass parsing and are retrieved directly via `matches.get_rest()`.

#### Scenario 7: Dynamic Package Version via `yukienv!`
To avoid desynchronization between `yuki.toml` and hardcoded strings, supply the compile-time macro `yukienv!("bin.version")`:

```yukino
set cli = init Line("mytool", yukienv!("bin.version"))
```
When users call `mytool -v` or `mytool --version`, the exact package version from `yuki.toml` is emitted.

---

## 1. Import: load std.line

Import the entire package:

```yukino
load std.line
```

This brings the following entities into scope:

- **Classes:** `Line` (alias `App`), `Arg`, `Matches`.
- **Utility and Suggestion Functions:** `levenshtein`, `find_suggestion`, `render_help`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.line`.

---

## 3. Interfaces

No interfaces are declared in `std.line`.

---

## 4. Data Structures and Error Types (Panics)

No error panic structures are declared in `std.line`. When an unexpected argument, missing value, unknown subcommand, or conflicting options are encountered, the parser renders a structured, styled error message with fuzzy suggestions ("Did you mean...") via `std.outf` to standard error and terminates the process with exit code `2`.

---

## 5. Classes

### 5.1 class Line (alias App)

The primary application builder and command-line parser class. It manages command names, versions, descriptions, author information, arguments, subcommands, and help behavior.

```yukino
public class Line {
    public name: string
    public version: string
    public about_text: string
    public author_text: string
    public parent_chain: string
    public args_list: Arg[]
    public subcommands_list: Line[]
    public help_disabled: bool
    public version_disabled: bool
    public help_override: string
    public version_override: string
}
```

#### 5.1.1 init

Creates a new parser instance with the specified application name and version. For automatic package version retrieval from `yuki.toml`, using the `yukienv!("bin.version")` macro is recommended.

```yukino
public fun init(name: string, version: string)
```

Returns an initialized `Line` object.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("mycli", yukienv!("bin.version"))
    println("App: " + cli.name + " v" + cli.version)
}
```

#### 5.1.2 about

Sets a brief summary description of the application or subcommand displayed in help headers.

```yukino
public fun about(text: string) -> Line
```

Returns the modified `Line` instance for method chaining.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("calc", "1.0.0").about("A simple calculator utility")
}
```

#### 5.1.3 author

Sets author or team credits displayed in help output.

```yukino
public fun author(text: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("tool", "1.0.0").author("Yukino Team")
}
```

#### 5.1.4 arg

Registers a pre-configured `Arg` instance into the command.

```yukino
public fun arg(a: Arg) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line, Arg

fun main() {
    set opt_verbose = init Arg("verbose").short("v").long("verbose").flag()
    set cli = init Line("app", "1.0.0").arg(opt_verbose)
}
```

#### 5.1.5 flag

Convenience method to register a boolean flag that takes no parameter values.

```yukino
public fun flag(name: string, short: string, about: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").flag("verbose", "v", "Enable verbose output")
}
```

#### 5.1.6 opt

Adds an optional named option requiring a string value.

```yukino
public fun opt(name: string, short: string, about: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "Path to configuration file")
}
```

#### 5.1.7 opt_default

Adds an optional named option with a fallback default value used when omitted by the user.

```yukino
public fun opt_default(name: string, short: string, about: string, default_val: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("server", "1.0.0").opt_default("port", "p", "Server port", "8080")
}
```

#### 5.1.8 opt_required

Adds a mandatory named option. If omitted by the user, the parser terminates with an error.

```yukino
public fun opt_required(name: string, short: string, about: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("db", "1.0.0").opt_required("host", "h", "Database host address")
}
```

#### 5.1.9 positional

Adds an optional positional argument. Positional arguments match in the order they were defined.

```yukino
public fun positional(name: string, about: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("cp", "1.0.0").positional("source", "Source file").positional("dest", "Destination file")
}
```

#### 5.1.10 positional_required

Adds a required positional argument.

```yukino
public fun positional_required(name: string, about: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("cat", "1.0.0").positional_required("path", "File path to read")
}
```

#### 5.1.11 subcommand

Registers a nested child subcommand `Line`. Subcommands maintain a parent invocation chain and can contain their own flags, options, and further nested subcommands.

```yukino
public fun subcommand(sub: Line) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set root = init Line("git", "2.40.0")
    set commit = init Line("commit", "2.40.0").flag("amend", "a", "Amend previous commit")
    root.subcommand(commit)
}
```

#### 5.1.12 disable_help

Disables automatic handling of `-h` and `--help` flags.

```yukino
public fun disable_help() -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").disable_help()
}
```

#### 5.1.13 disable_version

Disables automatic handling of `-v` and `--version` flags.

```yukino
public fun disable_version() -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").disable_version()
}
```

#### 5.1.14 override_help

Specifies custom text to print when help is requested instead of auto-generated formatting.

```yukino
public fun override_help(text: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").override_help("Custom application help text.")
}
```

#### 5.1.15 override_version

Specifies custom text to print when version is requested.

```yukino
public fun override_version(text: string) -> Line
```

Returns the current `Line` instance.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").override_version("v1.0.0-custom-build")
}
```

#### 5.1.16 print_help

Prints the generated or overridden help text for the current command to standard output.

```yukino
public fun print_help()
```

This function does not return a value.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("demo", "1.0.0").flag("verbose", "v", "Verbose mode")
    cli.print_help()
}
```

#### 5.1.17 print_version

Prints version information for the command in `<full_name> <version>` format or custom override text to standard output.

```yukino
public fun print_version()
```

This function does not return a value.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("demo", "1.0.0")
    cli.print_version()
}
```

#### 5.1.18 parse

Parses actual command-line arguments of the running process obtained via `args()`. If `-h`/`--help` or `-v`/`--version` are passed, the parser automatically prints the requested information and exits with code `0`. If validation fails, it outputs an error message and exits with code `2`.

```yukino
public fun parse() -> Matches
```

Returns a `Matches` object containing parsing results.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("tool", yukienv!("bin.version")).flag("dry-run", "d", "Dry run")
    set matches = cli.parse()
}
```

#### 5.1.19 parse_from

Parses a provided array of string arguments `input_args`. Useful for unit testing command-line configurations without modifying process arguments.

```yukino
public fun parse_from(input_args: string[]) -> Matches
```

Returns a `Matches` object containing parsing results.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("tool", "1.0.0").flag("verbose", "v", "Verbose")
    set matches = cli.parse_from(["-v"])
    println("Verbose: ${matches.get_bool(\"verbose\")}")
}
```

---

### 5.2 class Arg

Builder class for configuring individual arguments, options, or flags.

```yukino
public class Arg {
    public name: string
    public short_name: string
    public long_name: string
    public about_text: string
    public is_flag: bool
    public is_required: bool
    public is_positional: bool
    public is_multiple: bool
    public default_val: string
    public has_default: bool
    public value_hint: string
    public conflicts: string[]
}
```

#### 5.2.1 init

Creates an argument with the specified unique identifier. By default, the long option name matches `name`.

```yukino
public fun init(name: string)
```

Returns an initialized `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("output")
}
```

#### 5.2.2 short

Assigns a single-character short flag or option (e.g., `"o"` for `-o`).

```yukino
public fun short(s: string) -> Arg
```

Returns the modified `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("output").short("o")
}
```

#### 5.2.3 long

Assigns a long option name (e.g., `"output"` for `--output`).

```yukino
public fun long(l: string) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("out").long("output")
}
```

#### 5.2.4 about

Sets the descriptive explanation of the argument for help tables.

```yukino
public fun about(desc: string) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("output").about("Path to output file")
}
```

#### 5.2.5 flag

Switches the argument to a boolean flag (requires no subsequent value).

```yukino
public fun flag() -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("release").short("r").flag()
}
```

#### 5.2.6 required

Sets whether this argument is mandatory.

```yukino
public fun required(r: bool) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("token").required(true)
}
```

#### 5.2.7 positional

Marks the argument as positional (matched without `-` or `--` prefixes).

```yukino
public fun positional(p: bool) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("input").positional(true)
}
```

#### 5.2.8 multiple

Marks the argument as permitting multiple occurrences.

```yukino
public fun multiple(m: bool) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("include").short("I").multiple(true)
}
```

#### 5.2.9 default_value

Sets a fallback default value supplied by the parser when omitted by the user.

```yukino
public fun default_value(val: string) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("format").default_value("json")
}
```

#### 5.2.10 value_hint

Sets the value placeholder string displayed in help tables (e.g., `<PATH>`, `<PORT>`). Defaults to `"VALUE"`.

```yukino
public fun value_hint(hint: string) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("config").value_hint("FILE")
}
```

#### 5.2.11 conflicts_with

Registers a mutual exclusion constraint: this argument cannot be used simultaneously with `other_name`.

```yukino
public fun conflicts_with(other_name: string) -> Arg
```

Returns the current `Arg` instance.

This method never panics.

```yukino
load std.line: Arg

fun main() {
    set a = init Arg("quiet").conflicts_with("verbose")
}
```

---

### 5.3 class Matches

Result container for parsed command-line arguments. Provides typed accessors to check flags and extract string, integer, or floating-point values.

```yukino
public class Matches {
    public flag_names: string[]
    public opt_names: string[]
    public opt_values: string[]
    public pos_names: string[]
    public pos_values: string[]
    public all_pos: string[]
    public trailing_items: string[]
    public sub_name: string
    public sub_matches: Matches?
}
```

#### 5.3.1 init

Creates an empty result container.

```yukino
public fun init()
```

Returns an empty `Matches` instance.

This method never panics.

```yukino
load std.line: Matches

fun main() {
    set m = init Matches()
}
```

#### 5.3.2 get_bool

Checks whether a boolean flag with the given name was passed.

```yukino
public fun get_bool(name: string) -> bool
```

Returns `true` if the flag was specified, otherwise `false`. Not nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").flag("verbose", "v", "")
    set m = cli.parse_from(["-v"])
    if m.get_bool("verbose") {
        println("Verbose enabled")
    }
}
```

#### 5.3.3 has

Checks whether an argument with the specified name is present (among flags, options, or positional arguments).

```yukino
public fun has(name: string) -> bool
```

Returns `true` if present, otherwise `false`. Not nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "")
    set m = cli.parse_from(["-c", "app.toml"])
    set has_cfg = m.has("config")
}
```

#### 5.3.4 get

Retrieves the string value of an option or positional argument by its identifier.

```yukino
public fun get(name: string) -> string?
```

Returns the string value or `null` if not specified. Nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("config", "c", "")
    set m = cli.parse_from(["-c", "dev.cfg"])
    set cfg = m.get("config")
    if cfg != null {
        println("Config: " + (cfg ?? ""))
    }
}
```

#### 5.3.5 get_or

Retrieves the string value of an option, returning `fallback` if not specified.

```yukino
public fun get_or(name: string, fallback: string) -> string
```

Returns the string value or `fallback`. Not nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("host", "h", "")
    set m = cli.parse_from([])
    println("Host: " + m.get_or("host", "localhost"))
}
```

#### 5.3.6 get_int

Parses and returns the 64-bit integer value of an argument.

```yukino
public fun get_int(name: string) -> int64?
```

Returns `int64` or `null` if the argument is omitted or not a valid integer. Nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("port", "p", "")
    set m = cli.parse_from(["-p", "8080"])
    set port = m.get_int("port")
}
```

#### 5.3.7 get_int_or

Retrieves the integer value of an option, returning `fallback` if omitted or invalid.

```yukino
public fun get_int_or(name: string, fallback: int64) -> int64
```

Returns `int64`. Not nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("app", "1.0.0").opt("port", "p", "")
    set m = cli.parse_from([])
    println("Port: ${m.get_int_or(\"port\", 3000)}")
}
```

#### 5.3.8 get_float

Parses and returns the 64-bit floating-point value of an argument.

```yukino
public fun get_float(name: string) -> float64?
```

Returns `float64` or `null` if omitted or not a valid float. Nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("rate", "r", "")
    set m = cli.parse_from(["-r", "0.75"])
    set rate = m.get_float("rate")
}
```

#### 5.3.9 get_float_or

Retrieves the floating-point value of an option, returning `fallback` if omitted or invalid.

```yukino
public fun get_float_or(name: string, fallback: float64) -> float64
```

Returns `float64`. Not nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("app", "1.0.0").opt("scale", "s", "")
    set m = cli.parse_from([])
    set s = m.get_float_or("scale", 1.0)
}
```

#### 5.3.10 get_all

Returns all occurrences of an option (e.g. repeated `-I dir1 -I dir2`).

```yukino
public fun get_all(name: string) -> string[]
```

Returns an array of strings `string[]`. Empty if never passed. Not nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("compiler", "1.0.0").opt("include", "I", "")
    set m = cli.parse_from(["-I", "src", "-I", "lib"])
    set includes = m.get_all("include")
}
```

#### 5.3.11 trailing

Returns all arguments specified after the `--` terminator.

```yukino
public fun trailing() -> string[]
```

Returns an array of strings `string[]`. Not nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set cli = init Line("runner", "1.0.0")
    set m = cli.parse_from(["--", "-custom", "arg1"])
    println("Trailing: ${m.trailing().len()}")
}
```

#### 5.3.12 raw_positionals

Returns all positional argument strings passed to the command.

```yukino
public fun raw_positionals() -> string[]
```

Returns an array of strings `string[]`. Not nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set cli = init Line("tool", "1.0.0")
    set m = cli.parse_from(["file1.txt", "file2.txt"])
    set raw = m.raw_positionals()
}
```

#### 5.3.13 subcommand_name

Returns the name of the invoked subcommand.

```yukino
public fun subcommand_name() -> string?
```

Returns the subcommand name string or `null` if no subcommand was invoked. Nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("status", "2.40.0"))
    set m = root.parse_from(["status"])
    println("Command: " + (m.subcommand_name() ?? "none"))
}
```

#### 5.3.14 subcommand_matches

Returns the `Matches` instance for the invoked subcommand.

```yukino
public fun subcommand_matches() -> Matches?
```

Returns a `Matches` object or `null` if no subcommand was invoked. Nullable.

This method never panics.

```yukino
load std.line: Line
load std.io: println

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("commit", "2.40.0").opt("message", "m", ""))
    set m = root.parse_from(["commit", "-m", "init"])
    set sub_m = m.subcommand_matches()
    if sub_m != null {
        set s = sub_m ?? m
        println("Message: " + s.get_or("message", ""))
    }
}
```

#### 5.3.15 has_subcommand

Checks whether a subcommand with the given name was invoked.

```yukino
public fun has_subcommand(name: string) -> bool
```

Returns `true` if the named subcommand was invoked, otherwise `false`. Not nullable.

This method never panics.

```yukino
load std.line: Line

fun main() {
    set root = init Line("git", "2.40.0")
    root.subcommand(init Line("push", "2.40.0"))
    set m = root.parse_from(["push"])
    set is_push = m.has_subcommand("push")
}
```

---

## 6. Function Reference

### 6.1 levenshtein

Calculates the Levenshtein distance (minimum number of single-character insertions, deletions, or substitutions) between strings `a` and `b`. Used by the fuzzy suggestion engine for "Did you mean..." hints.

```yukino
public fun levenshtein(a: string, b: string) -> int64
```

Returns a non-negative integer `int64`. Not nullable.

This function never panics.

```yukino
load std.line: levenshtein
load std.io: println

fun main() {
    set w1 = "commit"
    set w2 = "comit"
    println("Distance: ${levenshtein(w1, w2)}")
}
```

### 6.2 find_suggestion

Searches for the closest candidate string in `candidates` for a mistyped string `target`. A suggestion is returned only if the Levenshtein distance is within the typo tolerance threshold (3 operations).

```yukino
public fun find_suggestion(target: string, candidates: string[]) -> string?
```

Returns the closest candidate string or `null` if no close match is found. Nullable.

This function never panics.

```yukino
load std.line: find_suggestion
load std.io: println

fun main() {
    set typo = "--verison"
    set cands: string[] = ["--help", "--version", "--verbose"]
    set sug = find_suggestion(typo, cands)
    if sug != null {
        println("Did you mean: " + (sug ?? ""))
    }
}
```

### 6.3 render_help

Low-level command help formatting function. Generates column-aligned sections for `USAGE:`, `FLAGS:`, `OPTIONS:`, `ARGS:`, and `SUBCOMMANDS:`, highlighted with `std.outf` styles.

```yukino
public fun render_help(
    name: string,
    version: string,
    about: string,
    author: string,
    parent_chain: string,
    args: Arg[],
    sub_names: string[],
    sub_abouts: string[],
    disable_help: bool,
    disable_version: bool,
    allow_short_h: bool,
    allow_long_help: bool,
    allow_short_v: bool,
    allow_long_version: bool
) -> string
```

Returns the formatted help string `string`. Not nullable.

This function never panics.

```yukino
load std.line: render_help, Arg
load std.io: println

fun main() {
    set args_list: Arg[] = []
    set sub_names: string[] = []
    set sub_abouts: string[] = []
    set text = render_help(
        "mycli", "1.0.0", "Sample application", "Author", "",
        args_list, sub_names, sub_abouts,
        false, false, true, true, true, true
    )
    println(text)
}
```

---

[← Previous: Module std.outf](22-outf.md) | [Next: Module std.itermtool →](24-itermtool.md)
