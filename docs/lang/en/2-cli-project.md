# Toolchain and Project Structure

The `yukino` CLI project manager handles the lifecycle of applications and libraries: it initializes templates, manages the `yuki.toml` manifest, resolves local, Git, and archive dependencies, tracks locks in `yuki.lock`, compiles `build.ykn` pre-build scripts, and coordinates the low-level `yukic` compiler.

---

## Contents

1. [yukino CLI commands](#1-yukino-cli-commands)  
   1.1 [Creating a project: yukino new](#11-creating-a-project-yukino-new)  
   1.2 [Building: yukino build](#12-building-yukino-build)  
   1.3 [Running: yukino run](#13-running-yukino-run)  
   1.4 [Testing: yukino test](#14-testing-yukino-test)  
   1.5 [Cleaning: yukino clean](#15-cleaning-yukino-clean)  
   1.6 [Global configuration: yukino config](#16-global-configuration-yukino-config)
2. [The yuki.toml manifest](#2-the-yukitoml-manifest)  
   2.1 [The [bin] section (executable)](#21-the-bin-section-executable)  
   2.2 [The [lib] section (library)](#22-the-lib-section-library)  
   2.3 [Managing dependencies: [libs.runtime] and [libs.build]](#23-managing-dependencies-libsruntime-and-libsbuild)  
   2.4 [Low-level C and system libraries: [libs.static]](#24-low-level-c-and-system-libraries-libsstatic)  
   2.5 [Target-specific dependencies: [targets.<triple>]](#25-target-specific-dependencies-targetstriple)  
   2.6 [Compilation profiles: [config.<profile>]](#26-compilation-profiles-configprofile)  
   2.7 [Build flags and conditional compilation: [features]](#27-build-flags-and-conditional-compilation-features)  
   2.8 [The test section: [test]](#28-the-test-section-test)
3. [The yuki-work working directory and the yuki.lock lockfile](#3-the-yuki-work-working-directory-and-the-yukilock)
4. [The build.ykn pre-build script](#4-the-buildykn-pre-build-script)  
   4.1 [Environment variables](#41-environment-variables)  
   4.2 [stdout directives](#42-stdout-directives)  
   4.3 [build.ykn example](#43-buildykn-example)

---

## 1. yukino CLI commands

The `yukino` utility is available globally on the system and accepts subcommands and flags.

### 1.1 Creating a project: yukino new

Creates a new project with a basic skeleton:

```sh
# Executable binary (default; generates src/main.ykn and the [bin] section in yuki.toml)
yukino new my_app

# Static library (generates <name>/lib.ykn and the [lib] section with output = "static" in lib.toml)
yukino new my_lib --static

# Dynamic / C-compatible library (.so / .dll, output = "shared" in lib.toml)
yukino new my_shared --dynamic

# Procedural macro package (generates <name>/lib.ykn and output = "macro" in lib.toml)
yukino new my_macro --macro

# Universal workspace (generates yukispace.toml, libs/, and apps/ directories)
yukino new my_workspace --workspace

# Initialize a project in the current directory (existing files are not overwritten)
yukino new -i
```

Flags of the `new` subcommand:
- `--app` — create an executable application (the default behavior).
- `--static` — create a static library (`output = "static"`).
- `--dynamic` — create a shared library (`output = "shared"`).
- `--macro` — create a procedural macro package (`output = "macro"`).
- `--workspace` — create a multi-package universal workspace (`yukispace.toml`).
- `-i` / `--init` — initialize the project in the current directory instead of creating a new one.

> [!NOTE]
> For monorepos and multi-package setups combining multiple libraries and applications in a single repository, Yukino provides universal workspaces (`yukispace.toml`). A dedicated guide covering workspace structure, virtual inline packages, and build inheritance is available at: [Workspaces and Multi-Package Projects](15-workspaces.md).

### 1.2 Building: yukino build

Compiles the project according to the manifest:

```sh
# Default build (debug profile)
yukino build

# Build with an explicit profile (debug, release, or any custom profile)
yukino build debug
yukino build release

# Force a rebuild of the project and all libraries bypassing the cache (-b / --rebuild, alias rb)
# Recompiles from scratch the project and all libraries (including stdlib and [libs.runtime] dependencies)
yukino build -b
yukino build --rebuild

# Force updating and re-downloading dependencies (--refresh-deps, alias rd)
yukino build --refresh-deps

# Cross-compilation for the specified target
yukino build --target x86_64-unknown-linux-musl

# Build and then run (app projects only)
yukino build -r

# Custom runtime archive, or building without the runtime at all
yukino build --runtime path/to/libyukino_rt.a
yukino build --no-runtime

# Enabling features (comma-separated, or by repeating the -F flag)
yukino build -F network,ssl --no-default-features

# Enable all manifest features and optional dependencies
yukino build --all-features

# Build a specific member package within a workspace (-p / --package)
yukino build -p core_lib

# Setting the number of compilation threads
yukino build -j 8

# Setting the number of codegen units (CGU)
yukino build --cgunits 4

# Overriding profile parameters on the fly (-o key=value)
yukino build -o opt_level=2 -o strip=false

# Disable compiler warnings output
yukino build --no-warns

# Display extended compilation information (profile, target, threads, features, compiler initialized)
yukino build --extra-info

# Disable compiler stdout output (outputs only 0 on success or 1 on error)
yukino build --no-out

# Disable library compilation logs (stdlib and dependencies, showing only main project files)
yukino build --no-lib-log

# Enable warnings from third-party libraries and stdlib (by default, library warnings are always muted)
yukino build --lib-warns
```

> [!WARNING]
> Binary files compiled in the `debug` profile (the default when invoking `yukino build` and `yukino run`) are very large (tens of megabytes, around 30–40 MB with the standard library). This occurs because debug builds include complete uncompressed debug symbol tables (DWARF / CodeView), disable aggressive optimizations (`opt_level = 0`), and skip strip/LTO to ensure fast compilation cycles.
>
> For production deployments, performance benchmarks, and distribution of finished executables, **always use the release profile**:
> ```sh
> yukino build release
> # or to build and immediately run:
> yukino run release
> ```
> The `release` profile automatically activates compiler optimizations (`opt_level = 3`), Link-Time Optimization (`lto = true`), Dead Code Elimination (`strip_unused = true`), and debug symbol stripping (`strip = true`), dramatically reducing the binary size and improving performance.

#### Supported platforms

The compiler supports the following target platforms:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-linux-android`

To print a table of supported platforms with architecture, OS, libc, and host target parameters, use the command:

```sh
yukic targets
```

### 1.3 Running: yukino run

Compiles the application (if the artifact is outdated) and runs it:

```sh
# Run the debug profile (the default profile)
yukino run debug

# Run a specific binary package from a workspace (-p / --package)
yukino run -p cli_tool

# Run with command-line arguments passed to the application
yukino run -- --port 8080 --host 0.0.0.0

# Run with active feature flags
yukino run -F debug_log,ssl
yukino run --all-features

# Run with configuration overrides
yukino run -o opt_level=1 -- arg1

# Run with warnings disabled or with extended information
yukino run --no-warns
yukino run --extra-info

# Force rebuilding the project and libraries before running (-b / --rebuild, alias rb)
yukino run -b
yukino run --rebuild

# Build without stdout output (0/1) before running
yukino run --no-out

# Build without library compilation logs before running
yukino run --no-lib-log

# Enable warnings from third-party libraries when building before running
yukino run --lib-warns
```

All arguments after the `--` separator are passed directly to the launched binary and are available through the `args()` call.

### 1.4 Testing: yukino test

Runs the Yukino test runner: `yukino` builds a test binary from `#[test]` functions in `yuki-work/local/build/test/<triple>/<name>/` and executes it.

```sh
# Run all project tests
yukino test

# Test a specific package in a workspace (-p / --package)
yukino test -p core_lib

# Run tests with all features enabled
yukino test --all-features

# Run tests with explicit features and disabled default features
yukino test -F integration --no-default-features

# Run with a test name or suite filter
yukino test test_auth

# Exact test name matching
yukino test "math::test_add" --exact

# Show test stdout/stderr live during execution
yukino test --nocapture

# Run only ignored tests (#[ignore])
yukino test --ignored

# Run all tests including ignored ones
yukino test --include-ignored

# Abort immediately upon first failure
yukino test --fail-fast

# List available tests without running
yukino test --list

# Override per-test timeout in milliseconds
yukino test --timeout 5000

# Number of parallel test execution threads
yukino test -j 4

# Force rebuilding test binary (-b / --rebuild, alias rb)
yukino test -b
yukino test --rebuild
```

A comprehensive guide covering test organization, `#[test]`, `#[should_panic]`, `#[ignore]`, `#[timeout_ms]` attributes, and the `tests/` directory is available at: [16. Testing and Test Runner](16-tests.md).

### 1.5 Cleaning: yukino clean

Removes temporary artifacts and compiled object files:

```sh
# Cleans the yuki-work/local/ directory (object files, built binaries)
yukino clean

# Full cleanup, including removal of the yuki-work/common/yuki.lock lockfile (-a / --all)
yukino clean -a
```

### 1.6 Global configuration: yukino config

Lets you view and modify the global settings of `yukino`. The configuration file location is platform-dependent: Linux — `$XDG_CONFIG_HOME/yukino-lang/config.json` (by default `~/.config/yukino-lang/config.json`), Windows — `%APPDATA%\yukino-lang\config.json`, macOS — `~/Library/Application Support/yukino-lang/config.json`. The path can be overridden with the `YUKINO_CONFIG_PATH` environment variable.

```sh
# Path to the global configuration file
yukino config path

# Print all saved settings
yukino config list

# Read and write an arbitrary value
yukino config get cthreads
yukino config set minibar true
yukino config set full-lib-log true
yukino config set macrotimeout 15

# Set global defaults
yukino config cthreads 8
yukino config macrotimeout 15
yukino config copy_output_path true
```

#### Configuration parameters

| Key | Type | Description |
|---|---|---|
| `cthreads` | `int` | Number of parallel compilation and codegen threads (overridden by `-j / --threads`). |
| `minibar` | `bool` | Core panel display mode during compilation. |
| `noanim` | `bool` | Disable compilation panel disappearance animation (`true` for instant wipe, `false` for animation enabled). |
| `full-lib-log` | `bool` | Verbose library compilation logs (`true` to show compilation of each file and CGU, `false` to show only library-level build status). |
| `macrotimeout` | `int` | Procedural macro execution timeout in seconds (hang protection). |
| `copy_output_path` | `bool` | Copy relative path of output binary to clipboard on successful compilation (`true` to copy, `false` disabled). |

#### Defaults

- `cthreads`: `8`
- `minibar`: `false`
- `noanim`: `false`
- `full-lib-log`: `false`
- `macrotimeout`: `10`
- `copy_output_path`: `false`

Detailed explanation of `copy_output_path`:
- With `copy_output_path = true`, upon successful project compilation the relative path to the compiled binary artifact (for example, `yuki-work/local/build/debug/x86_64-unknown-linux-gnu/app/app`) is automatically copied to the system clipboard (supported on both Linux and Windows).
- With `copy_output_path = false` (default), the binary path is not copied.

Detailed explanation of `full-lib-log`:
- With `full-lib-log = false` (default), only `Compiling {library} library` and `Compiled {library} library` are printed to the console log for dependencies and the standard library. Individual files and codegen units (CGU) of libraries do not clutter the log, while the interactive core dashboard continues to reflect their compilation in real time.
- With `full-lib-log = true`, verbose compilation logs for all intermediate library files and CGUs are printed.

Core panel display modes:
- With `minibar = false` (default), a multi-line dashboard is displayed with each active core, automatically moving idle cores to the bottom and collapsing them into `○ [##] <N> more cores are currently idle`:
  ```text
  ● [01] codegen    cgu_0 (fn_main)  [ 3/10 |  30%]
  ● [02] optimizing cgu_1            [ 2/2  | 100%]
  ○ [##] 6 more cores are currently idle
  ```
- With `minibar = true`, a single compact line is displayed with an active task summary:
  ```text
  Processing 2/8 busy: [01] codegen cgu_0 · [02] opt cgu_1
  ```

---

## 2. The yuki.toml manifest

The manifest is located at the project root: for applications it is `yuki.toml` (the `[bin]` section), for libraries — `lib.toml` (the `[lib]` section). The manifest defines build settings, the dependency tree, and compilation parameters.

### 2.1 The [bin] section (executable)

Used for binary applications:

```toml
[bin]
name = "my_app"              # Executable file name in release mode
namedbg = "my_app_debug"      # Executable file name when building the debug profile
version = "0.1.0"            # Project version
entry = "src/main.ykn"        # Main entry point
yukino = ["0.1.0", "+"]       # Supported compiler version range [min, max]
includes = ["stdlib"]        # Built-in packages for the project (defaults to ["stdlib"])
```

The `includes` configuration manages built-in compiler packages (builtins) for the project:
- When initializing a new project (`yukino new`), `includes = ["stdlib"]` is automatically added to the bottom of the `[bin]` or `[lib]` section.
- If `includes` is omitted from the manifest, the project defaults to `includes = ["stdlib"]`.
- If a specific builtin (such as `stdlib`) is not in the array (for example, `includes = []`), it is not included: its modules are unavailable (even when using `load std`), and its archives are neither compiled nor linked.
- The language runtime (`runtime`) is always linked independently of `includes` (unless `--no-runtime` is passed).

### 2.2 The [lib] section (library)

Used for library packages:

```toml
[lib]
name = "my_lib"
version = "0.1.0"
entry = "my_lib/lib.ykn"      # Library entry point
yukino = ["0.1.0", "+"]
output = "static"             # Library type: "static" (.a), "shared" (.so / .dll), or "macro" (host-compiled procedural macro); defaults to "static"
includes = ["stdlib"]        # Built-in packages for the project (defaults to ["stdlib"])
```

### 2.3 Managing dependencies: [libs.runtime] and [libs.build]

Yukino strictly separates runtime and build contexts:
- `[libs.runtime]` — runtime dependencies. Libraries are compiled for the build target platform (`target`) and are available only to the main application code (under `src/`).
- `[libs.build]` — build-time dependencies. All libraries in this section are compiled for the host platform (where the compiler runs) and enter the module loading scope of the pre-build script `build.ykn`. You can load them via `load <name>: ...` inside `build.ykn`. They are not bundled into the final application executable unless required at runtime. If a dependency is an executable code-generation utility (an app with `main`), Yukino automatically compiles and runs it before compiling the main code.

The preferred and compact style for describing dependencies is the `[libs.runtime]` (or `[libs.build]`) table with inline tables for each package:

```toml
[libs.runtime]
# 1. Local dependency (path)
local_math = { path = "../libs/local_math", features = ["simd"], default_features = false }

# 2. Git repository by tag
json_parser = { git = "https://github.com/example/json-yukino", tag = "v1.2.0" }

# 3. Git repository by branch
crypto_lib = { git = "https://github.com/example/crypto-yukino", branch = "main" }

# 4. Git repository by exact commit
exact_dep = { git = "https://github.com/example/exact-dep", commit = "d3b07384d113edec49eaa6238ad5ff00" }

# 5. Downloading an archive from a URL with hash verification (link, checksum)
zlib_wrapper = { link = "https://example.com/releases/zlib_wrapper-0.3.0.tar.zst", checksum = "blake3:5d0f8a1d4c3b5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b" }
```

> [!TIP]
> Writing dependencies as separate sections (for example, `[libs.runtime.local_math]`) is also valid per the TOML standard, but a single `[libs.runtime]` section with inline tables is preferred for readability.

Supported package description parameters (`DependencySpec`):
- `path`: relative path to the local library directory (containing `yuki.toml`, `lib.toml`, or `yukispace.toml`).
- `package`: name of a specific member package inside a multi-package repository or workspace (interchangeable aliases `sublib`, `subpath`, `lib` are also supported).
- `optional`: boolean flag (`optional = true`), making the dependency optional (it will not be built or linked unless explicitly activated by a `dep:<name>` feature rule).
- `git`: URL of a remote Git repository (`https://...` or `git@...`).
- `tag`: a specific Git release tag.
- `branch`: a Git branch (automatically retracked with `--refresh-deps`).
- `commit`: the exact SHA hash of a commit.
- `link`: direct URL of a downloadable archive (the `.tar.zst`, `.tar.gz`, `.tar.xz`, `.zip`, `.tar` formats and the `.tgz`, `.txz` shorthands are supported).
- `checksum`: BLAKE3 checksum of the archive in the `blake3:<hex>` form for integrity verification.
- `features`: list of dependency features (for example, `["simd", "ssl"]`), passed to the compiler when compiling the dependency.
- `default_features`: control over the dependency's default features (`default_features = false` disables the dependency's `default` feature list).

### 2.4 Low-level C and system libraries: [libs.static]

The `[libs.static]` section controls direct parameter passing to the linker:

```toml
[libs.static]
# Additional precompiled .o files
objects = ["src/native_glue.o"]

# Paths to static .a archives
archives = ["libs/libcustom_c.a"]

# Library search directories for the -L flag
search_paths = ["libs/", "/usr/local/lib"]

# System libraries for the -l flag (links -lm, -lpthread, -ldl)
system = ["m", "pthread", "dl"]
```

### 2.5 Target-specific dependencies: [targets.<triple>]

Allows overriding dependencies and system libraries depending on the target platform (Target Triple):

```toml
# Settings exclusively for Linux x86_64
[targets."x86_64-unknown-linux-gnu".libs.static]
system = ["pthread", "dl", "rt"]

# Settings for Windows MSVC
[targets."x86_64-pc-windows-msvc".libs.static]
system = ["ws2_32", "user32", "advapi32"]
```

### 2.6 Compilation profiles: [config.<profile>]

Each profile (for example, `[config.release]`, `[config.debug]`) describes optimization parameters. The `release` profile defaults to `opt_level = 3`, `lto = true`, `strip = true`, `strip_unused = true`, and `inline_statics = true`; any other profile starts from the debug defaults (`opt_level = 0`, `debug_assertions = true`). Unspecified fields fall back to these defaults.

> [!NOTE]
> Because debug builds include full uncompressed debug symbol tables and zero optimizations, binaries are substantially larger. Switching to the `release` profile enables `strip = true`, `opt_level = 3`, and `lto = true`, significantly reducing the final binary size.

Only the following fields currently affect the build:

```toml
[config.release]
opt_level = 3                 # Optimization level: 0, 1, 2, 3, "s" (size), "z" (aggressive size)
lto = true                    # Link-Time Optimization: false, true/"fat", "thin" (all enabled variants pass a single --lto flag)
strip = true                  # Stripping debug symbols from the final file
strip_unused = true           # Dead Code Elimination (DCE)
function_sections = true      # Place functions and static data into dedicated sections (.text.<sym>, .rodata.<sym>) for dead-code stripping by the linker (--gc-sections)
codegen_units = 8             # Number of parallel codegen units (CGU)
```

The `function_sections = true` option is generated by default under `[config.release]` when creating a new project.

Resolution priority for codegen units (`cgunits`):
1. Explicit CLI flag `--cgunits <N>` (or `--codegen-units <N>`).
2. The `codegen_units` setting in the active profile `[config.<profile>]`.
3. The `cthreads` setting in global `config.json`.
4. Number of CPU cores (fallback).

The remaining fields below are recognized by the manifest parser (and can be overridden with `-o`), but they are not passed to the `yukic` compiler yet and do not affect the build result:

```toml
inline_statics = true         # Inlining static constants at their usage sites (reserved)
debug_assertions = false      # Internal compiler assert checks (reserved)
panic_strategy = "unwind"     # Panic strategy: "unwind" (stack unwinding) or "abort" (reserved)
fast_math = false             # Aggressive FP optimizations (reserved)
cpu = "native"                # LLVM CPU model: "native", "generic", "x86-64-v3", etc. (reserved)
features = ["+avx2", "+fma"]  # Specific CPU instructions (reserved)
reloc_model = "pic"           # Relocation model: "pic", "static" (reserved)
code_model = "default"        # "default", "small", "kernel", "medium", "large" (reserved)
emit = ["bin"]                # Output formats: "bin", "obj", "asm", "llvm-ir" (reserved)
```

### 2.7 Build flags and conditional compilation: [features]

Defines the list of features, dependencies between features, forwarding into sub-dependencies, and optional dependency activation:

```toml
[libs.runtime.curl]
path = "../vendor/curl"
optional = true                 # Optional dependency

[features]
default = ["std_io"]
std_io = []
network = []
ssl = ["network", "dep:curl"]   # Activates network and pulls in optional dependency curl
crypto_fast = ["crypto/simd"]  # Forwards feature simd to dependency crypto
crypto_opt = ["crypto?/avx"]   # Conditional forwarding: forwards avx only if crypto is active
```

#### Feature Rule Syntax:
- `dep:name` — explicitly enables optional dependency `name` (`optional = true`). The dependency is compiled and linked only when this feature is activated.
- `dep_name/feat` — unconditional feature forwarding: enables feature `feat` in dependency `dep_name`.
- `dep_name?/feat` — conditional feature forwarding: enables feature `feat` in dependency `dep_name` only if `dep_name` is already activated via another feature rule.

#### Feature Unification Engine:
When multiple packages across a dependency tree require the same dependency with distinct feature sets (a diamond dependency graph: package `A` requires feature `X`, while package `B` requires feature `Y`), the `yukino` package manager performs automatic unification. The shared dependency is compiled exactly once with the union of all required active features `["X", "Y"]`.

#### Checking Features in Yukino Code:
Inside source code, features can be queried in several ways:

1. **Unified Configuration Attribute `#[cfg(...)]` (Standard form):**
   ```yukino
   #[cfg(feature = "network")]
   fun init_sockets() {
       space
   }

   #[cfg(all(feature = "ssl", target_os = "linux"))]
   fun init_linux_ssl() {
       space
   }
   ```

2. **Compile-time Constant Macro `cfg!(...)`:**
   Evaluates during syntax analysis directly into constant `true` or `false`:
   ```yukino
   if cfg!(feature = "network") {
       connect()
   }
   ```

3. **Shorthand Syntax `#[feature = "..."]` and `#[!feature = "..."]`:**
   An intentionally concise and readable shorthand without `cfg(...)`. The prefixed attribute `#[!feature = ...]` acts as negation: the item is compiled when the feature is disabled.

### 2.8 The test section: [test]

Configuration of the built-in test framework:

```toml
[test]
threads = "auto"              # Number of threads ("auto" or a fixed number, for example 4)
timeout_ms = 10000            # Timeout for a single test in milliseconds
capture_output = true         # Capture stdout/stderr during tests
fail_fast = false             # Stop execution on the first failed check
tests_dir = "tests"           # Directory with integration test files
```

### 2.9 Custom codegen units: [customcgus]

The `[customcgus]` section allows manually partitioning a project or library into isolated codegen units (CGUs). This accelerates parallel compilation and makes static libraries modular for linkers (the linker extracts only those `.o` object files from `.a` archives whose symbols are actually referenced):

```toml
[customcgus]
cgu_io = ["std/io", "std/platformio", "std/outf"]
cgu_fs = ["std/fs"]
cgu_strings = ["std/strings", "std/numf"]
cgu_math = ["std/math", "std/random"]
cgu_compress = ["std/compress"]
```

Partitioning rules:
- If a group name does not begin with `cgu_`, the compiler automatically prepends `cgu_` (for example, `io` becomes `cgu_io`).
- Values are arrays of file path substrings or glob patterns.
- All items and functions not matching any pattern in `[customcgus]` are automatically distributed among remaining CGUs using the compiler's LPT load-balancing algorithm.
- The `[customcgus]` section is not generated automatically on `yukino new`; authors add it whenever granular modularity tuning is required.

---

## 3. The yuki-work working directory and the yuki.lock lockfile

On the first build, `yukino` automatically creates a local working environment:

```
my_app/
├── yuki.toml                 # Project manifest
├── src/
│   └── main.ykn
└── yuki-work/                # Build working directory
    ├── common/
    │   └── yuki.lock         # File pinning exact versions, hashes, and commits
    └── local/                # Build directory (added to .gitignore entirely)
        ├── deps/             # Downloaded local copies of packages
        │   ├── cache/        # Downloaded tarball archives (.tar.zst and others)
        │   └── json_parser/  # Unpacked dependency sources
        ├── cache/            # Compiler CGU object cache
        └── build/
            └── debug/        # Object files and binaries of the debug profile
                └── x86_64-unknown-linux-gnu/
                    └── my_app/
                        └── my_app_debug
```

Only `yuki-work/local/` is added to `.gitignore` automatically; the `yuki-work/common/` directory with the `yuki.lock` file is meant to be committed to the repository. On Windows the executable file name gets the `.exe` extension.

### The yuki.lock lockfile

The `yuki-work/common/yuki.lock` file is generated automatically (TOML format) and pins the exact build state: the target triple, the compiler version, and the profile name, plus for every package:
- The package name, version, and type (`app`, `static`, etc.).
- The source (`path+...`, `link+...`, or `git+...`).
- The computed BLAKE3 hash (`checksum = "blake3:..."`): for downloaded archives it is the archive hash, for local sources — a checksum of the source tree.
- The exact pinned SHA of a Git dependency commit (`commit = "..."`), when present.
- The activated features and the dependency list (for the root package — the names of its runtime dependencies).

Thanks to `yuki.lock`, builds are deterministic and reproducible on any machine. To update the pinned versions, use the `yukino build --refresh-deps` flag.

---

## 4. The build.ykn pre-build script

If the project root contains a `build.ykn` file, `yukino` automatically compiles and executes it **before** building the main application.

The `build.ykn` script is written in pure Yukino and is used to:
1. Call system utilities (compiling C code with GCC/Clang, code generation).
2. Dynamically pass flags to the linker through standard output (`stdout`).
3. Read environment settings and the target build architecture.

### 4.1 Environment variables and macros

When the `build.ykn` script (and the `[libs.build]` utilities) is executed, it receives the following environment variables:

| Variable | Description |
| :--- | :--- |
| `YUKINO_PROJECT_DIR` | Project root (the directory with `yuki.toml` / `lib.toml`) |
| `YUKINO_OUT_DIR` | Artifact directory of the current build: `yuki-work/local/build/<profile>/<triple>/<name>` |
| `YUKINO_TARGET_TRIPLE` | Build target triple (for example, `x86_64-unknown-linux-gnu`) |
| `YUKINO_TARGET_OS` | OS of the target: `linux`, `windows`, or `android` |
| `YUKINO_TARGET_ARCH` | Architecture of the target: `x86_64` or `aarch64` |
| `YUKINO_PROFILE` | Active profile name (additionally, only for `[libs.build]` utilities) |

The script is compiled for the host target. Inside it, the standard library is available (imported via `load std.io: ...`), build-time libraries from the `[libs.build]` section of `yuki.toml` (imported via `load <name>: ...`), as well as built-in macros:

- `get_target!()` — the target build triple (for example, `"x86_64-unknown-linux-gnu"`).
- `get_profileconfig!()` — the name of the active build profile (`"release"`, `"debug"`, etc.).
- `get_features!()` — array of activated build features (`string[]`).
- `hostenv!("VAR_NAME")` — value of a host environment variable at build time.
- `yukienv!("key", [scope: int32])` — reading manifest settings (for example, `"bin.name"` or `"config.release.opt_level"`).
- `is_lib_connected!("lib_name", [scope: int32])` — checking whether a library is connected (`true` / `false`).

### 4.2 stdout directives

Printing certain prefixes to `stdout` from `build.ykn` is intercepted by the `yukino` build process:

| stdout directive | Description |
| :--- | :--- |
| `yuki:link-archive=PATH` | Adds a static archive (`.a`) or an object file to the link |
| `yuki:link-system=LIB` | Passes a system library to the linker (analog of `-l LIB`) |
| `yuki:warning=TEXT` | Prints a warning into the build log |

Other directives (`cfg`, `rerun-if-changed`, etc.) are not supported at the moment.

### 4.3 build.ykn example

An example `build.ykn` that passes a prebuilt native archive and a system library to the linker:

```yukino
load std.io: print

fun main() {
    // Get the target triple
    set target_arch = get_target!()
    
    // Pass a static archive (for example, one built from C code)
    print("yuki:link-archive=native_libs/libwrapper.a\n")
    
    // Pass a system library
    print("yuki:link-system=m\n")
    
    // A warning in the build log
    print("yuki:warning=native wrapper linked\n")
}
```

---

[← Back: Installing Yukino](1-setup.md) | [Next: Basic Syntax, Operators, and Control Flow →](3-basics.md)
