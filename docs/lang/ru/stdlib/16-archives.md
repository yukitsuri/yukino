# Модуль std.fs.archives

Модуль `std.fs.archives` предоставляет средства для работы с популярными архивными форматами TAR и ZIP, включая создание, чтение содержимого (листинг), извлечение файлов и каталогов, прозрачное сжатие (Gzip, Zstandard, XZ, Bzip2, Deflate) и парольную защиту (ZipCrypto, AES-CBC, AES-CTR, WinZip AES). Модуль криптографических примитивов доступен через подмодуль `std.fs.archives.pass`.

---

## 1. Подключение: load std.fs.archives

Импорт всего пакета:

```yukino
load std.fs.archives
// Для криптографических примитивов шифрования архивов:
load std.fs.archives.pass
```

При импорте модуля становятся доступны следующие сущности:

- **Структуры данных и ошибок:** `TarEntry`, `TarError`, `ZipEntry`, `ZipError`, `PassError`, `WinZipAesResult`.
- **Функции TAR:** `tar_pack`, `tar_pack_dir`, `tar_unpack`, `tar_list`, `tar_pack_encrypted`, `tar_unpack_encrypted`.
- **Функции ZIP:** `zip_pack`, `zip_pack_dir`, `zip_pack_with_password`, `zip_unpack`, `zip_unpack_with_password`, `zip_list`.
- **Функции PASS:** `zipcrypto_encrypt`, `zipcrypto_decrypt`, `aes_ctr`, `aes_cbc_encrypt`, `aes_cbc_decrypt`, `winzip_aes_encrypt`, `winzip_aes_decrypt`.

Сущности TAR и ZIP также автоматически становятся доступны при загрузке `load std.fs` либо всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.fs.archives` глобальные константы не объявлены.

---

## 3. Интерфейсы

В модуле `std.fs.archives` интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 TarEntry

Описывает отдельную запись внутри архива TAR (файл, каталог или ссылку).

```yukino
public struct TarEntry {
    path: string
    size: int64
    mode: uint32
    mtime: int64
    type_flag: uint8
    link_target: string
    is_file: bool
    is_dir: bool
}
```

### 4.2 TarError

Структура ошибки, инициируемой функциями работы с TAR при отсутствии файлов, некорректном формате заголовков ustar или ошибках ввода-вывода.

```yukino
public struct TarError {
    message: string
    code: int32
}
```

### 4.3 ZipEntry

Описывает отдельную запись внутри архива ZIP в таблице центрального каталога (Central Directory).

```yukino
public struct ZipEntry {
    path: string
    size: int64
    compressed_size: int64
    crc32: uint32
    method: uint16
    is_dir: bool
    is_encrypted: bool
}
```

### 4.4 ZipError

Структура ошибки, инициируемой функциями ZIP при отсутствии сигнатур EOCD/локальных заголовков, повреждении сжатых данных, несовпадении контрольной суммы CRC-32 либо неверном пароле.

```yukino
public struct ZipError {
    message: string
    code: int32
}
```

### 4.5 PassError

Структура ошибки, инициируемой криптографическими алгоритмами при некорректных размерах ключа/вектора инициализации, ошибках дополнения PKCS7 либо несовпадении контрольных значений WinZip AES.

```yukino
public struct PassError {
    message: string
    code: int32
}
```

### 4.6 WinZipAesResult

Результат шифрования блоком WinZip AES, содержащий зашифрованные данные и двухбайтовое проверочное значение для заголовка архива.

```yukino
public struct WinZipAesResult {
    data: bytes
    check_value: uint16
}
```

---

## 5. Классы

В модуле `std.fs.archives` классы не объявлены.

---

## 6. Справочник функций

### 6.1 tar_pack

Упаковывает список путей (файлов и директорий с рекурсивным обходом) в архив TAR по пути `archive_path`. Параметр `compression` по умолчанию равен `"auto"` и автоматически определяет алгоритм сжатия по расширению архива (`.tar`, `.tar.gz`/`.tgz`, `.tar.xz`/`.txz`, `.tar.zst`/`.tzst`, `.tar.bz2`/`.tbz2`), либо может принимать явные значения `"none"`, `"gz"`, `"xz"`, `"zstd"`, `"bz2"`.

```yukino
public fun tar_pack(archive_path: string, paths: string[], compression: string = "auto") -> bool
```

Возвращает логическое значение `true` при успешном создании архива. Возвращаемое значение не nullable.

В случае несуществующего пути или ошибки ввода-вывода вызывает панику со структурой `TarError` (код `404`).

```yukino
load std.fs.archives.tar: tar_pack
load std.io: println

