# Editor Extensions and Yukino LSP

This document describes the installation and configuration of official and community editor extensions, extension configuration options, and the command-line interface for the `yukino-lsp` language server.

---

## Contents

1. [Official extensions](#1-official-extensions)
   - 1.1 [Yukino LSP: Zed](#11-yukino-lsp-zed)
   - 1.2 [Yukino LSP: VSCode](#12-yukino-lsp-vscode)
2. [Command reference for yukino-lsp](#2-command-reference-for-yukino-lsp)
3. [Community extensions](#3-community-extensions)

---

## 1. Official extensions

### 1.1 Yukino LSP: Zed

Official Yukino extension for the Zed editor.

- Extension source code: [zed-yukino](https://github.com/yukitsuri/zed-yukino)
- Syntax highlighting grammar: [tree-sitter-yukino](https://github.com/yukitsuri/tree-sitter-yukino)

In Zed, extensions are manually reviewed by developers through pull requests to the official extension registry. Because of this, `Yukino LSP` will be in review for an extended period. Currently, installation is performed manually in development mode (Dev Extension).

#### Installation steps:

1. **Install the `yukino-lsp` language server via Lacie**

   The language server powers autocompletion, type checking, error highlighting, and symbol navigation. Without the LSP installed, roughly 80% of the extension capabilities will not function.

   - Lacie installation guide: [shareui/lacie install guide](https://github.com/shareui/lacie/tree/main/docs/install)
   - Language server source code: [yukitsuri/yukino-lsp](https://github.com/yukitsuri/yukino-lsp)

   Install `yukino-lsp` with Lacie:

   ```sh
   lacie add github:yukitsuri/yukino-lsp
   ```

   Ensure that the `yukino-lsp` binary is accessible in your system `PATH`.

2. **Clone the extension repository**

   Clone the Zed extension source repository:

   ```sh
   git clone https://github.com/yukitsuri/zed-yukino
   ```

3. **Install in Zed**

   - Open the Zed editor.
   - Press `Ctrl + Shift + X` to open the extensions panel.
   - Click the **Install Dev Extension** button.
   - Select the directory where the repository was cloned (the directory containing `extension.toml`).

#### Zed extension configuration:

By default, the Zed extension locates the `yukino-lsp` binary in `PATH` and runs it with the `lsp` subcommand.

If you need to explicitly configure the executable path or pass arguments, add the following section to your Zed `settings.json`:

```json
{
  "lsp": {
    "yukino-lsp": {
      "binary": {
        "path": "/path/to/yukino-lsp",
        "arguments": ["lsp"]
      }
    }
  }
}
```

---

### 1.2 Yukino LSP: VSCode

Official Yukino extension for Visual Studio Code.

- Extension source code: [vscode-yukino](https://github.com/yukitsuri/vscode-yukino)

#### Installation steps:

1. **Install the `yukino-lsp` language server via Lacie**

   The language server powers autocompletion, type checking, error highlighting, and symbol navigation. Without the LSP installed, roughly 80% of the extension capabilities will not function.

   - Lacie installation guide: [shareui/lacie install guide](https://github.com/shareui/lacie/tree/main/docs/install)
   - Language server source code: [yukitsuri/yukino-lsp](https://github.com/yukitsuri/yukino-lsp)

   Install `yukino-lsp` with Lacie:

   ```sh
   lacie add github:yukitsuri/yukino-lsp
   ```

2. **Install the extension in VS Code**

   - Open Visual Studio Code.
   - Open the Extensions search view with `Ctrl + Shift + X`.
   - Type `Yukino LSP` into the search bar.
   - Click the **Install** button.

#### VS Code extension settings:

Settings can be customized via the Settings UI (*Extensions -> Yukino*) or directly in your `settings.json`:

- `yukino.server.path` (`string | null`, default: `null`)  
  Absolute path to the `yukino-lsp` executable. If unset (`null`), the extension automatically searches system `PATH`, Lacie directories, and local paths.
- `yukino.server.extraArgs` (`string[]`, default: `[]`)  
  Additional command-line arguments to pass to the `yukino-lsp` server on startup.
- `yukino.server.trace` (`"off" | "messages" | "verbose"`, default: `"off"`)  
  Tracing level for communication between VS Code and the LSP server in the Output panel.
- `yukino.codeLens.enable` (`boolean`, default: `true`)  
  Enables CodeLens interactive buttons above runnable targets (`main` function, `#[test]` test runners).
- `yukino.inlayHints.enable` (`boolean`, default: `true`)  
  Displays inline hints for compiler-inferred variable types.

#### Available commands in Command Palette (`Ctrl + Shift + P`):

- `Yukino: Restart Language Server` — restarts the language server.
- `Yukino: Run (main)` — executes the project `main` function.
- `Yukino: Run Current File` — executes the active source file.
- `Yukino: Run Test` — runs the test under the cursor.
- `Yukino: Run All Tests` — runs all project tests.
- `Yukino: Build Project` — builds the project.
- `Yukino: Check Diagnostics` — triggers immediate diagnostic and syntax rechecking.
- `Yukino: Show Documentation` — opens the Yukino documentation.
- `Yukino: Open Status Menu` — opens the extension action status menu.

---

## 2. Command reference for yukino-lsp

> [!NOTE]
> **Regular editor users do not need to learn or run these commands manually.**  
> Editor extensions (Zed, VS Code) manage `yukino-lsp` automatically in the background over the standard LSP protocol.  
> This command-line reference is provided for developers creating integrations with other editors (e.g. Neovim, Emacs, Helix), automation scripts, CI/CD pipelines, or manual troubleshooting.

The `yukino-lsp` utility combines a Language Server Protocol server with command-line tools for static analysis and formatting of Yukino code.

### Command: `lsp`

Runs the Language Server Protocol server over standard I/O (stdio). This is the primary mode used by code editors.

```sh
yukino-lsp lsp
```

### Command: `check`

Checks syntax and emits diagnostics without launching an interactive server.

```sh
yukino-lsp check [path] [--json]
```

- `[path]` — path to a `.ykn` file or project directory (default: current directory `.`).
- `--json` — emits diagnostic results in JSON format (useful for linters and CI/CD).
- Exits with `0` when there are no errors, or non-zero when diagnostic errors are found.

### Command: `watch`

Continuously watches files for changes and automatically reruns diagnostics.

```sh
yukino-lsp watch [path] [--delay <ms>] [--json]
```

- `[path]` — target directory or file (default: `.`).
- `--delay <ms>` — debounce delay between checks in milliseconds (default: 300).
- `--json` — emits diagnostic results in machine-readable JSON format.

### Command: `context`

Inspects code context for a symbol at a specific line and column.

```sh
yukino-lsp context <file.ykn> --line <line> --col <col>
```

- `--line <line>` — 0-based line index.
- `--col <col>` — 0-based column index.

### Command: `doc`

Displays documentation for a named symbol (function, struct, type).

```sh
yukino-lsp doc <symbol_name> [--json]
```

### Command: `format`

Formats a Yukino source file in place.

```sh
yukino-lsp format <file.ykn>
```

### General flags

- `yukino-lsp --version`, `-v`, `version` — prints the version.
- `yukino-lsp --help`, `-h`, `help` — displays help information.

---

## 3. Community extensions

This section lists community-developed extensions, plugins, and editor configurations (for Neovim, Sublime Text, Emacs, Helix, etc.).

*Currently empty.*

If you have built an extension or tool related to Yukino and would like to list it here:  
Submit a [Pull Request](https://github.com/yukitsuri/yukino/pulls) to the official [yukitsuri/yukino](https://github.com/yukitsuri/yukino) repository, adding your extension name, repository link, and setup instructions to both documents:
- `docs/lang/ru/extensions.md`
- `docs/lang/en/extensions.md`

---

[← Back: Installing the Compiler](1-setup.md) | [Next: Toolchain and Project Structure →](2-cli-project.md)
