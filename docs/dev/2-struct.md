# Repository structure and learning guide

This document provides a comprehensive structural guide to the `yukino` codebase and an explicit step-by-step navigation map for developers. It explains how internal compiler functions connect across crates, where compilation data structures originate, and in what exact order to read the code.

---

## 1. Repository overview

The `yukino` repository is structured as a unified Cargo workspace containing compiler crates, embedded runtime/stdlib assets, build scripts, and documentation:

```text
yukino/
├── Cargo.toml              # Workspace manifest
├── flake.nix               # Nix development flake with LLVM 22 and tools
├── asmdbg / asmdbg.bat     # Host debug build and installation scripts
├── scripts/                # Cross-compilation and helper automation scripts
├── docs/                   # Developer and language documentation
├── examples/               # Example projects (tapp, docsexamples)
├── builtin/                # Embedded runtime and standard library sources
│   ├── runtime/            # Minimal static C ABI runtime in Rust (libyukino_rt.a)
│   ├── stdlib/             # Standard library (core/ in Rust, std/ in Yukino)
│   └── macrosdk/           # Macro SDK (core/ in Rust, macrosdk/ in Yukino, libmacrosdk.a)
└── crates/                 # Compiler toolchain crates
    ├── yukic-span          # Source byte spans, file IDs, and SourceMap
    ├── yukic-diagnostics   # Error codes and diagnostic emitter
    ├── yukic-target        # Target triples and platform ABI specs
    ├── yukic-lexer         # Tokenizer and lexical scanner
    ├── yukic-ast           # AST node representations and AST transformations
    ├── yukic-parser        # Recursive descent and Pratt expression parser
    ├── yukic-resolve       # Import graph, symbol registry, and conditional filtering
    ├── yukic-macro         # Macro expansion engine and built-in compile-time macros
    ├── yukic-types         # Type checker, semantic contracts, MRO, and type env
    ├── yukic-borrow        # Affine ownership, borrow checker, and escape analysis
    ├── yukic-dce           # Call graph reachability and dead code eliminator
    ├── yukic-codegen       # LLVM IR generation (Inkwell) and CGU partitioning
    ├── yukic-opt           # LLVM New Pass Manager optimizer pipeline and LTO
    ├── yukic-driver        # Compiler driver binary (yukic) and linking orchestration
    ├── yukino-macroproto   # Macro IPC wire protocol, length framing, and models
    ├── yukino-builtin      # Embedded runtime/stdlib unpacker and builder
    └── yukino              # Developer CLI tool and project manager (yukino)
```

---

## 2. Compiler structure and internal execution pipeline

To understand how the compiler transforms source files into a final executable or library, follow the execution flow below. Every phase lists its **entry function**, **what internal functions it calls next**, **what data structures it creates/passes**, and **the exact files and lines to read**.

### 2.1 Toolchain entry points: `yukino` CLI vs `yukic` compiler driver

Execution begins in one of two binaries:
- **Project manager (`yukino` CLI):**
  - Read `crates/yukino/src/main.rs` (`Cli` enum at line 18, `main` at line 467).
  - When running `yukino build`, it calls `build_project` in `crates/yukino/src/builder.rs:734`.
  - It resolves dependencies, reads `yuki.toml`, `lib.toml`, or `yukispace.toml` (`crates/yukino/src/manifest.rs`), executes build-time dependencies `[libs.build]` and pre-build scripts `build.ykn` if present (`crates/yukino/src/builder.rs`), performs feature unification (`crates/yukino/src/features.rs`), prepares the runtime/stdlib archives, and spawns the compiler executable `yukic` (`crates/yukino/src/builder.rs:158`).
- **Standalone compiler driver (`yukic`):**
  - Read `crates/yukic-driver/src/main.rs:12` (`main`).
  - It parses CLI flags into `Args` (`crates/yukic-driver/src/cli.rs:107`), converts them into `DriverConfig` (`crates/yukic-driver/src/lib.rs:243`), and invokes `run_compiler_driver` (`crates/yukic-driver/src/pipeline.rs:1518`).

### 2.2 End-to-end compiler execution road map

Below is the internal call graph and data transition from source code to final artifact:

L - Line

