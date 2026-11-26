# Модуль std.sharedc

Модуль `std.sharedc` (Shared C) предоставляет кроссплатформенный интерфейс для динамической загрузки разделяемых библиотек (`.so` в Linux/Unix, `.dll` в Windows, `.dylib` в macOS), получения указателей на экспортируемые C-символы и вызова нативных C-функций через приведение указателей к сигнатурам `cfun`. Модуль поддерживает флаги загрузки стандарта POSIX (`RTLD_NOW`, `RTLD_LAZY`, `RTLD_GLOBAL`, `RTLD_LOCAL`), доступ к пространству символов самого исполняемого процесса, безопасное закрытие дескрипторов и форматирование платформозависимых имен файлов библиотек.

---

## 1. Подключение: load std.sharedc

Импорт всего пакета:

```yukino
load std.sharedc
```

При импорте модуля доступны следующие сущности:

- **Константы:** `RTLD_LAZY`, `RTLD_NOW`, `RTLD_LOCAL`, `RTLD_GLOBAL`.
- **Перечисления:** `SymbolAbi`.
- **Структуры:** `SharedError`, `SharedExport`, `YukinoLibInfo`.
- **Классы:** `SharedLibrary`.
- **Функции:** `open`, `open_flags`, `try_open`, `try_open_flags`, `open_self`, `open_self_flags`, `filename`, `last_error`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле объявлены стандартные флаги управления поведением связывания символов при загрузке динамических библиотек:

```yukino
public static RTLD_LAZY: int32 = 1
public static RTLD_NOW: int32 = 2
public static RTLD_LOCAL: int32 = 4
public static RTLD_GLOBAL: int32 = 8
```

- `RTLD_LAZY` — отложенное разрешение неопределенных символов по мере их первого обращения.
- `RTLD_NOW` — немедленное разрешение всех неопределенных символов при открытии библиотеки (значение по умолчанию).
- `RTLD_LOCAL` — символы загружаемой библиотеки не экспортируются в глобальную область видимости процесса.
- `RTLD_GLOBAL` — экспортируемые символы загружаемой библиотеки становятся доступны для разрешения символов в последующих динамических библиотеках.

На платформе Windows флаги игнорируются: загрузка выполняется через `LoadLibraryW`, которая не предоставляет управления режимом связывания символов.

---

## 3. Интерфейсы

В модуле `std.sharedc` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 SharedError

Структура ошибки динамического загрузчика, используемая при вызове паники функциями и методами модуля:

```yukino
public struct SharedError {
    message: string
    code: int32
}
```

Поле `message` содержит подробное описание ошибки, возвращенное операционной системой (сообщение `dlerror` на Unix либо форматированное системное сообщение `GetLastError` на Windows). Поле `code` идентифицирует категорию ошибки:
- `1` — сбой при загрузке динамической библиотеки по указанному пути либо сбой открытия пространства процесса.
- `2` — запрошенный символ не найден в экспорте разделяемой библиотеки.
- `3` — попытка обращения к символу через дескриптор библиотеки, которая уже была закрыта.
- `4` — в разделяемой библиотеке отсутствуют метаданные `__yukino_metadata_v1` и не загружен `.ykni`-интерфейс (либо ошибка чтения `.ykni`).
- `5` — неоднозначный поиск символа среди нескольких перегруженных сигнатур.

---

## 5. Классы

### 5.1 SharedLibrary

Класс `SharedLibrary` инкапсулирует нативный системный дескриптор открытой динамической библиотеки или глобального пространства процесса.

```yukino
public class SharedLibrary {
    public handle: ptr<void>
    public path: string
    public is_closed: bool

    public fun init(handle: ptr<void>, path: string)
    public fun symbol(name: string) -> ptr<void>
    public fun symbol_or_panic(name: string) -> ptr<void>
    public fun close() -> bool
}
```

