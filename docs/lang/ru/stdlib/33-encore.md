# Модуль std.encore

Модуль `std.encore` предоставляет набор инструментов для низкоуровневой и прикладной работы с текстом в кодировках Unicode: валидацию последовательностей UTF-8, разбор и кодирование скалярных значений Unicode (codepoints), сегментацию расширенных кластеров графем по стандарту UAX #29 (включая комбинируемые диакритические знаки и составные последовательности ZWJ), а также двустороннее преобразование в UTF-16 с поддержкой суррогатных пар для прямого взаимодействия с Win32 API (`wchar_t*`, `LPCWSTR`) и средой виртуальных машин JVM (`jchar[]`).

---

## 1. Подключение: load std.encore

Импорт всего пакета:

```yukino
load std.encore
```

В текущую область видимости импортируются все публичные элементы модуля `std.encore`:

- **Структуры ошибок:**
  - `UtfError` — ошибка некорректной байтовой последовательности UTF-8, недопустимого скалярного значения codepoint или поврежденной суррогатной пары UTF-16 (коды `101`, `102`, `103`).
- **Структуры данных:**
  - `CodepointEntry` — дескриптор отдельного Unicode codepoint с его числовым значением, байтовым смещением и длиной в байтах;
  - `GraphemeCluster` — дескриптор кластера графем с подстрокой, байтовым смещением и длиной в исходном тексте.
- **Функции UTF-8:**
  - *Валидация и декодирование:* `is_valid_utf8`, `is_valid_utf8_raw`, `validate_utf8`, `from_utf8_lossy`.
- **Функции Unicode Codepoints:**
  - *Итерация и подсчет:* `count_codepoints`, `decode_codepoints`, `codepoints`;
  - *Кодирование:* `encode_codepoint`, `encode_codepoints`.
