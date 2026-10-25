# Standard Library (stdlib): Overview

The Yukino standard library is designed with a modular architecture, offering high-level abstractions, collections, system calls, and utilities without the overhead of a heavy runtime.

## Table of Contents

- [1. Standard Library Architecture](#1-standard-library-architecture) - two-tier model (Rust core + Yukino std).
- [2. Importing the Whole Standard Library](#2-importing-the-whole-standard-library) - exports included in `load std`.
- [3. Module Catalog](#3-standard-library-modules):
  - [std.strings - Strings and Text](1-strings.md)
  - [std.io - Stream and Console I/O](2-io.md)
  - [std.threads - System Threads and Mutexes](3-threads.md)
  - [std.platformio - Platform I/O (POSIX, Linux, Windows)](4-platformio.md)
  - [std.math - IEEE 754 Math Primitives and Bit Arithmetic](5-math.md)
  - [std.datetime - Date, Time, Timezones, Durations, and ISO 8601](6-datetime.md)
  - [std.random - Random Number Generation, SplitMix64, Distributions, and UUID v4](7-random.md)
  - [std.fs - File System: Paths, Files, Directories, Streams, and System Paths](8-fs.md)
  - [std.collections - Collections, Slices, Algorithms, and Containers](9-collections.md)
  - [std.process - Child Process Execution, Command Builder, and Environment](10-process.md)
  - [std.os - System Level and libc: POSIX, Linux, Windows, Android](11-os.md)
  - [std.sysinfo - System, CPU, Memory, and Disk Information](12-sysinfo.md)
  - [std.env - Environment Variables and Dotenv Configuration](13-env.md)
  - [std.simpleurl - RFC 3986 URL Parser and Builder](14-simpleurl.md)
  - [std.compress - Compression and Decompression (gzip, zstd, xz, deflate, zlib, bz2, lz4, crc32)](15-compress.md)
  - [std.fs.archives - TAR and ZIP Archives, Compression, and Password Protection](16-archives.md)
  - [std.locale - Language and Regional Settings, Locale, Country, and Timezone](17-locale.md)
  - [std.numf - Number Formatting, Radix Conversion, Separators, Bytes, and Metric](18-numf.md)
  - [std.sharedc - Dynamic Shared Library and Symbol Resolution](19-sharedc.md)
  - [std.json - Strict RFC 8259 JSON Parsing, Serialization, and JsonValue](20-json.md)
  - [std.jsonc - JSON with Comments and Trailing Commas](21-jsonc.md)
  - [std.outf - Terminal Output Formatting, Color Styling, TrueColor, and ANSI](22-outf.md)
  - [std.line - Declarative Command-Line Argument Parser](23-line.md)
  - [std.itermtool - Low-Level Interactive Terminal Engine and TUI Foundation](24-itermtool.md)
  - [std.stdotool - Dynamic Visual Output in Stdout, Animations, Progress Bars, and LiveArea](25-stdotool.md)
  - [std.hash - Hashing and Checksums: BitHash64, SHA-2, SHA-3, BLAKE3, and Cryptographic Utilities](26-hash.md)
  - [std.crypto - Cryptographic Primitives and Protocols: HMAC, ChaCha20, Poly1305, AEAD, X25519, Ed25519, HKDF, PBKDF2](27-crypto.md)
  - [std.net - Networking, Sockets, TCP/UDP, TLS, HTTP Codec, Server, and Requests Client](28-net.md)
  - [std.regex - Unicode Regular Expressions, Capture Groups, Replacement, and Splitting](29-regex.md)
  - [std.toml - TOML v1.0.0 Parsing, Serialization, and TomlValue](30-toml.md)
  - [std.yaml - YAML v1.2 Parsing, Serialization, and YamlValue](31-yaml.md)
  - [std.sqlite - Embedded SQLite 3 database, transactions, prepared statements, and SqliteValue / SqliteRow](32-sqlite.md)
  - [std.encore - UTF-8 validation, Unicode codepoints, UAX #29 grapheme clusters, and UTF-16 interop (Win32/JVM)](33-encore.md)
  - [Directory of Third-Party Libraries and Ecosystem](../../libs-en.md)

---

## 1. Standard Library Architecture

The standard library is structured into two isolated layers:

1. **Low-level system layer (`builtin/stdlib/core` in Rust):**
   Interacts directly with operating system syscalls (file descriptors, OS threads, platform I/O, low-level buffers). Compiled into the static library `libyukistd_core.a` and exports a pure C ABI.

2. **High-level Yukino layer (`builtin/stdlib/std` in Yukino):**
   Written in Yukino (`.ykn`). Wraps C ABI calls from the system layer into idiomatic, strictly-typed functions, structures, and interfaces. Compiled into `libstd.a`.

```text
+-------------------------------------------------------+
|                      User Code                        |
+-------------------------------------------------------+
                           |
                           v
+-------------------------------------------------------+
|                        std.*                          |
+-------------------------------------------------------+
                           | C ABI (cfun)
                           v
+-------------------------------------------------------+
|        core (Rust: POSIX / Win32 / Syscalls)          |
+-------------------------------------------------------+
```

---

## 2. Importing the Whole Standard Library

The root `std` package unifies the public interfaces of all standard library submodules:

### Importing all modules

```yukino
load std
```

> [!NOTE]
> The form `load std` is fully equivalent to `load std: *` (as well as `load std.lib`).

This brings all public components into the current scope:

- **`std.strings`** - string functions (slices, search, case conversion, trimming, formatting, parsing `int64`, `float64`, `bool`) and error structs (`StringIndexError`, `StringParseError`).
- **`std.io`** - console and stream I/O (`print`, `println`), input reading.
- **`std.threads`** (and companion module **`std.sync`**) - OS system threads, synchronization primitives (Mutex, RwLock, CondVar, Semaphore, Barrier, Once), atomic operations, and cross-thread message channels.
- **`std.platformio`** - platform-dependent low-level I/O for Linux, Unix, and Windows.
- **`std.math`** - IEEE 754 bitcasts, hardware square root and FMA, bit counters (`clz`, `ctz`, `popcount`), carry arithmetic (`mul_wide`, `add_carry`, `sub_borrow`), and fundamental constants.
- **`std.datetime`** - civil and monotonic time (`DateTime`, `Duration`), timezone offsets, the Gregorian calendar, intervals, `Stopwatch` and `Timer` classes for precise timing and countdowns, and ISO 8601 formatting/parsing.
- **`std.random`** - deterministic SplitMix64 PRNG (`class Rng`), global random values, ranges, sampling, UUID v4, and Gaussian normal distribution.
- **`std.fs`** - file system operations: cross-platform `paths`, directory management `dirs`, file operations and stream `class File` in `files`, TAR and ZIP archives in `archives`, and XDG/Windows system and user directories in `defaults`.
- **`std.collections`** - slices, search algorithms, functional transformations (`amap`, `afilter`, `areduce`), in-place mutation (`asort`, `areverse`, `afill`), string joining with `ajoin`, dynamic list `List<T>`, associative array `HashMap<K, V>`, set `HashSet<T>`, double-ended queue `Deque<T>`, and FIFO queue `Queue<T>`.
- **`std.process`** - process management: spawning programs (`run`, `run_shell`, `read_output`), `Command` builder, asynchronous `ChildProcess`, standard stream redirection (stdin/stdout/stderr), current process properties, and environment variables.
- **`std.os`** - system syscalls and libc bindings for POSIX/Unix (`fork`, `execve`, `waitpid`, `kill`, `chmod`, `chown`, `signals`), Linux (`epoll`, `eventfd`, `timerfd`, `memfd`, `prctl`, `gettid`), Windows (`terminate_process`, `wait_for_single_object`, `handles`), and Android (`android_log_print`).
- **`std.sysinfo`** - hardware and system information: platform, host, kernel, uptime, physical and logical CPU cores, CPU brand and frequency, load average, RAM and swap, disk drives and partitions, battery status.
- **`std.env`** - reading, writing, typed access (`get_int`, `get_bool`, `get_float`), removal and validation of process environment variables, retrieving all variables as an array or `HashMap<string, string>`, and loading and parsing `.env` (dotenv) configuration files.
- **`std.simpleurl`** (also available as **`std.url`**) - RFC 3986 URL parsing, `Url` class, `UrlBuilder`, percent-encoding and decoding, query parameter manipulation, and path joining.
- **`std.compress`** - compression and decompression of buffers and files with streaming codecs (`gzip`, `zstd`, `xz`, `deflate`, `zlib`, `bzip2`, `lz4`) and `crc32` checksum calculation.
- **`std.fs.archives`** - TAR and ZIP archive creation, unpacking, and listing with compression and password-based encryption (ZipCrypto, AES, WinZip AES).
- **`std.locale`** - operating system language and regional settings: primary language (ISO 639-1), full locale tag (POSIX / BCP 47), user preferred languages, country code (ISO 3166-1), and system timezone (IANA).
- **`std.numf`** - number formatting: radix conversion (bin, hex, oct, base 2..36), parsing with prefixes, thousand separators, to_fixed, pad_left, human_bytes (IEC/SI), human_metric, and percentages.
- **`std.sharedc`** - dynamic shared library loading (.so, .dll, .dylib), C symbol resolution, calling native C functions via `cfun`, and current process namespace access.
- **`std.json`** - strict RFC 8259 JSON parsing, validation, serialization, fluent non-panicking `JsonValue` class, and pretty-printing.
- **`std.jsonc`** - JSON with Comments (`//`, `/* */`), trailing commas support in objects/arrays, and comment stripping (`jsonc_strip_comments`) with line position preservation.
- **`std.outf`** - console output styling and terminal colors: 8 basic colors, 8 bright colors, 256 colors, TrueColor RGB, text modifiers (bold, dim, italic, underline, blink, reversed, hidden, strikethrough), fluent `Styled` class, color modes `COLOR_AUTO`/`ALWAYS`/`NEVER`, and terminal detection (`NO_COLOR`, `CLICOLOR`, `supports_color`, `strip_ansi`, `visible_len`).
- **`std.line`** - declarative object-oriented CLI argument parser in clap builder style: commands, subcommands, named options, flags, bundled short flags (`-abc`), `--` terminator, automatic help/version (`yukienv!("bin.version")`), conflicting arguments, and fuzzy suggestions ("Did you mean...").

- **`std.hash`** - checksum and cryptographic hash computation: ultra-fast BitHash64, SHA-256, SHA-512, SHA3-256, SHA3-512 standards, tree-based BLAKE3, hex encoding, and timing-safe comparison.
- **`std.crypto`** - cryptographic suite: HMAC (SHA-256, SHA-512), symmetric ChaCha20 cipher, Poly1305 MAC, ChaCha20-Poly1305 AEAD, X25519 Diffie-Hellman key exchange, Ed25519 digital signatures, HKDF, and PBKDF2.
- **`std.net`** - network communications: socket addresses SocketAddr, TCP client and server (TcpStream, TcpListener), UDP sockets (UdpSocket), event multiplexing via SocketPoller, TLS/HTTPS encryption (TlsStream), HTTP/1.1 codec (HttpRequest, HttpResponse, HttpHeaders), HTTP/HTTPS server (HttpServer, HttpsServer), and high-level requests client (get, post, post_json, put, delete, head, patch).
- **`std.regex`** - regular expressions: linear-time engine free of ReDoS, full Unicode support, compilation flags, match boundary structures (`Match`), named and numbered capture groups (`Captures`), substring replacement with expansions (`$1`, `${name}`), and regular expression splitting.
- **`std.toml`** - TOML v1.0.0 configuration format: parsing tables, table arrays, dates, and scalars, TomlValue class, factory constructors, and serialization.
- **`std.yaml`** - YAML v1.2 structured data format: block and flow collections, mappings, sequences, YamlValue class, validation, and serialization.
- **`std.sqlite`** - embedded SQLite 3 relational database: Database connections, SQL query execution, transactions, prepared statements, and typed SqliteRow records.
- **`std.encore`** - systems and application Unicode processing: UTF-8 validation and lossy decoding, codepoints extraction and encoding, UAX #29 grapheme cluster segmentation, and UTF-16 encoding/decoding for Win32 API and JVM interoperability.

## 3. Standard Library Modules

| Module | Description | Documentation |
| :--- | :--- | :--- |
| `std.strings` | String search, slicing, case conversion, replacement, padding, and primitive type parsing. | [1-strings.md](1-strings.md) |
| `std.io` | Stream and console I/O (`print`, `println`, `read_line`, generic `Reader<T>`/`Writer<T>` interfaces, raw terminal mode). | [2-io.md](2-io.md) |
| `std.threads` (`std.sync`) | OS thread management (`spawn`, `join`), synchronization primitives (`Mutex`, `RwLock`, `CondVar`, `Semaphore`, `Barrier`, `Once`), atomics, and `Channel`. | [3-threads.md](3-threads.md) |
| `std.platformio` | Low-level syscalls and descriptors for POSIX (pipes, dup, sync), Linux (splice, tee), and Windows (Console, pipes). | [4-platformio.md](4-platformio.md) |
| `std.math` | IEEE 754 bitcasts, float operations, bit twiddling, extended precision arithmetic. | [5-math.md](5-math.md) |
| `std.datetime` | Date, time, timezones, Duration intervals, Stopwatch, Timer, monotonic clock, ISO 8601. | [6-datetime.md](6-datetime.md) |
| `std.random` | SplitMix64 PRNG, class Rng, system entropy, ranges, UUID v4, and Gaussian distribution. | [7-random.md](7-random.md) |
| `std.fs` | Cross-platform paths, file and directory manipulation, stream `class File`, TAR/ZIP archives, XDG system paths. | [8-fs.md](8-fs.md) |
| `std.collections` | Slice algorithms (search, filter, sort, reverse), containers List, HashMap, HashSet, Deque, Queue. | [9-collections.md](9-collections.md) |
| `std.process` | Child process execution, stdout/stderr stream capture, Command builder, environment variables, and the current process. | [10-process.md](10-process.md) |
| `std.os` | System level and libc: POSIX/Unix (fork, exec, signals), Linux (epoll, eventfd, timerfd, prctl), Windows, Android. | [11-os.md](11-os.md) |
| `std.sysinfo` | Hardware and system diagnostics: host, OS, kernel, uptime, CPU (cores, brand, frequency), load average, RAM/Swap, disks, and battery. | [12-sysinfo.md](12-sysinfo.md) |
| `std.env` | Environment variable inspection, typed getters, bulk retrieval, and .env configuration file loading. | [13-env.md](13-env.md) |
| `std.simpleurl` (`std.url`) | RFC 3986 URL parsing and building, Url class, UrlBuilder, percent-encoding/decoding, query parameter helpers. | [14-simpleurl.md](14-simpleurl.md) |
| `std.compress` | Compression and decompression of data and files: Gzip, Zstandard, XZ, Deflate, Zlib, Bzip2, LZ4, CRC-32 checksums. | [15-compress.md](15-compress.md) |
| `std.fs.archives` | TAR and ZIP archives: pack, unpack, list entries, transparent compression, and encryption (ZipCrypto, AES, WinZip AES). | [16-archives.md](16-archives.md) |
| `std.locale` | Operating system locale settings: interface language (ISO 639-1), locale tag (POSIX/BCP 47), country (ISO 3166-1), preferred languages, timezone. | [17-locale.md](17-locale.md) |
| `std.numf` | Number formatting: radix conversion (bin, hex, oct, base 2..36), parsing with prefixes, thousand separators, to_fixed, pad_left, human_bytes (IEC/SI), human_metric, and percentages. | [18-numf.md](18-numf.md) |
| `std.sharedc` | Dynamic library loading (.so, .dll, .dylib), symbol resolution, cfun calling, process namespace access. | [19-sharedc.md](19-sharedc.md) |
| `std.json` | Strict RFC 8259 JSON parser, safe JsonValue class, node factories, compact and formatted serialization. | [20-json.md](20-json.md) |
| `std.jsonc` | JSON with single-line and multi-line comments, trailing commas, and jsonc_strip_comments. | [21-jsonc.md](21-jsonc.md) |
| `std.outf` | Terminal output styling and colors: 8/16/256 colors, TrueColor RGB, text modifiers, fluent Styled class, strip_ansi, TTY detection. | [22-outf.md](22-outf.md) |
| `std.line` | Command-line argument parser in clap builder style: commands, subcommands, flags, options, auto-help, Did you mean suggestions. | [23-line.md](23-line.md) |
| `std.itermtool` | Low-level interactive terminal engine for TUI: alternate screen, raw mode, 0-based cursor, TrueColor/256/Basic colors, keyboard and mouse event parsing (KeyEvent, MouseEvent), background and box rendering. | [24-itermtool.md](24-itermtool.md) |
| `std.stdotool` | Dynamic visual output in stdout: custom animation frame cycler (Frames), progress bar constructor (Bar), single-line (LiveLine) and multi-line (LiveArea) rendering, cursor controls, and formatters. | [25-stdotool.md](25-stdotool.md) |
| `std.hash` | Checksum and cryptographic hash computation: ultra-fast BitHash64, SHA-256, SHA-512, SHA3-256, SHA3-512 standards, tree-based BLAKE3, hex encoding, and timing-safe comparison. | [26-hash.md](26-hash.md) |
| `std.crypto` | Cryptographic primitives and protocols: HMAC, ChaCha20, Poly1305, AEAD, X25519, Ed25519, HKDF, PBKDF2, entropy, and constant-time utilities. | [27-crypto.md](27-crypto.md) |
| `std.net` | Networking stack: TCP client/server, UDP sockets, SocketPoller event multiplexing, TLS/HTTPS, HTTP/1.1 codec, HTTP server, and requests client. | [28-net.md](28-net.md) |
| `std.regex` | Unicode regular expression engine: compilation, flags, offset matching, named capture groups (Captures), replacement, and splitting. | [29-regex.md](29-regex.md) |
| `std.toml` | TOML v1.0.0 configuration format: parsing tables, table arrays, dates, and scalars, TomlValue class, factory constructors, and serialization. | [30-toml.md](30-toml.md) |
| `std.yaml` | YAML v1.2 structured data format: block and flow collections, mappings, sequences, YamlValue class, validation, and serialization. | [31-yaml.md](31-yaml.md) |
| `std.sqlite` | Embedded SQLite 3 database, transactions, prepared statements, and typed SqliteRow records. | [32-sqlite.md](32-sqlite.md) |
| `std.encore` | UTF-8 validation/decoding, Unicode scalar codepoints, UAX #29 grapheme segmentation, UTF-16 Win32/JVM interop. | [33-encore.md](33-encore.md) |
| *Ecosystem* | Official packages and community third-party libraries for Yukino. | [libs-en.md](../../libs-en.md) |

> [!TIP]
> After reading the documentation, it is recommended to explore the [standard library source code on GitHub](https://github.com/yukitsuri/yukino/tree/main/builtin/stdlib/std) to understand the actual structure and internals of `stdlib`.

---

[← Previous: Testing and Test Runner](../16-tests.md) | [Next: Module std.strings →](1-strings.md)
