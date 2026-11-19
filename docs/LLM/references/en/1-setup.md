# Installing Yukino

This guide covers installing the `yukino` project manager, the `yukic` compiler driver, and building binaries for other platforms.

> [!TIP]
> **Editor Extensions and LSP:**
> A complete guide on installing editor extensions (Zed, VS Code) and setting up the language server is available in [Editor Extensions and Yukino LSP](extensions.md).

## Contents

1. [Supported platforms](#supported-platforms)
2. [Installing on Linux](#installing-on-linux)
   - 2.1 [Nix (recommended)](#nix-recommended)
   - 2.2 [Manual Cargo build](#manual-cargo-build)
3. [Installing on Windows](#installing-on-windows)
4. [Cross-compilation](#cross-compilation)
   - 4.1 [Cross-compilation requirements](#cross-compilation-requirements)
   - 4.2 [Compiler](#cross-compiling-the-compiler)
   - 4.3 [Runtime](#cross-compiling-the-runtime)
5. [Verifying the installation](#verifying-the-installation)
6. [Editor extensions (Zed, VS Code)](#editor-extensions-zed-vscode)

## Supported platforms

The compiler supports these target platforms:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-linux-android`

macOS is not supported yet. Work on macOS support will start as soon as someone with a Mac is available to develop and test it.

## Installing on Linux

### Nix (recommended)

When using Nix, all necessary build dependencies (LLVM 22, Clang, LLD, system libraries) and environment variables are configured automatically by `flake.nix`. Manual package installation and exporting variables are not needed.

Install the release package directly from the release tag:

```sh
nix profile add github:yukitsuri/yukino/v0.1.0#yukino
```

For local installation from a cloned repository:

```sh
git clone --branch v0.1.0 https://github.com/yukitsuri/yukino.git
cd yukino
nix profile add .#yukino
```

The debug package is installed with:

```sh
nix profile add .#yukino-debug
```

It provides `yukino-dbg` and `yukic-dbg`.

To enter the ready development environment with all variables set:

```sh
nix develop
```

### Manual Cargo build

If you are not using Nix, all dependencies and environment variables must be configured manually before building.

#### Build dependencies

- Rust `1.98.1` or newer;
- Cargo;
- LLVM 22;
- Clang and `libclang`;
- LLD;
- `pkg-config`;
- `libffi`, `zlib`, `libxml2`, and `ncurses`.

Exact package names depend on your Linux distribution.

#### Environment variables

Before compiling through Cargo, paths to LLVM and `libclang` must be explicitly exported in your current shell session:

```sh
export LLVM_CONFIG=/path/to/llvm-config
export LLVM_SYS_221_PREFIX=/path/to/llvm
export LIBCLANG_PATH=/path/to/libclang/lib
```

#### Building and installing

Once the environment is configured, install both binaries from the release tag:

```sh
cargo install --git https://github.com/yukitsuri/yukino --tag v0.1.0 --locked yukino
cargo install --git https://github.com/yukitsuri/yukino --tag v0.1.0 --locked yukic-driver
```

Cargo installs binaries to `~/.cargo/bin`:

- `yukino` — project and build manager;
- `yukic` — low-level compiler driver.

## Installing on Windows

On Windows, use the PowerShell script, which automatically downloads prebuilt binaries of the latest release and adds them to `PATH`:

```powershell
irm https://raw.githubusercontent.com/yukitsuri/yukino/main/scripts/wininstall.ps1 | iex
```

The script does not require manual LLVM configuration or compiling from source. If you need to build from source via Cargo manually, ensure LLVM 22 and MSVC/MinGW are installed and corresponding LLVM environment variables are set.

## Cross-compilation

### Cross-compilation requirements

Building for other platforms additionally requires:

- `cargo-zigbuild` and Zig — for Linux targets different from your host;
- Android NDK — for Android targets;
- Rust target components installed via `rustup target add <target>`.

### Cross-compiling the compiler

Clone the repository at the required tag:

```sh
git clone --branch v0.1.0 https://github.com/yukitsuri/yukino.git
cd yukino
python3 scripts/crossbuild.py
```

The script will prompt you to select targets. Selecting `0` builds all targets supported by `crossbuild.py`:

- `x86_64-pc-windows-gnu`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`;
- `aarch64-unknown-linux-gnu`;
- `aarch64-unknown-linux-musl`;
- `aarch64-linux-android`.

Build outputs are placed in:

```text
build/0.1.0/yukino/yukino-<target>
build/0.1.0/yukic/yukic-<target>
```

On Windows, use `py -3` instead of `python3`.

### Cross-compiling the runtime

The runtime is built separately and only produces static `.a` archives:

```sh
python3 scripts/crossbuild-rt.py
```

Outputs are placed in:

```text
build/0.1.0/runtime/<target>.a
```

Pass this runtime to `yukic` for custom targets via `--runtime <path>`. For the host target, an external runtime archive is not required: `yukic` uses its built-in host runtime.

## Verifying the installation

```sh
yukino -v
yukic -v
yukino --help
yukic --help
yukic targets
```

If a command is not found, make sure `~/.cargo/bin` or `%USERPROFILE%\.cargo\bin` is in `PATH`.

## Editor extensions (Zed, VS Code)

To configure syntax highlighting, autocompletion, type hints, and code navigation, install the official editor extensions and the `yukino-lsp` language server. Refer to [Editor Extensions and Yukino LSP](extensions.md) for detailed step-by-step instructions.

---

[← Back: Documentation Overview](0-overview.md) | [Next: Toolchain and Project Structure →](2-cli-project.md)
