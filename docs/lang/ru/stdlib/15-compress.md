# Модуль std.compress

Модуль `std.compress` предоставляет средства для сжатия и распаковки данных в памяти и потоковых файлов. Поддерживаются распространенные алгоритмы и контейнеры: Gzip, Zstandard (zstd), XZ/LZMA, Deflate, Zlib, Bzip2, LZ4, а также вычисление контрольных сумм CRC-32.

---

## 1. Подключение: load std.compress

Импорт всего пакета:

```yukino
load std.compress
```

При импорте модуля становятся доступны следующие сущности:

- **Структуры данных и ошибок:** `CompressError`.
- **Функции:** `compress`, `decompress`, `compress_file`, `decompress_file`, `gzip`, `gunzip`, `zstd_compress`, `zstd_decompress`, `xz_compress`, `xz_decompress`, `deflate_compress`, `deflate_decompress`, `zlib_compress`, `zlib_decompress`, `bzip2_compress`, `bzip2_decompress`, `lz4_compress`, `lz4_decompress`, `crc32`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.compress` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.compress` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 CompressError

Структура ошибки, инициируемой функциями модуля при некорректных входных данных, поврежденных сжатых потоках или сбоях алгоритмов сжатия и распаковки.

```yukino
public struct CompressError {
    message: string
    code: int32
}
```

Поле `message` содержит детальное текстовое описание причины сбоя, а `code` — числовой код ошибки: `101` при некорректных входных данных, `102` при неподдерживаемом формате сжатия, `103` при ошибке сжатия, `104` при ошибке распаковки (включая поврежденные данные), `105` при ошибке открытия или создания файла.

---

## 5. Классы

В модуле `std.compress` классы не объявлены.

---

## 6. Справочник функций

### 6.1 compress

Выполняет сжатие буфера байтов по указанному формату с опциональным уровнем сжатия. Допустимые значения `format`: `"gzip"` / `"gz"`, `"zstd"` / `"zst"`, `"xz"`, `"lzma"`, `"deflate"`, `"zlib"`, `"bz2"` / `"bzip2"`, `"lz4"`. Уровень `-1` задает стандартный уровень компрессии выбранного алгоритма.

```yukino
public fun compress(data: bytes, format: string, level: int32 = -1) -> bytes
```

Возвращает новый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае неподдерживаемого формата либо ошибки сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: compress
load std.io: println

set raw = cast("Данные для сжатия", "bytes")
set compressed = compress(raw, "zstd", 3)
println("Сжато байт: ${compressed.len()}")
```

### 6.2 decompress

Выполняет распаковку сжатого буфера байтов по указанному формату. Поддерживает форматы `"gzip"` / `"gz"`, `"zstd"` / `"zst"`, `"xz"` / `"lzma"`, `"deflate"`, `"zlib"`, `"bz2"` / `"bzip2"`, `"lz4"`.

```yukino
public fun decompress(data: bytes, format: string) -> bytes
```

Возвращает исходный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае повреждения потока или неподдерживаемого формата вызывает панику со структурой `CompressError`.

```yukino
load std.compress: compress, decompress
load std.io: println

set original = cast("Привет мир", "bytes")
set packed = compress(original.cp(), "gzip")
set unpacked = decompress(packed, "gzip")
println(cast(unpacked, "string"))
```

### 6.3 compress_file

Выполняет потоковое сжатие файла `src_path` и сохраняет результат по пути `dst_path` с использованием выбранного формата и уровня компрессии.

```yukino
public fun compress_file(src_path: string, dst_path: string, format: string, level: int32 = -1) -> bool
```

Возвращает логическое значение `true` при успешном завершении операции. Возвращаемое значение не nullable.

В случае ошибки открытия файла или сбоя компрессии вызывает панику со структурой `CompressError`.

```yukino
load std.compress: compress_file
load std.io: println

