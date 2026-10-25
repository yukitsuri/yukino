# 16. Testing and Test Runner

Yukino provides a first-class built-in unit and integration testing system. The compiler and runtime include an integrated test runner featuring parallel test execution, process isolation for workers, watchdog timeouts to guard against infinite loops, flexible test filtering, and clean, colorful terminal reporting.

---

## Table of Contents

1. [Quick Start: yukino test](#1-quick-start-yukino-test)
2. [Test Organization in Projects](#2-test-organization-in-projects)
   - 2.1 [Unit Tests Inside src/](#21-unit-tests-inside-src)
   - 2.2 [Integration Tests in tests/ Directory](#22-integration-tests-in-tests-directory)
3. [Conditional Compilation: cfg(test) and cfg!(test)](#3-conditional-compilation-cfgtest-and-cfgtest)
4. [Test Attributes](#4-test-attributes)
   - 4.1 [#[test]](#41-test)
   - 4.2 [#[should_panic]](#42-should_panic)
   - 4.3 [#[ignore]](#43-ignore)
   - 4.4 [#[timeout_ms]](#44-timeout_ms)
5. [Command Line: yukino test Flags](#5-command-line-yukino-test-flags)
6. [Configuration in yuki.toml](#6-configuration-in-yukitoml)
7. [Reporting and Dashboard Output](#7-reporting-and-dashboard-output)

---

## 1. Quick Start: yukino test

To run all tests in the current project, invoke:

```sh
yukino test
```

The runner performs the following steps:
1. Automatically enables the test compilation profile (`cfg(test)`).
2. Compiles project modules and integration tests found in `tests/`.
3. Links an executable test binary with the built-in test runner entry point.
4. Executes tests in isolated worker processes with real-time live progress reporting.
5. Displays a concise summary table and detailed failure diagnostics if any test fails.

---

## 2. Test Organization in Projects

### 2.1 Unit Tests Inside src/

Unit tests can be written directly in the source files under `src/` alongside the implementation, or in dedicated test modules guarded by `#[cfg(test)]`:

```yukino
// src/math.ykn
public fun add(a: int32, b: int32): int32 {
    a + b
}

#[test]
fun test_add() {
    assert(add(2, 2) == 4)
}
```

During normal builds (`yukino build`), functions marked with `#[test]` are automatically stripped by the compiler (dead code elimination) and do not increase the size of release binaries.

### 2.2 Integration Tests in tests/ Directory

Integration tests are placed in a separate `tests/` directory at the root of the project (or the path defined by `tests_dir` in the manifest). Every `.ykn` file in this directory is compiled as a test module:

```
my_project/
├── yuki.toml
├── src/
│   ├── lib.ykn
│   └── calc.ykn
└── tests/
    └── integration.ykn
```

Integration tests can load project modules by the package name defined in `yuki.toml`:

```yukino
// tests/integration.ykn
load my_project.calc: evaluate

#[test]
fun test_end_to_end_calculation() {
    set result = evaluate("2 + 2 * 2")
    assert(result == 6)
}
```

Tests are grouped into suites according to their file and module names: tests in `src/main.ykn` appear under the `main` suite, and tests in `tests/integration.ykn` appear under `tests` or `integration`.

---

## 3. Conditional Compilation: cfg(test) and cfg!(test)

To compile code only when running tests (such as test helpers, mocks, or fixture generators), use the `test` predicate:

### Attribute #[cfg(test)]
```yukino
#[cfg(test)]
struct TestMock {
    data: string
}

#[cfg(test)]
fun create_mock_client(): TestMock {
    TestMock { data: "mock payload" }
}
```

### Built-in Macro cfg!(test)
```yukino
fun initialize_service() {
    if cfg!(test) {
        // Fast mock logic for tests
    } else {
        // Real external connection
    }
}
```

---

## 4. Test Attributes

### 4.1 #[test]

Marks a function as a test. A test function takes no parameters and returns no values:

```yukino
#[test]
fun test_basic_equality() {
    set x = 10
    set y = 20
    assert(x + y == 30)
}
```

### 4.2 #[should_panic]

Specifies that a test succeeds **only** if the function triggers a `panic`:

```yukino
struct ValidationError {
    message: string
}

#[test]
#[should_panic]
fun test_must_fail() {
    panic(ValidationError { message: "invalid input" })
}
```

Substring matching on the panic error payload is also supported:

```yukino
#[test]
#[should_panic = "division by zero"]
fun test_zero_div() {
    panic(ValidationError { message: "critical: division by zero detected" })
}
```

If the function finishes without panicking or if the panic message does not contain the expected substring, the test is marked as `FAIL`.

### 4.3 #[ignore]

Allows temporarily skipping slow, resource-heavy, or flaky tests during routine test runs:

```yukino
#[test]
#[ignore]
fun test_long_running_migration() {
    // Skipped during normal yukino test
}

#[test]
#[ignore = "Requires external PostgreSQL container"]
fun test_remote_api() {
    // Reason explanation
}
```

The compiler emits a `W0423` warning when ignored tests are present. To execute ignored tests, pass `--ignored` or `--include-ignored`.

### 4.4 #[timeout_ms]

Defines a per-test execution time limit in milliseconds. If execution exceeds this duration, the isolated worker process is terminated by the runtime watchdog, and the test is reported as failed with a timeout notice:

```yukino
#[test]
#[timeout_ms = 500]
fun test_responsive_operation() {
    // Must complete within 500 ms
}
```

---

## 5. Command Line: yukino test Flags

The `yukino test` command provides fine-grained controls for filtering and execution:

| Flag | Description |
| :--- | :--- |
| `<FILTER>` | Run tests whose qualified name contains the specified substring (e.g. `yukino test crypto`) |
| `--exact` | Require an exact match between test name and filter |
| `--nocapture` | Stream test stdout and stderr directly to the terminal |
| `-j, --threads <N>` | Number of concurrent test threads |
| `--ignored` | Run **only** ignored tests marked with `#[ignore]` |
| `--include-ignored` | Run all tests, including ignored ones |
| `--fail-fast` | Abort test execution immediately upon the first failure |
| `--list` | Print the list of matching tests without running them |
| `-q, --quiet` | Minimal output mode |
| `--timeout <MS>` | Override default test timeout in milliseconds |
| `-b, --rebuild` | Force complete rebuild of test binaries |
| `-p, --package <NAME>` | Run tests for a specific workspace package |
| `-F, --features <LIST>` | Enable comma-separated features |
| `--all-features` | Enable all project features |
| `--no-default-features` | Disable default features |

### Examples

```sh
# Run tests matching "crypto"
yukino test crypto

# Run an exact test function
yukino test "math::test_add" --exact

# Show test print statements live
yukino test --nocapture

# Run using 4 threads and stop on first error
yukino test -j 4 --fail-fast

# List all tests
yukino test --list
```

---

## 6. Configuration in yuki.toml

Default test runner preferences can be specified in the `[test]` section of `yuki.toml`:

```toml
[test]
# Concurrency level ("auto" or a positive integer)
threads = 4

# Global default timeout per test (in milliseconds)
timeout_ms = 30000

# Capture stdout/stderr (true) or pass through (false)
capture_output = true

# Abort on first failure
fail_fast = false

# Directory containing integration tests (defaults to "tests")
tests_dir = "tests"
```

---

## 7. Reporting and Dashboard Output

During execution, the runner displays a live single-line progress indicator:
- Animated worker spinner.
- Scoped test name with cyan `::` styling.
- Progress bar formatted as `[###       ]`.
- Counter indicating completed tests `[14/42]` and elapsed time.

Upon completion, a clean summary table is printed:

```
SUITE          TESTS     STATUS     TIME    
main           3/3       [OKIE]     154ms   
tests          1/1       [OKIE]     2ms     

Result: OKIE | 4 passed, 0 failed, 0 skipped in 0.08s
```

Suite status indicators:
- `[OKIE]` — all tests in the suite passed.
- `[FAIL]` — one or more tests failed.
- `[SKIP]` — all tests in the suite were skipped (`#[ignore]`).

When failures occur, detailed diagnostics are presented below the table, including captured error messages, panic details, and watchdog timeout alerts.

---

[← Back: Workspaces and Multi-Package Projects](15-workspaces.md) | [Next: Standard Library Overview →](stdlib/0-overview.md)
