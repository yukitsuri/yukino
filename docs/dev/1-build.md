# Build and development

## Toolchain

- Rust 1.98.1+, edition 2024.
- LLVM 22, selected through `llvmPackages_22` in Nix.
- Inkwell 0.10.0 with the `llvm22-1` feature.
- The compiler targets seven 64-bit triples: Linux glibc and musl for x86_64 and aarch64, Windows MinGW x86_64, Windows MSVC x86_64, and Android aarch64. See `crates/yukic-target` for the exact list.

The compiler needs LLVM, Clang/libclang, `pkg-config`, libffi, zlib, libxml2, and ncurses. The Nix flake provides these dependencies and configures `LLVM_CONFIG`, `LLVM_SYS_221_PREFIX`, and `LIBCLANG_PATH`.

## Workspace structure

The workspace is split by compilation stage:

- `yukic-span`, `yukic-diagnostics`: source locations and diagnostics.
- `yukic-target`: target triples, ABI information, and LLVM data layouts.
- `yukic-lexer`, `yukic-parser`, `yukic-ast`: source parsing.
- `yukic-resolve`, `yukic-macro`, `yukic-types`, `yukic-borrow`: name resolution, macros, types, and ownership checks.
- `yukic-dce`, `yukic-codegen`, `yukic-opt`: reachability analysis, LLVM IR generation, and optimization.
- `yukic-driver`: the `yukic` compiler driver, object generation, runtime handling, and linking.
- `yukino`: the project and build manager.
- `crates/yukino-builtin`: embeds runtime sources from `builtin/runtime` and provides on-demand compilation and toolchain verification.
- `builtin/runtime`: the C ABI runtime static library.

## Runtime and standard library linking

`yukino-builtin` embeds the source code of both `builtin/runtime` and `builtin/stdlib` into the compiler binary at compiler build time.

### Host prebuilt archives

During compiler build (`crates/yukino-builtin/build.rs`), the Rust-based components are precompiled for the host target (`TARGET`) and embedded into the binary as static archives (`HOST_PREBUILT_RT` and `HOST_PREBUILT_STD_CORE`):
- `libyukino_rt.a`: compiled via `rustc`.
- `libyukistd_core.a`: compiled via `cargo build` for `builtin/stdlib/Cargo.toml`.

When compiling applications for the host platform, `ensure_runtime` and `ensure_stdlib` write these prebuilt archives directly from memory without invoking `rustc` or `cargo`, eliminating the cold build delay.

### Debug and release profiles

The compiler build mode (`YUKINO_RELEASEBUILD` or Cargo profile) determines how runtime and standard library archives are compiled:
- **Release mode (`YUKINO_RELEASEBUILD=1`):** `libyukino_rt.a` is compiled with size optimizations (`-C opt-level=z`, `-C codegen-units=1`, `-C panic=abort`, `-C strip=debuginfo`), `libyukistd_core.a` uses Cargo's `--release` profile, `libstd.a` is compiled with `-O 3`, and debug symbols are stripped with `strip_archive`.
- **Debug mode (`YUKINO_RELEASEBUILD=0` / `./asmdbg`):** `libyukino_rt.a` is compiled with `-C opt-level=0 -C debuginfo=2`, `libyukistd_core.a` uses Cargo's dev profile, `libstd.a` is compiled with `-O 0`, and debug symbols are retained (no stripping) to allow developers to trace bugs and view backtraces.

### Cross-compilation on demand

For targets other than the host target (`target != HOST_PREBUILT_TARGET`), the compiler unpacks the embedded sources into `yuki-work/local/` and compiles the archives on demand for the requested target.

The `--no-runtime` flag in `yukic` and `yukino build` disables runtime linking. An explicit `--runtime <path>` can also be passed if desired.

## Release and debug configuration

`YUKINO_RELEASEBUILD` is consumed by both compiler build scripts:

- `1` uses `yukic`, `yukino`, `.yuki-lang`, and `.yukino-lang`.
- `0` uses `yukic-dbg`, `yukino-dbg`, `.yuki-lang-dbg`, and `.yukino-lang-dbg`.

When the variable is set, `build.rs` also writes `temp/build-config.json` and exports the selected values as compile-time Rust environment variables. Direct Cargo builds without the variable use release names but do not rewrite this file.

Release builds use optimization level 3, fat LTO, one codegen unit, panic abort, and symbol stripping. The Nix release package additionally runs `llvm-strip`. Debug builds keep Cargo debug settings and install the binaries with the `-dbg` suffix.

## Debug builds and installation

On Linux and other Unix systems:

```sh
./asmdbg
```

This runs `scripts/build-debug.py`, builds both binaries for the host, and installs `yukino-dbg` and `yukic-dbg` into `~/.local/bin`.

On Windows:

```bat
asmdbg.bat
```

The script uses `py -3` when available and falls back to `python`. It installs the `.exe` files into `%USERPROFILE%\.cargo\bin`. Existing binaries are replaced with a warning.

The script can also be called directly:

```sh
python3 scripts/build-debug.py
```

or on Windows:

```bat
py -3 scripts\build-debug.py
```

## Nix builds

The flake exposes separate release and debug packages:

```sh
nix build .#yukino
nix profile add .#yukino

nix build .#yukino-debug
nix profile add .#yukino-debug
```

The release package installs `yukino` and `yukic`. The debug package installs `yukino-dbg` and `yukic-dbg`.

For a direct Cargo debug build without installation:

```sh
YUKINO_RELEASEBUILD=0 cargo build --bin yukino --bin yukic
```

## Testing

Useful checks include:

```sh
cargo check --workspace --offline
cargo test --workspace --offline
cargo test --offline -p yukic-driver --all-targets
cargo test --offline -p yukino --lib --bin yukino
python3 -m py_compile scripts/build-debug.py scripts/crossbuild.py scripts/crossbuild-rt.py
```

`scripts/crossbuild.py` builds the compiler tools for supported targets. `scripts/crossbuild-rt.py` builds only runtime `.a` archives under `build/<version>/runtime/<target>.a`.

All tests currently in the repository were written by AI. They are regression checks and do not replace manual validation of installed binaries, target toolchains, or final linker behavior.

## Useful commands

```sh
cargo run -p yukic-driver --bin yukic -- --help
cargo run -p yukino --bin yukino -- --help
cargo fmt --all -- --check
git diff --check
```

When debugging a target build, first verify the target triple, the LLVM toolchain, and the runtime archive passed to `--runtime`. Host builds should not receive a runtime path.