tar_pack("release.tar.gz", ["src", "README.md"])
println("TAR архив упакован")
```

### 6.2 tar_pack_dir

Удобная обертка для упаковки одного каталога `dir_path` в архив TAR.

```yukino
public fun tar_pack_dir(archive_path: string, dir_path: string, compression: string = "auto") -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае отсутствия каталога вызывает панику со структурой `TarError`.

```yukino
load std.fs.archives.tar: tar_pack_dir
load std.io: println

tar_pack_dir("backup.tar.zst", "my_project")
println("Каталог заархивирован")
```

### 6.3 tar_unpack

Распаковывает архив TAR (с автоматическим определением сжатия по расширению файла) в целевой каталог `dest_dir` с воссозданием структуры подпапок.

```yukino
public fun tar_unpack(archive_path: string, dest_dir: string) -> bool
```

Возвращает логическое значение `true` при успешном извлечении. Возвращаемое значение не nullable.

В случае отсутствия архива или повреждения формата ustar вызывает панику со структурой `TarError`.

```yukino
load std.fs.archives.tar: tar_unpack
load std.io: println

tar_unpack("release.tar.gz", "extracted_files")
println("Файлы извлечены")
```

### 6.4 tar_list

Считывает архив TAR и возвращает массив записей `TarEntry` без извлечения файлов на диск.

```yukino
public fun tar_list(archive_path: string) -> TarEntry[]
```

Возвращает массив структур `TarEntry[]`. Возвращаемое значение не nullable.

В случае отсутствия архива вызывает панику со структурой `TarError`.

```yukino
load std.fs.archives.tar: tar_list
load std.io: println

set entries = tar_list("release.tar.gz")
println("Количество записей: ${entries.len()}")
```

### 6.5 tar_pack_encrypted

Упаковывает переданные пути в архив TAR и выполняет симметричное шифрование архива алгоритмом AES-256-CBC с использованием пароля `password`.

```yukino
public fun tar_pack_encrypted(archive_path: string, paths: string[], password: string) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае отсутствия файлов или ошибки шифрования вызывает панику со структурой `TarError` либо `PassError`.

```yukino
load std.fs.archives.tar: tar_pack_encrypted
load std.io: println

tar_pack_encrypted("secure.tar.enc", ["secrets"], "StrongMasterKey123")
println("Зашифрованный TAR создан")
```

### 6.6 tar_unpack_encrypted

Расшифровывает и распаковывает зашифрованный TAR архив в целевой каталог `dest_dir` с использованием пароля `password`.

```yukino
public fun tar_unpack_encrypted(archive_path: string, dest_dir: string, password: string) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае неверного пароля или отсутствия файла вызывает панику со структурой `TarError` либо `PassError`.

```yukino
load std.fs.archives.tar: tar_unpack_encrypted
load std.io: println

tar_unpack_encrypted("secure.tar.enc", "unpacked_secrets", "StrongMasterKey123")
println("Зашифрованный TAR распакован")
```

### 6.7 zip_pack

Упаковывает массив путей `paths` в стандартный архив ZIP. Параметр `method` задает метод сжатия: `8` для Deflate (по умолчанию), `0` для Store (без сжатия).

```yukino
public fun zip_pack(archive_path: string, paths: string[], method: uint16 = 8) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае несуществующего пути вызывает панику со структурой `ZipError` (код `404`).

```yukino
load std.fs.archives.zip: zip_pack
load std.io: println

