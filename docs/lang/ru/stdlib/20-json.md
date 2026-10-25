# Модуль std.json

Модуль `std.json` предоставляет высокопроизводительные средства для синтаксического анализа, безопасного извлечения данных, валидации и сериализации документов в строгом соответствии со спецификацией RFC 8259 JSON на чистом Yukino без сторонних зависимостей.

---

## 1. Подключение: load std.json

Импорт всего пакета:

```yukino
load std.json
```

При этом становятся доступны следующие сущности:

- **Константы типов узлов:** `JSON_NULL`, `JSON_BOOL`, `JSON_NUMBER`, `JSON_STRING`, `JSON_ARRAY`, `JSON_OBJECT`.
- **Структуры ошибок:** `JsonError`.
- **Классы:** `JsonValue`.
- **Функции парсинга и валидации:** `json_parse`, `json_try_parse`, `json_validate`.
- **Функции сериализации:** `json_stringify`, `json_pretty`, `json_pretty_indent`.
- **Фабричные функции конструирования:** `json_null`, `json_bool`, `json_number`, `json_int`, `json_string`, `json_array`, `json_object`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.json` объявлены целочисленные константы, определяющие тип узла в свойстве `kind` класса `JsonValue`:

```yukino
public static JSON_NULL: int32 = 0
public static JSON_BOOL: int32 = 1
public static JSON_NUMBER: int32 = 2
public static JSON_STRING: int32 = 3
public static JSON_ARRAY: int32 = 4
public static JSON_OBJECT: int32 = 5
```

Эти константы используются для низкоуровневой проверки типа узла `if val.kind == JSON_STRING`, хотя в большинстве сценариев предпочтительнее использовать встроенные методы-предикаты (`val.is_string()`, `val.is_null()` и другие).

---

## 3. Интерфейсы

В модуле `std.json` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 JsonError

Структура `JsonError` представляет ошибку синтаксического анализа JSON. Она генерируется через `panic` функцией `json_parse` в случае обнаружения некорректного синтаксиса, незакрытых литералов, недопустимых символов либо запрещенных в стандарте JSON комментариев и завершающих запятых.

```yukino
public struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: понятное текстовое описание ошибки (например, `"expected ':' after property name"`).
- `line`: номер строки в исходном тексте, где произошла ошибка (начиная с 1).
- `column`: номер колонки в строке (начиная с 1).
- `offset`: байтовое смещение от начала входной строки (начиная с 0).

Пример перехвата ошибки через конструкцию `sus / catch`:

```yukino
load std.json: json_parse, JsonError
load std.io: println

fun main() {
    sus {
        set doc = json_parse("{\"invalid\": }")
    } catch err: JsonError {
        println("Ошибка парсинга: ${err.message} на строке ${err.line}, колонка ${err.column}")
    }
}
```

---

## 5. Классы

### 5.1 JsonValue

Класс `JsonValue` представляет произвольный узел в дереве JSON (null, boolean, число, строку, массив или ассоциативный объект). Класс спроектирован по принципу Fluent API и гарантирует безопасную навигацию по вложенным свойствам: вызовы цепочек методов никогда не вызывают панику при отсутствии ключа или несоответствии типов.

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

#### Методы проверки типа (предикаты)

- `is_null() -> bool`: возвращает `true`, если узел равен `null` (или если узел представляет отсутствующее значение).
- `is_bool() -> bool`: возвращает `true`, если узел является логическим значением.
- `is_number() -> bool`: возвращает `true`, если узел является числом.
- `is_string() -> bool`: возвращает `true`, если узел является строкой.
- `is_array() -> bool`: возвращает `true`, если узел является массивом.
- `is_object() -> bool`: возвращает `true`, если узел является объектом.

#### Методы извлечения значений

