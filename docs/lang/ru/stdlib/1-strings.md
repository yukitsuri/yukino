# Модуль std.strings

Модуль `std.strings` предоставляет высокоэффективный набор функций и структур для работы со строками: манипуляции с подстроками, поиск, сравнение, преобразование регистра, форматирование, дополнение, разбиение, объединение и парсинг примитивных типов данных. Все функции чтения и преобразования принимают строки по значению (`s: string`), так как строки в Yukino являются легковесными 16-байтными Copy-дескрипторами `{ ptr, len }`, передаваемыми непосредственно в регистрах процессора без выделения памяти на куче.

---

## 1. Подключение: load std.strings

Импорт всего пакета:

```yukino
load std.strings
```

В текущую область видимости импортируются все публичные элементы модуля `std.strings`:

- **Структуры ошибок:**
  - `StringIndexError` — ошибка выхода индекса за границы строки (код `1`);
  - `StringParseError` — ошибка парсинга примитивных типов (коды `10`, `11`, `12`).
- **Функции работы со строками:**
  - *Длина и проверка на пустоту:* `utflen`, `is_empty`
  - *Доступ по индексам:* `byte_at`, `char_at`
  - *Срезы и подстроки:* `slice`
  - *Сравнение:* `eq`, `ne`, `compare`
  - *Поиск и вхождение:* `starts_with`, `ends_with`, `contains`, `index_of`, `index_of_from`, `last_index_of`, `count`
  - *Очистка и дополнение:* `trim`, `trim_start`, `trim_end`, `pad_start`, `pad_end`
  - *Регистр и разворот:* `upper`, `lower`, `reverse`
  - *Замена, повторение и объединение:* `replace`, `repeat`, `split`, `lines`, `str_join`
  - *Безопасный парсинг (возврат nullable):* `parse_int`, `parse_float`
  - *Строгий парсинг (с паникой при ошибке):* `to_int64`, `to_float64`, `to_bool`, `to_string`

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.strings` глобальные константы не объявлены. Все операции выполняются через функции и структуры ошибок.

---

## 3. Структуры данных и ошибок (паники)

Модуль определяет две типизированные структуры ошибок, которые могут передаваться в функцию `panic` при возникновении исключительных ситуаций.

### 3.1 StringIndexError

Вызывается при попытке доступа к индексу за пределами границ строки.

```yukino
public struct StringIndexError {
    message: string
    code: int32
}
```

- `message` — текстовое описание ошибки (`"string index out of bounds"`).
- `code` — числовой код ошибки: `1`.

### 3.2 StringParseError

Вызывается при невозможности распарсить строковое представление в целевой примитивный тип при строгом парсинге.

```yukino
public struct StringParseError {
    message: string
    code: int32
}
```

- `message` — текстовое описание ошибки (например, `"cannot parse string as int64"`).
- `code` — числовой код ошибки:
  - `10` — ошибка парсинга `int64`;
  - `11` — ошибка парсинга `float64`;
  - `12` — ошибка парсинга `bool`.

---

## 4. Справочник функций

### 4.1 utflen

Возвращает количество символов (Unicode codepoints) в UTF-8 строке `s`. Для получения сырого размера в байтах используйте встроенный метод `.len()`.

```yukino
public fun utflen(s: string) -> int64
```

Возвращает количество Unicode-символов как `int64`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: utflen
load std.io: println

set str = "Привет, мир!"
set count = utflen(str)
println("Символов: ${count}")
```

### 4.2 is_empty

Проверяет, является ли строка `s` пустой (ее байтовая длина равна нулю).

```yukino
public fun is_empty(s: string) -> bool
```

Возвращает логическое значение типа `bool`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: is_empty
load std.io: println

set str = ""
set empty = is_empty(str)
println("Пустая: ${empty}")
```

### 4.3 byte_at

Возвращает байт по указанному нулевому индексу `index` в строке `s`.

```yukino
public fun byte_at(s: string, index: int64) -> uint8
```

Возвращает байтовое числовое значение как `uint8`. Не является nullable.

Если индекс `index < 0` или `index >= s.len()`, вызывает панику со структурой `StringIndexError` (код `1`).

```yukino
load std.strings: byte_at, StringIndexError
load std.io: println

