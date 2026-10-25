## Description
<!-- Provide a clear and concise summary of what this PR changes and why. -->

## Related Issue / Roadmap
<!-- Link related issues or roadmap items (e.g., "Fixes #123", "Refs https://github.com/yukitsuri/yukino/blob/main/docs/roadmap.md"). -->

## Type of Change
<!-- Check all applicable items with [x] -->
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that alters existing behavior or APIs)
- [ ] Standard library addition or fix
- [ ] Performance optimization
- [ ] Documentation update
- [ ] Toolchain / Build / Internal refactoring

## Standard Library & Platform Checklist (if applicable)
<!-- Mark N/A if not touching standard library or platform code -->
- [ ] Implemented primarily in Yukino (Rust used only where strictly required)
- [ ] Cross-platform behavior verified (or explicitly constrained to platform modules)
- [ ] Follows canonical error conventions (null for absent values, typed panic structs, no raw string panics)
- [ ] Target platform additions represent real-world platforms, are registered in `targets_view.rs`, and documented in `2-cli-project.md`

## Verification & Quality Checklist
<!-- Ensure all requirements are met before requesting review -->
- [ ] I have read and followed [CONTRIBUTING.md](CONTRIBUTING.md).
- [ ] All workspace tests pass (`cargo test --workspace`).
- [ ] Added or updated tests covering the new or modified logic.
- [ ] Documentation is updated accordingly:
  - [ ] Russian: `docs/lang/ru/` (if syntax or stdlib changed)
  - [ ] English: `docs/lang/en/` (if syntax or stdlib changed)
  - [ ] Compiler internals: `docs/dev/` (if compiler architecture changed)
- [ ] Code is self-reviewed, free of unverified AI-generated code, and follows existing project style.