```text
[CLI Args]
    │
    ▼
crates/yukic-driver/src/pipeline.rs::run_compiler_driver (L1518)
    ├── crates/yukic-driver/src/pipeline.rs::prepare_runtime (L1473)
    │     └── crates/yukino-builtin/src/lib.rs::ensure_runtime (L26)
    ├── crates/yukic-driver/src/pipeline.rs::prepare_stdlib (L1498)
    │     └── crates/yukino-builtin/src/lib.rs::ensure_stdlib (L50)
    ├── crates/yukic-driver/src/pipeline.rs::prepare_macrosdk (L1523)
    │     └── crates/yukino-builtin/src/lib.rs::ensure_macrosdk (L83)
    │
    └── crates/yukic-driver/src/pipeline.rs::compile_ykn_file (L1333)
          │
          ├── [1. Project discovery & loading]
          │     ├── crates/yukic-driver/src/pipeline.rs::collect_ykn_files (L240)
          │     ├── crates/yukic-driver/src/pipeline.rs::load_project_modules (L361)
          │     └── crates/yukic-driver/src/pipeline.rs::discover_external_modules (L383)
          │           │
          │           └── crates/yukic-driver/src/pipeline.rs::parse_and_expand_file (L306)
          │                 ├── crates/yukic-span/src/map.rs::SourceMap::add_file (L16)
          │                 ├── crates/yukic-lexer/src/lexer.rs::Lexer::tokenize (L38)
          │                 ├── crates/yukic-parser/src/lib.rs::Parser::parse_module (L33)
          │                 ├── crates/yukic-resolve/src/conditional.rs::filter_module (L25)
          │                 └── crates/yukic-macro/src/lib.rs::MacroExpander::expand_module (L48)
          │
          ├── [2. Symbol resolution & AST normalization]
          │     ├── crates/yukic-driver/src/pipeline.rs::run_resolution_pass (L676)
          │     │     └── crates/yukic-resolve/src/lib.rs::Resolver::resolve_all (L96)
          │     ├── crates/yukic-driver/src/pipeline.rs::merge_project_ast (L724)
          │     ├── crates/yukic-ast/src/defaults.rs::expand_default_args (L97)
          │     └── crates/yukic-ast/src/aliases.rs::substitute_module_aliases (L11)
          │
          ├── [3. Semantic analysis & verification]
          │     ├── crates/yukic-driver/src/pipeline.rs::run_type_check_pass (L1014)
          │     │     └── crates/yukic-types/src/checker.rs::TypeChecker::check_module (L33)
          │     ├── crates/yukic-driver/src/pipeline.rs::run_borrow_check_pass (L1129)
          │     │     └── crates/yukic-borrow/src/checker.rs::BorrowChecker::check_module (L52)
          │     └── crates/yukic-driver/src/lint.rs::run_lint_pass (L16)
          │
          ├── [4. Dead code elimination]
          │     └── crates/yukic-driver/src/pipeline.rs::run_dce_pass (L1159)
          │           └── crates/yukic-dce/src/eliminator.rs::DeadCodeEliminator::eliminate (L29)
          │
          └── [5. Backend code generation, optimization & linking]
                └── crates/yukic-driver/src/pipeline.rs::run_backend_pipeline (L1189)
                      ├── crates/yukic-codegen/src/cgu.rs::partition_module_with_custom (L129)
                      ├── crates/yukic-codegen/src/cgu.rs::compile_units_parallel (L231)
                      │     └── crates/yukic-codegen/src/generator.rs::CodeGenerator (L39)
                      ├── crates/yukic-opt/src/parallel.rs::optimize_cgus_parallel (L47)
                      │     └── crates/yukic-opt/src/pipeline.rs::run_pipeline (L72)
                      ├── crates/yukic-driver/src/emitter.rs::emit_cgus_parallel (L417)
                      │     └── crates/yukic-driver/src/cgu_cache.rs::CguCacheManifest (L40)
                      ├── crates/yukic-driver/src/header.rs::generate_c_header_from_ast (L90)
                      └── crates/yukic-driver/src/pipeline.rs::emit_final_artifact (L73)
                            ├── crates/yukic-driver/src/executable.rs::link_executable (L202)
                            ├── crates/yukic-driver/src/archive.rs::create_static_archive (L10)
                            └── crates/yukic-driver/src/shared.rs::link_shared_library (L157)
```

### 2.3 Phase breakdown: functions, dataflow, and reading paths