set word = "Yukino"
set code = byte_at(word, 0)
println("Байт: ${code}")
```

### 4.4 char_at

Извлекает байт по указанному нулевому индексу `index` и возвращает его в виде символа типа `char`.

```yukino
public fun char_at(s: string, index: int64) -> char
```

Возвращает символ типа `char`. Не является nullable.

Если индекс выходит за допустимые границы строки `[0, s.len() - 1]`, вызывает панику со структурой `StringIndexError` (код `1`).

```yukino
load std.strings: char_at
load std.io: println

set word = "Hello"
set ch = char_at(word, 0)
println("Первый символ: ${ch}")
```

### 4.5 slice

Возвращает срез подстроки от индекса `start` (включительно) до `end` (исключительно). Границы индексов автоматически зажимаются в допустимый диапазон строки: `start` приводится к `[0, len]`, а `end` — к `[start, len]`.

```yukino
public fun slice(s: string, start: int64, end: int64) -> string
```

Возвращает подстроку типа `string`. Если длина среза неположительна, возвращает пустую строку `""`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: slice
load std.io: println

set text = "compiler"
set sub = slice(text, 0, 4)
println("Срез: ${sub}")
```

### 4.6 eq

Проверяет побайтовое равенство содержимого двух строк `a` и `b`.

```yukino
public fun eq(a: string, b: string) -> bool
```

Возвращает `true`, если строки имеют равную длину и идентичные байты, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: eq
load std.io: println

set first = "status"
set second = "status"
set match = eq(first, second)
println("Равны: ${match}")
```

### 4.7 ne

Проверяет побайтовое неравенство содержимого двух строк `a` и `b`.

```yukino
public fun ne(a: string, b: string) -> bool
```

Возвращает `true`, если строки отличаются по длине или побайтовому составу, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: ne
load std.io: println

set first = "draft"
set second = "published"
set diff = ne(first, second)
println("Различаются: ${diff}")
```

### 4.8 compare

Выполняет побайтовое лексикографическое сравнение строк `a` и `b`.

```yukino
public fun compare(a: string, b: string) -> int32
```

Возвращает `-1`, если `a < b`; `0`, если строки побайтово равны `a == b`; `1`, если `a > b`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: compare
load std.io: println

set first = "apple"
set second = "banana"
set cmp = compare(first, second)
println("Результат сравнения: ${cmp}")
```

### 4.9 starts_with

Проверяет, начинается ли строка `s` с префикса `prefix`. Если префикс `prefix` пустой `""`, возвращает `null`. Если длина префикса превышает длину строки, возвращает `false`.

```yukino
public fun starts_with(s: string, prefix: string) -> bool?
```

Возвращает `true`, если строка начинается с указанного префикса, `false`, если не совпадает, либо `null`, если префикс пустой `""`. Является nullable (`bool?`).

Функция никогда не вызывает панику.

```yukino
load std.strings: starts_with
load std.io: println

set text = "yukic-driver"
set ok = starts_with(text, "yukic")
println("Префикс совпал: ${ok}")
```

### 4.10 ends_with

Проверяет, оканчивается ли строка `s` суффиксом `suffix`. Если суффикс `suffix` пустой `""`, возвращает `null`. Если длина суффикса превышает длину строки, возвращает `false`.

```yukino
public fun ends_with(s: string, suffix: string) -> bool?
```

Возвращает `true`, если строка оканчивается указанным суффиксом, `false`, если не совпадает, либо `null`, если суффикс пустой `""`. Является nullable (`bool?`).

Функция никогда не вызывает панику.

```yukino
load std.strings: ends_with
load std.io: println

set text = "main.ykn"
set ok = ends_with(text, ".ykn")
println("Суффикс совпал: ${ok}")
```

### 4.11 contains

Проверяет, содержится ли подстрока `sub` внутри строки `s`.

```yukino
public fun contains(s: string, sub: string) -> bool
```

Возвращает `true`, если подстрока найдена, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: contains
load std.io: println

set text = "standard library"
set has = contains(text, "lib")
println("Содержит: ${has}")
```

