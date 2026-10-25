# Yukino language

> [!WARNING]
> Unfinished

Yukino is a statically typed compiled language for native applications. It aims to be maximally simple and concise while remaining native, fast to compile, and practical to use.

The project is built around a small compiler and a lightweight runtime. Yukino does not require a virtual machine such as the JVM and does not depend on a tracing garbage collector. Memory is designed around ownership and regions, so applications can stay close to the native model without carrying a large runtime system.

The compiler uses LLVM IR through Inkwell. The toolchain consists of two command-line tools:

- `yukic` — the compiler driver that turns Yukino source into native output;
- `yukino` — the project manager and build toolchain.

## About project

This is just an experimental project; I don't position it as a professional programming language.

See the [syntax example](#example) for a small complete program.

The language is still under active development. Some parts of the toolchain and standard library are incomplete, so the documentation describes the currently implemented behavior and available workflows.

[Discord community](https://discord.gg/f2tePA6H8S)

## What the compiler can do

The compiler can produce:

- native executables;
- static libraries;
- dynamic libraries.

It also provides C ABI and C FFI support for exporting Yukino functions and using native C-compatible functions and libraries.

## Ideal use cases

Yukino is especially suited for:

- servers and other native network services;
- command-line tools and system utilities;
- native libs, etc

## Documentation

Start with the overview for your language:

- [English documentation](docs/lang/en/0-overview.md)
- [Русская документация](docs/lang/ru/0-overview.md)

Installation guides are available separately:

- [Installation on Linux, Windows, and cross-compilation](docs/lang/en/1-setup.md)
- [Установка на Linux, Windows и кросс-сборка](docs/lang/ru/1-setup.md)

For compiler development, see:

- [Yukino developer documentation](https://github.com/yukitsuri/yukino/blob/main/docs/dev/0-owerview.md)
- [Build and development](docs/dev/1-build.md)

The numbered documents are intended to be read in order. The documentation assumes familiarity with typed languages, functional programming, object-oriented programming, and common development tools.

## Example

```yukino
load std.io: print

struct MathError {
    message: string
}

fun calc(a: int32, op: char, b: int32) -> int32 {
    return switch op {
        '*' -> a * b,
        '+' -> a + b,
        '-' -> a - b,
        '/' -> if b == 0 {
            panic(MathError { message: "division by zero" })
        } else {
            a / b
        },
        else -> {
            print("Invalid operator: ${op}\n")
            exit(1)
        }
    }
}

fun main() {
    set input: string[] = args()

    if input.len() < 3 {
        print("Please enter 3 arguments: <num1> <op> <num2>\n")
        exit(1)
    }

    set input_a: int32 = cast(input[0], "int32")
    set input_op: char = cast(input[1], "char")
    set input_b: int32 = cast(input[2], "int32")

    sus {
        set result: int32 = calc(input_a, input_op, input_b)
        print("Your result: ${result}\n")
    } catch e: MathError {
        print("Math error: ${e.message}\n")
        exit(1)
    } catch e {
        print("Error: ${e.message}\n")
        exit(1)
    }
}
```

Yukino also supports C ABI functions with explicit calling-convention and optimization attributes:

```yukino
|cdecl, inline_always| public cfun yukino_add(a: c_int, b: c_int) -> c_int {
    return a + b
}

|cdecl| cfun puts(message: ptr<c_char>) -> c_int
```

## License

[Apache License 2.0](LICENSE) 2026
