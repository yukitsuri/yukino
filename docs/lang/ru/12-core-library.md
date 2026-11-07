# Библиотека ядра: core (Core Library)

Пакет `core` — это фундаментальная низкоуровневая библиотека языка Yukino. Она содержит примитивы среды выполнения, функции преобразования типов, управления памятью, обработки паник, системных процессов и встроенных макросов компилятора.

Пакет `core` автоматически подключается компилятором через секцию манифеста `includes = ["core", "stdlib"]`, а его сущности импортируются стандартной директивой `load`.

---

## Оглавление

1. [Архитектура и структура пакета core](#1-архитектура-и-структура-пакета-core)
2. [Модуль core.utils: приведение типов и утверждения](#2-модуль-coreutils-приведение-типов-и-утверждения)  
   2.1 [Приведение типов: cast](#21-приведение-типов-cast)  
   2.2 [Матрица допустимых преобразований cast](#22-матрица-допустимых-преобразований-cast)  
   2.3 [Утверждения: assert, assert_eq, assert_ne](#23-утверждения-assert-assert_eq-assert_ne)
3. [Модуль core.panic: обработка аварийных остановок](#3-модуль-corepanic-обработка-аварийных-остановок)
4. [Модуль core.process: управление процессом и аргументы](#4-модуль-coreprocess-управление-процессом-и-аргументы)  
   4.1 [Завершение процесса: exit](#41-завершение-процесса-exit)  
   4.2 [Аргументы командной строки: args и raw_args](#42-аргументы-командной-строки-args-и-raw_args)
5. [Модуль core.mem: низкоуровневая память и арены](#5-модуль-coremem-низкоуровневая-память-и-арены)  
   5.1 [Оператор адреса: ptr_of](#51-оператор-адреса-ptr_of)  
   5.2 [Конструирование из сырых частей: from_raw_parts](#52-конструирование-из-сырых-частей-from_raw_parts)  
   5.3 [Интроспекция региональной памяти: region_used, region_cap, region_reset](#53-интроспекция-региональной-памяти-region_used-region_cap-region_reset)  
   5.4 [Экспорт и долгоживущие объекты: export, persistent](#54-экспорт-и-долгоживущие-объекты-export-persistent)  
   5.5 [Класс первоклассных арен: class Arena](#55-класс-первоклассных-арен-class-arena)
6. [Дескрипторы коллекций: core.slices, core.strings, core.bytes](#6-дескрипторы-коллекций-coreslices-corestrings-corebytes)  
   6.1 [Структура Slice и методы срезов](#61-структура-slice-и-методы-срезов)  
   6.2 [Структура String и неизменяемость строк](#62-структура-string-и-неизменяемость-строк)  
   6.3 [Структура Bytes](#63-структура-bytes)
7. [Модуль встроенных макросов: core.macro](#7-модуль-встроенных-макросов-coremacro)  
   7.1 [Интроспекция типов: typeof!](#71-интроспекция-типов-typeof)  
   7.2 [Макросы исходного кода: file!, line!, col!](#72-макросы-исходного-кода-file-line-col)  
   7.3 [Макросы компиляции и условной сборки: compile_print!, compile_error!, cfg!](#73-макросы-компиляции-и-условной-сборки-compile_print-compile_error-cfg)  
   7.4 [Макросы внедрения ресурсов: include_str!, include_bytes!](#74-макросы-внедрения-ресурсов-include_str-include_bytes)  
   7.5 [Макросы метаданных окружения: hostenv!, yukienv!, is_lib_connected!, get_features!, get_profileconfig!, get_target!, get_yukic_version!](#75-макросы-метаданных-окружения)
8. [Сводная таблица функций пакета core](#8-сводная-таблица-функций-пакета-core)

---

## 1. Архитектура и структура пакета core

Пакет `core` разбит на специализированные модули с четким разграничением ответственности:

| Модуль | Экспортируемые сущности | Назначение |
| :--- | :--- | :--- |
| `core.utils` | `cast`, `assert`, `assert_eq`, `assert_ne` | Явное приведение типов и базовые проверки инвариантов |
| `core.panic` | `panic` | Вызов типизированных паник и аварийная остановка программы |
| `core.process` | `exit`, `args`, `raw_args` | Код возврата процесса и аргументы командной строки CLI |
| `core.mem` | `ptr_of`, `from_raw_parts`, `region_used`, `region_cap`, `region_reset`, `export`, `persistent`, `Arena` | Сырые указатели, ручные арены памяти и региональная интроспекция |
| `core.slices` | `Slice` | Базовое представление срезов `T[]` |
| `core.strings` | `String` | Базовое представление неизменяемых строк `string` |
| `core.bytes` | `Bytes` | Базовое представление буферов байт `bytes` |
| `core.macro` | `typeof`, `file`, `line`, `col`, `cfg`, `compile_print`, `compile_error`, `include_str`, `include_bytes`, `hostenv`, `yukienv`, `is_lib_connected`, `get_features`, `get_profileconfig`, `get_target`, `get_yukic_version` | Встроенные макросы раскрытия на этапе компиляции |

---

## 2. Модуль core.utils: приведение типов и утверждения

Модуль подключается через:
```yukino
load core.utils: cast, assert, assert_eq, assert_ne
```

### 2.1 Приведение типов: cast

В Yukino строгая типизация и отсутствуют неявные преобразования типов: числа автоматически не расширяются и не сужаются, а указатели не приводятся к адресам. Любое явное преобразование выполняется через функцию `cast`:

```yukino
cast(value: T, target: "target_type" | typeof!(expr)) -> U
```

- **Параметр 1 (`value`):** исходное значение любого допустимого типа `T`.
- **Параметр 2 (`target`):** строковый литерал с целевым типом (например, `"int32"`, `"float64"`, `"string"`, `"ptr<char>"`) либо вызов `typeof!(expr)`.
- **Возвращаемое значение:** значение целевого типа `U`.

Пример использования:
```yukino
load core.utils: cast

set count: int64 = 42
set small: int32 = cast(count, "int32")
set str_val: string = cast(small, "string")
```

### 2.2 Матрица допустимых преобразований cast

| Исходный тип `T` | Целевой тип `U` | Поведение приведения |
| :--- | :--- | :--- |
| `int*` / `uint*` | `int*` / `uint*` | Беззнаковое/знаковое усечение (trunc) или расширение (zext/sext) |
| `int*` / `uint*` | `float*` | Точное или округленное преобразование числа в представление с плавающей точкой |
| `float*` | `int*` / `uint*` | Усечение дробной части к нулю |
| `char` | `int*` / `uint*` | Получение 32-битного Unicode кодового значения (code point) |
| `int*` / `uint*` | `char` | Преобразование кодовой точки в `char` (при некорректном code point — замена на U+FFFD) |
| `int*` / `uint*` | `string` | Десятичное строковое форматирование в текущий регион |
| `float*` | `string` | Форматирование числа с плавающей точкой в строку |
| `bool` | `string` | Возвращает строковый литерал `"true"` или `"false"` |
| `string` | `int*` / `uint*` | Парсинг строки; при некорректном формате возвращает 0 |
| `string` | `float*` | Парсинг строки в число с плавающей точкой |
| `string` | `bool` | Строка `"true"` преобразуется в `true`, все остальные — в `false` |
| `bool` | `int*` / `uint*` | `true` -> 1, `false` -> 0 |
| `int*` / `uint*` | `bool` | `0` -> `false`, любое ненулевое число -> `true` |
| `ptr<T>` | `ptr<U>` | Побитовое переинтерпретирование адреса указателя |
| `ptr<T>` | `int64` / `uint64` | Преобразование адреса в целое число машинной разрядности |
| `cfun(...)` | `ptr<void>` | Преобразование указателя на функцию C ABI в нетипизированный указатель |
| `bytes` | `string` | Валидация UTF-8; создание строки из среза байтов |
| `string` | `bytes` | Создание среза байт из строки без копирования буфера |

### 2.3 Утверждения: assert, assert_eq, assert_ne

Функции утверждений проверяют инварианты программы в рантайме:

```yukino
assert(condition: bool, message?: string) -> void
assert_eq<T>(left: T, right: T, message?: string) -> void
assert_ne<T>(left: T, right: T, message?: string) -> void
```

Если условие ложно, функция вызывает аварийный останов с выводом переданного сообщения.

```yukino
load core.utils: assert, assert_eq

fun test_math() {
    assert(2 + 2 == 4, "math is broken")
    assert_eq(10 * 10, 100, "multiplication failed")
}
```

---

## 3. Модуль core.panic: обработка аварийных остановок

Модуль подключается через:
```yukino
load core.panic: panic
```

### 3.1 Функция panic

Функция `panic` инициирует контролируемый сбой текущего потока:

```yukino
panic(payload: any) -> void
```

- Принимает структуру ошибки любого типа, строку или примитивное значение.
- Инициирует раскрутку стека (`unwind`), которая может быть перехвачена конструкцией `sus { ... } catch e { ... }`.
- Если паника не перехвачена, процесс выводит диагностический трейс и завершается с ненулевым статусом.

Пример:
```yukino
load core.panic: panic

struct DatabaseError {
    code: int32
    message: string
}

fun query_db(table: string) {
    if table.len() == 0 {
        panic(DatabaseError {
            code: 400,
            message: "table name cannot be empty",
        })
    }
}
```

---

## 4. Модуль core.process: управление процессом и аргументы

Модуль подключается через:
```yukino
load core.process: exit, args, raw_args
```

### 4.1 Завершение процесса: exit

```yukino
exit(code: int32) -> void
```

Мгновенно завершает выполнение программы и возвращает операционной системе указанный код выхода. Перед завершением вызываются зарегистрированные хуки `defer` в текущем контексте.

```yukino
load core.process: exit

fun main() {
    exit(0)
}
```

### 4.2 Аргументы командной строки: args и raw_args

```yukino
args() -> string[]
raw_args() -> string
```

- `args()`: возвращает массив строк, представляющий отдельные аргументы командной строки (нулевой аргумент содержит путь к исполняемому файлу программы).
- `raw_args()`: возвращает объединенную строку аргументов, переданных процессу операционной системой.

Пример:
```yukino
load core.process: args

fun main() {
    set argv = args()
    set i = 0
    while i < argv.len() {
        println("arg[${i}]: ${argv[i]}")
        rewrite i = i + 1
    }
}
```

---

## 5. Модуль core.mem: низкоуровневая память и арены

Модуль подключается через:
```yukino
load core.mem: ptr_of, from_raw_parts, region_used, region_cap, region_reset, export, persistent, Arena
```

### 5.1 Оператор адреса: ptr_of

```yukino
ptr_of<T>(variable: T) -> ptr<T>
```

Возвращает сырой указатель на ячейку памяти переданной переменной. Используется преимущественно при взаимодействии с Си-библиотеками (FFI) и низкоуровневыми структурами.

### 5.2 Конструирование из сырых частей: from_raw_parts

```yukino
from_raw_parts<T>(ptr: ptr<T>, len: int64, cap: int64) -> T[]
```

Конструирует срез типа `T[]` из сырого указателя `ptr`, заданной длины `len` и емкости `cap`. Не выделяет новой памяти и не копирует элементы.

### 5.3 Интроспекция региональной памяти: region_used, region_cap, region_reset

Функции позволяют контролировать параметры локального региона функции:

- `region_used() -> int64`: объем памяти в байтах, занятый объектами в текущем регионе.
- `region_cap() -> int64`: суммарная емкость текущих системных чанков региона.
- `region_reset() -> void`: мгновенный откат указателя bump-аллокации локального региона в O(1).

```yukino
load core.mem: region_used, region_cap, region_reset

fun heavy_loop() {
    set i = 0
    while i < 1000 {
        set temp = "iteration data " + cast(i, "string")
        region_reset()
        rewrite i = i + 1
    }
}
```

### 5.4 Экспорт и долгоживущие объекты: export, persistent

- `export<T>(val: T) -> T`: перемещает значение `val` из вложенного региона в родительский регион вызывающего кода.
- `persistent<T>(val: T) -> T`: перемещает значение `val` в глобальный долгоживущий регион процесса, переживающий вызовы функций и потоков.

### 5.5 Класс первоклассных арен: class Arena

Класс `Arena` предоставляет независимый пул памяти с ручным контролем жизненного цикла:

```yukino
load core.mem: Arena

class Arena {
    public fun init(cap: int64)
    public fun cap() -> int64
    public fun used() -> int64
    public fun reset() -> void
    public fun destroy() -> void
    public fun as_ptr() -> ptr<void>
}
```

Пример создания и использования арены:
```yukino
load core.mem: Arena

fun run_tasks() {
    set pool = init Arena(65536)
    defer {
        pool.destroy()
    }

    region(pool) {
        set packet = "network payload"
        println("Used in pool: ${pool.used()} / ${pool.cap()}")
    }

    pool.reset()
}
```

---

## 6. Дескрипторы коллекций: core.slices, core.strings, core.bytes

Модули описывают физическое представление коллекций в памяти:

### 6.1 Структура Slice и методы срезов

Срез `T[]` представляется структурой `Slice`:
```yukino
struct Slice {
    ptr: ptr<void>
    len: int64
    cap: int64
}
```

Для срезов доступны встроенные методы:
- `.len() -> int64`: текущее количество элементов.
- `.cap() -> int64`: текущая вместимость буфера.
- `.as_ptr() -> ptr<T>`: сырой указатель на первый элемент.
- `.cp() -> T[]`: поверхностная копия среза.
- `.push(item: T) -> void`: добавление элемента в конец среза.
- `.pop() -> T?`: извлечение последнего элемента.
- `.del(index: int64) -> void`: удаление элемента по индексу со сдвигом.
- `.clear() -> void`: обнуление длины среза.
- `.reserve(additional: int64) -> void`: предварительное резервирование емкости.

### 6.2 Структура String и неизменяемость строк

Строка `string` в Yukino является строго **неизменяемой** (immutable):
```yukino
struct String {
    ptr: ptr<uint8>
    len: int64
}
```

Строки поддерживают методы чтения: `.len()`, `.as_ptr()`, `.cp()`. Любые мутирующие методы (`.push`, `.del`, `.clear`) на строках **запрещены компилятором** на этапе типизации. Для изменяемых текстовых буферов используется тип `bytes` или модуль `std.strings`.

### 6.3 Структура Bytes

Буфер байтов `bytes` представляется структурой `Bytes`:
```yukino
struct Bytes {
    ptr: ptr<uint8>
    len: int64
    cap: int64
}
```

---

## 7. Модуль встроенных макросов: core.macro

Модуль предоставляет встроенные макросы раскрытия, вычисляемые компилятором во время сборки.

### 7.1 Интроспекция типов: typeof!

```yukino
typeof!(expr) -> string
```

Определяет статический тип выражения во время компиляции и подставляет дедуплицированный строковый литерал из секции `rodata`:

```yukino
set id = 100
set t = typeof!(id) // "int32"
```

### 7.2 Макросы исходного кода: file!, line!, col!

- `file!() -> string`: возвращает имя текущего исходного файла.
- `line!() -> int32`: возвращает номер текущей строки кода.
- `col!() -> int32`: возвращает номер текущей колонки кода.

### 7.3 Макросы компиляции и условной сборки: compile_print!, compile_error!, cfg!

- `compile_print!(msg)`: выводит сообщение в терминал во время компиляции.
- `compile_error!(msg)`: прерывает компиляцию с диагностической ошибкой.
- `cfg!(predicate) -> bool`: возвращает `true`, если целевая платформа или флаг удовлетворяет условию (например, `cfg!(target_os = "linux")`).

### 7.4 Макросы внедрения ресурсов: include_str!, include_bytes!

- `include_str!(path) -> string`: встраивает содержимое текстового файла UTF-8 как константную строку `string`.
- `include_bytes!(path) -> bytes`: встраивает содержимое бинарного файла как срез байт `bytes`.

### 7.5 Макросы метаданных окружения

- `hostenv!(var) -> string`: значение переменной окружения сборочного хоста.
- `yukienv!(key, scope) -> string`: чтение настроек манифеста проекта.
- `is_lib_connected!(lib, scope) -> bool`: проверка подключения библиотеки к проекту.
- `get_features!() -> string[]`: список активированных фич сборки.
- `get_profileconfig!() -> string`: текущий профиль сборки (`"debug"` или `"release"`).
- `get_target!() -> string`: целевой триплет сборки (например, `"x86_64-unknown-linux-gnu"`).
- `get_yukic_version!() -> string`: версия компилятора `yukic`.

---

## 8. Сводная таблица функций пакета core

| Модуль | Имя сущности | Сигнатура | Возвращаемое значение | Паника / Ошибка |
| :--- | :--- | :--- | :--- | :--- |
| `core.utils` | `cast` | `cast(val, "type")` | Значение приведенного типа | Нет (возвращает null/дефолт при ошибке парсинга) |
| `core.utils` | `assert` | `assert(cond, msg?)` | `void` | Паника при ложном условии |
| `core.utils` | `assert_eq` | `assert_eq(a, b, msg?)` | `void` | Паника при неравенстве |
| `core.utils` | `assert_ne` | `assert_ne(a, b, msg?)` | `void` | Паника при равенстве |
| `core.panic` | `panic` | `panic(payload)` | `void` | Инициирует контролируемый сбой / раскрутку стека |
| `core.process` | `exit` | `exit(code)` | `void` | Немедленный выход из процесса |
| `core.process` | `args` | `args()` | `string[]` | Список аргументов командной строки |
| `core.process` | `raw_args` | `raw_args()` | `string` | Необработанная строка аргументов |
| `core.mem` | `ptr_of` | `ptr_of(variable)` | `ptr<T>` | Сырой адрес ячейки памяти переменной |
| `core.mem` | `from_raw_parts` | `from_raw_parts(p, len, cap)` | `T[]` | Срез из сырого буфера |
| `core.mem` | `region_used` | `region_used()` | `int64` | Количество занятых байт в регионе |
| `core.mem` | `region_cap` | `region_cap()` | `int64` | Емкость системных чанков региона |
| `core.mem` | `region_reset` | `region_reset()` | `void` | Сброс указателя региона за O(1) |
| `core.mem` | `export` | `export(val)` | `T` | Перемещение в родительский регион |
| `core.mem` | `persistent` | `persistent(val)` | `T` | Перемещение в процессный регион |
| `core.mem` | `Arena` | `class Arena` | Экземпляр арены | Методы `.cap()`, `.used()`, `.reset()`, `.destroy()`, `.as_ptr()` |

---

[← Назад: C ABI и FFI](11-cabi.md) | [Вперед: Атрибуты компилятора →](13-attributes.md)