### 4.12 index_of

Возвращает нулевой байтовый индекс первого вхождения подстроки `sub` в строке `s`, либо `null`, если подстрока не найдена.

```yukino
public fun index_of(s: string, sub: string) -> int64?
```

Возвращает нулевой индекс первого вхождения как `int64`, либо `null`, если подстрока не найдена в `s`. Поиск пустой подстроки `""` возвращает `0`. Является nullable (`int64?`).

Функция никогда не вызывает панику.

```yukino
load std.strings: index_of
load std.io: println

set text = "banana"
set idx = index_of(text, "na")
println("Индекс: ${idx}")
```

### 4.13 index_of_from

Возвращает нулевой байтовый индекс первого вхождения подстроки `sub` в строке `s`, начиная поиск со смещения `from_idx`, либо `null`, если подстрока не найдена.

```yukino
public fun index_of_from(s: string, sub: string, from_idx: int64) -> int64?
```

Возвращает нулевой индекс вхождения как `int64`, либо `null`, если подстрока не найдена или `from_idx >= s.len()`. Если `from_idx < 0`, поиск начинается с индекса `0`. Является nullable (`int64?`).

Функция никогда не вызывает панику.

```yukino
load std.strings: index_of_from
load std.io: println

set text = "banana"
set idx = index_of_from(text, "na", 3)
println("Индекс с позиции 3: ${idx}")
```

### 4.14 last_index_of

Возвращает нулевой байтовый индекс последнего вхождения подстроки `sub` в строке `s`, либо `null`, если подстрока не найдена.

```yukino
public fun last_index_of(s: string, sub: string) -> int64?
```

Возвращает нулевой индекс как `int64`, либо `null`, если подстрока не найдена. Поиск пустой подстроки `""` возвращает `s.len()`. Является nullable (`int64?`).

Функция никогда не вызывает панику.

```yukino
load std.strings: last_index_of
load std.io: println

set path = "src/driver/main.ykn"
set last = last_index_of(path, "/")
println("Последний слеш: ${last}")
```

### 4.15 count

Подсчитывает количество непересекающихся вхождений подстроки `sub` в строке `s`.

```yukino
public fun count(s: string, sub: string) -> int64
```

Возвращает количество вхождений как `int64`. Если `s` или `sub` пустые, либо длина `sub` превышает длину `s`, возвращает `0`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: count
load std.io: println

set text = "ab_ab_ab"
set n = count(text, "ab")
println("Вхождений: ${n}")
```

### 4.16 trim

Удаляет все начальные и конечные пробельные символы (пробелы, символы табуляции, переводы строк, возвраты каретки и прочие пробельные символы Unicode) с обоих концов строки `s`.

```yukino
public fun trim(s: string) -> string
```

Возвращает очищенную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: trim
load std.io: println

set raw = "  hello world  \n"
set clean = trim(raw)
println("Результат: '${clean}'")
```

### 4.17 trim_start

Удаляет пробельные символы только в начале строки `s`. Пробельные символы в конце строки остаются без изменений.

```yukino
public fun trim_start(s: string) -> string
```

Возвращает строку `string` без ведущих пробелов. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: trim_start
load std.io: println

set raw = "   start_only   "
set clean = trim_start(raw)
println("Результат: '${clean}'")
```

### 4.18 trim_end

Удаляет пробельные символы только в конце строки `s`. Пробельные символы в начале строки остаются без изменений.

```yukino
public fun trim_end(s: string) -> string
```

Возвращает строку `string` без замыкающих пробелов. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: trim_end
load std.io: println

set raw = "   end_only   "
set clean = trim_end(raw)
println("Результат: '${clean}'")
```

### 4.19 upper

Преобразует все символы строки `s` в верхний регистр с корректной обработкой правил UTF-8.

```yukino
public fun upper(s: string) -> string
```

Возвращает новую строку `string` в верхнем регистре. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: upper
load std.io: println