- **Функции кластеров графем (UAX #29):**
  - *Сегментация и выборка:* `count_graphemes`, `graphemes`, `grapheme_clusters`, `grapheme_at`.
- **Функции UTF-16 (Win32 / JVM Interop):**
  - *Кодирование в UTF-16:* `to_utf16`, `to_utf16_null_terminated`;
  - *Декодирование из UTF-16:* `from_utf16`, `from_utf16_lossy`, `from_utf16_null_terminated`;
  - *Суррогатные пары:* `is_high_surrogate`, `is_low_surrogate`, `is_surrogate`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.encore` глобальные константы не объявлены. Все операции выполняются через функции, структуры данных и структуры ошибок.

---

## 3. Интерфейсы

В модуле `std.encore` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 UtfError

Структура ошибки, передаваемая в `panic` при обнаружении недопустимых байтовых последовательностей UTF-8, некорректных скаляров или ошибок декодирования суррогатных пар UTF-16.

```yukino
public struct UtfError {
    message: string
    code: int32
}
```

- `message` — текстовое описание причины ошибки;
- `code` — числовой код ошибки:
  - `101` — недопустимая последовательность байтов UTF-8 при строгой валидации;
  - `102` — некорректное скалярное значение Unicode (вне диапазона `0x0000..0x10FFFF` либо диапазон суррогатов `0xD800..0xDFFF`);
  - `103` — некорректная последовательность суррогатов UTF-16 при строгом декодировании.

### 4.2 CodepointEntry

Описывает отдельный Unicode codepoint внутри строки с указанием его позиции и размера.

```yukino
public struct CodepointEntry {
    codepoint: uint32
    byte_offset: int64
    byte_len: int32
}
```

- `codepoint` — 32-битное беззнаковое числовое значение символа Unicode;
- `byte_offset` — байтовое смещение начала символа от начала строки;
- `byte_len` — длина UTF-8 представления символа в байтах (от 1 до 4).

### 4.3 GraphemeCluster

Описывает неделимый кластер графем (пользовательский символ) согласно стандарту Unicode UAX #29.

```yukino
public struct GraphemeCluster {
    text: string
    byte_offset: int64
    byte_len: int32
}
```

- `text` — подстрока, содержащая полный кластер графем;
- `byte_offset` — начальное байтовое смещение кластера в исходной строке;
- `byte_len` — полная байтовая длина кластера.

---

## 5. Классы

В модуле `std.encore` классы не объявлены.

---

## 6. Справочник функций

### 6.1 is_valid_utf8

Проверяет, является ли переданный срез байтов `bytes` корректной последовательностью UTF-8.

```yukino
public fun is_valid_utf8(ref bytes: bytes) -> bool
```

Возвращает логическое значение `true`, если все байты образуют валидный UTF-8 поток, либо `false` при наличии поврежденных или незавершенных последовательностей. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: is_valid_utf8
load std.io: println

set raw: bytes = cast("Yukino text", "bytes")
if is_valid_utf8(ref raw) {
    println("Valid UTF-8 sequence")
}
```

### 6.2 is_valid_utf8_raw

Проверяет корректность UTF-8 последовательности по сырому указателю `ptr<uint8>` и заданной длине `len` в байтах. Используется в сценариях системного FFI и работы с памятью C ABI.

```yukino
public fun is_valid_utf8_raw(p: ptr<uint8>, len: int64) -> bool
```

Возвращает `true`, если блок памяти длины `len` содержит корректный UTF-8, иначе `false`. Если передан `null`, возвращает `true` только при `len == 0`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: is_valid_utf8_raw
load std.io: println

set text = "Safety check"
set ok = is_valid_utf8_raw(text.as_ptr(), text.len())
println("Pointer valid: " + cast(ok, "string"))
```

### 6.3 validate_utf8

Строго валидирует переданный срез байтов `bytes`. При успехе возвращает результирующую строку `string` без лишнего копирования данных.

```yukino
public fun validate_utf8(ref bytes: bytes) -> string
```

Возвращает валидную строку типа `string`. Не является nullable.

При обнаружении недопустимой последовательности UTF-8 функция вызывает панику со структурой `UtfError` (код `101`).

```yukino
load std.encore: validate_utf8
load std.io: println

set raw: bytes = cast("Hello world", "bytes")
set str = validate_utf8(ref raw)
println("Validated text: " + str)
```

### 6.4 from_utf8_lossy

Выполняет мягкое (lossy) преобразование байтов `bytes` в строку `string`. Любые некорректные или разорванные последовательности байтов заменяются стандартным замещающим символом Unicode `U+FFFD`.

```yukino
public fun from_utf8_lossy(ref bytes: bytes) -> string
```

Возвращает строку типа `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: from_utf8_lossy
load std.io: println

set raw_buf: uint8[] = []
raw_buf.push(0xFFu8)
raw_buf.push(0xFEu8)
set raw_bytes: bytes = from_raw_parts(raw_buf.as_ptr(), 2, 2)

set safe_str = from_utf8_lossy(ref raw_bytes)
println("Lossy result length: " + cast(safe_str.len(), "string"))
```

### 6.5 codepoints

Разбирает строку `s` на массив подробных дескрипторов `CodepointEntry`, содержащих числовой код, байтовое смещение и размер каждого символа.

```yukino
public fun codepoints(s: string) -> CodepointEntry[]
```

Возвращает массив структур `CodepointEntry[]`. Если строка пуста, возвращает пустой срез. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: codepoints
load std.io: println

set entries = codepoints("A-B")
for i in 0..entries.len() {
    set e = entries[i]
    println("Codepoint: " + cast(e.codepoint, "string") + " at offset: " + cast(e.byte_offset, "string"))
}
```

### 6.6 decode_codepoints

Извлекает из строки `s` плоский массив 32-битных кодов символов Unicode (`uint32[]`).

```yukino
public fun decode_codepoints(s: string) -> uint32[]
```

Возвращает массив скалярных значений `uint32[]`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: decode_codepoints
load std.io: println

set codes = decode_codepoints("Yukino")
for i in 0..codes.len() {
    println("Char code: " + cast(codes[i], "string"))
}
```

### 6.7 encode_codepoint

Кодирует отдельный 32-битный Unicode codepoint `cp` в строку UTF-8 размером от 1 до 4 байтов.

```yukino
public fun encode_codepoint(cp: uint32) -> string
```

Возвращает односимвольную строку типа `string`. Не является nullable.

При передаче значения больше `0x10FFFF` либо из суррогатного диапазона `0xD800..0xDFFF` вызывает панику со структурой `UtfError` (код `102`).

```yukino
load std.encore: encode_codepoint
load std.io: println

set letter_a = encode_codepoint(65u32)
println("Encoded: " + letter_a)
```

### 6.8 encode_codepoints

Кодирует массив 32-битных скаляров `cps` в единую UTF-8 строку. Если массив содержит некорректные скаляры, они заменяются на `U+FFFD`.

```yukino
public fun encode_codepoints(ref cps: uint32[]) -> string
```

Возвращает собранную строку типа `string`. Если передан пустой массив, возвращает пустую строку `""`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: encode_codepoints
load std.io: println

set list: uint32[] = [72u32, 105u32, 33u32]
set res = encode_codepoints(ref list)
println("Word: " + res)
```

### 6.9 count_codepoints

Вычисляет общее количество скалярных символов Unicode (codepoints) в строке `s`.

```yukino
public fun count_codepoints(s: string) -> int64
```

Возвращает количество символов как `int64`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: count_codepoints
load std.io: println

set count = count_codepoints("Привет")
println("Количество символов: " + cast(count, "string"))
```

### 6.10 graphemes

Выполняет сегментацию строки `s` на видимые кластеры графем (пользовательские символы) согласно алгоритму UAX #29 и возвращает массив подстрок `string[]`.

```yukino
public fun graphemes(s: string) -> string[]
```

Возвращает массив строк `string[]`, где каждый элемент представляет один кластер графем. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: graphemes
load std.io: println

set text = "e\u{0301}"
set list = graphemes(text)
println("Количество графем: " + cast(list.len(), "string"))
```

### 6.11 grapheme_clusters

Выполняет сегментацию строки `s` с возвратом массива подробных дескрипторов `GraphemeCluster[]`, содержащих текст кластера, его смещение и байтовый размер.

```yukino
public fun grapheme_clusters(s: string) -> GraphemeCluster[]
```

Возвращает срез структур `GraphemeCluster[]`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: grapheme_clusters
load std.io: println

set clusters = grapheme_clusters("Yukino")
for i in 0..clusters.len() {
    set c = clusters[i]
    println("Графема: " + c.text + ", байт: " + cast(c.byte_len, "string"))
}
```

### 6.12 count_graphemes

Вычисляет количество неделимых кластеров графем в строке `s`. Корректно объединяет базовые буквы с диакритическими знаками, составные эмодзи и флаги.

```yukino
public fun count_graphemes(s: string) -> int64
```

Возвращает число графем как `int64`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: count_graphemes
load std.io: println

set total = count_graphemes("cafe\u{0301}")
println("Видимых символов: " + cast(total, "string"))
```

### 6.13 grapheme_at

Возвращает кластер графем по указанному логическому индексу `index` (начиная с `0`).

```yukino
public fun grapheme_at(s: string, index: int64) -> string?
```

Возвращает подстроку типа `string` с запрошенной графемой либо `null`, если индекс отрицателен или выходит за границы диапазона графем строки. Является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: grapheme_at
load std.io: println

set char_opt = grapheme_at("Привет", 0)
if char_opt != null {
    println("Первая графема: " + (char_opt ?? ""))
}
```

### 6.14 to_utf16

Кодирует строку `s` в массив 16-битных кодовых единиц `uint16[]` (UTF-16). Символы за пределами BMP кодируются суррогатными парами.

```yukino
public fun to_utf16(s: string) -> uint16[]
```

Возвращает массив кодовых единиц `uint16[]`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: to_utf16
load std.io: println

set u16 = to_utf16("JVM String")
println("UTF-16 units: " + cast(u16.len(), "string"))
```

### 6.15 to_utf16_null_terminated

Кодирует строку `s` в массив UTF-16 с добавлением завершающего нулевого символа `0u16` в конце. Предназначена для прямой передачи строк в Windows C ABI функции, принимающие `LPCWSTR` / `wchar_t*`.

```yukino
public fun to_utf16_null_terminated(s: string) -> uint16[]
```

Возвращает массив `uint16[]`, гарантированно оканчивающийся нулем (`0u16`). Длина массива равна количеству кодовых единиц плюс один. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: to_utf16_null_terminated
load std.io: println

set wstr = to_utf16_null_terminated("MessageBoxW title")
println("Last zero unit: " + cast(wstr[wstr.len() - 1], "string"))
```

### 6.16 from_utf16

Строго декодирует массив 16-битных кодовых единиц `units` в строку UTF-8.

```yukino
public fun from_utf16(ref units: uint16[]) -> string
```

Возвращает декодированную строку типа `string`. Не является nullable.

При наличии некорректных или оборванных суррогатных пар функция вызывает панику со структурой `UtfError` (код `103`).

```yukino
load std.encore: to_utf16, from_utf16
load std.io: println

set original = "Проверка UTF-16"
set units = to_utf16(original)
set restored = from_utf16(ref units)
println("Восстановлено: " + restored)
```

### 6.17 from_utf16_lossy

Мягко (lossy) декодирует массив кодовых единиц `units` в строку UTF-8. Некорректные суррогатные единицы заменяются на `U+FFFD`.

```yukino
public fun from_utf16_lossy(ref units: uint16[]) -> string
```

Возвращает безопасную строку типа `string`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: from_utf16_lossy
load std.io: println

set bad_units: uint16[] = [0xD800u16, 65u16]
set safe_text = from_utf16_lossy(ref bad_units)
println("Safe text length: " + cast(safe_text.len(), "string"))
```

### 6.18 from_utf16_null_terminated

Декодирует нуль-терминированную строку UTF-16 по сырому указателю `ptr<uint16>` (`LPCWSTR` / `wchar_t*`) до первого нулевого терминатора `0u16`.

```yukino
public fun from_utf16_null_terminated(p: ptr<uint16>) -> string
```

Возвращает строку типа `string`. Если передан `null`, возвращает пустую строку `""`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: to_utf16_null_terminated, from_utf16_null_terminated
load std.io: println

set win_buffer = to_utf16_null_terminated("Win32 API Output")
set result = from_utf16_null_terminated(win_buffer.as_ptr())
println("Win32 text: " + result)
```

### 6.19 is_high_surrogate

Проверяет, является ли 16-битная единица `unit` старшим (ведущим) суррогатом UTF-16 в диапазоне `0xD800..0xDBFF`.

```yukino
public fun is_high_surrogate(unit: uint16) -> bool
```

Возвращает логическое значение `true`, если единица входит в диапазон старших суррогатов, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: is_high_surrogate
load std.io: println

set is_high = is_high_surrogate(0xD83Du16)
println("Is high: " + cast(is_high, "string"))
```

### 6.20 is_low_surrogate

Проверяет, является ли 16-битная единица `unit` младшим (замыкающим) суррогатом UTF-16 в диапазоне `0xDC00..0xDFFF`.

```yukino
public fun is_low_surrogate(unit: uint16) -> bool
```

Возвращает логическое значение `true`, если единица входит в диапазон младших суррогатов, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: is_low_surrogate
load std.io: println

set is_low = is_low_surrogate(0xDE00u16)
println("Is low: " + cast(is_low, "string"))
```

### 6.21 is_surrogate

Проверяет, принадлежит ли 16-битная единица `unit` любому суррогатному значению UTF-16 в диапазоне `0xD800..0xDFFF`.

```yukino
public fun is_surrogate(unit: uint16) -> bool
```

Возвращает логическое значение `true`, если значение является суррогатом, иначе `false`. Не является nullable.

Функция никогда не вызывает панику.

```yukino
load std.encore: is_surrogate
load std.io: println

set check = is_surrogate(65u16)
println("Is surrogate: " + cast(check, "string"))
```

---

[← Назад: Модуль std.sqlite](32-sqlite.md) | [Вперед: Каталог сторонних библиотек →](../../libs-ru.md)