zip_pack("archive.zip", ["docs", "config.json"])
println("ZIP архив создан")
```

### 6.8 zip_pack_dir

Упаковывает один каталог `dir_path` в архив ZIP.

```yukino
public fun zip_pack_dir(archive_path: string, dir_path: string, method: uint16 = 8) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае ошибки вызывает панику со структурой `ZipError`.

```yukino
load std.fs.archives.zip: zip_pack_dir
load std.io: println

zip_pack_dir("project.zip", "my_folder")
println("Папка упакована в ZIP")
```

### 6.9 zip_pack_with_password

Упаковывает файлы в архив ZIP с шифрованием содержимого алгоритмом ZipCrypto с использованием переданного пароля `password`.

```yukino
public fun zip_pack_with_password(archive_path: string, paths: string[], password: string, method: uint16 = 8) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае ошибки вызывает панику со структурой `ZipError`.

```yukino
load std.fs.archives.zip: zip_pack_with_password
load std.io: println

zip_pack_with_password("protected.zip", ["private_data"], "Secret123")
println("Защищенный паролем ZIP создан")
```

### 6.10 zip_unpack

Распаковывает архив ZIP в целевой каталог `dest_dir`.

```yukino
public fun zip_unpack(archive_path: string, dest_dir: string) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае отсутствия архива, повреждения структуры ZIP или несовпадения CRC-32 вызывает панику со структурой `ZipError`. Если архив зашифрован паролем, вызывает панику с кодом `302`.

```yukino
load std.fs.archives.zip: zip_unpack
load std.io: println

zip_unpack("archive.zip", "extracted_zip")
println("Архив ZIP извлечен")
```

### 6.11 zip_unpack_with_password

Распаковывает архив ZIP с указанием пароля `password` для расшифровки защищенных файлов.

```yukino
public fun zip_unpack_with_password(archive_path: string, dest_dir: string, password: string) -> bool
```

Возвращает логическое значение `true`. Возвращаемое значение не nullable.

В случае неверного пароля, повреждения данных или несовпадения CRC-32 вызывает панику со структурой `ZipError`.

```yukino
load std.fs.archives.zip: zip_unpack_with_password
load std.io: println

zip_unpack_with_password("protected.zip", "unpacked_zip", "Secret123")
println("Защищенный ZIP успешно извлечен")
```

### 6.12 zip_list

Считывает центральный каталог архива ZIP и возвращает массив записей `ZipEntry` без распаковки файлов.

```yukino
public fun zip_list(archive_path: string) -> ZipEntry[]
```

Возвращает массив `ZipEntry[]`. Возвращаемое значение не nullable.

В случае отсутствия архива или повреждения EOCD вызывает панику со структурой `ZipError`.

```yukino
load std.fs.archives.zip: zip_list
load std.io: println

set items = zip_list("archive.zip")
println("Записей в ZIP: ${items.len()}")
```

### 6.13 zipcrypto_encrypt

Выполняет шифрование буфера байтов классическим алгоритмом PKWARE ZipCrypto.

```yukino
public fun zipcrypto_encrypt(password: string, data: bytes) -> bytes
```

Возвращает зашифрованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае сбоя вызывает панику со структурой `PassError`.

```yukino
load std.fs.archives.pass: zipcrypto_encrypt
load std.io: println

set enc = zipcrypto_encrypt("key", cast("secret data", "bytes"))
println("Зашифровано байт: ${enc.len()}")
```

### 6.14 zipcrypto_decrypt

Выполняет расшифровку буфера байтов алгоритмом PKWARE ZipCrypto.

```yukino
public fun zipcrypto_decrypt(password: string, data: bytes) -> bytes
```

Возвращает расшифрованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае ошибки вызывает панику со структурой `PassError`.

```yukino
load std.fs.archives.pass: zipcrypto_decrypt
load std.io: println

set dec = zipcrypto_decrypt("key", enc_bytes)
println(cast(dec, "string"))
```

### 6.15 aes_ctr

