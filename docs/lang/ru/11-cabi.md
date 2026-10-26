# Системный C ABI и C FFI

Yukino спроектирован для бесшовного взаимодействия с языками C, C++, Rust и системными библиотеками операционной системы. Язык поддерживает прямое объявление внешних C-функций, C-структуры с контролируемой раскладкой памяти, кастомные соглашения о вызовах и экспорт функций по стандарту C ABI.

---

## Оглавление

1. [Внешние C-декларации и блок externc](#1-внешние-c-декларации-и-блок-externc)  
   1.1 [Одиночные декларации на верхнем уровне (без externc)](#11-одиночные-декларации-на-верхнем-уровне-без-externc)  
   1.2 [Группирующий блок externc](#12-группирующий-блок-externc)  
   1.3 [Объявление C-функций: cfun](#13-объявление-c-функций-cfun)  
   1.4 [Синтаксис C-атрибутов: \|attr1, attr2\|](#14-синтаксис-c-атрибутов-attr1-attr2)  
   1.5 [Соглашения о вызовах (Calling Conventions)](#15-соглашения-о-вызовах-calling-conventions)  
   1.6 [C-атрибуты линковки и оптимизации](#16-c-атрибуты-линковки-и-оптимизации)  
   1.7 [Синтаксис завершения: точка с запятой (;)](#17-синтаксис-завершения-точка-с-запятой-)
2. [C-структуры: cstruct и раскладка памяти](#2-c-структуры-cstruct-и-раскладка-памяти)  
   2.1 [Объявление и доступ к полям](#21-объявление-и-доступ-к-полям)  
   2.2 [Атрибуты packed и align](#22-атрибуты-packed-и-align)
3. [Экспорт функций Yukino в C ABI](#3-экспорт-функций-yukino-в-c-abi)
4. [Сырые указатели: ptr\<T\> и оператор ptr_of](#4-сырые-указатели-ptrt-и-оператор-ptr_of)
5. [Интеграция C-файлов в процесс сборки](#5-интеграция-c-файлов-в-процесс-сборки)
6. [Комплексный пример: разные типы в C и Yukino](#6-комплексный-пример-разные-типы-в-c-и-yukino)

---

## 1. Внешние C-декларации и блок externc

Для импорта функций и структур из внешних объектных файлов (`.o`), статических библиотек (`.a`), разделяемых библиотек (`.so`/`.dll`) или системного рантайма C в Yukino используются ключевые слова `cfun` и `cstruct`.

> [!NOTE]
> Блок `externc { ... }` **не является обязательным**. Функции `cfun` и структуры `cstruct` являются полноправными элементами модуля верхнего уровня и могут объявляться как обособленно, так и группами внутри блока `externc`.

> [!TIP]
> **Вызов через пространство имен `extern::`:**
> Объявленные C-функции можно вызывать как напрямую по имени (`puts(ptr)`), так и с префиксом пространства имен `extern::` (`extern::puts(ptr)`). Использование `extern::` гарантирует вызов именно C ABI функции и предотвращает коллизии, если в проекте объявлена одноименная функция Yukino.

### 1.1 Одиночные декларации на верхнем уровне (без externc)

Если вам требуется импортировать одну или две внешние функции либо определить C-совместимую структуру, их можно объявить прямо в глобальной области видимости файла:

```yukino
// Декларация C-функции без externc:
|cdecl|
cfun puts(s: ptr<c_char>) -> c_int;

|cdecl|
cfun abs(x: int32) -> int32;

|noreturn, cold|
cfun exit(code: int32) -> void;

// Декларация C-структуры на верхнем уровне:
|packed|
cstruct PacketHeader {
    magic: uint16,
    length: uint32,
}
```

### 1.2 Группирующий блок externc

Блок `externc` удобен для логической организации множества внешних объявлений, а также для явного связывания с C-заголовком при интеграции:

```yukino
externc {
    // Импорт функций стандартной библиотеки Libc
    |cdecl|
    cfun free(p: ptr<void>) -> void;

    |cdecl, returns_nonnull|
    cfun malloc(size: int64) -> ptr<void>;
}

// Указание имени заголовочного файла (для метаинформации и C-трансляторов):
externc "math.h" {
    cfun sin(x: float64) -> float64;
    cfun cos(x: float64) -> float64;
}
```

### 1.3 Объявление C-функций: cfun

В отличие от стандартных функций `fun`, для C-функций используется ключевое слово `cfun`. Это сигнализирует компилятору об использовании системного C ABI без внутреннего манглинга Yukino.

Для типов параметров и возвращаемых значений используются как стандартные типы Yukino (`int32`, `int64`, `float64`), так и платформенные C-типы:
- `c_char`, `c_schar`, `c_uchar`
- `c_short`, `c_ushort`
- `c_int`, `c_uint`
- `c_long`, `c_ulong`, `c_longlong`, `c_ulonglong`
- `c_size_t`, `c_ssize_t`
- `ptr<void>`, `ptr<c_char>`

### 1.4 Синтаксис C-атрибутов: |attr1, attr2|

C-атрибуты заключаются в вертикальные черты без лишних пробелов: `|attr1, attr2="value"|`. Внутри одной пары черт можно перечислять несколько атрибутов через запятую:

```yukino
// Один атрибут
|cdecl|
cfun basic_func() -> void;

// Несколько атрибутов через запятую
|cold, inline_never|
cfun slow_error_handler(code: int32) -> void;

// Атрибут со строковым значением
|section="hot_code", inline_always|
cfun fast_worker(data: int32) -> int32;
```

### 1.5 Соглашения о вызовах (Calling Conventions)

Yukino поддерживает все стандартные соглашения о вызовах LLVM:

| Атрибут | Название | Назначение |
| :--- | :--- | :--- |
| `\|cdecl\|` | C Declaration (Default) | Стандартное соглашение о вызовах платформы C |
| `\|stdcall\|` | Standard Call | Win32 API функции (очистка стека вызываемым) |
| `\|fastcall\|` | Fast Call | Передача первых аргументов через регистры процессора |
| `\|thiscall\|` | This Call | Вызов методов C++ классов на x86 |
| `\|sysv64\|` | System V AMD64 ABI | Явное соглашение POSIX/Linux x86_64 |
| `\|win64\|` | Microsoft x64 ABI | Явное соглашение Windows x86_64 |

Примеры практического применения:

```yukino
externc {
    // Вызов системной функции WinAPI
    |stdcall|
    cfun GetSystemMetrics(n_index: int32) -> int32;

    // Высокопроизводительный внутренний C-воркер
    |fastcall|
    cfun process_chunk(src: ptr<void>, len: int64) -> int32;
}
```

### 1.6 C-атрибуты линковки и оптимизации

| C-атрибут | LLVM эквивалент | Назначение |
| :--- | :--- | :--- |
| `\|weak\|` | `Linkage::ExternalWeak` | Слабая линковка (символ может отсутствовать при компоновке) |
| `\|noreturn\|` | `Attribute::NoReturn` | Функция никогда не возвращает управление (`exit`, `abort`) |
| `\|cold\|` | `Attribute::Cold` | Редко исполняемый код (выносится из горячего кэша инструкций) |
| `\|hot\|` | `Attribute::Hot` | Критическая по скорости функция (приоритет инлайнинга) |
| `\|inline_always\|` | `Attribute::AlwaysInline` | Принудительное встраивание тела функции |
| `\|inline_never\|` | `Attribute::NoInline` | Запрет инлайнинга функции |
| `\|returns_nonnull\|` | `Attribute::NonNull` | Возвращаемый указатель гарантированно не равен `NULL` |
| `\|section="name"\|` | `.text.name` | Размещение функции в специализированной секции ELF/PE |

Практические комбинации:

```yukino
externc {
    // 1. Аварийная остановка системы (холодный путь, не возвращает управление)
    |noreturn, cold|
    cfun fatal_abort(msg: ptr<c_char>) -> void;

    // 2. Системный аллокатор памяти (всегда возвращает ненулевой указатель)
    |cdecl, returns_nonnull|
    cfun custom_malloc(size: int64) -> ptr<void>;

    // 3. Опциональный плагин (слабая линковка, может быть переопределен)
    |weak, cdecl|
    cfun optional_plugin_hook() -> int32;
}
```

### 1.7 Синтаксис завершения: точка с запятой (;)

Сигнатуры `cfun` без тела — это декларации, и их принято завершать точкой с запятой `;`:

```yukino
// Корректно:
cfun exit(code: int32) -> void;

// Точка с запятой не обязательна — это тоже корректный синтаксис
// (следующая декларация или `}` блока завершают текущую):
cfun abs(x: int32) -> int32
```

Парсер одинаково принимает обе формы как на верхнем уровне, так и внутри блока `externc`; точка с запятой служит визуальным разделителем списка деклараций.

---

## 2. C-структуры: cstruct и раскладка памяти

В отличие от стандартных структур Yukino `struct`, для C-структур используется ключевое слово `cstruct`. Они следуют размерности, смещениям полей и ABI целевой платформы: поля раскладываются по естественному выравниванию типов, а атрибут `packed` отключает паддинг.

### 2.1 Объявление и доступ к полям

```yukino
externc {
    cstruct NativePoint {
        x: int32,
        y: int32,
    }

    cstruct TimeSpec {
        tv_sec: int64,
        tv_nsec: int64,
    }
}

fun inspect_point() {
    set pt = NativePoint { x: 10, y: 20 }
    
    // При чтении полей cstruct компилятор требует явной аннотации типа:
    set pt_x: int32 = pt.x
    set pt_y: int32 = pt.y
}
```

### 2.2 Атрибуты packed и align

Для работы с сетевыми протоколами, аппаратными заголовками или структурами с нестандартным выравниванием используются атрибуты `packed` и `align="N"` (или `align=N`):

```yukino
externc {
    // Упакованный сетевой заголовок без padding-байтов
    |packed|
    cstruct EthernetHeader {
        dst_mac: uint64,
        src_mac: uint64,
        ethertype: uint16,
    }

    // Буфер, выровненный по границе 16 байт (например, для SSE/AVX или DMA)
    |align="16"|
    cstruct AlignedBuffer {
        data: uint32,
    }

    // Комбинация packed и явного выравнивания
    |packed, align="4"|
    cstruct PackedAligned {
        tag: uint8,
        payload: uint16,
    }
}
```

Без `packed` поля раскладываются по естественному выравниванию типов, как в C.

Атрибут `align="N"` (или `align=N`) задает явное машинное выравнивание структуры в байтах (например, `|align="16"|` или `|align=8|`). При этом общий размер структуры дополняется хвостовыми padding-байтами до значения, кратного выравниванию, а переменные структуры и операции выделения памяти на стеке размещаются с соблюдением указанного выравнивания. Атрибут можно комбинировать с `packed` (например, `|packed, align="4"|`).

---

## 3. Экспорт функций Yukino в C ABI

Yukino позволяет писать библиотеки, которые вызываются из языков C, C++, Python (`ctypes`), Rust или C# (`P/Invoke`). Для этого функция объявляется с ключевым словом `cfun`.

> [!IMPORTANT]
> На верхнем уровне модуля C-атрибуты `|...|` **всегда ставятся перед ключевым словом `public`**:

```yukino
// 1. Экспорт функции с соглашением cdecl и хинтом hot
|cdecl, hot|
public cfun yukino_add(a: int32, b: int32) -> int32 {
    return a + b
}

struct HostExitError {
    message: string
    code: int32
}

// 2. Экспорт функции аварийного выхода с хинтами noreturn и cold
|cold, noreturn|
public cfun yukino_panic_exit(code: int32) -> void {
    panic(HostExitError {
        message: "fatal exit requested by host",
        code: code,
    })
}
```

Скомпилированная библиотека экспортирует эти функции с чистыми C-именами (unmangled symbols).

---

## 4. Сырые указатели: ptr\<T\> и оператор ptr_of

Yukino поддерживает сырые типизированные указатели `ptr<T>`:

- `ptr<c_char>`: сырой указатель на C-строку.
- `ptr<void>`: универсальный opaque-указатель (`void*`).
- `ptr<int32>`: указатель на конкретный тип данных.

### Оператор ptr_of

Оператор `ptr_of(variable)` возвращает сырой указатель на локальную переменную:

```yukino
set val = 123
set raw_address = ptr_of(val) // ptr<int64>: целочисленный литерал по умолчанию имеет тип int64
```

### Передача строк в C-функции

`cast` между `string` и `ptr<T>` **не является конвертацией C-строк**: он разбирает текст строки как шестнадцатеричный адрес (или форматирует указатель в строку `0x...`). Для передачи строк в C-функции используйте метод `.as_ptr()` — буферы строковых литералов завершаются нулевым байтом:

```yukino
externc {
    |cdecl|
    cfun write(fd: int32, buf: ptr<uint8>, count: int64) -> int64;
}

fun main() {
    set greeting = "Hello, C!\n"
    write(1, greeting.as_ptr(), greeting.len())
}
```

При необходимости `ptr<uint8>` переименовывается в `ptr<char>` через `cast` (преобразование между типами указателей не генерирует кода). Подробности — в разделе [5 документа о строках](8-strings.md#5-интероп-со-строками-языка-c).

---

## 5. Интеграция C-файлов в процесс сборки

Для сборки проектов, содержащих как код на Yukino, так и на C, используется файл `yuki.toml`:

### Шаг 1: Компиляция C-кода

Скомпилируйте ваш `.c` файл компилятором `gcc` или `clang` в объектный файл `.o`:

```sh
gcc -c src/native_helper.c -o src/native_helper.o
```

### Шаг 2: Подключение в манифест yuki.toml

Добавьте скомпилированный объектный файл в секцию `[libs.static]`:

```toml
[libs.static]
objects = ["src/native_helper.o"]
system = ["m", "pthread"]
```

### Шаг 3: Объявление и вызов в коде Yukino

```yukino
externc {
    cfun custom_c_multiply(a: int32, b: int32) -> int32;
}

fun main() {
    set res = custom_c_multiply(6, 7) // 42
}
```

Команда `yukino run` автоматически слинкует объектный файл `native_helper.o` и системные библиотеки с итоговым бинарником.

---

## 6. Комплексный пример: разные типы в C и Yukino

Ниже приводится законченный пример взаимодействия между кодовой базой C и языком Yukino, охватывающий примитивы, числа с плавающей точкой, булевы флаги, C-строки, структуры с C ABI раскладкой полей (`cstruct`), выходные параметры по указателю (`ptr<T>`), а также обратные вызовы (коллбэки `cfun`).

### Исходный C-код (sensor_native.h и sensor_native.c)

```c
// sensor_native.h
#include <stdint.h>
#include <stdbool.h>

typedef struct {
    int32_t sensor_id;
    double temperature;
    float humidity;
    bool is_active;
    const char* label;
} SensorReport;

typedef int32_t (*FilterCallback)(int32_t value, int32_t threshold);

// Вычисление данных датчика: принимает скаляры, строку и структуру
double process_sensor(
    const SensorReport* report,
    double calibration_offset,
    int32_t* out_status_code
);

// Применение коллбэка к массиву целых чисел
int32_t filter_samples(
    const int32_t* samples,
    int32_t count,
    int32_t threshold,
    FilterCallback callback
);
```

```c
// sensor_native.c
#include "sensor_native.h"
#include <string.h>

double process_sensor(
    const SensorReport* report,
    double calibration_offset,
    int32_t* out_status_code
) {
    if (!report || !report->is_active) {
        if (out_status_code) *out_status_code = -1;
        return 0.0;
    }
    if (out_status_code) *out_status_code = 0;
    return report->temperature + calibration_offset;
}

int32_t filter_samples(
    const int32_t* samples,
    int32_t count,
    int32_t threshold,
    FilterCallback callback
) {
    int32_t passed = 0;
    for (int32_t i = 0; i < count; i++) {
        if (callback(samples[i], threshold) > 0) {
            passed++;
        }
    }
    return passed;
}
```

### Код на Yukino (main.ykn)

```yukino
// 1. Описание внешней структуры данных с C ABI раскладкой полей
externc {
    cstruct SensorReport {
        sensor_id: int32,
        temperature: float64,
        humidity: float32,
        is_active: bool,
        label: ptr<char>,
    };

    // 2. Декларации внешних функций с различными типами данных
    |cdecl|
    cfun process_sensor(
        report: ptr<SensorReport>,
        calibration_offset: float64,
        out_status_code: ptr<int32>
    ) -> float64;

    |cdecl|
    cfun filter_samples(
        samples: ptr<int32>,
        count: int32,
        threshold: int32,
        callback: cfun(int32, int32) -> int32
    ) -> int32;
}

// 3. Функция обратного вызова, написанная на Yukino и экспортируемая для Си
|cdecl|
public cfun threshold_filter(value: int32, threshold: int32) -> int32 {
    if value >= threshold {
        return 1
    }
    return 0
}

fun main() {
    // Подготовка C-строки: литерал завершается \0 в памяти,
    // а указатель ptr<uint8> переименовывается в ptr<char> без кода
    set label_text = "Living Room Sensor\0"
    set c_label = cast(label_text.as_ptr(), "ptr<char>")

    // Создание C-структуры
    set report = SensorReport {
        sensor_id: 101,
        temperature: 23.5,
        humidity: 48.0,
        is_active: true,
        label: c_label,
    }

    // Выходной параметр статуса через указатель
    set status_code: int32 = 0

    // Вызов C-функции с передачей указателя на структуру и выходной указатель
    set calibrated_temp = process_sensor(
        ptr_of(report),
        -0.5,
        ptr_of(status_code)
    )

    if status_code == 0 {
        // Успешная обработка
    }

    // Передача массива и коллбэка в Си-функцию
    set samples: int32[] = [12, 45, 68, 90, 22]
    set passed_count = filter_samples(
        samples.as_ptr(),
        cast(samples.len(), "int32"),
        40,
        threshold_filter
    )
    // passed_count == 3 (45, 68, 90)
}
```

---

[← Назад: Макросистема define](10-macros.md) | [Вперед: Встроенные функции компилятора (Intrinsics) →](12-intrinsics.md)