set text = "yukino"
set s = upper(text)
println("Верхний регистр: ${s}")
```

### 4.20 lower

Преобразует все символы строки `s` в нижний регистр с корректной обработкой правил UTF-8.

```yukino
public fun lower(s: string) -> string
```

Возвращает новую строку `string` в нижнем регистре. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: lower
load std.io: println

set text = "YUKINO"
set s = lower(text)
println("Нижний регистр: ${s}")
```

### 4.21 replace

Заменяет все непересекающиеся вхождения подстроки `from_sub` на подстроку `to_sub` в строке `s`. Если `from_sub` не содержится в строке, возвращает исходную строку. Пустая `from_sub` соответствует каждой позиции строки: `to_sub` вставляется в начало и конец строки, а также между каждой парой соседних символов.

```yukino
public fun replace(s: string, from_sub: string, to_sub: string) -> string
```

Возвращает преобразованную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: replace
load std.io: println

set text = "one apple, two apples"
set res = replace(text, "apple", "orange")
println("Результат: ${res}")
```

### 4.22 repeat

Формирует новую строку путем повторения исходной строки `s` указанное количество раз `count_num`. Если `count_num <= 0` или строка пуста, возвращает пустую строку `""`.

```yukino
public fun repeat(s: string, count_num: int64) -> string
```

Возвращает повторенную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: repeat
load std.io: println

set dash = "-"
set border = repeat(dash, 5)
println("Линия: ${border}")
```

### 4.23 reverse

Возвращает строку, в которой символы расположены в обратном порядке с корректным сохранением валидности символов Unicode.

```yukino
public fun reverse(s: string) -> string
```

Возвращает развернутую строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: reverse
load std.io: println

set text = "Привет"
set rev = reverse(text)
println("Разворот: ${rev}")
```

### 4.24 pad_start

Дополняет строку `s` слева подстрокой `pad` до достижения целевой длины `target_len`. Аргумент `pad` имеет значение по умолчанию `" "` (одиночный пробел). Если исходная длина строки больше или равна `target_len`, либо длина подстроки заполнителя равна нулю, строка возвращается без изменений.

```yukino
public fun pad_start(s: string, target_len: int64, pad: string = " ") -> string
```

Возвращает дополненную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: pad_start
load std.io: println

set text = "42"
set num = pad_start(text, 5, "0")
println("Результат: ${num}")
```

### 4.25 pad_end

Дополняет строку `s` справа подстрокой `pad` до достижения целевой длины `target_len`. Аргумент `pad` имеет значение по умолчанию `" "` (одиночный пробел). Если исходная длина строки уже не меньше `target_len`, либо длина заполнителя равна нулю, строка возвращается без изменений.

```yukino
public fun pad_end(s: string, target_len: int64, pad: string = " ") -> string
```

Возвращает дополненную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: pad_end
load std.io: println

set text = "abc"
set padded = pad_end(text, 6)
println("Результат: '${padded}'")
```

### 4.26 split

Разбивает строку `s` на массив подстрок `string[]` по разделителю `delimiter`. Если строка пуста, возвращает пустой массив. Если разделитель пуст, возвращает массив из одного элемента со всей строкой `s`.

```yukino
public fun split(s: string, delimiter: string) -> string[]
```

Возвращает массив строк `string[]`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: split
load std.io: println

set fruits = "apple,orange,banana"
set tokens = split(fruits, ",")
println("Элементов: ${tokens.len()}")
```

### 4.27 lines

Разбивает многострочный текст `s` на массив отдельных строк `string[]`. В качестве разделителей строк распознаются стандартные последовательности перевода строк: LF (`\n`) и CRLF (`\r\n`). Сами разделители строк исключаются из результирующих строк.

```yukino
public fun lines(s: string) -> string[]
```

Возвращает массив строк `string[]`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: lines
load std.io: println

