# Contributing to Yukino

Thank you for your interest in contributing to Yukino. This document outlines the guidelines, architecture principles, and quality standards for developing the compiler, runtime, and standard library.

---

## 1. Priorities & Roadmap

- Pull requests directly addressing goals in the [Official Roadmap](https://github.com/yukitsuri/yukino/blob/main/docs/roadmap.md) receive the highest review and merge priority.
- If you are planning a large feature or architectural change, please consult the roadmap first to ensure your work aligns with the project direction.

---

## 2. Core Philosophy & Architecture

- **Review Architecture First:** Before contributing to the compiler or runtime, we strongly recommend reading [docs/dev/2-struct.md](docs/dev/2-struct.md). Reviewing the repository structure, crate layout, and compilation stages upfront will save you substantial time and prevent architectural friction.
- **Language Philosophy:** All contributions must strictly adhere to the foundational goals and philosophy defined in [Language Overview](docs/lang/en/0-overview.md#goals-and-philosophy):
  - **Explicitness over Magic:** No hidden control flow, unexpected allocations, or invisible mutations. The code does exactly what is written.
  - **Maximum Simplicity and Native Performance:** Zero heavy runtime baggage, no garbage collection pauses, and execution speed matching C and C++.
  - **Conciseness and Transparency:** Clean, readable syntax without ceremonial boilerplate or ambiguous implicit behavior.
  - **Minimal Compiler Footprint:** Keep the `yukic` compiler and toolchain lightweight, self-contained, and blazingly fast.
- Any deviation from this philosophy requires compelling, well-documented justification and prior consensus from the core team.

---

## 3. Community & Design Discussions

- For controversial, non-trivial, or architecture-level design decisions, **we strongly recommend discussing them first in our Discord server** (linked in `README.md`).
- This is purely an advisory recommendation to protect your time: aligning with maintainers early helps refine ideas and reduces the risk of a pull request being rejected after investing significant effort.

---

## 4. Standard Library Principles

- **Yukino-First Implementation:** Standard library code must be written in the Yukino language whenever possible. Rust is strictly reserved for functionality that cannot be expressed in Yukino or where reimplementation would require an unreasonable amount of low-level infrastructure (for example, native SQLite integration, cryptographic primitives, or raw OS bindings).
- **Cross-Platform by Default:** All standard library modules must work identically across all supported platforms, unless explicitly intended for platform-specific capabilities (such as `std.platformio` or `std.os`).
- **Idiomatic Error Handling:**
  - Return `null` when a value is legitimately absent in normal execution (such as a missing map key or an optional parameter).
  - Use `panic` with a strongly typed error `struct` for contract violations and runtime failures. **Never panic with raw string messages.**
  - Operations without an inherent return value should return a status indicator (`bool`, code, or result) so callers can inspect execution status.

---

## 5. Adding Target Platforms

- **Real-World Relevance:** Any proposed compilation target must represent an established, practically used platform or operating system with a genuine user base. Obscure, niche hobby, or toy operating systems will not be accepted.
- **Driver and Toolchain Integration:** The target must be registered in the target views and specifications in `crates/yukic-driver/src/targets_view.rs` and `crates/yukic-target/` so that `yukic targets` and toolchain drivers recognize and handle it properly.
- **Documentation:** The new target must be documented in the supported platforms section in both `docs/lang/ru/2-cli-project.md` and `docs/lang/en/2-cli-project.md`.

---

## 6. Documentation Requirements

- **Language and Standard Library Docs:** Every new feature, syntax modification, or standard library addition must include corresponding, up-to-date documentation in **both languages**:
  - Russian: `docs/lang/ru/`
  - English: `docs/lang/en/`
  Documentation must follow the existing canonical style: clear, approachable for developers of any experience level, and accompanied by practical, runnable code snippets.
- **Internal Compiler Docs:** Any changes to compiler architecture, crate responsibilities, compilation pipelines, or internal data structures must be documented in `docs/dev/`. Internal developer guides and architecture specifications must always be kept completely accurate and up-to-date.

---

## 7. Testing & Quality Assurance

- **100% Pass Rate:** All workspace tests (`cargo test --workspace`) must pass without failures or regressions.
- **Coverage for New Logic:** Every new feature, compiler optimization, or bug fix must come with thorough tests covering both typical usage and edge cases.
- Before submitting your contribution, verify your changes by building the compiler (`./asmdbg`) and running the test suite.

---

## 8. Code Quality & AI Guidelines

- **No AI Slop:** While the use of AI tools as assistants is not banned, unverified, unreviewed, or low-quality AI-generated submissions will be rejected.
- Authors are expected to fully understand, test, and defend every line of code in their pull request.
- Keep patches focused and surgical: match the surrounding code style, keep functions cohesive and minimal, and avoid unrelated refactoring or formatting changes.

---

## 9. Special

- Try your best, and good luck! :)