#### Phase 1: Foundational infrastructure
Before source code is parsed, three foundational crates supply essential shared types:
- **`crates/yukic-span`**:
  - `pos.rs`: defines `BytePos`, `FileId`, and `Span` (half-open byte offsets within a source file).
  - `file.rs`: defines `SourceFile` (stores text and maps byte offsets to line/column).
  - `map.rs`: defines `SourceMap` (central repository of all loaded files). Key function to read: `SourceMap::add_file` (`map.rs:16`).
- **`crates/yukic-diagnostics`**:
  - `model.rs`: defines `Diagnostic`, `DiagnosticSeverity`, and `Suggestion`.
  - `code.rs`: diagnostic error and warning codes (`ErrorCode`, `WarningCode`).
  - `emitter.rs`: reads `DiagnosticEmitter` (`emitter.rs:15`). Entry function: `emit` (`emitter.rs:48`).
- **`crates/yukic-target`**:
  - `triple.rs`: defines supported `TargetTriple` variants (read line 11).
  - `spec.rs`: defines `TargetSpec` (OS, architecture, data layout, and ABI configurations).

#### Phase 2: Lexing, parsing, and AST
- **`crates/yukic-lexer`**:
  - Entry point: `crates/yukic-lexer/src/lexer.rs:38` (`Lexer::tokenize`).
  - Advances UTF-8 characters via `cursor.rs:13` (`Cursor`), delegates number scanning to `number.rs:14` (`scan_number`), string literals to `string.rs:31` (`scan_string_literal` with string interpolation `${expr}` support), and keywords to `words.rs:8` (`lookup_keyword`).
  - Output: `Vec<Token>` (`token.rs:85`).
- **`crates/yukic-ast`**:
  - Defines AST node structures: `item.rs` (`Module`, `Item`, `ItemKind`), `stmt.rs` (`Stmt`, `Block`), `expr.rs` (`Expr`, `ExprKind`), `types.rs` (`TypeNode`), and `attr.rs` (`AttributeNode`).
- **`crates/yukic-parser`**:
  - Entry point: `crates/yukic-parser/src/lib.rs:33` (`Parser::parse_module`).
  - Consumes tokens via `stream.rs:20` (`TokenStream`).
  - Parses top-level declarations in `item.rs:16` (`parse_module`), blocks and statements in `stmt.rs:12` (`parse_stmt`), expressions in `expr.rs:15` (`parse_expr_with_bp`) using Pratt precedence table in `prec.rs:11`, and type annotations in `types.rs:12` (`parse_type`).
  - Error recovery synchronization is in `recovery.rs`.
  - Output: `yukic_ast::Module`.

#### Phase 3: Conditional compilation and macro expansion
- **`crates/yukic-resolve` (filtering)**:
  - Called from `crates/yukic-driver/src/pipeline.rs:327`.
  - Function to read: `crates/yukic-resolve/src/conditional.rs:25` (`filter_module`). Evaluates `#[cfg(...)]`, `#[cfg_attr(...)]`, `#[target(...)]`, and `#[feature(...)]` against `ConditionalConfig`, stripping disabled items *before* name resolution.
- **`crates/yukic-macro` & `crates/yukino-macroproto`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:329`.
  - Entry point: `crates/yukic-macro/src/lib.rs:50` (`MacroExpander::expand_module` and `expand_module_tracked`).
  - Traversal in `expand.rs:18` (`AstExpander`), built-in compile-time macros in `builtin.rs:52` (`eval_builtin_macro` for `file!`, `line!`, `col!`, `cfg!`, `yukienv!`, `hostenv!`, `compile_print!`, manifest/workspace scoping `1..4`, config-relative resource inclusion `include_str!`/`include_bytes!`, etc.), hygiene renaming in `hygiene.rs:15` (`apply_hygiene`), and AST substitution in `subst.rs:15` (`substitute_macro`).
  - Procedural macro runner IPC: `crates/yukic-macro/src/runner.rs` (`MacroRunnerClient`) communicates with external runner executables registered via `--macro-runner <NAME=PATH>`, using 4-byte little-endian length prefix framing and JSON wire format defined in `crates/yukino-macroproto`. Handles a 10s timeout, child process lifecycle, stderr crash capturing, and tracked file collection (`tracked_files`).
  - Macro scopes and tables are managed in `table.rs:30` (`MacroTable`).

#### Phase 4: Module discovery, name resolution, and AST desugaring
- **`crates/yukic-resolve`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:676` (`run_resolution_pass`).
  - Entry point: `crates/yukic-resolve/src/lib.rs:96` (`Resolver::resolve_all`).
  - Files and library paths are located via `module_path.rs:43` (`SearchPaths`).
  - Module dependencies are modeled in `graph.rs:45` (`ModuleGraph`), checking for dependency cycles via `detect_cycles`.
  - Symbols are collected by `collector.rs:16` (`collect_module_symbols`) into `symbol.rs:24` (`SymbolRegistry`).
  - Scoping, visibility (`pub`), and imports (`frommod`, `frompack`) are validated in `resolver.rs:18` (`resolve_imports`) and `scope.rs` (`ModuleScope`).
