# Модуль std.platformio

Модуль `std.platformio` предоставляет прямой низкоуровневый доступ к системным API ввода-вывода конкретных целевых операционных систем: POSIX (Unix), Linux-специфичные системные вызовы нулевого копирования данных и Win32 API для Windows.

---

## 1. Подключение: load std.platformio

Импорт всего пакета:

```yukino
load std.platformio
```

В текущую область видимости импортируются константы, структуры и функции платформенного ввода-вывода:

- **Константы стандартных дескрипторов:**
  - *POSIX:* `STDIN_FILENO`, `STDOUT_FILENO`, `STDERR_FILENO`;
  - *Windows:* `STD_INPUT_HANDLE`, `STD_OUTPUT_HANDLE`, `STD_ERROR_HANDLE`.
- **Структуры дескрипторов:**
  - `PipeFds` — пара файловых дескрипторов конвейера для Unix/Linux (`read_fd`, `write_fd`);
  - `WinPipeHandles` — пара системных хэндлов конвейера Windows (`read_handle`, `write_handle`);
  - `IoError` — структура системной ошибки ввода-вывода (коды `201`–`210`).
- **Функции POSIX (Unix):**
  - `isatty`, `pipe`, `dup`, `dup2`, `fsync`, `fdatasync`.
- **Функции Linux:**
  - `pipe2`, `dup3`, `splice`, `tee`.
- **Функции Windows (Win32):**
  - `get_std_handle`, `is_console`, `get_console_mode`, `set_console_mode`, `create_pipe`.

Все эти сущности также доступны при загрузке всей стандартной библиотеки: `load std`.

---

## 2. Константы

### 2.1 Константы POSIX дескрипторов

```yukino
public static STDIN_FILENO: int32 = 0
public static STDOUT_FILENO: int32 = 1
public static STDERR_FILENO: int32 = 2
```

- `STDIN_FILENO` (0) — стандартный файловый дескриптор ввода.
- `STDOUT_FILENO` (1) — стандартный файловый дескриптор вывода.
- `STDERR_FILENO` (2) — стандартный файловый дескриптор вывода ошибок.

### 2.2 Константы Windows дескрипторов

```yukino
public static STD_INPUT_HANDLE: int32 = -10
public static STD_OUTPUT_HANDLE: int32 = -11
public static STD_ERROR_HANDLE: int32 = -12
```

- `STD_INPUT_HANDLE` (-10) — идентификатор стандартного устройства ввода консоли Win32.
- `STD_OUTPUT_HANDLE` (-11) — идентификатор стандартного устройства вывода консоли Win32.
- `STD_ERROR_HANDLE` (-12) — идентификатор стандартного устройства вывода ошибок Win32.

---

## 3. Структуры данных и ошибок

### 3.1 PipeFds (Unix / Linux)

Хранит пару файловых дескрипторов анонимного конвейера.

```yukino
public struct PipeFds {
    read_fd: int32
    write_fd: int32
}
```

- `read_fd` — файловый дескриптор для чтения данных из канала.
- `write_fd` — файловый дескриптор для записи данных в канал.

### 3.2 WinPipeHandles (Windows)

Хранит пару 64-битных хэндлов анонимного конвейера Windows.

```yukino
public struct WinPipeHandles {
    read_handle: int64
    write_handle: int64
}
```

- `read_handle` — хэндл для чтения из конвейера.
- `write_handle` — хэндл для записи в конвейер.

### 3.3 IoError

Структура ошибки, генерируемая при системных сбоях вызовов ядра операционной системы.

```yukino
public struct IoError {
    message: string
    code: int32
}
```

- `code: 201` — ошибка вызова `pipe` (POSIX);
- `code: 202` — ошибка дублирования дескриптора `dup` (POSIX);
- `code: 203` — ошибка дублирования дескриптора `dup2` (POSIX);
- `code: 204` — сбой сброса буферов на диск `fsync` (POSIX);
- `code: 205` — сбой сброса данных на диск `fdatasync` (POSIX);
- `code: 206` — ошибка создания конвейера `pipe2` (Linux);
- `code: 207` — ошибка дублирования дескриптора `dup3` (Linux);
- `code: 208` — системная ошибка перемещения данных `splice` (Linux);
- `code: 209` — системная ошибка дублирования данных `tee` (Linux);
- `code: 210` — сбой создания анонимного канала `create_pipe` (Windows).

---

## 4. Справочник функций: POSIX (Unix)

