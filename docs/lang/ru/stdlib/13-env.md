# Модуль std.env

Модуль `std.env` предоставляет кроссплатформенный инструментарий для чтения, записи, валидации и удаления переменных окружения текущего процесса, чтения всех переменных окружения в виде типизированных структур или ассоциативных карт, а также парсинга файлов конфигурации `.env` (dotenv) с поддержкой экранирования, интерполяции и значений по умолчанию.

---

## 1. Подключение: load std.env

Импорт всего пакета:

```yukino
load std.env
```

При импорте модуля становятся доступны следующие сущности:

- **Структуры данных и ошибок:** `EnvEntry`, `EnvError`.
- **Функции работы с переменными:** `get_env`, `env`, `get_or`, `get_int`, `get_bool`, `get_float`, `require`, `require_env`, `set_env`, `remove_env`, `remove`, `has_env`, `has`.
- **Функции группового чтения:** `all_entries`, `all`, `all_envs`.
- **Функции dotenv:** `parse_dotenv`, `load_env`, `load_override`, `dotenv`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.env` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.env` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 EnvEntry

Структура пары «ключ-значение» переменной окружения или строки dotenv-файла.

```yukino
public struct EnvEntry {
    key: string
    value: string
}
```

### 4.2 EnvError

Структура ошибки, инициируемой при отсутствии обязательной переменной окружения.

```yukino
public struct EnvError {
    message: string
    key: string
}
```

---

## 5. Классы

В модуле `std.env` классы не объявлены.

---

## 6. Справочник функций

### 6.1 get_env

Получает строковое значение переменной окружения по ее имени. Функция `env` является полным синонимом функции `get_env`.

```yukino
public fun get_env(key: string) -> string?
public fun env(key: string) -> string?
```

Возвращает строковое значение типа `string` либо `null`, если переменная окружения с указанным именем не задана в текущем процессе.

Функция никогда не вызывает панику.

```yukino
load std.env: get_env
load std.io: println

set path_var = get_env("PATH")
if path_var != null {
    println("PATH: " + (path_var ?? ""))
}
```

### 6.2 get_or

Получает значение переменной окружения, возвращая указанное резервное значение, если переменная не найдена.

```yukino
public fun get_or(key: string, default_val: string) -> string
```

Возвращает не-nullable строковое значение типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.env: get_or
load std.io: println

set host = get_or("APP_HOST", "127.0.0.1")
println("Хост: " + host)
```

### 6.3 get_int

Получает значение переменной окружения и преобразует его в 64-битное целое число. Если переменная не найдена либо ее значение не является корректным целым числом, возвращает значение по умолчанию `default_val` (по умолчанию `0`).

```yukino
public fun get_int(key: string, default_val: int64 = 0) -> int64
```

Возвращает числовое значение типа `int64`.

Функция никогда не вызывает панику.

```yukino
load std.env: get_int
load std.io: println

set port = get_int("APP_PORT", 8080)
println("Порт: ${port}")
```

### 6.4 get_bool

Получает значение переменной окружения и преобразует его в логическое значение. Значения `"true"`, `"1"`, `"yes"`, `"on"` (без учета регистра) трактуются как `true`; значения `"false"`, `"0"`, `"no"`, `"off"` трактуются как `false`. При отсутствии или некорректном формате возвращает `default_val` (по умолчанию `false`).

```yukino
public fun get_bool(key: string, default_val: bool = false) -> bool
```

Возвращает логическое значение типа `bool`.

Функция никогда не вызывает панику.

```yukino
load std.env: get_bool
load std.io: println

set is_debug = get_bool("APP_DEBUG", false)
if is_debug {
    println("Режим отладки включен")
}
```

### 6.5 get_float

Получает значение переменной окружения и преобразует его в число с плавающей точкой `float64`. При отсутствии переменной или ошибке разбора возвращает значение `default_val` (по умолчанию `0.0`).

```yukino
public fun get_float(key: string, default_val: float64 = 0.0) -> float64
```

Возвращает дробное значение типа `float64`.

Функция никогда не вызывает панику.

```yukino
load std.env: get_float
load std.io: println

set rate = get_float("SAMPLE_RATE", 1.0)
println("Частота выборки: ${rate}")
```

### 6.6 require

Проверяет наличие обязательной переменной окружения и возвращает ее строковое значение (в том числе пустое, если переменная установлена пустой). Функция `require_env` является полным синонимом `require`.

```yukino
public fun require(key: string) -> string
public fun require_env(key: string) -> string
```

Возвращает строковое значение типа `string`.

Если переменная с указанным именем не установлена в текущем процессе, функция вызывает панику со структурой `EnvError`.

```yukino
load std.env: require
load std.io: println

set secret = require("DATABASE_URL")
println("Подключение инициализировано")
```

### 6.7 set_env

Устанавливает или перезаписывает значение переменной окружения для текущего процесса и порождаемых им дочерних процессов.

```yukino
public fun set_env(key: string, val: string) -> bool
```

Возвращает логическое значение `true` после установки значения; `false` возвращается только при пустом имени переменной.

Функция никогда не вызывает панику.

```yukino
load std.env: set_env
load std.io: println

