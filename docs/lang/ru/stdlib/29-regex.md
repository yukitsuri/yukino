# Модуль std.regex

Модуль `std.regex` предоставляет высокопроизводительный движок регулярных выражений с полной поддержкой стандарта Unicode и гарантией линейного времени выполнения без уязвимостей катастрофического отката (ReDoS). Модуль включает компиляцию шаблонов, флаги сопоставления, поиск совпадений и смещений, именованные и нумерованные группы захвата (`Captures`), замену подстрок с подстановкой групп (`$1`, `${name}`) и разбиение текста по регулярным выражениям.

---

## 1. Подключение: load std.regex

Импорт всего пакета:

```yukino
load std.regex
```

При импорте модуля становятся доступны следующие компоненты:

- **Константы:**
  - Флаги компиляции: `CASE_INSENSITIVE`, `MULTILINE`, `DOT_MATCHES_NEW_LINE`, `CRLF`, `SWAP_GREED`, `IGNORE_WHITESPACE`, `UNICODE`
- **Структуры данных и ошибок:**
  - `RegexError` — структура ошибки при некорректном синтаксисе регулярного выражения или невалидном хэндле
  - `Match` — результат найденного совпадения со смещениями и текстом
  - `CaptureGroup` — информация об отдельной именованной или нумерованной группе захвата
- **Классы:**
  - `Captures` — коллекция совпадений групп захвата с доступом по числовому индексу и имени
  - `Regex` — скомпилированное регулярное выражение с методами сопоставления, поиска, замены и разделения
- **Функции:**
  - Фабрики: `compile`, `compile_flags`
  - Утилиты сопоставления и поиска: `is_match`, `regex_find`, `regex_find_all`
  - Замена и разделение: `regex_replace`, `regex_replace_all`, `regex_split`
  - Экранирование: `escape`

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

### 2.1 Флаги компиляции

Битовые флаги для настройки поведения регулярного выражения при компиляции:

```yukino
public static CASE_INSENSITIVE: int32 = 1
public static MULTILINE: int32 = 2
public static DOT_MATCHES_NEW_LINE: int32 = 4
public static CRLF: int32 = 8
public static SWAP_GREED: int32 = 16
public static IGNORE_WHITESPACE: int32 = 32
public static UNICODE: int32 = 64
```

- `CASE_INSENSITIVE` (1) — регистронезависимое сопоставление (эквивалент флага `(?i)`).
- `MULTILINE` (2) — многострочный режим: метасимволы `^` и `$` сопоставляются с началом и концом каждой строки, а не всего текста (эквивалент флага `(?m)`).
- `DOT_MATCHES_NEW_LINE` (4) — символ точки `.` сопоставляется с любым символом, включая перевод строки `\n` (эквивалент флага `(?s)`).
- `CRLF` (8) — режим CRLF: распознавание последовательности `\r\n` как границы строк.
- `SWAP_GREED` (16) — инвертирование жадности квантификаторов: квантификаторы по умолчанию становятся ленивыми, а с суффиксом `?` — жадными.
- `IGNORE_WHITESPACE` (32) — игнорирование неэкранированных пробельных символов и поддержка комментариев `#` внутри шаблона (эквивалент флага `(?x)`).
- `UNICODE` (64) — явное включение поддержки классов символов Unicode (по умолчанию включена).

---

## 3. Интерфейсы

В модуле `std.regex` глобальные интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 RegexError

Структура ошибки, вызываемая через `panic` при ошибках компиляции некорректного синтаксиса шаблона или обращении к недействительному дескриптору.

```yukino
public struct RegexError {
    message: string
    code: int32
}
```

- `code = 101` — ошибка компиляции шаблона (незакрытые скобки, неверный синтаксис классов символов, недопустимый UTF-8);
- `code = 102` — дескриптор регулярного выражения закрыт или указывает на нулевой указатель.

### 4.2 Match

Структура, содержащая границы и текстовое содержимое совпадения в строке.

```yukino
public struct Match {
    start: int64
    end: int64
    text: string
}
```

- `start: int64` — начальное байтовое смещение совпадения в исходной строке (включительно);
- `end: int64` — конечное байтовое смещение совпадения в исходной строке (исключительно);
- `text: string` — подстрока совпадения.

### 4.3 CaptureGroup

Информация о состоянии отдельной группы захвата.

```yukino
public struct CaptureGroup {
    index: int32
    name: string
    start: int64
    end: int64
    text: string
    matched: bool
}
```