- **AST Desugaring**:
  - Project modules are merged in `crates/yukic-driver/src/pipeline.rs:724` (`merge_project_ast`).
  - Default function arguments are desugared in `crates/yukic-ast/src/defaults.rs:97` (`expand_default_args`).
  - Module aliases are substituted in `crates/yukic-ast/src/aliases.rs:11` (`substitute_module_aliases`).
  - File-level attributes (`!#[...]`) are propagated onto every function-like declaration of the file in `crates/yukic-ast/src/module_attrs.rs` (`propagate_module_attributes`), right after conditional filtering in `parse_and_expand_file`.

#### Phase 5: Semantic type checking
- **`crates/yukic-types`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1014` (`run_type_check_pass`).
  - Entry point: `crates/yukic-types/src/checker.rs:33` (`TypeChecker::check_module`).
  - Type environment and definitions: `env.rs:48` (`TypeEnv`, `FnSig`, `ClassDef`, `StructDef`).
  - Syntactic to semantic type conversions: `convert.rs:14` (`ast_type_to_type`).
  - Method Resolution Order (MRO) for inheritance: `mro.rs:14` (`MethodResolution::resolve_method`).
  - Pattern match exhaustiveness: `exhaust.rs:12` (`ExhaustivenessChecker::check_switch`).
  - Semantic contracts and C ABI validation: `contracts.rs:20` (`validate_cfun`, `validate_cast`).

#### Phase 6: Affine borrow checking and escape analysis
- **`crates/yukic-borrow`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1129` (`run_borrow_check_pass`).
  - Entry point: `crates/yukic-borrow/src/checker.rs:52` (`BorrowChecker::check_module`).
  - Type affinity classification: `affinity.rs:12` (`TypeAffinity::is_copy`, `is_affine`).
  - Variable states and exclusivity: `state.rs:38` (`StateEnv`).
  - Escape analysis for region allocation tier selection: `escape.rs:35` (`EscapeAnalysis::analyze_module`).

#### Phase 7: Linters and dead code elimination (DCE)
- **`crates/yukic-driver/src/lint.rs`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1801` (`run_lint_pass`).
  - Configures warning filters and deny lists via `configure_warning_filter` from file-level and item-level attributes (`#![lint::ignore(...)]`, `#[lint::ignore(...)]`, `#![lint::deny(...)]`, `#[lint::deny(...)]`).
  - Denied diagnostics are escalated in `yukic-diagnostics::DiagnosticEmitter` from `Warning` to `Error`, causing `emitter.has_errors()` to trigger a fatal `DriverError::FrontendValidation` failure.
  - Checks for unused variables, unused imports, dead functions, and deprecated item usages (`#[deprecated]` -> `W0420`).
