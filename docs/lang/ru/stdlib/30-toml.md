# Модуль std.toml

Модуль `std.toml` предоставляет высокопроизводительные средства для синтаксического анализа, безопасного извлечения данных, валидации и сериализации конфигурационных документов в формате TOML (Tom's Obvious Minimal Language) v1.0.0 на чистом Yukino без сторонних зависимостей.

---

## 1. Подключение: load std.toml

Импорт всего пакета:

```yukino
load std.toml
```

При этом становятся доступны следующие сущности:

- **Константы типов узлов:** `TOML_NULL`, `TOML_BOOL`, `TOML_INT`, `TOML_FLOAT`, `TOML_STRING`, `TOML_DATETIME`, `TOML_ARRAY`, `TOML_TABLE`.
- **Структуры ошибок:** `TomlError`.
- **Классы:** `TomlValue`.
- **Функции парсинга и валидации:** `parse`, `try_parse`, `validate`, `toml_parse`, `toml_try_parse`, `toml_validate`.
- **Функции сериализации:** `stringify`, `pretty`, `pretty_indent`, `toml_stringify`, `toml_pretty`, `toml_pretty_indent`.
- **Фабричные функции конструирования:** `toml_null`, `toml_bool`, `toml_int`, `toml_float`, `toml_number`, `toml_string`, `toml_datetime`, `toml_array`, `toml_table`, `toml_object`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.toml` объявлены целочисленные константы, определяющие тип узла в свойстве `kind` класса `TomlValue`:

```yukino
public static TOML_NULL: int32 = 0
public static TOML_BOOL: int32 = 1
public static TOML_INT: int32 = 2
public static TOML_FLOAT: int32 = 3
public static TOML_STRING: int32 = 4
public static TOML_DATETIME: int32 = 5
public static TOML_ARRAY: int32 = 6
public static TOML_TABLE: int32 = 7
```

Константы используются для прямой проверки типа узла `if val.kind == TOML_TABLE`, хотя в большинстве сценариев предпочтительнее использовать встроенные методы-предикаты (`val.is_table()`, `val.is_string()`, `val.is_int()`, `val.is_datetime()` и другие).

---

## 3. Интерфейсы

В модуле `std.toml` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 TomlError

Структура `TomlError` представляет ошибку синтаксического анализа документа TOML. Она генерируется через `panic` функциями `parse` и `toml_parse` при обнаружении некорректного синтаксиса, незакрытых строк, неверных заголовков таблиц либо недопустимых литералов.

```yukino
public struct TomlError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: понятное текстовое описание ошибки (например, `"expected '=' after TOML key"`).
- `line`: номер строки в исходном тексте, где произошла ошибка (начиная с 1).
- `column`: номер колонки в строке (начиная с 1).
- `offset`: байтовое смещение от начала входной строки (начиная с 0).

Пример перехвата ошибки через конструкцию `sus / catch`:

```yukino
load std.toml: parse, TomlError
load std.io: println

fun main() {
    sus {
        set doc = parse("[broken table")
    } catch err: TomlError {
        println("Ошибка TOML: ${err.message} на строке ${err.line}, колонка ${err.column}")
    }
}
```

---

## 5. Классы

### 5.1 TomlValue

Класс `TomlValue` представляет произвольный узел в дереве документа TOML (null, boolean, целое число, вещественное число, строку, дату/время, массив либо таблицу). Класс спроектирован по принципу Fluent API и гарантирует безопасную навигацию по вложенным секциям и ключам: вызовы методов навигации никогда не вызывают панику при отсутствии ключа или несоответствии типов.

```yukino
public class TomlValue {
    public kind: int32
    public bool_val: bool
    public int_val: int64
    public float_val: float64
    public str_val: string
    public array_val: TomlValue[]
    public obj_keys: string[]
    public obj_vals: TomlValue[]
}
```

#### Методы проверки типа (предикаты)

- `is_null() -> bool`: возвращает `true`, если узел равен `null` (или представляет отсутствующее значение).
- `is_bool() -> bool`: возвращает `true`, если узел является логическим значением.
- `is_number() -> bool`: возвращает `true`, если узел является целым или вещественным числом.
- `is_int() -> bool`: возвращает `true`, если узел является целым числом.
- `is_float() -> bool`: возвращает `true`, если узел является числом с плавающей точкой.
- `is_string() -> bool`: возвращает `true`, если узел является строкой.
- `is_datetime() -> bool`: возвращает `true`, если узел содержит строковое представление даты и времени по стандарту RFC 3339 / ISO 8601.
- `is_array() -> bool`: возвращает `true`, если узел является массивом.
- `is_table() -> bool`: возвращает `true`, если узел является таблицей TOML.
- `is_object() -> bool`: синоним метода `is_table()`, возвращает `true`, если узел является таблицей.
- `is_table_array() -> bool`: возвращает `true`, если узел является массивом таблиц `[[table]]`.

#### Методы извлечения значений

- `as_bool() -> bool`: возвращает булево значение. Если узел не является булевым, возвращает `false`.
- `as_bool_or(fallback: bool) -> bool`: возвращает булево значение либо `fallback`, если узел не является булевым.
- `as_int() -> int64`: возвращает целочисленное значение `int64`. Если узел вещественный, выполняет приведение типа; если не числовой, возвращает `0`.
- `as_int_or(fallback: int64) -> int64`: возвращает целое число либо `fallback`, если узел не числовой.
- `as_float() -> float64`: возвращает вещественное число `float64`. Если узел целочисленный, выполняет приведение типа; если не числовой, возвращает `0.0`.
- `as_float_or(fallback: float64) -> float64`: возвращает вещественное число либо `fallback`.
- `as_string() -> string`: возвращает строковое значение. Для узлов даты/времени возвращает их ISO-строку. Если узел не строковый, возвращает пустую строку `""`.
- `as_string_or(fallback: string) -> string`: возвращает строку либо `fallback`, если узел не строковый.
- `as_array() -> TomlValue[]`: возвращает внутренний массив элементов `TomlValue[]`. Если узел не является массивом, возвращает пустой список.

#### Методы навигации по таблицам и массивам

- `get(key: string) -> TomlValue`: возвращает значение свойства таблицы по имени `key`. Если ключ отсутствует или узел не является таблицей, возвращает безопасный узел `TomlValue` типа null. Это позволяет строить безопасные цепочки вида `doc.get("database").get("ports").at(0).as_int()`.
- `get_opt(key: string) -> TomlValue?`: возвращает nullable-значение `TomlValue?`, равное `null`, если ключ отсутствует или узел не является таблицей.
- `has(key: string) -> bool`: возвращает `true`, если таблица содержит указанный ключ.
- `keys() -> string[]`: возвращает массив строковых ключей таблицы.
- `at(index: int64) -> TomlValue`: возвращает элемент массива по индексу `index`. Если индекс выходит за границы или узел не массив, возвращает безопасный узел `TomlValue` типа null.
- `at_opt(index: int64) -> TomlValue?`: возвращает nullable-элемент `TomlValue?` по индексу либо `null` при выходе за границы.
- `len() -> int64`: возвращает количество элементов для массива, количество ключей для таблицы либо длину строки в байтах для строкового узла. Для остальных типов возвращает `0`.

#### Методы мутации и построения

- `set(key: string, val: TomlValue) -> TomlValue`: добавляет или перезаписывает ключ таблицы и возвращает `this` для цепочечных вызовов.
- `push(item: TomlValue) -> TomlValue`: добавляет элемент в массив и возвращает `this`.
- `set_path(path: string[], val: TomlValue) -> TomlValue`: рекурсивно создает промежуточные таблицы по цепочке ключей `path` и устанавливает значение конечного свойства.
- `cp() -> TomlValue`: выполняет глубокое клонирование всего дерева `TomlValue`.

---

## 6. Справочник функций

### 6.1 parse

Разбирает входную строку в формате TOML v1.0.0 и возвращает корневую таблицу `TomlValue`. Поддерживает стандартные таблицы `[table]`, вложенные секции с точечной нотацией `[a.b.c]`, массивы таблиц `[[items]]`, инлайн-таблицы, массивы любых типов, комментарии и все форматы чисел и дат.

```yukino
public fun parse(text: string) -> TomlValue
```

Возвращает разобранный корневой узел таблицы типа `TomlValue`. Возвращаемое значение не nullable.

В случае синтаксической ошибки, незакрытой строки или недопустимой структуры вызывает панику со структурой `TomlError`.

```yukino
load std.toml: parse
load std.io: println

fun main() {
    set doc = parse("title = \"Yukino Project\"\n\n[owner]\nname = \"Alice\"\n")
    println("Заголовок: " + doc.get("title").as_string())
    println("Владелец: " + doc.get("owner").get("name").as_string())
}
```

### 6.2 try_parse

Выполняет безопасную попытку синтаксического анализа строки в формате TOML без генерации паники при ошибках разбора.

```yukino
public fun try_parse(text: string) -> TomlValue?
```

Возвращает узел `TomlValue` в случае успешного разбора либо `null`, если входной текст содержит синтаксические ошибки. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.toml: try_parse
load std.io: println

fun main() {
    set res = try_parse("broken = [1, 2")
    if res == null {
        println("Строка не является корректным TOML")
    }
}
```

### 6.3 validate

Проверяет, является ли переданная строка синтаксически корректным документом TOML.

```yukino
public fun validate(text: string) -> bool
```

Возвращает `true`, если строка является валидным TOML, либо `false` в противном случае. Возвращаемое значение типа `bool`, не nullable.

Функция никогда не вызывает панику.

```yukino
load std.toml: validate
load std.io: println

fun main() {
    if validate("key = \"value\"\n[section]\nid = 100\n") {
        println("Документ TOML валиден")
    }
}
```

### 6.4 stringify

Выполняет сериализацию узла `TomlValue` в каноническую строку формата TOML с автоматическим разделением на секции таблиц и массивы таблиц.

```yukino
public fun stringify(val: TomlValue) -> string
```

Возвращает сериализованную строку TOML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.toml: toml_table, toml_string, toml_int, stringify
load std.io: println

fun main() {
    set doc = toml_table()
    doc.set("app", toml_string("server"))
    doc.set("port", toml_int(8080))
    println(stringify(doc))
}
```

### 6.5 pretty

Сериализует узел `TomlValue` в структурированную строку TOML со стандартным отступом для инлайн-структур.

```yukino
public fun pretty(val: TomlValue) -> string
```

Возвращает отформатированную строку TOML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.toml: parse, pretty
load std.io: println

fun main() {
    set doc = parse("name = \"demo\"\n[server]\nhost = \"127.0.0.1\"\n")
    println(pretty(doc))
}
```

### 6.6 pretty_indent

Сериализует узел `TomlValue` в форматированную строку TOML с настраиваемой строкой отступа.

```yukino
public fun pretty_indent(val: TomlValue, indent: string) -> string
```

Возвращает отформатированную строку TOML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.toml: parse, pretty_indent
load std.io: println

fun main() {
    set doc = parse("name = \"demo\"\n")
    println(pretty_indent(doc, "    "))
}
```

### 6.7 Синонимы функций с префиксом toml_

Для максимального удобства и полной совместимости со стилем `std.json` модуль предоставляет функции-синонимы с явным префиксом `toml_`:

- `toml_parse(text: string) -> TomlValue`: эквивалент `parse`.
- `toml_try_parse(text: string) -> TomlValue?`: эквивалент `try_parse`.
- `toml_validate(text: string) -> bool`: эквивалент `validate`.
- `toml_stringify(val: TomlValue) -> string`: эквивалент `stringify`.
- `toml_pretty(val: TomlValue) -> string`: эквивалент `pretty`.
- `toml_pretty_indent(val: TomlValue, indent: string) -> string`: эквивалент `pretty_indent`.

Поведение, типы возвращаемых значений, условия паники и nullability полностью идентичны базовым функциям.

```yukino
load std.toml: toml_parse, toml_stringify
load std.io: println

fun main() {
    set doc = toml_parse("version = 1\n")
    println(toml_stringify(doc))
}
```

### 6.8 Фабричные функции конструирования узлов

Модуль предоставляет полный набор фабричных функций для конструирования дерева `TomlValue`:

- `toml_null() -> TomlValue`: создает пустой узел типа null.
- `toml_bool(b: bool) -> TomlValue`: создает логический узел со значением `b`.
- `toml_int(n: int64) -> TomlValue`: создает целочисленный узел со значением `n`.
- `toml_float(f: float64) -> TomlValue`: создает вещественный узел со значением `f`.
- `toml_number(f: float64) -> TomlValue`: создает числовой узел со значением `f`.
- `toml_string(s: string) -> TomlValue`: создает строковый узел со значением `s`.
- `toml_datetime(dt: string) -> TomlValue`: создает узел даты и времени с форматом ISO 8601 / RFC 3339.
- `toml_array(items: TomlValue[]) -> TomlValue`: создает массив из переданного списка элементов `items`.
- `toml_table() -> TomlValue`: создает пустую таблицу TOML.
- `toml_object() -> TomlValue`: синоним `toml_table()`, создает пустую таблицу.

Все фабричные функции возвращают узел типа `TomlValue` (не nullable) и никогда не вызывают панику.

```yukino
load std.toml: toml_table, toml_string, toml_array, toml_int, toml_stringify
load std.io: println

fun main() {
    set root = toml_table()
    root.set("title", toml_string("Config"))
    set ports = toml_array([])
    ports.push(toml_int(80))
    ports.push(toml_int(443))
    root.set("ports", ports)
    println(toml_stringify(root))
}
```

---

[← Назад: Модуль std.regex](29-regex.md) | [Вперед: Модуль std.yaml →](31-yaml.md)
