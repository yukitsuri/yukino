---
name: Bug report
about: Create a report to help us improve Yukino
title: "[BUG]: "
labels: ["bug"]
assignees: ""
---

<!--
  IMPORTANT:
  - This issue tracker is EXCLUSIVELY for bugs in the Yukino compiler, runtime, and standard library.
  - Feature requests, proposals, and general suggestions belong in our Discord server (see link in README.md). Do not open issues for suggestions here.
  - Platform tags: If and only if the issue is strictly platform-specific, mention `@Windows` or `@Linux`. If the bug is platform-agnostic, do NOT tag any group.
-->

### Problem Description
<!-- A clear and concise description of what the bug is. -->

### Steps to Reproduce
<!-- Steps to reproduce the behavior: -->
1. Create a file `main.ykn` with the following content:
```yukino
// Paste your minimal reproducible code here
```
2. Run command: `yukino run` (or `yukino build --release --target ...`)
3. See error

### Expected Behavior
<!-- A clear and concise description of what you expected to happen. -->

### Actual Behavior
<!-- What actually happened (e.g. compiler error, crash, unexpected output). -->

### Diagnostic Information
Please provide the exact output of the following commands:

- **Yukino Version:**
  ```text
  $ yukino -v
  $ yukic -v
  ```
- **Operating System and Architecture:** (e.g., Linux x86_64, Windows 11 x86_64, Android aarch64)
- **Target Triple:** (e.g., `x86_64-unknown-linux-gnu`, `x86_pc-windows-gnu`)
- **Compiler / Toolchain Logs:**
  ```text
  <!-- Paste compiler diagnostics, verbose output, or stack trace here -->
  ```

### Platform Mentions
<!--
  If and ONLY if this bug is confirmed to be platform-specific:
  - For Windows-specific issues: @Windows
  - For Linux-specific issues: @Linux
  Otherwise, leave this section blank.
-->

### Proposed Solution (Optional)
<!-- If you have researched the cause or have ideas on how to fix this, describe them here. -->

### Attachments & Additional Context
<!-- Attach logs, screenshots, or sample project files (.zip / .tar.gz) if applicable. -->