Поле `handle` содержит сырой системный указатель на загруженную библиотеку. Поле `path` хранит путь к файлу библиотеки, переданный при открытии (либо `"<self>"` для адресного пространства процесса). Поле `is_closed` указывает, был ли дескриптор уже освобожден вызовом `close`.

#### 5.1.1 symbol

Возвращает сырой указатель на экспортируемый символ по его имени. Если символ отсутствует в таблице экспорта или библиотека закрыта, возвращает нулевой указатель (`null`).

```yukino
public fun symbol(name: string) -> ptr<void>
```

Возвращает указатель типа `ptr<void>`. Возвращаемое значение nullable (может быть `null`).

Функция никогда не вызывает панику.

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    set sym = lib.symbol("calculate")
    if sym != null {
        set calc = cast(sym, "cfun(int32) -> int32")
        println(calc(10))
    }
    lib.close()
}
```

#### 5.1.2 symbol_or_panic

Возвращает сырой указатель на экспортируемый символ по его имени. Если библиотека уже закрыта либо символ не найден, генерирует панику с подробным сообщением операционной системы.

```yukino
public fun symbol_or_panic(name: string) -> ptr<void>
```

Возвращает указатель типа `ptr<void>`. Возвращаемое значение гарантированно не равно `null`.

В случае ошибки вызывает панику со структурой `SharedError`: с кодом `3`, если библиотека уже закрыта, либо с кодом `2`, если символ не найден в таблице экспорта.

```yukino
load std.sharedc: open, SharedError
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    sus {
        set sym = lib.symbol_or_panic("add_numbers")
        set add = cast(sym, "cfun(int32, int32) -> int32")
        println(add(5, 7))
    } catch err: SharedError {
        println("Ошибка поиска символа: ${err.message}")
    }
    lib.close()
}
```

#### 5.1.3 close

Выгружает динамическую библиотеку и освобождает связанный системный дескриптор. Повторные вызовы на уже закрытом объекте безопасны и возвращают `true`.

```yukino
public fun close() -> bool
```

Возвращает логическое значение типа `bool`: `true`, если дескриптор успешно освобожден (или уже был освобожден ранее), либо `false` в случае системного сбоя. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    set ok = lib.close()
    println("Библиотека выгружена: ${ok}")
}
```

---

## 6. Справочник функций

### 6.1 open

Загружает разделяемую динамическую библиотеку по указанному пути с флагом по умолчанию `RTLD_NOW`. Если загрузка завершилась сбоем, вызывает панику.

```yukino
public fun open(path: string) -> SharedLibrary
```

Возвращает экземпляр класса `SharedLibrary`. Возвращаемое значение не nullable.

В случае невозможности обнаружить или загрузить файл библиотеки вызывает панику со структурой `SharedError` (код `1`).

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libtest.so")
    println("Загружена библиотека: ${lib.path}")
    lib.close()
}
```

### 6.2 open_flags

Загружает разделяемую динамическую библиотеку по указанному пути с пользовательской комбинацией битовых флагов связывания `flags` (например, `RTLD_LAZY | RTLD_GLOBAL`).

```yukino
public fun open_flags(path: string, flags: int32) -> SharedLibrary
```

Возвращает экземпляр класса `SharedLibrary`. Возвращаемое значение не nullable.

В случае невозможности загрузить библиотеку вызывает панику со структурой `SharedError` (код `1`).

```yukino
load std.sharedc: open_flags, RTLD_LAZY, RTLD_GLOBAL
load std.io: println

fun main() {
    set lib = open_flags("./libplugin.so", RTLD_LAZY | RTLD_GLOBAL)
    println("Плагин загружен: ${lib.path}")
    lib.close()
}
```

### 6.3 try_open

Безопасно пытается загрузить динамическую библиотеку с флагом по умолчанию `RTLD_NOW`. В отличие от `open`, при ошибке не вызывает панику, а возвращает `null`.

```yukino
public fun try_open(path: string) -> SharedLibrary?
```

Возвращает экземпляр класса `SharedLibrary` при успехе либо `null`, если библиотека не найдена или не может быть загружена. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.sharedc: try_open
load std.io: println

fun main() {
    set lib = try_open("./optional_accelerator.so")
    if lib == null {
        println("Дополнительный модуль не найден, используем встроенную реализацию")
    } else {
        println("Ускоритель подключен")
        lib.close()
    }
}
```