- `as_bool() -> bool`: возвращает булево значение. Если узел не является булевым, возвращает `false`.
- `as_bool_or(fallback: bool) -> bool`: возвращает булево значение либо `fallback`, если узел не является булевым.
- `as_int() -> int64`: возвращает целочисленное значение (число приводится к `int64`). Если узел не числовой, возвращает `0`.
- `as_int_or(fallback: int64) -> int64`: возвращает целое число либо `fallback`, если узел не числовой.
- `as_float() -> float64`: возвращает число с плавающей точкой `float64`. Если узел не числовой, возвращает `0.0`.
- `as_float_or(fallback: float64) -> float64`: возвращает число с плавающей точкой либо `fallback`.
- `as_string() -> string`: возвращает строку. Если узел не является строкой, возвращает пустую строку `""`.
- `as_string_or(fallback: string) -> string`: возвращает строку либо `fallback`, если узел не строковый.
- `as_array() -> JsonValue[]`: возвращает внутренний массив элементов `JsonValue[]`. Если узел не является массивом, возвращает пустой массив.

#### Методы навигации по объектам и массивам

- `get(key: string) -> JsonValue`: возвращает значение свойства объекта по имени `key`. Если объект не содержит такого ключа или узел не является объектом, возвращает безопасный узел `JsonValue` типа null. Это позволяет строить непрерывные цепочки вида `doc.get("user").get("profile").get("email").as_string()`.
- `get_opt(key: string) -> JsonValue?`: возвращает nullable-значение `JsonValue?`, равное `null`, если ключ отсутствует.
- `has(key: string) -> bool`: возвращает `true`, если объект содержит указанный ключ.
- `keys() -> string[]`: возвращает массив всех строковых ключей объекта.
- `at(index: int64) -> JsonValue`: возвращает элемент массива по индексу `index`. Если индекс выходит за границы или узел не массив, возвращает безопасный узел `JsonValue` типа null.
- `at_opt(index: int64) -> JsonValue?`: возвращает nullable-элемент `JsonValue?` по индексу либо `null` при выходе за границы.
- `len() -> int64`: возвращает количество элементов для массива, количество ключей для объекта либо длину строки в байтах для строкового узла. Для остальных типов возвращает `0`.

#### Методы мутации и построения

- `set(key: string, val: JsonValue) -> JsonValue`: добавляет или перезаписывает свойство объекта и возвращает `this` для цепочечных вызовов.
- `push(item: JsonValue) -> JsonValue`: добавляет элемент в конец массива и возвращает `this`.
- `cp() -> JsonValue`: выполняет глубокое клонирование всего дерева `JsonValue`.

#### Рекомендация: сопоставление JsonValue в структуры приложения

Так как Yukino является строго типизированным компилируемым языком со статической проверкой типов, наиболее чистым, надежным и производительным подходом является преобразование динамического `JsonValue` в типизированные пользовательские структуры (`struct`). Для этого рекомендуется реализовывать простые функции-конструкторы:

```yukino
load std.json: json_parse, JsonValue
load std.io: println

struct UserProfile {
    id: int64
    username: string
    is_active: bool
    email: string
}

fun user_profile_from_json(j: JsonValue) -> UserProfile {
    return UserProfile {
        id: j.get("id").as_int(),
        username: j.get("username").as_string(),
        is_active: j.get("is_active").as_bool_or(true),
        email: j.get("email").as_string_or("unknown@example.com"),
    }
}

fun main() {
    set raw = "{\"id\": 42, \"username\": \"yukino\", \"email\": \"yuki@local\"}"
    set doc = json_parse(raw)
    set user = user_profile_from_json(doc)

    println("Пользователь: ${user.username}, ID: ${user.id}, Email: ${user.email}")
}
```

Этот подход исключает работу с динамическими типами в бизнес-логике и гарантирует типобезопасность на этапе компиляции.

---

## 6. Справочник функций

### 6.1 json_parse

Разбирает входную строку в соответствии со строгим стандартом RFC 8259 JSON и возвращает корневой узел `JsonValue`. В строгом режиме комментарии и завершающие запятые (trailing commas) запрещены.

```yukino
public fun json_parse(text: string) -> JsonValue
```

Возвращает разобранный узел `JsonValue`. Возвращаемое значение не nullable.

В случае синтаксической ошибки, недопустимых символов, незакрытых строк или наличия комментариев вызывает панику со структурой `JsonError`.