- `index: int32` — порядковый индекс группы захвата (0 для всего совпадения);
- `name: string` — имя группы для именованных групп (`(?P<name>...)`) либо пустая строка для безымянных;
- `start: int64` — начальное байтовое смещение (или `-1`, если группа не участвовала в совпадении);
- `end: int64` — конечное байтовое смещение (или `-1`, если группа не участвовала в совпадении);
- `text: string` — текст совпавшей группы либо пустая строка;
- `matched: bool` — флаг успешного участия группы в текущем совпадении.

---

## 5. Классы

### 5.1 class Captures

Коллекция групп захвата, сформированная в результате выполнения метода `captures`.

```yukino
public class Captures {
    public fun init(groups: CaptureGroup[])
    public fun len() -> int32
    public fun get(index: int32) -> Match?
    public fun get_by_name(name: string) -> Match?
    public fun all() -> Match[]
}
```

#### len

Возвращает общее количество групп захвата, объявленных в регулярном выражении, включая нулевую группу (полное совпадение).

```yukino
public fun len() -> int32
```

Возвращает неотрицательное целое число типа `int32`.

Метод никогда не вызывает панику.

#### get

Возвращает совпадение для группы с указанным числовым индексом `index`. Нулевой индекс соответствует полному совпадению.

```yukino
public fun get(index: int32) -> Match?
```

Возвращает структуру типа `Match` либо `null`, если индекс находится вне диапазона или группа не участвовала в совпадении (например, находилась в ненаступившей ветке чередования).

Метод никогда не вызывает панику.

#### get_by_name

Возвращает совпадение для именованной группы захвата по ее идентификатору `name`.

```yukino
public fun get_by_name(name: string) -> Match?
```

Возвращает структуру типа `Match` либо `null`, если группа с таким именем отсутствует в шаблоне или не участвовала в текущем совпадении.

Метод никогда не вызывает панику.

#### all

Возвращает список всех успешно сопоставленных групп в виде массива структур `Match`.

```yukino
public fun all() -> Match[]
```

Возвращает массив типа `Match[]`. Если совпадений нет, возвращает пустой массив.

Метод никогда не вызывает панику.

### 5.2 class Regex

Объектно-ориентированная обертка над скомпилированным регулярным выражением.

```yukino
public class Regex {
    public handle: ptr<void>
    public pattern: string
    public flags: int32
    public is_closed: bool

    public fun init(pattern: string, flags: int32 = 0)
    public fun is_match(text: string) -> bool
    public fun find(text: string) -> Match?
    public fun find_at(text: string, start_offset: int64) -> Match?
    public fun find_all(text: string) -> Match[]
    public fun captures(text: string) -> Captures?
    public fun replace(text: string, rep: string) -> string
    public fun replace_all(text: string, rep: string) -> string
    public fun split(text: string) -> string[]
    public fun free()
}
```

#### init

Создает и компилирует регулярное выражение с заданным шаблоном и битовой маской флагов.

```yukino
public fun init(pattern: string, flags: int32 = 0)
```

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

#### is_match

Проверяет, содержит ли переданный текст хотя бы одно совпадение с регулярным выражением.

```yukino
public fun is_match(text: string) -> bool
```

Возвращает логическое значение типа `bool`.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### find

Выполняет поиск первого вхождения регулярного выражения в переданном тексте.

```yukino
public fun find(text: string) -> Match?
```

Возвращает структуру типа `Match` либо `null`, если совпадение не обнаружено.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### find_at

Выполняет поиск первого вхождения регулярного выражения начиная с указанного байтового смещения `start_offset`.

```yukino
public fun find_at(text: string, start_offset: int64) -> Match?
```

Возвращает структуру типа `Match` либо `null`, если совпадение не найдено или смещение некорректно.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### find_all

Находит все непересекающиеся вхождения регулярного выражения в тексте.

```yukino
public fun find_all(text: string) -> Match[]
```

Возвращает динамический массив типа `Match[]`. Если совпадений нет, возвращает пустой массив.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### captures

Находит первое совпадение и формирует объект `Captures` для извлечения содержимого нумерованных и именованных подгрупп.

```yukino
public fun captures(text: string) -> Captures?
```

Возвращает экземпляр типа `Captures` либо `null`, если шаблон не совпал с текстом.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### replace

Заменяет первое совпадение в тексте строкой замены `rep`. Поддерживает подстановку групп через `$1` или `${name}`.

```yukino
public fun replace(text: string, rep: string) -> string
```

Возвращает новую строку типа `string`.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### replace_all

Заменяет все непересекающиеся совпадения в тексте строкой замены `rep`. Поддерживает подстановку групп через `$1` или `${name}`.

```yukino
public fun replace_all(text: string, rep: string) -> string
```

Возвращает новую строку типа `string`.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### split

Разбивает текст на части по разделителям, соответствующим регулярному выражению.

```yukino
public fun split(text: string) -> string[]
```

