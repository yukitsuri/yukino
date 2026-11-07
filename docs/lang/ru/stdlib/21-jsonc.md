# Модуль std.jsonc

Модуль `std.jsonc` предоставляет полнофункциональные средства для разбора, валидации, очистки комментариев и сериализации документов формата JSON with Comments (JSONC). Модуль полностью поддерживает однострочные комментарии `//`, многострочные блочные комментарии `/* ... */`, а также завершающие запятые (trailing commas) в объектах и массивах.

---

## 1. Подключение: load std.jsonc

Импорт всего пакета:

```yukino
load std.jsonc
```

При этом становятся доступны следующие сущности:

- **Константы типов узлов:** `JSON_NULL`, `JSON_BOOL`, `JSON_NUMBER`, `JSON_STRING`, `JSON_ARRAY`, `JSON_OBJECT`.
- **Структуры ошибок:** `JsonError`.
- **Классы:** `JsonValue`.
- **Функции парсинга и валидации:** `jsonc_parse`, `jsonc_try_parse`, `jsonc_validate`, `jsonc_strip_comments`.
- **Функции сериализации:** `jsonc_stringify`, `jsonc_pretty`, `jsonc_pretty_indent`.
- **Фабричные функции конструирования:** `json_null`, `json_bool`, `json_number`, `json_int`, `json_string`, `json_array`, `json_object`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.jsonc` объявлены целочисленные константы, определяющие тип узла в свойстве `kind` класса `JsonValue`:

```yukino
public static JSON_NULL: int32 = 0
public static JSON_BOOL: int32 = 1
public static JSON_NUMBER: int32 = 2
public static JSON_STRING: int32 = 3
public static JSON_ARRAY: int32 = 4
public static JSON_OBJECT: int32 = 5
```

Константы соответствуют типам узлов в документе и совпадают с константами модуля `std.json`.

---

## 3. Интерфейсы

В модуле `std.jsonc` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 JsonError

Структура `JsonError` представляет ошибку синтаксического анализа JSONC документа. Она генерируется через `panic` функцией `jsonc_parse` в случае обнаружения некорректного синтаксиса (например, незакрытых строк, незавершенных блочных комментариев либо отсутствующих двоеточий).

```yukino
public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: текстовое описание ошибки синтаксиса.
- `line`: номер строки в исходном тексте (начиная с 1).
- `column`: номер колонки в строке (начиная с 1).
- `offset`: смещение в байтах от начала исходного текста.

Пример перехвата ошибки через конструкцию `sus / catch`:

```yukino
load std.jsonc: jsonc_parse, JsonError
load std.io: println

fun main() {
    sus {
        set doc = jsonc_parse("/* незакрытый комментарий")
    } catch err: JsonError {
        println("Ошибка JSONC: ${err.message} на строке ${err.line}")
    }
}
```

---

## 5. Классы

### 5.1 JsonValue

Модуль `std.jsonc` оперирует единым представлением дерева документа через класс `JsonValue`. Класс предоставляет Fluent API с безопасным доступом к свойствам и элементам без паники при отсутствии ключей:

```yukino
public class JsonValue {
    public kind: int32
    public bool_val: bool
    public num_val: float64
    public str_val: string
    public array_val: JsonValue[]
    public obj_keys: string[]
    public obj_vals: JsonValue[]
}
```

Все методы класса `JsonValue` (`is_null`, `is_string`, `get`, `get_opt`, `at`, `at_opt`, `as_string`, `as_int`, `as_bool`, `as_float`, `has`, `keys`, `len`, `set`, `push`, `cp`) полностью идентичны описанным в документации модуля `std.json`.

#### Рекомендация: сопоставление конфигурационных файлов в структуры

Формат JSONC широко применяется для файлов конфигурации проектов, настроек серверов и параметров запуска. Рекомендуется преобразовывать разобранный `JsonValue` в типизированную структуру настроек:

```yukino
load std.jsonc: jsonc_parse, JsonValue
load std.io: println

struct ServerConfig {
    host: string
    port: int64
    workers: int64
    debug: bool
}

fun server_config_from_json(j: JsonValue) -> ServerConfig {
    return ServerConfig {
        host: j.get("host").as_string_or("127.0.0.1"),
        port: j.get("port").as_int_or(8080),
        workers: j.get("workers").as_int_or(4),
        debug: j.get("debug").as_bool_or(false),
    }
}

fun main() {
    set raw_config = "{\n  // Сетевые параметры\n  \"host\": \"0.0.0.0\",\n  \"port\": 9000,\n  /* Включить отладку */\n  \"debug\": true,\n}"
    set doc = jsonc_parse(raw_config)
    set cfg = server_config_from_json(doc)

    println("Сервер запущен на ${cfg.host}:${cfg.port}")
}
```