Выполняет симметричное шифрование или расшифровку потока алгоритмом AES в режиме счетчика (CTR). Ключ `key` может иметь размер 16, 24 или 32 байта (AES-128/192/256), `iv` — 16 байт.

```yukino
public fun aes_ctr(key: bytes, iv: bytes, data: bytes) -> bytes
```

Возвращает обработанный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае некорректной длины вектора инициализации вызывает панику со структурой `PassError` (код `202`), в случае некорректной длины ключа — код `203`.

```yukino
load std.fs.archives.pass: aes_ctr
load std.io: println

set key = cast("0123456789abcdef0123456789abcdef", "bytes")
set iv = cast("0123456789abcdef", "bytes")
set enc = aes_ctr(key.cp(), iv.cp(), cast("payload", "bytes"))
println("AES-CTR длина: ${enc.len()}")
```

### 6.16 aes_cbc_encrypt

Выполняет шифрование буфера алгоритмом AES в режиме сцепления блоков (CBC) с дополнением PKCS7.

```yukino
public fun aes_cbc_encrypt(key: bytes, iv: bytes, data: bytes) -> bytes
```

Возвращает зашифрованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае некорректных размеров параметров вызывает панику со структурой `PassError`.

```yukino
load std.fs.archives.pass: aes_cbc_encrypt
load std.io: println

set enc = aes_cbc_encrypt(key, iv, cast("payload", "bytes"))
println("AES-CBC размер: ${enc.len()}")
```

### 6.17 aes_cbc_decrypt

Выполняет расшифровку буфера алгоритмом AES в режиме CBC с валидацией и удалением дополнения PKCS7.

```yukino
public fun aes_cbc_decrypt(key: bytes, iv: bytes, data: bytes) -> bytes
```

Возвращает расшифрованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае неверного дополнения или некорректных ключей вызывает панику со структурой `PassError`.

```yukino
load std.fs.archives.pass: aes_cbc_decrypt
load std.io: println

set dec = aes_cbc_decrypt(key, iv, enc)
println(cast(dec, "string"))
```

### 6.18 winzip_aes_encrypt

Шифрует данные по спецификации WinZip AES (PBKDF2-HMAC-SHA1 + AES-CTR). Принимает разрядность шифрования `aes_bits` (`128`, `192` или `256`) и случайную соль `salt` (рекомендуемые по спецификации WinZip размеры: 8 байт для 128 бит, 16 байт для 256 бит; длина соли кодом не проверяется).

```yukino
public fun winzip_aes_encrypt(password: string, aes_bits: int32, salt: bytes, data: bytes) -> WinZipAesResult
```

Возвращает структуру `WinZipAesResult`, содержащую зашифрованный буфер байтов `data` и двухбайтовое проверочное значение `check_value`. Возвращаемое значение не nullable.

В случае ошибки генерации ключа вызывает панику со структурой `PassError`.

```yukino
load std.fs.archives.pass: winzip_aes_encrypt
load std.io: println

set res = winzip_aes_encrypt("Pass", 256, salt, cast("secret", "bytes"))
println("Контрольное значение: ${res.check_value}")
```

### 6.19 winzip_aes_decrypt

Расшифровывает данные формата WinZip AES с верификацией пароля по переданному значению `check_val`.

```yukino
public fun winzip_aes_decrypt(password: string, aes_bits: int32, salt: bytes, data: bytes, check_val: uint16) -> bytes
```

Возвращает расшифрованный буфер байтов типа `bytes`. Возвращаемое значение не nullable.

В случае несовпадения проверочного значения (неверного пароля) вызывает панику со структурой `PassError` (код `204`), при других ошибках расшифровки — код `203`.

```yukino
load std.fs.archives.pass: winzip_aes_decrypt
load std.io: println

set plain = winzip_aes_decrypt("Pass", 256, salt, res.data, res.check_value)
println(cast(plain, "string"))
```

---

[← Назад: Модуль std.compress](15-compress.md) | [Вперед: Модуль std.locale →](17-locale.md)