### 6.4 try_open_flags

Безопасно пытается загрузить динамическую библиотеку с явными флагами связывания `flags`. При ошибке возвращает `null` без вызова паники.

```yukino
public fun try_open_flags(path: string, flags: int32) -> SharedLibrary?
```

Возвращает экземпляр класса `SharedLibrary` при успехе либо `null` при ошибке. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.sharedc: try_open_flags, RTLD_LAZY
load std.io: println

fun main() {
    set lib = try_open_flags("./optional_module.so", RTLD_LAZY)
    if lib != null {
        println("Модуль успешно открыт")
        lib.close()
    }
}
```

### 6.5 open_self

Открывает дескриптор адресного пространства текущего исполняемого процесса с флагом `RTLD_NOW`. Позволяет находить и вызывать символы, экспортированные основной программой или динамически прилинкованной стандартной библиотекой C (например, `puts`, `malloc`, `getpid`).

```yukino
public fun open_self() -> SharedLibrary
```

Возвращает экземпляр класса `SharedLibrary` с путем `"<self>"`. Возвращаемое значение не nullable.

В случае системного сбоя вызывает панику со структурой `SharedError` (код `1`).

```yukino
load std.sharedc: open_self
load std.io: println

fun main() {
    set proc = open_self()
    set puts_ptr = proc.symbol("puts")
    if puts_ptr != null {
        println("Символ puts обнаружен в адресном пространстве процесса")
    }
    proc.close()
}
```

### 6.6 open_self_flags

Открывает дескриптор адресного пространства текущего процесса с явными флагами связывания `flags`.

```yukino
public fun open_self_flags(flags: int32) -> SharedLibrary
```

Возвращает экземпляр класса `SharedLibrary`. Возвращаемое значение не nullable.

В случае системного сбоя вызывает панику со структурой `SharedError` (код `1`).

```yukino
load std.sharedc: open_self_flags, RTLD_GLOBAL
load std.io: println

fun main() {
    set proc = open_self_flags(RTLD_GLOBAL)
    println("Пространство процесса открыто с RTLD_GLOBAL")
    proc.close()
}
```

### 6.7 filename

Форматирует базовое имя библиотеки `name` в каноническое имя файла динамической библиотеки для текущей целевой операционной системы. Например, для имени `"crypto"` возвращает `"libcrypto.so"` на Linux/Unix, `"crypto.dll"` на Windows и `"libcrypto.dylib"` на macOS. Расширение добавляется всегда: наличие суффикса (`.so`, `.dll`, `.dylib`) во входной строке не проверяется, поэтому для `"mylib.so"` будет возвращено `"libmylib.so.so"` на Linux.

```yukino
public fun filename(name: string) -> string
```

Возвращает форматированную строку имени файла типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.sharedc: filename
load std.io: println

fun main() {
    println(filename("sqlite3")) // "libsqlite3.so" на Linux, "sqlite3.dll" на Windows
    println(filename("mylib.so")) // "libmylib.so.so" на Linux, "mylib.so.dll" на Windows
}
```

### 6.8 last_error

Возвращает сообщение последней ошибки динамического загрузчика в виде строки либо `null`, если ошибок не зафиксировано. Сообщение формируется в момент сбоя операции (на Unix из `dlerror()`, на Windows из кода `GetLastError()` через `FormatMessageW`) и сохраняется до следующей операции загрузки или поиска символа.

```yukino
public fun last_error() -> string?
```

Возвращает строку описания ошибки типа `string` либо `null`. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.sharedc: last_error, try_open
load std.io: println