```yukino
load std.json: json_parse
load std.io: println

fun main() {
    set doc = json_parse("{\"title\": \"Yukino\", \"version\": 1}")
    println("Заголовок: " + doc.get("title").as_string())
}
```

### 6.2 json_try_parse

Выполняет безопасную попытку синтаксического анализа строки JSON без генерации паники при ошибках.

```yukino
public fun json_try_parse(text: string) -> JsonValue?
```

Возвращает узел `JsonValue` в случае успешного разбора либо `null`, если входной текст содержит синтаксические ошибки. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.json: json_try_parse
load std.io: println

fun main() {
    set res = json_try_parse("{\"broken\": ")
    if res == null {
        println("Строка не является корректным JSON")
    }
}
```

### 6.3 json_validate

Проверяет, является ли переданная строка синтаксически корректным документом строгого стандарта JSON.

```yukino
public fun json_validate(text: string) -> bool
```

Возвращает `true`, если строка валидна, либо `false` в противном случае. Возвращаемое значение типа `bool`, не nullable.

Функция никогда не вызывает панику.

```yukino
load std.json: json_validate
load std.io: println

fun main() {
    if json_validate("{\"valid\": true}") {
        println("JSON валиден")
    }
}
```

### 6.4 json_stringify

Выполняет компактную сериализацию узла `JsonValue` в строку без лишних пробелов и переносов строк. Все специальные символы в строках корректно экранируются.

```yukino
public fun json_stringify(val: JsonValue) -> string
```

Возвращает компактную строку JSON типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.json: json_object, json_string, json_stringify
load std.io: println

fun main() {
    set obj = json_object()
    obj.set("status", json_string("ok"))
    println(json_stringify(obj))
}
```

### 6.5 json_pretty

Сериализует узел `JsonValue` в форматированную строку с переносами строк и стандартным отступом в 2 пробела на каждый уровень вложенности.

```yukino
public fun json_pretty(val: JsonValue) -> string
```

Возвращает отформатированную строку JSON типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.json: json_parse, json_pretty
load std.io: println

fun main() {
    set doc = json_parse("{\"name\": \"Alice\", \"items\": [1, 2]}")
    println(json_pretty(doc))
}
```

### 6.6 json_pretty_indent

Сериализует узел `JsonValue` в форматированную строку с пользовательским шагом отступа (например, 4 пробела или символ табуляции `"\t"`).

```yukino
public fun json_pretty_indent(val: JsonValue, indent: string) -> string
```

Возвращает отформатированную строку JSON типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.json: json_parse, json_pretty_indent
load std.io: println

fun main() {
    set doc = json_parse("{\"key\": \"val\"}")
    println(json_pretty_indent(doc, "    "))
}
```

### 6.7 Фабричные функции конструирования узлов

Модуль предоставляет набор функций для удобного создания узлов `JsonValue`:

- `json_null() -> JsonValue`: создает узел типа null.
- `json_bool(b: bool) -> JsonValue`: создает логический узел со значением `b`.
- `json_number(n: float64) -> JsonValue`: создает числовой узел со значением `n`.
- `json_int(n: int64) -> JsonValue`: создает числовой узел из целого числа `n`.
- `json_string(s: string) -> JsonValue`: создает строковый узел со значением `s`.
- `json_array(items: JsonValue[]) -> JsonValue`: создает массив из переданного списка элементов.
- `json_object() -> JsonValue`: создает пустой объект.

Все фабричные функции возвращают узел `JsonValue` и никогда не вызывают панику.

```yukino
load std.json: json_object, json_string, json_int, json_array, json_stringify
load std.io: println

fun main() {
    set root = json_object()
    root.set("service", json_string("gateway"))
    root.set("port", json_int(8080))
    set list = json_array([])
    list.push(json_string("api_v1"))
    root.set("routes", list)
    println(json_stringify(root))
}
```

---

[← Назад: Модуль std.sharedc](19-sharedc.md) | [Вперед: Модуль std.jsonc →](21-jsonc.md)