- **`crates/yukic-dce`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1803` (`run_dce_pass`).
  - Entry point: `crates/yukic-dce/src/eliminator.rs:29` (`DeadCodeEliminator::eliminate`).
  - Discovers root items (`main` or public library exports) in `roots.rs:20` (`discover_roots`).
  - Traverses the call graph DFS in `reachability.rs:22` (`ReachabilityAnalysis::compute_live_items`).
  - Strips unreachable items from AST and compiles metrics in `metrics.rs`.

#### Phase 8: Code generation (LLVM IR via Inkwell)
- **`crates/yukic-codegen`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1189` (`run_backend_pipeline`).
  - Step 1: CGU partitioning in `cgu.rs:129` (`partition_module_with_custom` / `partition_module`). Groups AST items into Codegen Units respecting `[customcgus]` mappings and balancing remaining items using Greedy LPT scheduling.
  - Step 2: Parallel compilation entry point in `cgu.rs:231` (`compile_units_parallel`). Spawns worker threads with isolated Inkwell `Context` instances, custom CGU naming, and optional function sections (`function_sections`).
  - Step 3: Module generator in `generator.rs:39` (`CodeGenerator`).
  - Step 4: Function body lowering in `body/mod.rs:48` (`BodyCompiler`), `body/stmt.rs:14` (`compile_stmt`), `body/expr.rs:15` (`compile_expr`), `body/call.rs:18` (`compile_call`).
  - Specialized codegen managers:
    - `registry.rs:42` (`TypeRegistry`) & `types.rs:28` (`TypeLowerer`): memory and LLVM type layouts.
    - `region.rs:17` (`RegionManager`): bump allocator prologs/epilogs and loop restore markers.
    - `vtable.rs:36` (`VTableManager`): vtable/itable generation and dynamic dispatch fat pointers.
    - `closure.rs:41` (`ClosureManager`): closure environment layout and `{ fn_ptr, env_ptr }` fat pointers.
    - `exception.rs:51` (`ScopeStack`): cleanup tracking and `@yukino_personality` landing pads for `sus`/`catch`/`defer`.
    - `cabi.rs:52` (`CAbiManager`): C ABI linkage, calling conventions, `ptr<T>` operations, and C-string conversions.
    - `intrinsic.rs:21` (`IntrinsicManager`): lowering of `typeof`, `exit`, `cast`, `args`, and slice intrinsics.
    - `rodata.rs:25` (`RoDataManager`): deduplicated `.rodata` constants.
    - `mangle.rs:1` (`mangle_fun_symbol`): symbol name mangling for Yukino functions (`__yukifun_<name>_<args>__<ret>`) and attribute overrides.
    - `debug.rs:35` (`DebugInfoManager`): DWARF and CodeView metadata generation via `DIBuilder`.

#### Phase 9: LLVM pass optimization pipeline
- **`crates/yukic-opt`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1218`.
  - Entry point: `crates/yukic-opt/src/parallel.rs:47` (`optimize_cgus_parallel`).
  - Attaches AST function attributes: `attributes.rs:45` (`apply_ast_attributes`).
  - Runs LLVM New Pass Manager: `pipeline.rs:72` (`run_pipeline`) based on level in `level.rs:11` (`OptLevel`).
  - Generates LTO bitcode if configured: `lto.rs:25` (`write_bitcode_to_memory`).

#### Phase 10: Machine code emission and incremental caching
- **`crates/yukic-driver`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1653`.
  - Entry point: `crates/yukic-driver/src/emitter.rs:420` (`emit_cgus_parallel`).
  - Checks and updates incremental CGU cache in `crates/yukic-driver/src/cgu_cache.rs:61` (`CguCacheManifest::compute_cgu_hash`).
  - Integrates cryptographic BLAKE3 checksums of external tracked files (`ctx.track_file(...)`) and procedural macro runner binaries via `pipeline.rs::compute_cgu_extra_hashes`, invalidating cached units upon any dependency mutation.
  - Emits `.o` object files or `.s` assembly files via `TargetMachine::write_to_file` in `emitter.rs:208` (`emit_object_file`).
  - Generates C headers for exported libraries in `crates/yukic-driver/src/header.rs:90` (`generate_c_header_from_ast`).

#### Phase 11: Final linking
- **`crates/yukic-driver`**:
  - Called from `crates/yukic-driver/src/pipeline.rs:1294` -> `pipeline.rs:73` (`emit_final_artifact`).
  - Output targets:
    - Executable (`OutputType::App`): `crates/yukic-driver/src/executable.rs:202` (`link_executable`). Resolves dynamic linkers, CRT files, and invokes LLD (`ld.lld` on Linux, `lld-link` on Windows), linking CGU objects, runtime (`libyukino_rt.a`), and stdlib archives.
    - Static archive (`OutputType::Static`): `crates/yukic-driver/src/archive.rs:10` (`create_static_archive`) invokes `llvm-ar`.
    - Shared library (`OutputType::Shared`): `crates/yukic-driver/src/shared.rs:157` (`link_shared_library`) invokes LLD with `-shared` / `/DLL`.

---

### 2.4 Runtime and standard library architecture

The runtime and standard library are embedded directly into the compiler binary and unpacked/built on demand:

```text
builtin/
├── runtime/              # Minimal C ABI static runtime in Rust
│   └── src/              # Region allocator, panic handlers, unwinder personality, slices
└── stdlib/               # Standard library
    ├── core/             # Low-level platform layer in Rust (C ABI, OS syscalls, raw threads)
    └── std/              # High-level standard library written in Yukino (.ykn)
```