fun main() {
    set lib = try_open("./missing.so")
    if lib == null {
        set err = last_error()
        if err != null {
            println("Причина сбоя: ${err}")
        }
    }
}
```

---

## 7. Динамическая загрузка библиотек Yukino

Модуль `std.sharedc` позволяет не только загружать обычные C-библиотеки, но и работать во время выполнения с разделяемыми библиотеками, собранными на Yukino (`lib*.so`, `*.dll`, `*.dylib`). Подробно об устройстве интерфейсных файлов `.ykni`, встроенных метаданных и соглашении вызова `ptr<fun(...) -> ...>` рассказано в разделе [Интерфейсы скомпилированных библиотек (.ykni)](../9-modules.md#6-интерфейсы-скомпилированных-библиотек-ykni).

### 7.1 Типы метаданных и коды ошибок

```yukino
public enum SymbolAbi {
    CDecl,
    YukinoFastCall
}

public struct SharedExport {
    name: string
    mangled: string
    signature: string
    abi: SymbolAbi
    kind: string
}

public struct YukinoLibInfo {
    package_name: string
    yukic_version: string
    target: string
    abi_hash: string
}
```

Дополнительные коды `SharedError`:
- `4` — в загруженной библиотеке нет метаданных Yukino и не подключен внешний `.ykni` через `load_interface`.
- `5` — имя символа неоднозначно из-за перегрузок; требуется уточнение сигнатуры через `yukino_symbol_sig` / `method_symbol`.

### 7.2 Использование методов SharedLibrary для библиотек Yukino

- `lib.is_yukino() -> bool` — проверяет, доступны ли для открытого дескриптора встроенные метаданные Yukino или загруженный `.ykni`.
- `lib.yukino_info() -> YukinoLibInfo?` — возвращает сведения о пакете, версии компилятора, таргете и хэше ABI, либо `null`.
- `lib.exports() -> SharedExport[]` — возвращает список всех экспортируемых символов с их сигнатурами и ABI.
- `lib.load_interface(ykni_path: string) -> bool` — загружает внешний `.ykni`-файл для открытой библиотеки (полезно, если библиотека собрана без метаданных или нужно привязать сторонний `.ykni`).
- `lib.yukino_symbol(name)` / `lib.yukino_symbol_or_panic(name)` — ищет символ по короткому логическому имени без ручного манглинга.
- `lib.yukino_symbol_sig(name, signature)` / `lib.yukino_symbol_sig_or_panic(name, signature)` — выбирает нужную перегрузку по строковой сигнатуре (например, `"fun(string) -> string"`).
- `lib.method_symbol(class_name, method_name, param_count = -1)` / `lib.method_symbol_or_panic(...)` — ищет скомпилированный метод класса или структуры (с опциональной фильтрацией по числу аргументов).

Полученный указатель приводится через `cast` к `ptr<fun(...) -> ...>` для функций Yukino либо к `cfun(...) -> ...` для функций C ABI:

```yukino
load std.sharedc
load std.io: println
load core.utils: cast

fun main() {
    set lib = open("./libplugin.so")

    if !lib.is_yukino() {
        // При необходимости можно подключить внешний файл интерфейса вручную
        lib.load_interface("./plugin.ykni")
    }

    set info = lib.yukino_info()
    println("Пакет: ${info?.package_name ?? \"unknown\"}")

    set greet_ptr = lib.yukino_symbol_sig_or_panic("format_greeting", "fun(string) -> string")
    set greet_fn = cast(greet_ptr, "ptr<fun(string) -> string>")
    println(greet_fn("world"))

    set c_ptr = lib.yukino_symbol_or_panic("native_ping")
    set c_fn = cast(c_ptr, "cfun(int32) -> int32")
    println(c_fn(41))

    lib.close()
}
```

---

[← Назад: Модуль std.numf](18-numf.md) | [Вперед: Модуль std.json →](20-json.md)