set poem = "первая строка\nвторая строка\nтретья строка"
set rows = lines(poem)
println("Строк: ${rows.len()}")
```

### 4.28 str_join

Объединяет элементы массива строк `items` в единую строку, вставляя между элементами разделитель `delimiter`. По умолчанию `delimiter` равен пустой строке `""`. Если массив пуст, возвращает пустую строку `""`.

```yukino
public fun str_join(items: string[], delimiter: string = "") -> string
```

Возвращает объединенную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: str_join
load std.io: println

set elements = ["yukic", "span", "parser"]
set joined = str_join(elements, " -> ")
println("Результат: ${joined}")
```

### 4.29 parse_int

Пытается распарсить строку `s` в 64-битное знаковое целое число. Ведущие и замыкающие пробелы игнорируются.

```yukino
public fun parse_int(s: string) -> int64?
```

Возвращает распарсенное число типа `int64` либо `null`, если строка имеет некорректный формат или выходит за пределы диапазона.

Функция никогда не вызывает панику.

```yukino
load std.strings: parse_int
load std.io: println

set text = "12345"
set num = parse_int(text)
if num != null {
    println("Число: ${num ?? 0}")
}
```

### 4.30 parse_float

Пытается распарсить строку `s` в вещественное число двойной точности `float64`. Ведущие и замыкающие пробелы игнорируются.

```yukino
public fun parse_float(s: string) -> float64?
```

Возвращает распарсенное вещественное число типа `float64` либо `null`, если строка имеет некорректный формат.

Функция никогда не вызывает панику.

```yukino
load std.strings: parse_float
load std.io: println

set text = "3.1415"
set val = parse_float(text)
if val != null {
    println("Значение: ${val ?? 0.0}")
}
```

### 4.31 to_int64

Парсит строку `s` в 64-битное знаковое целое число `int64`. Ведущие и замыкающие пробельные символы предварительно обрезаются.

```yukino
public fun to_int64(s: string) -> int64
```

Возвращает распарсенное число типа `int64`. Не является nullable.

Если строка не содержит валидного представления целого числа или выходит за границы диапазона `int64`, функция генерирует панику со структурой `StringParseError` (код `10`).

```yukino
load std.strings: to_int64, StringParseError
load std.io: println

sus {
    set text = "   42000   "
    set number = to_int64(text)
    println("Число: ${number}")
} catch e: StringParseError {
    println("Ошибка: ${e.message}")
}
```

### 4.32 to_float64

Парсит строку `s` в вещественное число двойной точности `float64`. Ведущие и замыкающие пробелы обрезаются.

```yukino
public fun to_float64(s: string) -> float64
```

Возвращает распарсенное вещественное число типа `float64`. Не является nullable.

Если формат строкового числа невалиден, функция генерирует панику со структурой `StringParseError` (код `11`).

```yukino
load std.strings: to_float64, StringParseError
load std.io: println

sus {
    set text = "3.14159265"
    set val = to_float64(text)
    println("Значение: ${val}")
} catch e: StringParseError {
    println("Ошибка: ${e.message}")
}
```

### 4.33 to_bool

Парсит логическое значение из строки `s` (после автоматической обрезки пробелов). Значения `"true"`, `"1"`, `"yes"`, `"on"` возвращают `true`. Значения `"false"`, `"0"`, `"no"`, `"off"` возвращают `false`.

```yukino
public fun to_bool(s: string) -> bool
```

Возвращает логическое значение типа `bool`. Не является nullable.

Для любых других строковых значений вызывает панику со структурой `StringParseError` (код `12`).

```yukino
load std.strings: to_bool, StringParseError
load std.io: println

sus {
    set text = "yes"
    set flag = to_bool(text)
    if flag {
        println("Флаг установлен в true")
    }
} catch e: StringParseError {
    println("Ошибка: ${e.message}")
}
```

### 4.34 to_string

Тождественная функция строкового преобразования: принимает строку `s` и возвращает ее значение. Полезна для передачи в качестве колбэка или в контекстах приведения типов.

```yukino
public fun to_string(s: string) -> string
```

Возвращает переданную строку `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.strings: to_string
load std.io: println

set text = "hello"
set s = to_string(text)
println("Строка: ${s}")
```

---

[← Назад: Обзор стандартной библиотеки](0-overview.md) | [Вперед: Модуль std.io →](2-io.md)