1. **`builtin/runtime` (`libyukino_rt.a`)**:
   - Written in Rust with `no_std`-friendly conventions and C ABI exports.
   - Provides low-level runtime support:
     - `region.rs`: region bump allocation (`yukino_region_create`, fast-path allocation, loop save/restore marks).
     - `personality.rs`: DWARF/SEH personality function `@yukino_personality` for exception handling unwinding.
     - `panic.rs`: runtime panic emitter (`@yukino_raise_panic`, `@yukino_raise_typed_panic`, `@yukino_panic_abort`) and exception metadata/payload getters (`@yukino_exception_get_message`, `@yukino_exception_get_kind`, `@yukino_exception_has_code`, `@yukino_exception_get_code`, `@yukino_exception_get_payload`).
     - `rt_slice.rs` & `utf8.rs`: slice bounds checks, conversions, and UTF-8 validation.
     - `rt_args.rs`: command-line argument passing into Yukino `main(args)`.

2. **`builtin/stdlib`**:
   - The standard library separates low-level operating system bindings from user-facing language APIs:
     - **`core/` (Native Rust layer)**:
       Implements raw platform integration (OS I/O descriptors, threads, locks, low-level system calls) in Rust. It exposes pure C ABI symbols and is compiled into `libyukistd_core.a`.
     - **`std/` (Yukino layer)**:
       Written entirely in Yukino (`.ykn`). Contains idiomatic classes, interfaces, and methods that import and wrap the low-level C ABI primitives from `core/` or compiler intrinsics. It is compiled into `libstd.a`.

3. **`crates/yukino-builtin` (Compiler embedding and builder)**:
   - Embeds source archives of `builtin/runtime` and `builtin/stdlib` into `yukic` via `include_bytes!`.
   - Host prebuilt archives: precompiles `libyukino_rt_host.a` and `libyukistd_core_host.a` during compiler build for instant host linking (`HOST_PREBUILT_RT`, `HOST_PREBUILT_STD_CORE`).
   - `unpack.rs`: `unpack_runtime_source` (line 38) and `unpack_stdlib_source` (line 42) unpack sources into `yuki-work/local/` on demand.
   - `compile.rs`: `compile_runtime_archive` (line 33), `compile_stdlib_rust` (line 177), and `compile_stdlib_yukino` (line 218) invoke `rustc` / `cargo` / `yukic` to build `.a` archives for the target platform.
   - `lib.rs`: `ensure_runtime` (line 26) and `ensure_stdlib` (line 50) orchestrate fast-path prebuilt extraction, unpacking, building, and caching.

---

### 2.5 Recommended developer reading order

When modifying compiler features or tracing bugs, study the codebase in this specific sequence:

1. **Execution driver**:
   Start at `crates/yukic-driver/src/pipeline.rs:1518` (`run_compiler_driver`) and trace `compile_ykn_file` (line 1333) to get the whole picture of the compilation lifecycle.
2. **Frontend pipeline**:
   Trace `crates/yukic-lexer/src/lexer.rs` -> `crates/yukic-parser/src/lib.rs` -> `crates/yukic-ast/src/item.rs`.
3. **Macro expansion & resolution**:
   Read `crates/yukic-resolve/src/conditional.rs` -> `crates/yukic-macro/src/lib.rs` -> `crates/yukic-resolve/src/lib.rs`.
4. **Type system & contracts**:
   Study `crates/yukic-types/src/checker.rs`, `crates/yukic-types/src/env.rs`, and `crates/yukic-types/src/mro.rs`.
5. **Ownership & DCE**:
   Read `crates/yukic-borrow/src/checker.rs` -> `crates/yukic-dce/src/eliminator.rs`.
6. **Backend code generation**:
   Study `crates/yukic-codegen/src/cgu.rs`, `generator.rs`, and `body/mod.rs`, then explore specialized managers (`region.rs`, `vtable.rs`, `exception.rs`, `cabi.rs`, `intrinsic.rs`, `mangle.rs`).
7. **Optimization, emission & linking**:
   Study `crates/yukic-opt/src/parallel.rs` -> `crates/yukic-driver/src/emitter.rs` -> `crates/yukic-driver/src/executable.rs`.
8. **Runtime & standard library**:
   Read `crates/yukino-builtin/src/lib.rs` -> `builtin/runtime/src/lib.rs` -> `builtin/stdlib/`.
