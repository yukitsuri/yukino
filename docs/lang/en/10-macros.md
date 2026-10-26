# Macro System and Built-in Macros

Macros in Yukino are expanded during syntactic analysis, before type checking and LLVM IR generation. The macro system supports both user-defined hygienic macros declared with `define` and a set of built-in compiler macros.

---

## Contents

1. [Custom Macros: the define Declaration](#1-custom-macros-the-define-declaration)  
   1.1 [Expression Macros](#11-expression-macros)  
   1.2 [Block Macros](#12-block-macros)  
   1.3 [Macro Hygiene](#13-macro-hygiene)  
   1.4 [Macro Export and Visibility (public define)](#14-macro-export-and-visibility-public-define)
2. [Built-in Compiler Macros](#2-built-in-compiler-macros)  
   2.1 [Source Location: file!, line!, col!](#21-source-location-file-line-col)  
   2.2 [Build Environment: hostenv!, get_target!, get_yukic_version!](#22-build-environment-hostenv-get_target-get_yukic_version)  
   2.3 [Configuration and Manifest: yukienv!, get_profileconfig!, get_features!, is_lib_connected!](#23-configuration-and-manifest-yukienv-get_profileconfig-get_features-is_lib_connected)  
   2.4 [Embedding Binary and Text Resources: include_str!, include_bytes!](#24-embedding-binary-and-text-resources-include_str-include_bytes)  
   2.5 [Static Compilation Error: compile_error!](#25-static-compilation-error-compile_error)  
   2.6 [Compile-time Configuration Check: cfg!](#26-compile-time-configuration-check-cfg)  
   2.7 [Compile-time Output: compile_print!](#27-compile-time-output-compile_print)
3. [Procedural Macro Packages and Macro SDK](#3-procedural-macro-packages-and-macro-sdk)  
   3.1 [What are Procedural Macros and Why Use Them](#31-what-are-procedural-macros-and-why-use-them)  
   3.2 [The Official Macro SDK (macrosdk)](#32-the-official-macro-sdk-macrosdk)  
   3.3 [Core SDK Entities: TokenStream, MacroContext, MacroRunner](#33-core-sdk-entities-tokenstream-macrocontext-macrorunner)  
   3.4 [Step-by-Step Tutorial: Creating a Macro with yukino new --macro](#34-step-by-step-tutorial-creating-a-macro-with-yukino-new---macro)  
   3.5 [Connecting and Invoking Macros in Applications](#35-connecting-and-invoking-macros-in-applications)  
   3.6 [Execution Architecture, IPC Safety, and Timeouts](#36-execution-architecture-ipc-safety-and-timeouts)  
   3.7 [External File Tracking (track_file) and Incremental CGU Cache](#37-external-file-tracking-track_file-and-incremental-cgu-cache)  
4. [Macro SDK API Reference (macrosdk)](#4-macro-sdk-api-reference-macrosdk)  
   4.1 [Package Import: load macrosdk](#41-package-import-load-macrosdk)  
   4.2 [Constants](#42-constants)  
   4.3 [Types](#43-types)  
   4.4 [Data Structures](#44-data-structures)  
   4.5 [Classes](#45-classes)  
   4.6 [Function and Method Reference](#46-function-and-method-reference)

---

## 1. Custom Macros: the define Declaration

Custom macros are declared with the `define` keyword at the top level of a module. Any macro invocation in code always ends with an exclamation mark: `macro_name!(...)`.

### 1.1 Expression Macros

An expression macro splices the expression's AST tree in place of the call site:

```yukino
// Declaration with the '=>' arrow
define square(x) => x * x
define cube(x) => x * x * x

fun calculate() -> int32 {
    // Expanded at the AST stage into: 5 * 5
    set a = square!(5)
    
    // Expanded into: (a + 1) * (a + 1) * (a + 1)
    set b = cube!(a + 1)
    return b
}
```

### 1.2 Block Macros

If the macro body is enclosed in curly braces `{}`, it expands into a sequence of statements (statement splicing) directly in the current scope:

```yukino
define init_point_2d(target_x, target_y) {
    set target_x = 100
    set target_y = 200
}

fun run() {
    set x = 0
    set y = 0
    init_point_2d!(x, y)
    // x and y are initialized
}
```

### 1.3 Macro Hygiene

The Yukino macro system is **hygienic**: any internal variables created inside a macro body are automatically renamed by the compiler (gensym) to avoid unintended capture or conflicts with variables in the calling code:

```yukino
define execute_logged(action) {
    // The internal variable 'temp_state' is automatically isolated (for example, temp_state_g1)
    set temp_state = 1
    action
}

fun main() {
    set temp_state = 999
    execute_logged!(print("Action done\n"))
    // The calling function's local variable temp_state is guaranteed to keep the value 999
}
```

### 1.4 Macro Export and Visibility (public define)

By default, a macro is visible only within its own `.ykn` file. To make a macro available to other modules via `load`, use the `public` modifier:

```yukino
// math_macros.ykn
public define max_of(a, b) => a > b ? a : b

// main.ykn
load .math_macros: max_of

fun main() {
    set m = max_of!(10, 20)
}
```

---

## 2. Built-in Compiler Macros

Built-in macros are handled directly by the `yukic` compiler and provide access to compilation metadata and the host environment.

### 2.1 Source Location: file!, line!, col!

They return string and numeric literals of the current position:

```yukino
set current_source: string = file!() // For example, "src/main.ykn"
set current_line: int32 = line!()    // Line number (1-based)
set current_column: int32 = col!()   // Column number (1-based)

fun log_error(msg: string) {
    print("${file!()}:${line!()}: ${msg}\n")
}
```

### 2.2 Build Environment: hostenv!, get_target!, get_yukic_version!

- `hostenv!("VARIABLE_NAME")`: reads the host environment variable value at compile time and embeds it as a string literal:
  ```yukino
  static BUILD_USER: string = hostenv!("USER")
  static CI_COMMIT: string = hostenv!("GIT_COMMIT")
  ```
  If the environment variable is not set on the host, the compiler raises an error.

- `get_target!()`: returns the target platform's Target Triple (for example, `"x86_64-unknown-linux-gnu"`):
  ```yukino
  static TARGET_ARCH: string = get_target!()
  ```

- `get_yukic_version!()`: returns the `yukic` compiler version string:
  ```yukino
  static COMPILER_VER: string = get_yukic_version!()
  ```

### 2.3 Configuration and Manifest: yukienv!, get_profileconfig!, get_features!, is_lib_connected!

- `yukienv!("key", [scope: int32])`: reads the string value of any parameter or nested key from the project's TOML configuration files at compile time. An optional second `int32` parameter specifies the lookup `scope`:
  - `1` (default if omitted) — checks the current package/library's own configuration file (`yuki.toml` or `lib.toml`);
  - `2` — checks the current package's own workspace manifest (`yukispace.toml`);
  - `3` — checks the host application's workspace manifest (`yukispace.toml` of the root application; for the host application itself, equivalent to `2`);
  - `4` — checks the host application's configuration file (`yuki.toml` / `lib.toml` of the root application; for the host application itself, equivalent to `1`);
  - `else` (any other integer) — checks the current package's own configuration file (`1`).

  ```yukino
  // Read from current package config (scope 1)
  static APP_NAME: string = yukienv!("bin.name")
  static APP_VER: string = yukienv!("bin.version", 1)

  // Read from host application workspace (scope 3)
  static WS_NAME: string = yukienv!("workspace.name", 3)

  // Read from host application manifest (scope 4)
  static HOST_APP_NAME: string = yukienv!("bin.name", 4)
  ```
  If the requested key is missing from the specified manifest or the manifest file is not found, compilation aborts with error `E0306`.

- `get_profileconfig!()`: called without arguments and returns the name of the active build profile (`"debug"`, `"release"`, or a custom profile from `config.*`):
  ```yukino
  static CURRENT_PROFILE: string = get_profileconfig!()
  ```

- `get_features!()`: called without arguments and returns a `string[]` array of strings containing the list of all features enabled in the current build:
  ```yukino
  static ACTIVE_FEATURES: string[] = get_features!()
  ```

- `is_lib_connected!("lib_name", [scope: int32])`: returns a boolean value (`true` / `false`) by checking whether the specified library (for example, `"std"`) is connected to the project within the designated configuration scope:
  - `1` (default) — checks the current package's own config (`yuki.toml` / `lib.toml`);
  - `2` — checks the current package's own workspace (`yukispace.toml`);
  - `3` — checks the host application's workspace;
  - `4` — checks the host application's config;
  - `else` — checks the current package's own config (`1`).

  ```yukino
  // Check in current manifest
  static HAS_STD: bool = is_lib_connected!("std")

  // Dependency checks if a library is connected in the host application
  static HOST_HAS_CRYPTO: bool = is_lib_connected!("crypto", 4)
  ```

### 2.4 Embedding Binary and Text Resources: include_str!, include_bytes!

These macros let you pack external files directly into the final binary:

- `include_str!("relative/path.txt")` (the aliases `include_string!` and `include_strings!` are also available): reads the file as UTF-8 and embeds it as a constant `string`:
  ```yukino
  static SHADER_SRC: string = include_str!("shaders/vertex.glsl")
  ```

- `include_bytes!("relative/path.bin")`: reads the file as a raw byte sequence and embeds it as `bytes`:
  ```yukino
  static FONT_DATA: bytes = include_bytes!("assets/font.ttf")
  ```

The relative path is resolved relative to the directory containing the configuration file (`yuki.toml`, `lib.toml`, or `yukispace.toml`) in whose context the macro is invoked (if a project is compiled without a manifest, the path falls back to being resolved relative to the source file directory).

### 2.5 Static Compilation Error: compile_error!

Forces compilation to abort with the given message:

```yukino
// Build condition check
define assert_supported_target() {
    compile_error!("This architecture is not currently supported")
}
```

### 2.6 Compile-time Configuration Check: cfg!

The `cfg!(...)` macro evaluates a conditional compilation predicate during syntax analysis and returns a boolean literal (`true` or `false`):

```yukino
fun connect() {
    if cfg!(feature = "ssl") {
        print("Connecting via TLS\n")
    } else {
        print("Connecting via plaintext\n")
    }
}
```

Arbitrary predicates using boolean logic operators `all(...)`, `any(...)`, `not(...)` are supported:
- `feature = "name"` — checks if the project feature is enabled.
- `target_os = "linux"` — checks the target operating system (`"linux"`, `"windows"`, `"macos"`, `"android"`).
- `target_arch = "x86_64"` — checks the CPU architecture (`"x86_64"`, `"aarch64"`, `"arm"`).
- `target_env = "gnu"` — checks the environment and libc (`"gnu"`, `"musl"`, `"msvc"`).

Example with a compound predicate:

```yukino
if cfg!(all(target_os = "linux", not(feature = "embedded"))) {
    init_epoll_engine()
}
```

Because `cfg!(...)` resolves to a boolean literal during compilation, the dead code eliminator (`yukic-dce`) completely strips dead branches, guaranteeing zero runtime overhead.

### 2.7 Compile-time Output: compile_print!

The `compile_print!(...)` built-in macro prints formatted text directly to the compiler process's standard output (`stdout`) during compilation and immediately flushes the output stream:

```yukino
// Informational message emitted while building:
compile_print!("compiling graphics module for target: ${get_target!()}\n")

// Used in expressions (evaluates to the generated string literal):
static BUILD_BANNER: string = compile_print!("Yukino App v1.0 [profile: ${get_profileconfig!()}]\n")
```

Key aspects:
- **Instant flush**: flushes `stdout` immediately to avoid buffering delays on terminal output.
- **String interpolation**: supports `${...}` interpolation with build environment values (`target`, `version`, `profile`, `yukienv` keys) as well as nested macro invocations (`${get_target!()}`, `${file!()}`, `${line!()}`).
- **Exact text**: prints exactly what is provided without appending an implicit newline. If a newline is needed, include `\n`.
- **Return value**: evaluates to a compile-time constant `string` literal, allowing `compile_print!` to be used either as an expression statement or within variable/constant initializers.

---

## 3. Procedural Macro Packages and Macro SDK

### 3.1 What are Procedural Macros and Why Use Them

Standard application code runs when the compiled binary is executed by a user (at runtime).

A **procedural macro** is a dedicated Yukino program that executes **during project compilation**. The macro accepts a piece of source code as input tokens, performs arbitrary computations (such as reading external configuration files or database schemas), and generates new Yukino source code that the compiler immediately splices in place of the macro invocation.

Why procedural macros are useful:
1. **Eliminating Boilerplate Code:** when you need dozens of repetitive data structures, classes, or boilerplate conversion methods, a macro can generate them automatically from a high-level definition or list of arguments.
2. **Code Generation from External Files:** a macro can inspect a `schema.json` file or database schema at build time and generate strongly typed structs directly into your project.
3. **Compile-time Validation:** if arguments provided to a macro are invalid, it can emit a detailed compile-time error pointing directly to the offending file and line number.

Unlike simple syntactic `define` macros, procedural macros have full access to the standard library `std` (including filesystem, JSON/TOML parsing, cryptography, and networking), making them a complete metaprogramming platform.

### 3.2 The Official Macro SDK (macrosdk)

To streamline procedural macro development, Yukino ships an official built-in package — **Macro SDK (`macrosdk`)**.

Macro SDK is decoupled from ordinary application binaries and enabled in the package manifest via the `includes` array in `lib.toml`:

```toml
[lib]
name = "my_codegen"
version = "0.1.0"
entry = "my_codegen/lib.ykn"
output = "macro"
includes = ["stdlib", "macrosdk"]
```

Declaring `"macrosdk"` in `includes` instructs the compiler to link against the SDK host library, exposing the `macrosdk` module.

### 3.3 Core SDK Entities: TokenStream, MacroContext, MacroRunner

The `macrosdk` package exposes three central developer components:

#### 1. TokenStream — Code and Token Streams
The `TokenStream` class encapsulates a sequence of source code tokens. It accepts input tokens from the compiler, enables assembling multiple code snippets, and returns the final code back to the compiler:
- `token_stream(code: string) -> TokenStream` — factory function creating a token stream from a code string.
- `.to_string() -> string` — returns the underlying text of the token stream.
- `.is_empty() -> bool` — checks if the stream contains no code.
- `.append(other: TokenStream) -> TokenStream` — appends another token stream separated by a newline.
- `.append_str(snippet: string) -> TokenStream` — appends a raw code string snippet.

#### 2. MacroContext — Invocation Environment
The `MacroContext` class is passed to macro handler functions on each invocation. It provides execution metadata and compiler interaction primitives:
- `ctx.macro_name: string` — the name of the invoked macro.
- `ctx.working_dir: string` — absolute path to the project root directory.
- `ctx.kind: string` — macro invocation category (`"FunctionLike"`, `"Attribute"`, or `"Mutation"`).
- `ctx.attr_args: string` — raw string of arguments passed to the attribute (for example, `"GET, \"/users\""`).
- `ctx.is_attribute() -> bool` — checks if the macro was invoked as an attribute decorator `#[...]`.
- `ctx.is_mutation() -> bool` — checks if the macro was invoked as a type mutation `#[mutation(...)]`.
- `ctx.is_function_like() -> bool` — checks if the macro was invoked functionally: `name!(...)`.
- `ctx.track_file(path: string) -> MacroContext` — registers an external file read by the macro (such as a JSON schema). When this file changes, the compiler automatically triggers re-compilation.
- `ctx.error(msg: string) -> MacroContext` — emits a compile-time error.
- `ctx.error_at(span: Span, msg: string) -> MacroContext` — emits an error attached to a specific source code location.
- `ctx.warning(msg: string) -> MacroContext` — emits a compiler warning.
- `ctx.warning_at(span: Span, msg: string) -> MacroContext` — emits a warning attached to a source code location.

#### 3. MacroRunner — Dispatcher and IPC Server
The `MacroRunner` class handles the lifecycle of the macro process. It listens for compiler requests over standard input/output (IPC), resolves registered handlers, and returns generated code back:
- `runner.register(macro_name: string, handler: MacroFn) -> MacroRunner` — registers a general-purpose macro handler function under the specified name.
- `runner.register_attribute(name: string, handler: MacroFn) -> MacroRunner` — semantic helper for registering attribute-macro handlers.
- `runner.register_mutation(name: string, handler: MacroFn) -> MacroRunner` — semantic helper for registering type-mutation handlers.
- `runner.run() -> int32` — starts the IPC request processing loop until the build concludes.

The signature of macro handler functions is defined by:
```yukino
public type MacroFn = fun(MacroContext, TokenStream) -> TokenStream
```

### 3.4 Step-by-Step Tutorial: Creating a Macro with yukino new --macro

To initialize a new procedural macro package, run `yukino new` with the `--macro` flag:

```sh
yukino new my_codegen --macro
```

The Yukino project generator creates the following layout:
```text
my_codegen/
├── lib.toml
└── my_codegen/
    └── lib.ykn
```

The `lib.toml` manifest specifies `output = "macro"` and includes the SDK:
```toml
[lib]
name = "my_codegen"
version = "0.1.0"
entry = "my_codegen/lib.ykn"
yukino = ["0.1.0", "+"]
output = "macro"
includes = ["stdlib", "macrosdk"]
```

The `my_codegen/lib.ykn` entry point contains the runner bootstrap code. Here is a practical example of a macro `generate_user_model` that accepts a type name and generates a complete structure:

```yukino
// my_codegen/my_codegen/lib.ykn
load macrosdk: MacroRunner, MacroContext, TokenStream, token_stream

fun expand_user_model(ctx: MacroContext, input: TokenStream) -> TokenStream {
    set type_name = input.to_string()
    if type_name.len() == 0 {
        ctx.error("generate_user_model requires a struct name")
        return token_stream("")
    }

    // Emit a helpful build warning
    ctx.warning("generating data model for: ${type_name}")

    // Generate new source code
    set code = "struct ${type_name} {\n    id: int64\n    name: string\n    active: bool\n}"
    return token_stream(code)
}

fun main() {
    set runner = init MacroRunner()
    set _ = runner.register("generate_user_model", expand_user_model)
    set _ = runner.run()
}
```

### 3.5 Connecting and Invoking Macros in Applications

To consume the macro in an application, declare it under the `[libs.macro]` table in your `app.toml`:

```toml
# my_app/app.toml
[bin]
name = "my_app"
version = "0.1.0"
entry = "src/main.ykn"

[libs.macro.my_codegen]
path = "../my_codegen"
```

In your application code, invoke the macro using exclamation mark syntax:

```yukino
// my_app/src/main.ykn
load std.io: println

// Procedural macro invocation at compile time:
// The compiler launches my_codegen and splices in the generated structure definition!
generate_user_model!(UserAccount)

fun main() {
    // UserAccount is already a fully formed struct in your codebase:
    set user = UserAccount {
        id: 42,
        name: "Alice",
        active: true,
    }

    println("User ID: ${user.id}, Name: ${user.name}")
}
```

When invoking `yukino build` or `yukino run`:
1. The builder detects the dependency in `[libs.macro]`.
2. The `my_codegen` package is automatically built for the host machine architecture.
3. During parsing of `src/main.ykn`, the compiler encounters `generate_user_model!(UserAccount)`.
4. The compiler forwards the input token `UserAccount` to `my_codegen`.
5. The `expand_user_model` function generates `struct UserAccount { ... }`.
6. The compiler splices the generated AST into the module and produces the final binary.

### 3.6 Execution Architecture, IPC Safety, and Timeouts

Yukino deliberately avoids loading macros as dynamic libraries (`.so` / `.dll`) into the compiler's own address space.

Instead, an **isolated host process (IPC Runner)** architecture is enforced:

```text
[ Compiler yukic ]
        │
   (IPC: stdin)  ──>  [ 4-byte LE length prefix ] + [ JSON MacroRequest ]
        │
        ▼
[ Macro Binary (Host Process) ]  <──> Full access to std (std.fs, std.net)
        │
   (IPC: stdout) ──>  [ 4-byte LE length prefix ] + [ JSON MacroResponse ]
        │
        ▼
[ AST Splicing in yukic ] (Expressions or Statements)
```

Architectural Guarantees:
- **Crash Guard:** If a macro triggers a panic or runtime abort, the compiler remains stable. It captures `stderr` from the macro child process and displays an informative diagnostic pointing to the invocation site.
- **Timeout Protection:** Macro execution is protected by a time limit (10 seconds by default). This limit is configurable via the `macrotimeout` setting in `yukino config` (for example, `yukino config set macrotimeout 20`). If a macro enters an infinite loop or execution exceeds the timeout, the process is aborted and the compiler reports a timeout error.
- **Cross-Compilation Transparency:** When targeting Android, ARM, or WebAssembly, macro packages automatically compile for the host CPU architecture and run transparently.

### 3.7 External File Tracking (track_file) and Incremental CGU Cache

When a procedural macro reads external disk assets (such as via `ctx.track_file("schema.json")`), the Yukino build engine incorporates the cryptographic hashes of tracked files into its codegen unit cache key (CGU Cache).

Key Benefits:
- If `schema.json` is unchanged, the compiler reuses cached artifacts and completes builds sub-second.
- As soon as `schema.json` is modified, the compiler detects the hash change and automatically re-executes the macro, updating generated types without manual `yukino clean` invocations.

---

## 4. Macro SDK API Reference (macrosdk)

The built-in `macrosdk` package provides official toolchain primitives for developing procedural macros in pure Yukino. It includes types for code stream manipulation (`TokenStream`), compiler execution metadata (`MacroContext`), and the IPC child-process dispatcher (`MacroRunner`).

---

### 4.1 Package Import: load macrosdk

Whole package import:

```yukino
load macrosdk
```

Imported symbols:

- **Constants:**
  - No global constants are declared in `macrosdk`.
- **Types:**
  - `MacroFn` — procedural macro handler signature (`fun(MacroContext, TokenStream) -> TokenStream`).
- **Data Structures:**
  - `Span` — source code location coordinates (`file_id`, `start`, `end`, `line`, `col`);
  - `Ident` — identifier token with associated `Span` coordinates;
  - `Literal` — literal token with raw text representation and `Span` coordinates;
  - `Punct` — punctuation symbol or operator with `Span` coordinates;
  - `Diagnostic` — compiler diagnostic message record (error or warning).
- **Classes:**
  - `TokenStream` — stream of tokens and source code exchanged between the compiler and the macro;
  - `MacroContext` — invocation context, external dependency file registration, and diagnostic emission;
  - `MacroRunner` — host process macro dispatcher managing IPC communication with the compiler.
- **Functions and Factories:**
  - `span_default` — creates a `Span` with default zero coordinates;
  - `token_stream` — creates a `TokenStream` instance from a source code snippet string.

To activate `macrosdk`, procedural macro libraries declare it under the `includes` array in `lib.toml`:

```toml
[build]
includes = [
    "stdlib",
    "macrosdk",
]
```

---

### 4.2 Constants

No global constants are declared in `macrosdk`.

---

### 4.3 Types

#### 4.3.1 MacroFn

Function type alias representing a procedural macro handler callback.

```yukino
public type MacroFn = fun(MacroContext, TokenStream) -> TokenStream
```

Accepts a reference to the active `MacroContext` and the input `TokenStream` passed to the macro call site. Returns a new `TokenStream` containing generated code to be spliced into the compiler AST.

---

### 4.4 Data Structures

#### 4.4.1 Span

Represents source code coordinate offsets for an AST node or token.

```yukino
public struct Span {
    file_id: uint32
    start: uint32
    end: uint32
    line: uint32
    col: uint32
}
```

- `file_id` — integer file identifier in the compiler `SourceMap`;
- `start` — byte offset of the token start relative to the file beginning;
- `end` — byte offset of the token end relative to the file beginning;
- `line` — 1-based line number in the source file;
- `col` — 1-based column number in the source file.

#### 4.4.2 Ident

Represents an identifier token (variable, function, struct, or keyword name).

```yukino
public struct Ident {
    name: string
    span: Span
}
```

- `name` — string identifier name;
- `span` — source coordinates of the token.

#### 4.4.3 Literal

Represents a literal token (integer, float, string, boolean, or byte literal).

```yukino
public struct Literal {
    raw: string
    span: Span
}
```

- `raw` — exact raw text representation of the literal as written in source code;
- `span` — source coordinates of the literal.

#### 4.4.4 Punct

Represents a punctuation token or operator (`+`, `-`, `*`, `::`, `->`, etc.).

```yukino
public struct Punct {
    ch: string
    span: Span
}
```

- `ch` — string punctuation character or sequence;
- `span` — source coordinates of the punctuation mark.

#### 4.4.5 Diagnostic

Represents a compiler diagnostic message emitted by a macro back to the compiler.

```yukino
public struct Diagnostic {
    severity: string
    message: string
    file_id: uint32
    line: uint32
    col: uint32
}
```

- `severity` — message severity level (`"Error"` or `"Warning"`);
- `message` — human-readable diagnostic message;
- `file_id` — source file ID (0 if unassociated with a specific file);
- `line` — line number (0 if unassociated);
- `col` — column number (0 if unassociated).

---

### 4.5 Classes

#### 4.5.1 class TokenStream

Encapsulates a stream of tokens and generated source code fragments passed to and returned from procedural macros.

```yukino
public class TokenStream {
    code: string

    public fun init(code: string)
    public fun to_string() -> string
    public fun is_empty() -> bool
    public fun append(other: TokenStream) -> TokenStream
    public fun append_str(snippet: string) -> TokenStream
}
```

The `code` field stores the textual representation of the code stream.

#### 4.5.2 class MacroContext

Provides invocation context for the currently executing macro, project root directory access, external dependency file registration, and diagnostic emission.

```yukino
public class MacroContext {
    public tracked_files: string[]
    public diagnostics: Diagnostic[]
    public macro_name: string
    public working_dir: string
    public kind: string
    public attr_args: string

    public fun init(
        macro_name: string,
        working_dir: string,
        kind: string = "FunctionLike",
        attr_args: string = ""
    )
    public fun is_attribute() -> bool
    public fun is_mutation() -> bool
    public fun is_function_like() -> bool
    public fun track_file(path: string) -> MacroContext
    public fun error(msg: string) -> MacroContext
    public fun error_at(span: Span, msg: string) -> MacroContext
    public fun warning(msg: string) -> MacroContext
    public fun warning_at(span: Span, msg: string) -> MacroContext
}
```

- `tracked_files` — array of external file paths registered for CGU cache invalidation;
- `diagnostics` — array of diagnostic messages accumulated during macro evaluation;
- `macro_name` — name of the invoked macro;
- `working_dir` — absolute path of the project workspace being compiled;
- `kind` — category name string (`"FunctionLike"`, `"Attribute"`, `"Mutation"`);
- `attr_args` — raw attribute argument string (if invoked as an attribute).

#### 4.5.3 class MacroRunner

Host child-process dispatcher. Reads JSON requests from `stdin`, dispatches to registered handler callbacks, and writes JSON responses to `stdout`.

```yukino
public class MacroRunner {
    names: string[]
    handlers: MacroFn[]

    public fun init()
    public fun register(name: string, handler: MacroFn) -> MacroRunner
    public fun register_attribute(name: string, handler: MacroFn) -> MacroRunner
    public fun register_mutation(name: string, handler: MacroFn) -> MacroRunner
    public fun run() -> int32
}
```

- `names` — array of registered macro names;
- `handlers` — array of registered `MacroFn` handlers.

---

### 4.6 Function and Method Reference

#### 4.6.1 span_default

Creates a `Span` instance with default zero coordinates: `file_id = 0`, `start = 0`, `end = 0`, `line = 1`, and `col = 1`. Used when generating synthetic AST nodes that lack physical coordinates in user source files.

```yukino
fun span_default() -> Span
```

Returns a `Span` struct instance. The returned struct is not nullable.

The function never panics.

```yukino
load macrosdk.tokens: Span, span_default
load std.io: println

set sp: Span = span_default()
println("Line: ${sp.line}, Col: ${sp.col}")
```

#### 4.6.2 token_stream

Factory function that constructs a new `TokenStream` instance from a source code snippet string.

```yukino
fun token_stream(code: string) -> TokenStream
```

Returns a new `TokenStream` instance. The returned instance is not nullable.

The function never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("println(\"hello world\")")
println("Stream code: ${stream.to_string()}")
```

#### 4.6.3 TokenStream.init

Constructor for `TokenStream`. Initializes the internal code buffer with the provided snippet string `code`.

```yukino
fun init(code: string)
```

Initializes the `TokenStream` instance and returns no value.

The constructor never panics.

```yukino
load macrosdk.tokens: TokenStream
load std.io: println

set stream = init TokenStream("set x: int32 = 42")
println("Code: ${stream.to_string()}")
```

#### 4.6.4 TokenStream.to_string

Converts the token stream into its string representation for consumption by the compiler or logging.

```yukino
fun to_string() -> string
```

Returns a `string` primitive. The returned string is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("fun add(a: int32, b: int32) -> int32 { return a + b }")
set code: string = stream.to_string()
println("Generated code length: ${code.len()}")
```

#### 4.6.5 TokenStream.is_empty

Checks whether the token stream contains an empty code snippet.

```yukino
fun is_empty() -> bool
```

Returns a `bool` primitive. Returns `true` if the internal string length is 0, otherwise `false`.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set empty_stream: TokenStream = token_stream("")
set non_empty: TokenStream = token_stream("42")

println("Empty: ${empty_stream.is_empty()}")
println("Non-empty: ${non_empty.is_empty()}")
```

#### 4.6.6 TokenStream.append

Appends the contents of another `TokenStream` instance `other` to the current stream. When the current stream is non-empty, the addition is separated by a newline (`\n`), cleanly separating multiline blocks of code.

```yukino
fun append(other: TokenStream) -> TokenStream
```

Returns a reference to the current `TokenStream` instance to enable fluent chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set part1: TokenStream = token_stream("set x: int32 = 10")
set part2: TokenStream = token_stream("set y: int32 = 20")

part1.append(part2)
println(part1.to_string())
```

#### 4.6.7 TokenStream.append_str

Appends a raw code snippet string `snippet` to the current token stream. If the current stream is non-empty, the snippet is appended on a new line.

```yukino
fun append_str(snippet: string) -> TokenStream
```

Returns a reference to the current `TokenStream` instance to enable fluent chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load std.io: println

set stream: TokenStream = token_stream("public struct Config {")
stream.append_str("    host: string")
stream.append_str("    port: int32")
stream.append_str("}")

println(stream.to_string())
```

#### 4.6.8 MacroContext.init

Constructor for `MacroContext`. Creates a new invocation context with the given `macro_name`, project working directory path `working_dir`, invocation category `kind` (`"FunctionLike"`, `"Attribute"`, `"Mutation"`), and raw attribute arguments string `attr_args`. Initializes `tracked_files` and `diagnostics` as empty arrays.

```yukino
fun init(
    macro_name: string,
    working_dir: string,
    kind: string = "FunctionLike",
    attr_args: string = ""
)
```

Initializes the `MacroContext` instance and returns no value.

The constructor never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("my_macro", "/workspace", "Attribute", "GET, \"/users\"")
println("Macro: ${ctx.macro_name}, Kind: ${ctx.kind}, Args: ${ctx.attr_args}")
```

#### 4.6.9 MacroContext.is_attribute

Checks whether the macro was invoked as an attribute decorator on an item declaration (`#[macro_name(...)]`).

```yukino
fun is_attribute() -> bool
```

Returns `true` if the category is `"Attribute"`, otherwise `false`.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("logged", "/workspace", "Attribute", "")
println("Is attribute: ${ctx.is_attribute()}")
```

#### 4.6.10 MacroContext.is_mutation

Checks whether the macro was invoked as a type mutation macro via `#[mutation(macro_name)]`.

```yukino
fun is_mutation() -> bool
```

Returns `true` if the category is `"Mutation"`, otherwise `false`.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("JsonModel", "/workspace", "Mutation", "")
println("Is mutation: ${ctx.is_mutation()}")
```

#### 4.6.11 MacroContext.is_function_like

Checks whether the macro was invoked in functional form: `macro_name!(...)`.

```yukino
fun is_function_like() -> bool
```

Returns `true` if the category is `"FunctionLike"`, otherwise `false`.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("ping", "/workspace", "FunctionLike", "")
println("Is function-like: ${ctx.is_function_like()}")
```

#### 4.6.12 MacroContext.track_file

Registers an external file path `path` (relative or absolute) as a build dependency. The compiler hashes all tracked files and automatically re-invokes the procedural macro whenever file contents change on disk.

```yukino
fun track_file(path: string) -> MacroContext
```

Returns a reference to the current `MacroContext` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("schema_macro", "/workspace")
ctx.track_file("models/user_schema.json")

println("Tracked file count: ${ctx.tracked_files.len()}")
```

#### 4.6.13 MacroContext.error

Emits a general compilation error without an explicit source span (`file_id = 0`, `line = 0`, `col = 0`). The presence of any `"Error"` diagnostic aborts compilation and prints the error message to the terminal.

```yukino
fun error(msg: string) -> MacroContext
```

Returns a reference to the current `MacroContext` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("validate_macro", "/workspace")
ctx.error("Configuration file schema.json not found")

println("Diagnostic count: ${ctx.diagnostics.len()}")
```

#### 4.6.14 MacroContext.error_at

Emits a compilation error anchored to a specific source coordinate `span`. The compiler highlights the exact source location in the user's file in red and displays the provided error message.

```yukino
fun error_at(span: Span, msg: string) -> MacroContext
```

Returns a reference to the current `MacroContext` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: Span, span_default
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("syntax_macro", "/workspace")
set sp: Span = span_default()
ctx.error_at(sp, "Expected string model name")

println("Diagnostic severity: ${ctx.diagnostics[0].severity}")
```

#### 4.6.15 MacroContext.warning

Emits a general compiler warning (`"Warning"`) without a specific source coordinate. The warning is displayed in compiler diagnostics but does not halt the build.

```yukino
fun warning(msg: string) -> MacroContext
```

Returns a reference to the current `MacroContext` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("lint_macro", "/workspace")
ctx.warning("Field 'legacy_token' is deprecated and will be removed in the next release")

println("Warning message: ${ctx.diagnostics[0].message}")
```

#### 4.6.16 MacroContext.warning_at

Emits a compiler warning anchored to specific source coordinates `span` in user code.

```yukino
fun warning_at(span: Span, msg: string) -> MacroContext
```

Returns a reference to the current `MacroContext` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: Span, span_default
load macrosdk.context: MacroContext
load std.io: println

set ctx = init MacroContext("lint_macro", "/workspace")
set sp: Span = span_default()
ctx.warning_at(sp, "Usage of deprecated field type")

println("Warning count: ${ctx.diagnostics.len()}")
```

#### 4.6.17 MacroRunner.init

Constructor for `MacroRunner`. Initializes a macro host dispatcher with empty registries of macro names and handler callbacks.

```yukino
fun init()
```

Initializes the `MacroRunner` instance and returns no value.

The constructor never panics.

```yukino
load macrosdk.runner: MacroRunner
load std.io: println

set runner = init MacroRunner()
println("MacroRunner initialized")
```

#### 4.6.18 MacroRunner.register

Registers a macro handler function `handler` under the specified macro name `name`. When the compiler encounters an invocation of this macro name, it forwards the call to this handler.

```yukino
fun register(name: string, handler: MacroFn) -> MacroRunner
```

Returns a reference to the current `MacroRunner` instance to support fluent method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun my_handler(ctx: MacroContext, in_stream: TokenStream) -> TokenStream {
    return token_stream("println(\"macro invoked\")")
}

set runner = init MacroRunner()
runner.register("my_macro", my_handler)
println("Handler registered successfully")
```

#### 4.6.19 MacroRunner.register_attribute

Registers an attribute-macro handler callback `handler` under the specified macro name `name`. When decorating language items (`fun`, `struct`, `class`, `enum`) with this attribute `#[name(...)]`, the compiler forwards the target item source text as `input_tokens` and attribute argument string in `ctx.attr_args`.

```yukino
fun register_attribute(name: string, handler: MacroFn) -> MacroRunner
```

Returns a reference to the current `MacroRunner` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun expand_trace(ctx: MacroContext, item_stream: TokenStream) -> TokenStream {
    return token_stream("public fun logged_worker() -> int32 { return 100 }")
}

set runner = init MacroRunner()
runner.register_attribute("trace", expand_trace)
println("Attribute macro registered successfully")
```

#### 4.6.20 MacroRunner.register_mutation

Registers a type mutation macro handler callback `handler` under the specified macro name `name`. When annotating a struct, class, or enum with `#[mutation(name)]`, the compiler forwards the type declaration in `input_tokens` and splices returned companion code (methods, serializers, converters) into the module.

```yukino
fun register_mutation(name: string, handler: MacroFn) -> MacroRunner
```

Returns a reference to the current `MacroRunner` instance for method chaining. The returned reference is not nullable.

The method never panics.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner
load std.io: println

fun expand_to_string(ctx: MacroContext, type_decl: TokenStream) -> TokenStream {
    return token_stream("public fun user_to_string(u: User) -> string { return \"User\" }")
}

set runner = init MacroRunner()
runner.register_mutation("ToString", expand_to_string)
println("Mutation macro registered successfully")
```

#### 4.6.21 MacroRunner.run

Starts an event loop listening for compiler IPC requests over `stdin` and returning responses over `stdout`. For each incoming request, `MacroRunner` identifies the matching macro name, dispatches execution, and formats output tokens, tracked files, and diagnostics into a framed JSON payload. Exits cleanly returning `0` upon receiving EOF from the compiler.

```yukino
fun run() -> int32
```

Returns a 32-bit integer exit code `int32` (`0` on clean compiler session termination).

The method never panics; all internal handler errors and exceptions are caught and returned to the compiler as structured protocol diagnostic errors.

```yukino
load macrosdk.tokens: TokenStream, token_stream
load macrosdk.context: MacroContext
load macrosdk.runner: MacroRunner

fun expand_ping(ctx: MacroContext, in_stream: TokenStream) -> TokenStream {
    return token_stream("println(\"pong\")")
}

fun main() -> int32 {
    set runner = init MacroRunner()
    runner.register("ping", expand_ping)
    return runner.run()
}
```

---

[← Back: Modules, Imports, and Visibility](9-modules.md) | [Next: System C ABI and C FFI →](11-cabi.md)
