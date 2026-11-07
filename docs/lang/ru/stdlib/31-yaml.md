# Модуль std.yaml

Модуль `std.yaml` предоставляет высокопроизводительные средства для синтаксического анализа, безопасного извлечения данных, валидации и сериализации структурированных документов в формате YAML (YAML Ain't Markup Language) v1.2 на чистом Yukino без сторонних зависимостей.

---

## 1. Подключение: load std.yaml

Импорт всего пакета:

```yukino
load std.yaml
```

При этом становятся доступны следующие сущности:

- **Константы типов узлов:** `YAML_NULL`, `YAML_BOOL`, `YAML_NUMBER`, `YAML_STRING`, `YAML_ARRAY`, `YAML_OBJECT`.
- **Структуры ошибок:** `YamlError`.
- **Классы:** `YamlValue`.
- **Функции парсинга и валидации:** `parse`, `try_parse`, `validate`, `yaml_parse`, `yaml_try_parse`, `yaml_validate`.
- **Функции сериализации:** `stringify`, `pretty`, `pretty_indent`, `yaml_stringify`, `yaml_pretty`, `yaml_pretty_indent`.
- **Фабричные функции конструирования:** `yaml_null`, `yaml_bool`, `yaml_number`, `yaml_int`, `yaml_float`, `yaml_string`, `yaml_array`, `yaml_object`, `yaml_mapping`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.yaml` объявлены целочисленные константы, определяющие тип узла в свойстве `kind` класса `YamlValue`:

```yukino
public static YAML_NULL: int32 = 0
public static YAML_BOOL: int32 = 1
public static YAML_NUMBER: int32 = 2
public static YAML_STRING: int32 = 3
public static YAML_ARRAY: int32 = 4
public static YAML_OBJECT: int32 = 5
```

Константы используются для прямой проверки типа узла `if val.kind == YAML_OBJECT`, хотя в прикладном коде предпочтительнее использовать встроенные методы-предикаты (`val.is_object()`, `val.is_array()`, `val.is_string()`, `val.is_null()` и другие).

---

## 3. Интерфейсы

В модуле `std.yaml` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 YamlError

Структура `YamlError` представляет ошибку синтаксического анализа документа YAML. Она генерируется через `panic` функциями `parse` и `yaml_parse` при обнаружении нарушений отступов, незакрытых скобок в flow-стиле, некорректных скаляров либо недопустимых разделителей.

```yukino
public struct YamlError {
    message: string
    line: int64
    column: int64
    offset: int64
}
```

- `message`: понятное текстовое описание ошибки (например, `"expected ':' after mapping key"`).
- `line`: номер строки в исходном тексте, где произошла ошибка (начиная с 1).
- `column`: номер колонки в строке (начиная с 1).
- `offset`: байтовое смещение от начала входной строки (начиная с 0).

Пример перехвата ошибки через конструкцию `sus / catch`:

```yukino
load std.yaml: parse, YamlError
load std.io: println

fun main() {
    sus {
        set doc = parse("unclosed: [1, 2")
    } catch err: YamlError {
        println("Ошибка YAML: ${err.message} на строке ${err.line}, колонка ${err.column}")
    }
}
```

---

## 5. Классы

### 5.1 YamlValue

Класс `YamlValue` представляет произвольный узел в дереве документа YAML (null, boolean, число, строку, последовательность/массив или отображение/объект). Класс спроектирован по принципу Fluent API и гарантирует безопасную навигацию по вложенным свойствам: цепочки вызовов никогда не вызывают панику при отсутствии ключа или несоответствии типов.

```yukino
public class YamlValue {
    public kind: int32
    public bool_val: bool
    public num_val: float64
    public str_val: string
    public array_val: YamlValue[]
    public obj_keys: string[]
    public obj_vals: YamlValue[]
}
```

#### Методы проверки типа (предикаты)

- `is_null() -> bool`: возвращает `true`, если узел равен `null` (или представляет отсутствующее значение).
- `is_bool() -> bool`: возвращает `true`, если узел является логическим значением.
- `is_number() -> bool`: возвращает `true`, если узел является числом (целым или с плавающей точкой).
- `is_string() -> bool`: возвращает `true`, если узел является строкой.
- `is_array() -> bool`: возвращает `true`, если узел является списком / последовательностью YAML.
- `is_object() -> bool`: возвращает `true`, если узел является ассоциативным отображением (маппингом) YAML.
- `is_map() -> bool`: синоним метода `is_object()`, возвращает `true` для отображений.

#### Методы извлечения значений

- `as_bool() -> bool`: возвращает булево значение. Если узел не является булевым, возвращает `false`.
- `as_bool_or(fallback: bool) -> bool`: возвращает булево значение либо `fallback`, если узел не является булевым.
- `as_int() -> int64`: возвращает целочисленное значение (число приводится к `int64`). Если узел не числовой, возвращает `0`.
- `as_int_or(fallback: int64) -> int64`: возвращает целое число либо `fallback`, если узел не числовой.
- `as_float() -> float64`: возвращает вещественное число `float64`. Если узел не числовой, возвращает `0.0`.
- `as_float_or(fallback: float64) -> float64`: возвращает число с плавающей точкой либо `fallback`.
- `as_string() -> string`: возвращает строку. Если узел не является строкой, возвращает пустую строку `""`.
- `as_string_or(fallback: string) -> string`: возвращает строку либо `fallback`, если узел не строковый.
- `as_array() -> YamlValue[]`: возвращает внутренний массив элементов `YamlValue[]`. Если узел не является последовательностью, возвращает пустой список.

#### Методы навигации по отображениям и последовательностям

- `get(key: string) -> YamlValue`: возвращает значение свойства отображения по имени `key`. Если ключ отсутствует или узел не является отображением, возвращает безопасный узел `YamlValue` типа null. Это позволяет строить цепочки вызовов вида `doc.get("server").get("host").as_string()`.
- `get_opt(key: string) -> YamlValue?`: возвращает nullable-значение `YamlValue?`, равное `null`, если ключ отсутствует.
- `has(key: string) -> bool`: возвращает `true`, если отображение содержит указанный ключ.
- `keys() -> string[]`: возвращает массив строковых ключей отображения.
- `at(index: int64) -> YamlValue`: возвращает элемент последовательности по индексу `index`. Если индекс выходит за границы или узел не является последовательностью, возвращает безопасный узел `YamlValue` типа null.
- `at_opt(index: int64) -> YamlValue?`: возвращает nullable-элемент `YamlValue?` по индексу либо `null` при выходе за границы.
- `len() -> int64`: возвращает количество элементов для последовательности, количество ключей для отображения либо длину строки в байтах для строкового узла. Для остальных типов возвращает `0`.

#### Методы мутации и построения

- `set(key: string, val: YamlValue) -> YamlValue`: добавляет или перезаписывает свойство отображения и возвращает `this` для цепочечных вызовов.
- `push(item: YamlValue) -> YamlValue`: добавляет элемент в конец последовательности и возвращает `this`.
- `cp() -> YamlValue`: выполняет глубокое клонирование всего дерева `YamlValue`.

---

## 6. Справочник функций

### 6.1 parse

Разбирает входную строку в формате YAML v1.2 и возвращает корневой узел `YamlValue`. Поддерживает блочную структуру с отступами, списки (`- item`), ассоциативные карты (`key: value`), flow-структуры (`[a, b]`, `{x: 1, y: 2}`), строки в кавычках и без кавычек, комментарии (`# comment`), а также явные литералы `null`, `true`, `false`.

```yukino
public fun parse(text: string) -> YamlValue
```

Возвращает разобранный корневой узел типа `YamlValue`. Возвращаемое значение не nullable.

В случае синтаксической ошибки, некорректных отступов или незакрытых литералов вызывает панику со структурой `YamlError`.

```yukino
load std.yaml: parse
load std.io: println

fun main() {
    set raw = "app: my_service\nport: 8080\nservices:\n  - web\n  - db\n"
    set doc = parse(raw)
    println("Приложение: " + doc.get("app").as_string())
    println("Порт: " + cast(doc.get("port").as_int(), "string"))
    println("Первый сервис: " + doc.get("services").at(0).as_string())
}
```

### 6.2 try_parse

Выполняет безопасную попытку синтаксического анализа строки YAML без генерации паники при синтаксических ошибках.

```yukino
public fun try_parse(text: string) -> YamlValue?
```

Возвращает узел `YamlValue` в случае успешного разбора либо `null`, если входной текст содержит синтаксические ошибки. Возвращаемое значение nullable.

Функция никогда не вызывает панику.

```yukino
load std.yaml: try_parse
load std.io: println

fun main() {
    set res = try_parse("broken: [1, 2")
    if res == null {
        println("Строка не является корректным YAML")
    }
}
```

### 6.3 validate

Проверяет, является ли переданная строка синтаксически корректным документом YAML.

```yukino
public fun validate(text: string) -> bool
```

Возвращает `true`, если строка валидна, либо `false` в противном случае. Возвращаемое значение типа `bool`, не nullable.

Функция никогда не вызывает панику.

```yukino
load std.yaml: validate
load std.io: println

fun main() {
    if validate("key: value\nflag: true\n") {
        println("YAML документ валиден")
    }
}
```

### 6.4 stringify

Выполняет сериализацию узла `YamlValue` в каноническую строку формата YAML с блочными отступами в 2 пробела.

```yukino
public fun stringify(val: YamlValue) -> string
```

Возвращает сериализованную строку YAML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.yaml: yaml_object, yaml_string, yaml_int, stringify
load std.io: println

fun main() {
    set doc = yaml_object()
    doc.set("name", yaml_string("production"))
    doc.set("replicas", yaml_int(3))
    println(stringify(doc))
}
```

### 6.5 pretty

Сериализует узел `YamlValue` в читаемую строку YAML со стандартным шагом отступа в 2 пробела на каждый уровень вложенности.

```yukino
public fun pretty(val: YamlValue) -> string
```

Возвращает отформатированную строку YAML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.yaml: parse, pretty
load std.io: println

fun main() {
    set doc = parse("server:\n  host: localhost\n  port: 80\n")
    println(pretty(doc))
}
```

### 6.6 pretty_indent

Сериализует узел `YamlValue` в форматированную строку YAML с пользовательской строкой отступа (например, 4 пробела).

```yukino
public fun pretty_indent(val: YamlValue, indent: string) -> string
```

Возвращает отформатированную строку YAML типа `string`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.yaml: parse, pretty_indent
load std.io: println

fun main() {
    set doc = parse("key: val\n")
    println(pretty_indent(doc, "    "))
}
```

### 6.7 Синонимы функций с префиксом yaml_

Для максимального удобства и полной совместимости со стилем `std.json` модуль предоставляет функции-синонимы с явным префиксом `yaml_`:

- `yaml_parse(text: string) -> YamlValue`: эквивалент `parse`.
- `yaml_try_parse(text: string) -> YamlValue?`: эквивалент `try_parse`.
- `yaml_validate(text: string) -> bool`: эквивалент `validate`.
- `yaml_stringify(val: YamlValue) -> string`: эквивалент `stringify`.
- `yaml_pretty(val: YamlValue) -> string`: эквивалент `pretty`.
- `yaml_pretty_indent(val: YamlValue, indent: string) -> string`: эквивалент `pretty_indent`.

Поведение, типы возвращаемых значений, условия паники и nullability полностью идентичны базовым функциям.

```yukino
load std.yaml: yaml_parse, yaml_stringify
load std.io: println

fun main() {
    set doc = yaml_parse("debug: true\n")
    println(yaml_stringify(doc))
}
```

### 6.8 Фабричные функции конструирования узлов

Модуль предоставляет полный набор фабричных функций для конструирования дерева `YamlValue`:

- `yaml_null() -> YamlValue`: создает узел типа null.
- `yaml_bool(b: bool) -> YamlValue`: создает логический узел со значением `b`.
- `yaml_number(n: float64) -> YamlValue`: создает числовой узел со значением `n`.
- `yaml_int(n: int64) -> YamlValue`: создает числовой узел из целого числа `n`.
- `yaml_float(f: float64) -> YamlValue`: создает числовой узел из вещественного числа `f`.
- `yaml_string(s: string) -> YamlValue`: создает строковый узел со значением `s`.
- `yaml_array(items: YamlValue[]) -> YamlValue`: создает последовательность из переданного списка элементов `items`.
- `yaml_object() -> YamlValue`: создает пустое отображение (маппинг).
- `yaml_mapping() -> YamlValue`: синоним `yaml_object()`, создает пустое отображение.

Все фабричные функции возвращают узел типа `YamlValue` (не nullable) и никогда не вызывают панику.

```yukino
load std.yaml: yaml_object, yaml_string, yaml_array, yaml_int, yaml_stringify
load std.io: println

fun main() {
    set root = yaml_object()
    root.set("cluster", yaml_string("primary"))
    set nodes = yaml_array([])
    nodes.push(yaml_string("node-1"))
    nodes.push(yaml_string("node-2"))
    root.set("nodes", nodes)
    println(yaml_stringify(root))
}
```

---

[← Назад: Модуль std.toml](30-toml.md) | [Вперед: Модуль std.sqlite →](32-sqlite.md)
