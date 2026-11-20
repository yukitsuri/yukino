# Installing Yukino

This guide covers installing the `yukino` project manager, the `yukic` compiler driver, and building binaries for other platforms.

> [!TIP]
> **Editor Extensions and LSP:**
> A complete guide on installing editor extensions (Zed, VS Code) and setting up the language server is available in [Editor Extensions and Yukino LSP](extensions.md).

## Contents

1. [Supported platforms](#supported-platforms)
2. [Installation via Lacie (recommended for all OS)](#installation-via-lacie-recommended-for-all-os)
3. [Installing on Linux](#installing-on-linux)
   - 3.1 [Nix](#nix)
   - 3.2 [Manual Cargo build](#manual-cargo-build)
4. [Installing on Windows](#installing-on-windows)
5. [Cross-compilation](#cross-compilation)
   - 5.1 [Cross-compilation requirements](#cross-compilation-requirements)
   - 5.2 [Compiler](#cross-compiling-the-compiler)
   - 5.3 [Runtime](#cross-compiling-the-runtime)
6. [Verifying the installation](#verifying-the-installation)
7. [Editor extensions (Zed, VS Code)](#editor-extensions-zed-vscode)

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

## Installation via Lacie (Recommended for all OS)

The recommended and simplest way to install the prebuilt Yukino toolchain on any supported operating system (Linux, Windows) is through the **[Lacie](https://github.com/shareui/lacie)** package manager. It automatically fetches prebuilt binaries, resolves dependencies, configures active binaries in `PATH`, and handles updates without requiring local C or Rust build tools.

### 1. Install Lacie

If Lacie is not yet installed on your system, install it using the official guide:
- **[Lacie Installation Guide](https://github.com/shareui/lacie/tree/main/docs/install/en.md)**

### 2. Install Yukino Toolchain

To install all Yukino packages (`yukic` compiler driver and `yukino` build toolchain) for tag `v0.1.0` from `yukitsuri/yukino`:

```sh
lacie add yukitsuri/yukino v0.1.0
```

> [!TIP]
> Use `-y` (`--yes`) to approve the installation prompt automatically:
> ```sh
> lacie add yukitsuri/yukino v0.1.0 -y
> ```

After installation completes, `yukino` and `yukic` will be immediately available in your terminal.

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

Install the debug package with:

```sh
nix profile add .#yukino-debug
```

It provides `yukino-dbg` and `yukic-dbg`.

To enter a development shell with all dependencies and environment variables ready:

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

Before building with Cargo, LLVM and `libclang` paths must be explicitly exported in your shell session:

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

Cargo installs the binaries into `~/.cargo/bin`:

- `yukino` — the project manager and build tool;
- `yukic` — the low-level compiler driver.

## Installing on Windows

For Windows, use the PowerShell script that automatically downloads prebuilt release binaries and configures `PATH`:

```powershell
irm https://raw.githubusercontent.com/yukitsuri/yukino/main/scripts/wininstall.ps1 | iex
```

The script does not require configuring LLVM or compiling from source. If you need to build from source manually via Cargo, ensure LLVM 22 and MSVC/MinGW are installed and the respective LLVM environment variables are set.

## Cross-compilation

### Cross-compilation requirements

Cross-compilation additionally requires:

- `cargo-zigbuild` and Zig for Linux targets other than the normal host build;
- Android NDK for Android targets;
- Target components installed via `rustup target add <target>` if not already present.

### Cross-compiling the compiler

Clone the repository at the required tag:

```sh
git clone --branch v0.1.0 https://github.com/yukitsuri/yukino.git
cd yukino
python3 scripts/crossbuild.py
```

The script asks you to select one or more targets. Enter `0` to build all targets currently available in `crossbuild.py`:

- `x86_64-pc-windows-gnu`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`;
- `aarch64-unknown-linux-gnu`;
- `aarch64-unknown-linux-musl`;
- `aarch64-linux-android`.

Outputs are written to:

```text
build/0.1.0/yukino/yukino-<target>
build/0.1.0/yukic/yukic-<target>
```

On Windows, use `py -3` instead of `python3`.

### Cross-compiling the runtime

The runtime is built separately and produces static `.a` archives only:

```sh
python3 scripts/crossbuild-rt.py
```

Outputs are written to:

```text
build/0.1.0/runtime/<target>.a
```

Pass the resulting runtime to `yukic` for a custom target with `--runtime <path>`. A separate runtime is not needed for the host target: `yukic` uses its embedded host runtime.

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