### 4.1 isatty

Проверяет, ссылается ли указанный числовой дескриптор `fd` на открытое терминальное устройство (TTY).

```yukino
public fun isatty(fd: int32) -> bool
```

Возвращает `true`, если дескриптор указывает на терминал, иначе `false`.

Функция никогда не паникует.

```yukino
load std.platformio.unix: isatty, STDOUT_FILENO
load std.io: println

if isatty(STDOUT_FILENO) {
    println("Вывод направлен в интерактивный терминал")
}
```

### 4.2 pipe

Создает однонаправленный канал данных (анонимный конвейер) в пространстве ядра операционной системы.

```yukino
public fun pipe() -> PipeFds
```

Возвращает структуру `PipeFds` с парой дескрипторов `read_fd` (чтение) и `write_fd` (запись).

В случае ошибки системного вызова вызывает панику со структурой `IoError` (`code = 201`).

```yukino
load std.platformio.unix: pipe, PipeFds
load std.io: println

set p = pipe()
println("Канал pipe открыт: чтение ${p.read_fd}, запись ${p.write_fd}")
```

### 4.3 dup

Создает копию дескриптора `oldfd`, выделяя наименьший доступный номер свободного дескриптора.

```yukino
public fun dup(oldfd: int32) -> int32
```

Возвращает новый файловый дескриптор как `int32`.

В случае ошибки дублирования вызывает панику со структурой `IoError` (`code = 202`).

```yukino
load std.platformio.unix: dup, STDOUT_FILENO
load std.io: println

set copy_fd = dup(STDOUT_FILENO)
println("Дубликат stdout дескриптора: ${copy_fd}")
```

### 4.4 dup2

Атомарно перенаправляет дескриптор `newfd` на тот же открытый файл, что и `oldfd`.

```yukino
public fun dup2(oldfd: int32, newfd: int32) -> int32
```

Возвращает дескриптор `newfd`. Если `newfd` был предварительно открыт, он безопасно закрывается перед перенаправлением.

В случае системного сбоя вызывает панику со структурой `IoError` (`code = 203`).

```yukino
load std.platformio.unix: dup2, STDOUT_FILENO
load std.io: println

set res = dup2(3, STDOUT_FILENO)
println("Дескриптор перенаправлен: ${res}")
```

### 4.5 fsync

Принудительно синхронизирует все модифицированные буферы файла (как данные, так и метаданные) дескриптора `fd` с постоянным физическим носителем.

```yukino
public fun fsync(fd: int32) -> int32
```

Возвращает `0` при успешной синхронизации.

В случае ошибки сброса буферов на физический диск вызывает панику со структурой `IoError` (`code = 204`).

```yukino
load std.platformio.unix: fsync
load std.io: println

set status = fsync(1)
println("Сброс fsync завершен со статусом: ${status}")
```

### 4.6 fdatasync

Синхронизирует данные дескриптора `fd` на физический диск без обязательного обновления неизмененных метаданных (например, временных меток доступа).

```yukino
public fun fdatasync(fd: int32) -> int32
```

Возвращает `0` при успешной синхронизации данных.

В случае ошибки записи данных на диск вызывает панику со структурой `IoError` (`code = 205`).

```yukino
load std.platformio.unix: fdatasync
load std.io: println

set status = fdatasync(1)
println("Сброс fdatasync завершен со статусом: ${status}")
```

---

## 5. Справочник функций: Linux

### 5.1 pipe2

Создает конвейер с атомарной установкой системных флагов ядра Linux (например, `O_CLOEXEC` или `O_NONBLOCK`).

```yukino
public fun pipe2(flags: int32) -> PipeFds
```

Возвращает структуру `PipeFds` с парой файловых дескрипторов `read_fd` и `write_fd`.

В случае ошибки создания канала вызывает панику со структурой `IoError` (`code = 206`).

```yukino
load std.platformio.linux: pipe2, PipeFds
load std.io: println

set flags: int32 = 0x80000 // O_CLOEXEC
set p = pipe2(flags)
println("Конвейер pipe2 создан")
```

### 5.2 dup3

Дублирует дескриптор `oldfd` в `newfd` с атомарным выставлением системных флагов Linux (например, `O_CLOEXEC`).

```yukino
public fun dup3(oldfd: int32, newfd: int32, flags: int32) -> int32
```

Возвращает новый дескриптор `newfd`.

В случае сбоя вызывает панику со структурой `IoError` (`code = 207`).

