# Установка Yukino

Это руководство описывает установку компилятора `yukic`, менеджера проектов `yukino` и сборку бинарников под другие платформы.

## Оглавление

1. [Поддерживаемые платформы](#поддерживаемые-платформы)
2. [Установка на Linux](#установка-на-linux)
   - 2.1 [Nix (рекомендуется)](#nix-рекомендуется)
   - 2.2 [Ручная сборка через Cargo](#ручная-сборка-через-cargo)
3. [Установка на Windows](#установка-на-windows)
4. [Кросс-сборка](#кросс-сборка)
   - 4.1 [Зависимости для кросс-сборки](#зависимости-для-кросс-сборки)
   - 4.2 [Компилятор](#кросс-сборка-компилятора)
   - 4.3 [Runtime](#кросс-сборка-runtime)
5. [Проверка установки](#проверка-установки)

## Поддерживаемые платформы

Компилятор поддерживает следующие целевые платформы:

- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-linux-android`

macOS пока не поддерживается. Работа над поддержкой macOS начнется, как только появится человек с Mac, готовый заниматься разработкой и тестированием.

## Установка на Linux

### Nix (рекомендуется)

При использовании Nix все необходимые зависимости сборки (LLVM 22, Clang, LLD, системные библиотеки) и переменные окружения настраиваются автоматически через `flake.nix`. Ручной экспорт переменных и установка пакетов не требуются.

Установка готового release-пакета из конкретного тега:

```sh
nix profile add github:yukitsuri/yukino/v0.1.0#yukino
```

Для локальной установки из клонированного репозитория:

```sh
git clone --branch v0.1.0 https://github.com/yukitsuri/yukino.git
cd yukino
nix profile add .#yukino
```

Debug-пакет устанавливается так:

```sh
nix profile add .#yukino-debug
```

Он предоставляет `yukino-dbg` и `yukic-dbg`.

Для входа в готовое окружение разработки со всеми настроенными переменными:

```sh
nix develop
```

### Ручная сборка через Cargo

Если вы не используете Nix, все зависимости и переменные окружения необходимо настроить вручную перед сборкой.

#### Зависимости сборки

- Rust `1.98.1` или новее;
- Cargo;
- LLVM 22;
- Clang и `libclang`;
- LLD;
- `pkg-config`;
- `libffi`, `zlib`, `libxml2` и `ncurses`.

Точные имена пакетов зависят от вашего дистрибутива Linux.

#### Переменные окружения

Перед компиляцией через Cargo пути к LLVM и `libclang` должны быть явно экспортированы в текущей сессии шелла:

```sh
export LLVM_CONFIG=/path/to/llvm-config
export LLVM_SYS_221_PREFIX=/path/to/llvm
export LIBCLANG_PATH=/path/to/libclang/lib
```

#### Сборка и установка

После настройки окружения установите оба бинарника из тега релиза:

```sh
cargo install --git https://github.com/yukitsuri/yukino --tag v0.1.0 --locked --package yukino
cargo install --git https://github.com/yukitsuri/yukino --tag v0.1.0 --locked --package yukic-driver
```

Cargo установит бинарники в `~/.cargo/bin`:

- `yukino` — менеджер проектов и сборки;
- `yukic` — низкоуровневый компилятор.

## Установка на Windows

Для Windows используйте PowerShell-скрипт, который автоматически скачивает готовые бинарники последнего релиза и устанавливает их в `PATH`:

```powershell
irm https://raw.githubusercontent.com/yukitsuri/yukino/main/scripts/wininstall.ps1 | iex
```

Скрипт не требует ручной настройки LLVM или сборки компилятора из исходников. Если вам требуется собрать проект из исходников вручную через Cargo, убедитесь, что в системе установлены LLVM 22, MSVC/MinGW и заданы соответствующие переменные окружения LLVM.

## Кросс-сборка

### Зависимости для кросс-сборки

Для сборки под другие платформы дополнительно требуются:

- `cargo-zigbuild` и Zig — для Linux-таргетов, отличных от текущей хост-системы;
- Android NDK — для Android-таргетов;
- Компоненты целевых платформ Rust, устанавливаемые через `rustup target add <target>`.

### Кросс-сборка компилятора

Склонируйте репозиторий с нужным тегом:

```sh
git clone --branch v0.1.0 https://github.com/yukitsuri/yukino.git
cd yukino
python3 scripts/crossbuild.py
```

Скрипт предложит выбрать один или несколько таргетов. `0` собирает все доступные для `crossbuild.py` таргеты:

- `x86_64-pc-windows-gnu`;
- `x86_64-unknown-linux-gnu`;
- `x86_64-unknown-linux-musl`;
- `aarch64-unknown-linux-gnu`;
- `aarch64-unknown-linux-musl`;
- `aarch64-linux-android`.

Результаты находятся в:

```text
build/0.1.0/yukino/yukino-<target>
build/0.1.0/yukic/yukic-<target>
```

На Windows вместо `python3` используйте `py -3`.

### Кросс-сборка runtime

Runtime собирается отдельно и создает только статические архивы `.a`:

```sh
python3 scripts/crossbuild-rt.py
```

Результаты находятся в:

```text
build/0.1.0/runtime/<target>.a
```

Этот runtime передается `yukic` для custom target через `--runtime <path>`. Для host target отдельный runtime не нужен: `yukic` использует встроенный host-runtime.

## Проверка установки

```sh
yukino -v
yukic -v
yukino --help
yukic --help
yukic targets
```

Если команда не найдена, проверьте, что `~/.cargo/bin` или `%USERPROFILE%\.cargo\bin` добавлен в `PATH`.

---

[← Назад: Обзор документации](0-overview.md) | [Вперед: Тулчейн и структура проекта →](2-cli-project.md)