---

## 6. Справочник функций

### 6.1 jsonc_parse

Разбирает входную строку с поддержкой комментариев (`//` и `/* */`) и завершающих запятых в объектах и массивах, возвращая корневой узел `JsonValue`.

```yukino
public fun jsonc_parse(text: string) -> JsonValue
```

Возвращает разобранный узел `JsonValue`. Возвращаемое значение не nullable.

В случае синтаксической ошибки, незакрытого блочного комментария или недопустимых символов вызывает панику со структурой `JsonError`.

```yukino
load std.jsonc: jsonc_parse
load std.io: println

fun main() {
    set text = "{\n  // комментарий\n  \"env\": \"production\",\n}"
    set doc = jsonc_parse(text)
    println("Окружение: " + doc.get("env").as_string())
}
```

### 6.2 jsonc_try_parse

Выполняет безопасную попытку синтаксического анализа строки JSONC без вызова паники при ошибках.

```yukino
public fun jsonc_try_parse(text: string) -> JsonValue?
```

Возвращает узел `JsonValue` при успешном разборе либо `null` при синтаксической ошибке. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_try_parse
load std.io: println

fun main() {
    set res = jsonc_try_parse("{\n  // битый json\n  \"a\": \n}")
    if res == null {
        println("Не удалось разобрать конфигурацию")
    }
}
```

### 6.3 jsonc_validate

Проверяет, является ли строка корректным документом формата JSONC.

```yukino
public fun jsonc_validate(text: string) -> bool
```

Возвращает логическое значение `true`, если синтаксис верен, либо `false` при наличии ошибок. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_validate
load std.io: println

fun main() {
    set valid = jsonc_validate("// config\n{\"port\": 3000,}")
    if valid {
        println("Конфигурация валидна")
    }
}
```

### 6.4 jsonc_strip_comments

Удаляет все однострочные (`//`) и многострочные (`/* */`) комментарии из исходного текста JSONC, заменяя их пробелами и сохраняя символы переноса строки `\n`. Это позволяет передавать очищенный текст в стандартные строгие JSON-парсеры без нарушения соответствия номеров строк и колонок при диагностике.

```yukino
public fun jsonc_strip_comments(text: string) -> string
```

Возвращает строку очищенного текста типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_strip_comments
load std.json: json_parse
load std.io: println

fun main() {
    set text_with_comments = "{\n  // Пользовательское значение\n  \"key\": \"value\"\n}"
    set clean_json = jsonc_strip_comments(text_with_comments)
    set doc = json_parse(clean_json)
    println("Ключ: " + doc.get("key").as_string())
}
```

### 6.5 jsonc_stringify

Сериализует узел `JsonValue` в компактную строку стандартного JSON без комментариев и форматирования.

```yukino
public fun jsonc_stringify(val: JsonValue) -> string
```

Возвращает компактную строку типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_parse, jsonc_stringify
load std.io: println

fun main() {
    set doc = jsonc_parse("// настройки\n{\"a\": 1, \"b\": 2,}")
    println(jsonc_stringify(doc))
}
```

### 6.6 jsonc_pretty

Сериализует узел `JsonValue` в отформатированную строку с отступами в 2 пробела на каждый уровень вложенности.

```yukino
public fun jsonc_pretty(val: JsonValue) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_parse, jsonc_pretty
load std.io: println

fun main() {
    set doc = jsonc_parse("{\"enabled\": true,}")
    println(jsonc_pretty(doc))
}
```

### 6.7 jsonc_pretty_indent

Сериализует узел `JsonValue` в форматированную строку с произвольным строковым шагом отступа (например, 4 пробела или символ табуляции `"\t"`).

```yukino
public fun jsonc_pretty_indent(val: JsonValue, indent: string) -> string
```

Возвращает отформатированную строку типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.jsonc: jsonc_parse, jsonc_pretty_indent
load std.io: println

fun main() {
    set doc = jsonc_parse("{\"name\": \"config\"}")
    println(jsonc_pretty_indent(doc, "    "))
}
```

---

[← Назад: Модуль std.json](20-json.md) | [Вперед: Модуль std.outf →](22-outf.md)