```yukino
load std.platformio.linux: dup3
load std.io: println

set res = dup3(3, 4, 0x80000)
println("Дескриптор dup3: ${res}")
```

### 5.3 splice

Перемещает до `len` байтов данных между двумя файловыми дескрипторами (один из которых обязан быть каналом pipe) без копирования данных в пространство пользователя (zero-copy в ядре Linux).

```yukino
public fun splice(fd_in: int32, fd_out: int32, len: int64, flags: uint32) -> int64
```

Возвращает фактическое число перемещенных байт как `int64`.

В случае системной ошибки перемещения вызывает панику со структурой `IoError` (`code = 208`).

```yukino
load std.platformio.linux: splice
load std.io: println

set transferred = splice(0, 1, 4096, 0u32)
println("Передано байт через splice: ${transferred}")
```

### 5.4 tee

Дублирует до `len` байтов данных из одного канала `fd_in` в другой канал `fd_out` без извлечения данных из исходного канала и без копирования в память пользователя.

```yukino
public fun tee(fd_in: int32, fd_out: int32, len: int64, flags: uint32) -> int64
```

Возвращает количество успешно сдублированных байт как `int64`.

В случае ошибки системного вызова вызывает панику со структурой `IoError` (`code = 209`).

```yukino
load std.platformio.linux: tee
load std.io: println

set count = tee(3, 4, 1024, 0u32)
println("Сдублировано байт через tee: ${count}")
```

---

## 6. Справочник функций: Windows (Win32)

### 6.1 get_std_handle

Обертка над Win32 вызовом `GetStdHandle`. Получает дескриптор устройства ввода, вывода или вывода ошибок.

```yukino
public fun get_std_handle(n: int32) -> int64
```

Возвращает 64-битный дескриптор `HANDLE` или `-1` (`INVALID_HANDLE_VALUE`) при отсутствии.

Функция никогда не паникует.

```yukino
load std.platformio.windows: get_std_handle, STD_OUTPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_OUTPUT_HANDLE)
println("Хэндл stdout: ${h}")
```

### 6.2 is_console

Проверяет, указывает ли системный дескриптор `handle` на активный буфер экрана или буфер ввода консоли Windows.

```yukino
public fun is_console(handle: int64) -> bool
```

Возвращает `true`, если дескриптор связан с консолью Windows, иначе `false`.

Функция никогда не паникует.

```yukino
load std.platformio.windows: get_std_handle, is_console, STD_OUTPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_OUTPUT_HANDLE)
if is_console(h) {
    println("Вывод в консоль Windows")
}
```

### 6.3 get_console_mode

Запрашивает текущую битовую маску флагов режима консоли Windows (`GetConsoleMode`).

```yukino
public fun get_console_mode(handle: int64) -> int32
```

Возвращает 32-битную маску флагов режима консоли или `-1` при ошибке.

Функция никогда не паникует.

```yukino
load std.platformio.windows: get_std_handle, get_console_mode, STD_INPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_INPUT_HANDLE)
set mode = get_console_mode(h)
println("Режим консоли Windows: ${mode}")
```

### 6.4 set_console_mode

Устанавливает новую битовую маску режима консоли Windows (`SetConsoleMode`) для указанного дескриптора.

```yukino
public fun set_console_mode(handle: int64, mode: int32) -> bool
```

Возвращает `true`, если операция завершилась успешно, иначе `false`.

Функция никогда не паникует.

```yukino
load std.platformio.windows: get_std_handle, set_console_mode, STD_INPUT_HANDLE
load std.io: println

set h = get_std_handle(STD_INPUT_HANDLE)
set ok = set_console_mode(h, 7)
println("Режим обновлен: ${ok}")
```

### 6.5 create_pipe

Создает анонимный канал (конвейер) в Windows через системный вызов `CreatePipe`.

```yukino
public fun create_pipe() -> WinPipeHandles
```

Возвращает структуру `WinPipeHandles` с дескрипторами `read_handle` и `write_handle`.

В случае ошибки создания канала операционной системой вызывает панику со структурой `IoError` (`code = 210`).

```yukino
load std.platformio.windows: create_pipe, WinPipeHandles
load std.io: println

set p = create_pipe()
println("Хэндл чтения: ${p.read_handle}, записи: ${p.write_handle}")
```

---

[← Назад: Модуль std.threads](3-threads.md) | [Вперед: Модуль std.math →](5-math.md)