Возвращает массив строк типа `string[]`.

В случае закрытого или недействительного дескриптора вызывает панику со структурой `RegexError` (код 102).

#### free

Освобождает внутренний нативный дескриптор скомпилированного регулярного выражения в памяти.

```yukino
public fun free()
```

Возвращаемого значения нет (`void`).

Функция никогда не вызывает панику.

---

## 6. Справочник функций

### 6.1 compile

Компилирует регулярное выражение со стандартными флагами.

```yukino
public fun compile(pattern: string) -> Regex
```

Возвращает готовый экземпляр класса `Regex`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: compile, Regex

fun main() {
    set re: Regex = compile("\\d+")
    println(cast(re.is_match("order 1234"), "string"))
    re.free()
}
```

### 6.2 compile_flags

Компилирует регулярное выражение с указанной комбинацией битовых флагов.

```yukino
public fun compile_flags(pattern: string, flags: int32) -> Regex
```

Возвращает готовый экземпляр класса `Regex`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: compile_flags, Regex, CASE_INSENSITIVE

fun main() {
    set re: Regex = compile_flags("error", CASE_INSENSITIVE)
    println(cast(re.is_match("FATAL ERROR OCCURRED"), "string"))
    re.free()
}
```

### 6.3 is_match

Утилитарная функция для быстрой однократной проверки соответствия текста регулярному выражению.

```yukino
public fun is_match(pattern: string, text: string) -> bool
```

Возвращает логическое значение типа `bool`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: is_match

fun main() {
    set ok: bool = is_match("^user_[a-z0-9]+$", "user_admin1")
    println(cast(ok, "string"))
}
```

### 6.4 regex_find

Находит первое совпадение регулярного выражения в тексте.

```yukino
public fun regex_find(pattern: string, text: string) -> Match?
```

Возвращает структуру типа `Match` либо `null`, если совпадение не обнаружено.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: regex_find, Match

fun main() {
    set m: Match? = regex_find("\\d+", "Item count: 42 pieces")
    if m != null {
        set item: Match = m ?? Match { start: 0, end: 0, text: "" }
        println("Found: " + item.text)
    }
}
```

### 6.5 regex_find_all

Находит все непересекающиеся вхождения регулярного выражения в строке.

```yukino
public fun regex_find_all(pattern: string, text: string) -> Match[]
```

Возвращает массив структур типа `Match[]`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: regex_find_all, Match

fun main() {
    set matches: Match[] = regex_find_all("\\b[A-Z]{3}\\b", "USD, EUR and GBP")
    set i: int64 = 0
    while i < matches.len() {
        println(matches[i].text)
        rewrite i = i + 1
    }
}
```

### 6.6 regex_replace

Заменяет первое совпадение шаблона в строке `text` на строку замены `rep`.

```yukino
public fun regex_replace(pattern: string, text: string, rep: string) -> string
```

Возвращает модифицированную строку типа `string`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: regex_replace

fun main() {
    set res: string = regex_replace("\\d+", "port 8080 active", "9000")
    println(res) // port 9000 active
}
```

### 6.7 regex_replace_all

Заменяет все непересекающиеся совпадения шаблона в строке `text` на строку замены `rep`.

```yukino
public fun regex_replace_all(pattern: string, text: string, rep: string) -> string
```

Возвращает модифицированную строку типа `string`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: regex_replace_all

fun main() {
    set res: string = regex_replace_all("\\s+", "a   b   c", " ")
    println(res) // a b c
}
```

### 6.8 regex_split

Разбивает строку на части по регулярному выражению `pattern`.

```yukino
public fun regex_split(pattern: string, text: string) -> string[]
```

Возвращает массив строк типа `string[]`.

В случае синтаксической ошибки в шаблоне вызывает панику со структурой `RegexError` (код 101).

```yukino
load std.io: println
load std.regex: regex_split

fun main() {
    set parts: string[] = regex_split("[,;]\\s*", "apple, orange; banana")
    set i: int64 = 0
    while i < parts.len() {
        println(parts[i])
        rewrite i = i + 1
    }
}
```

### 6.9 escape

Экранирует все метасимволы регулярного выражения в переданной строке, позволяя безопасно вставлять пользовательский текст в динамические шаблоны.

```yukino
public fun escape(s: string) -> string
```

Возвращает экранированную строку типа `string`.

Функция никогда не вызывает панику.

```yukino
load std.io: println
load std.regex: escape

fun main() {
    set safe_pattern: string = escape("price ($USD)")
    println(safe_pattern) // price \(\$USD\)
}
```

---

[← Назад: Модуль std.net](28-net.md) | [Вперед: Модуль std.toml →](30-toml.md)