if set_env("SERVICE_NAME", "gateway") {
    println("Переменная SERVICE_NAME успешно установлена")
}
```

### 6.8 remove_env

Удаляет указанную переменную окружения из текущего процесса. Функция `remove` является полным синонимом `remove_env`.

```yukino
public fun remove_env(key: string) -> bool
public fun remove(key: string) -> bool
```

Возвращает логическое значение `true` при непустом имени переменной (в том числе если переменная не была установлена), либо `false` при пустом имени.

Функция никогда не вызывает панику.

```yukino
load std.env: remove_env
load std.io: println

if remove_env("TEMPORARY_TOKEN") {
    println("Токен удален из окружения")
}
```

### 6.9 has_env

Проверяет, объявлена ли переменная окружения с указанным именем. Функция `has` является полным синонимом `has_env`.

```yukino
public fun has_env(key: string) -> bool
public fun has(key: string) -> bool
```

Возвращает логическое значение типа `bool`.

Функция никогда не вызывает панику.

```yukino
load std.env: has_env
load std.io: println

if has_env("CI") {
    println("Программа выполняется в среде Continuous Integration")
}
```

### 6.10 all_entries

Считывает все переменные окружения текущего процесса и возвращает их в виде динамического массива структур `EnvEntry`.

```yukino
public fun all_entries() -> EnvEntry[]
```

Возвращает динамический массив структур `EnvEntry[]`.

Функция никогда не вызывает панику.

```yukino
load std.env: all_entries
load std.io: println

set entries = all_entries()
for i in 0..entries.len() {
    println(entries[i].key + " = " + entries[i].value)
}
```

### 6.11 all

Считывает все переменные окружения текущего процесса и помещает их в ассоциативную карту `HashMap<string, string>`. Функция `all_envs` является полным синонимом `all`.

```yukino
public fun all() -> HashMap<string, string>
public fun all_envs() -> HashMap<string, string>
```

Возвращает ассоциативную карту типа `HashMap<string, string>`.

Функция никогда не вызывает панику.

```yukino
load std.env: all
load std.io: println

set env_map = all()
set user = env_map.get("USER")
if user != null {
    println("Текущий пользователь: " + (user ?? ""))
}
```

### 6.12 parse_dotenv

Парсит переданный текст формата `.env` в массив структур `EnvEntry`. Поддерживает однострочные и многострочные значения в одинарных и двойных кавычках, экранирование (`\n`, `\r`, `\t`, `\\`, `\"`, `\$`), пропуск пустых строк и комментариев (`#`), префикс `export `, интерполяцию переменных вида `${VAR}` и выражения со значениями по умолчанию `${VAR:-default}`.

```yukino
public fun parse_dotenv(content: string) -> EnvEntry[]
```

Возвращает массив структур `EnvEntry[]`.

Функция никогда не вызывает панику.

```yukino
load std.env: parse_dotenv
load std.io: println

// \x24 — символ "$" в виде escape-последовательности,
// чтобы интерполяция строк Yukino не сработала внутри литерала
set raw = "HOST=127.0.0.1\nPORT=9000\nURL=http://\x24{HOST}:\x24{PORT}\n"
set items = parse_dotenv(raw)
for i in 0..items.len() {
    println(items[i].key + " -> " + items[i].value)
}
```

### 6.13 load_env

Считывает указанный dotenv-файл (по умолчанию `".env"`), парсит его и сохраняет переменные в окружение процесса, только если они еще не были установлены ранее (без перезаписи существующих значений). Функция `dotenv` является полным синонимом `load_env`.

```yukino
public fun load_env(path: string = ".env") -> bool
public fun dotenv(path: string = ".env") -> bool
```

Возвращает логическое значение `true`, если файл существует и был успешно прочитан, либо `false`, если файл отсутствует.

Функция никогда не вызывает панику.

```yukino
load std.env: load_env, get_env
load std.io: println

if load_env(".env.local") {
    println("Загружена конфигурация из .env.local")
    println("База данных: " + (get_env("DB_NAME") ?? "не задана"))
}
```

### 6.14 load_override

Считывает указанный dotenv-файл и безусловно перезаписывает переменные окружения процесса новыми значениями из файла, даже если переменные уже были объявлены ранее.

```yukino
public fun load_override(path: string = ".env") -> bool
```

Возвращает логическое значение `true`, если файл существует и был успешно прочитан, либо `false`, если файл отсутствует.

Функция никогда не вызывает панику.

```yukino
load std.env: load_override, get_env
load std.io: println

if load_override(".env.test") {
    println("Тестовое окружение успешно переопределено")
}
```

---

[← Назад: Модуль std.sysinfo](12-sysinfo.md) | [Вперед: Модуль std.simpleurl →](14-simpleurl.md)