compress_file("input.txt", "input.txt.gz", "gzip")
println("Файл успешно сжат")
```

### 6.4 decompress_file

Выполняет потоковую распаковку сжатого файла `src_path` и записывает распакованные данные в файл по пути `dst_path`.

```yukino
public fun decompress_file(src_path: string, dst_path: string, format: string) -> bool
```

Возвращает логическое значение `true` при успешном завершении операции. Возвращаемое значение не nullable.

В случае повреждения файла или ошибки записи вызывает панику со структурой `CompressError`.

```yukino
load std.compress: decompress_file
load std.io: println

decompress_file("input.txt.gz", "restored.txt", "gzip")
println("Файл успешно распакован")
```

### 6.5 gzip

Сжимает буфер байтов в формат Gzip. Принимает опциональный уровень компрессии `level` от `0` до `9`; значение по умолчанию `-1` выбирает стандартный уровень кодека.

```yukino
public fun gzip(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае сбоя сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: gzip
load std.io: println

set data = cast("текст для gzip", "bytes")
set gz = gzip(data)
println("Размер gzip: ${gz.len()}")
```

### 6.6 gunzip

Распаковывает буфер байтов из формата Gzip.

```yukino
public fun gunzip(data: bytes) -> bytes
```

Возвращает распакованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае поврежденного заголовка или контрольной суммы вызывает панику со структурой `CompressError`.

```yukino
load std.compress: gzip, gunzip
load std.io: println

set raw = cast("проверка gunzip", "bytes")
set gz = gzip(raw)
set un = gunzip(gz)
println(cast(un, "string"))
```

### 6.7 zstd_compress

Сжимает буфер байтов с использованием алгоритма Zstandard. Принимает уровень компрессии `level` в диапазоне от `1` до `22`; значение по умолчанию `-1` соответствует стандартному уровню кодека (`3`).

```yukino
public fun zstd_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае сбоя алгоритма вызывает панику со структурой `CompressError`.

```yukino
load std.compress: zstd_compress
load std.io: println

set zst = zstd_compress(cast("Данные для Zstandard", "bytes"))
println("Размер zstd: ${zst.len()}")
```

### 6.8 zstd_decompress

Распаковывает буфер байтов, сжатый алгоритмом Zstandard.

```yukino
public fun zstd_decompress(data: bytes) -> bytes
```

Возвращает исходный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае некорректного формата потока вызывает панику со структурой `CompressError`.

```yukino
load std.compress: zstd_compress, zstd_decompress
load std.io: println

set packed = zstd_compress(cast("Тестовая строка", "bytes"))
set restored = zstd_decompress(packed)
println(cast(restored, "string"))
```

### 6.9 xz_compress

Сжимает буфер байтов алгоритмом XZ/LZMA2. Параметр `level` присутствует в сигнатуре, но кодеком XZ не используется.

```yukino
public fun xz_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: xz_compress
load std.io: println

set xz_data = xz_compress(cast("Высокая степень сжатия LZMA2", "bytes"))
println("Размер xz: ${xz_data.len()}")
```

### 6.10 xz_decompress

Распаковывает буфер байтов формата XZ или LZMA.

```yukino
public fun xz_decompress(data: bytes) -> bytes
```

Возвращает распакованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае повреждения потока данных вызывает панику со структурой `CompressError`.

```yukino
load std.compress: xz_compress, xz_decompress
load std.io: println

set packed = xz_compress(cast("XZ распаковка", "bytes"))
set unpacked = xz_decompress(packed)
println(cast(unpacked, "string"))
```

### 6.11 deflate_compress

Сжимает буфер байтов чистым алгоритмом Deflate без заголовков контейнеров. Принимает уровень компрессии `level` от `0` до `9`; значение по умолчанию `-1` выбирает стандартный уровень кодека.

```yukino
public fun deflate_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: deflate_compress
load std.io: println

set def = deflate_compress(cast("raw deflate stream", "bytes"))
println("Deflate размер: ${def.len()}")
```

### 6.12 deflate_decompress

Распаковывает сырой поток Deflate.

```yukino
public fun deflate_decompress(data: bytes) -> bytes
```

Возвращает исходный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки декомпрессии вызывает панику со структурой `CompressError`.

```yukino
load std.compress: deflate_compress, deflate_decompress
load std.io: println

set def = deflate_compress(cast("сырой поток", "bytes"))
set un = deflate_decompress(def)
println(cast(un, "string"))
```

### 6.13 zlib_compress

Сжимает буфер байтов в контейнер Zlib (с заголовком RFC 1950 и контрольной суммой Adler-32). Принимает уровень компрессии `level` от `0` до `9`; значение по умолчанию `-1` выбирает стандартный уровень кодека.

```yukino
public fun zlib_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: zlib_compress
load std.io: println

set zl = zlib_compress(cast("zlib данные", "bytes"))
println("Zlib размер: ${zl.len()}")
```

### 6.14 zlib_decompress

Распаковывает поток данных формата Zlib.

```yukino
public fun zlib_decompress(data: bytes) -> bytes
```

Возвращает распакованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае повреждения потока или несовпадения Adler-32 вызывает панику со структурой `CompressError`.

```yukino
load std.compress: zlib_compress, zlib_decompress
load std.io: println

set zl = zlib_compress(cast("zlib распаковка", "bytes"))
set un = zlib_decompress(zl)
println(cast(un, "string"))
```

### 6.15 bzip2_compress

Сжимает буфер байтов алгоритмом Bzip2 (преобразование Барроуза — Уилера). Принимает уровень сжатия `level` от `1` до `9`; значение по умолчанию `-1` выбирает стандартный уровень кодека.

```yukino
public fun bzip2_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки сжатия вызывает панику со структурой `CompressError`.

```yukino
load std.compress: bzip2_compress
load std.io: println

set bz = bzip2_compress(cast("bzip2 данные", "bytes"))
println("Bzip2 размер: ${bz.len()}")
```

### 6.16 bzip2_decompress

Распаковывает поток байтов, сжатый алгоритмом Bzip2.

```yukino
public fun bzip2_decompress(data: bytes) -> bytes
```

Возвращает распакованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае повреждения блоков вызывает панику со структурой `CompressError`.

```yukino
load std.compress: bzip2_compress, bzip2_decompress
load std.io: println

set bz = bzip2_compress(cast("bzip2 распаковка", "bytes"))
set un = bzip2_decompress(bz)
println(cast(un, "string"))
```

### 6.17 lz4_compress

Сжимает буфер байтов сверхбыстрым алгоритмом LZ4 в стандартный покадровый контейнер LZ4 frame. Параметр `level` присутствует в сигнатуре, но кодеком LZ4 не используется.

```yukino
public fun lz4_compress(data: bytes, level: int32 = -1) -> bytes
```

Возвращает сжатый буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки компрессии вызывает панику со структурой `CompressError`.

```yukino
load std.compress: lz4_compress
load std.io: println

set lz = lz4_compress(cast("сверхбыстрое сжатие lz4", "bytes"))
println("LZ4 размер: ${lz.len()}")
```

### 6.18 lz4_decompress

Распаковывает данные из стандартного фрейма LZ4.

```yukino
public fun lz4_decompress(data: bytes) -> bytes
```

Возвращает распакованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае некорректного заголовка или повреждения фрейма вызывает панику со структурой `CompressError`.

```yukino
load std.compress: lz4_compress, lz4_decompress
load std.io: println

set lz = lz4_compress(cast("распаковка lz4", "bytes"))
set un = lz4_decompress(lz)
println(cast(un, "string"))
```

### 6.19 crc32

Вычисляет 32-битную циклическую контрольную сумму CRC-32 (полином IEEE 802.3, `0xEDB88320`) для переданного буфера байтов.

```yukino
public fun crc32(data: bytes) -> uint32
```

Возвращает 32-битное беззнаковое целое число типа `uint32`. Возвращаемое значение не nullable.

Функция никогда не вызывает панику.

```yukino
load std.compress: crc32
load std.io: println

set sum = crc32(cast("123456789", "bytes"))
println("CRC32: ${sum}")
```

---

[← Назад: Модуль std.simpleurl](14-simpleurl.md) | [Вперед: Модуль std.fs.archives →](16-archives.md)
