# Модуль std.crypto

Модуль `std.crypto` предоставляет всесторонний криптографический инструментарий промышленного уровня, реализованный полностью на чистом Yukino без привлечения тяжелого внешнего C/C++ рантайма. Единственной внешней точкой интеграции является получение криптографически стойкой системной энтропии от ядра операционной системы (`getrandom` в Linux, `arc4random_buf` в BSD/macOS, `BCryptGenRandom` в Windows).

Архитектура модуля объединяет современные признанные международные криптографические стандарты и протоколы с постоянным временем выполнения (constant-time) для защиты от атак по сторонним каналам (side-channel timing attacks):

- **HMAC (RFC 2104):** имитовставки и аутентификация сообщений на базе SHA-256 и SHA-512 с поддержкой однократного вычисления и потокового обновления.
- **ChaCha20 (RFC 8439):** высокоскоростной симметричный потоковый шифр с 256-битным ключом, 96-битным одноразовым номером (nonce) и 32-битным счетчиком блоков.
- **Poly1305 (RFC 8439):** одноразовый аутентификатор сообщений (MAC) над полем 2^130 - 5.
- **ChaCha20-Poly1305 AEAD (RFC 8439 Section 2.8):** аутентифицированное шифрование с присоединенными открытыми данными (Authenticated Encryption with Associated Data).
- **X25519 (RFC 7748):** протокол согласования общих ключей Диффи-Хеллмана на базе лестницы Монтгомери над эллиптической кривой Curve25519.
- **Ed25519 (RFC 8032):** алгоритм формирования и проверки цифровой электронной подписи Эдвардса на базе витой кривой Эдвардса Ed25519 и SHA-512.
- **HKDF (RFC 5869):** функция извлечения и расширения ключей (HMAC-based Extract-and-Expand Key Derivation Function) на основе SHA-256.
- **PBKDF2 (RFC 2898 / PKCS #5):** функция деривации криптографических ключей из паролей на основе итеративного применения HMAC-SHA256.
- **Криптографические утилиты:** генерация энтропии, безопасное обнуление памяти, постоянное по времени сравнение и сериализация чисел в форматах Little-Endian и Big-Endian.

---

## 1. Подключение: load std.crypto

Импорт всего пакета:

```yukino
load std.crypto
```

Это сделает доступными следующие сущности:

- **Структуры ошибок:** `CryptoError`.
- **Классы потоковой обработки:** `HmacSha256`, `HmacSha512`, `ChaCha20`.
- **Функции HMAC:** `hmac_sha256`, `hmac_sha256_hex`, `hmac_sha256_verify`, `hmac_sha512`, `hmac_sha512_hex`, `hmac_sha512_verify`, `hmac_sha256_init`, `hmac_sha512_init`.
- **Функции симметричного шифрования:** `chacha20_crypt`, `chacha20_block`, `chacha20_poly1305_encrypt`, `chacha20_poly1305_decrypt`.
- **Функции аутентификации Poly1305:** `poly1305_mac`, `poly1305_hex`, `poly1305_verify`.
- **Функции асимметричной криптографии на Curve25519:** `x25519_public_key`, `x25519_diffie_hellman`, `x25519`, `ed25519_public_key`, `ed25519_sign`, `ed25519_verify`.
- **Функции формирования ключей:** `hkdf_extract`, `hkdf_expand`, `hkdf`, `pbkdf2_sha256`.
- **Вспомогательные утилиты:** `constant_time_eq`, `zero_bytes`, `random_bytes`, `load32_le`, `store32_le`, `load64_le`, `store64_le`, `load32_be`, `store32_be`, `load64_be`, `store64_be`.

Все перечисленные сущности также автоматически становятся доступны при загрузке всей стандартной библиотеки целиком: `load std`.

---

## 2. Константы

В модуле `std.crypto` глобальные константы не объявлены. Все внутренние коэффициенты алгоритмов (константы ChaCha20 sigma, параметры поля Curve25519, порядок подгруппы L) инкапсулированы внутри соответствующих модулей.

---

## 3. Интерфейсы

В модуле `std.crypto` специализированные интерфейсы не объявлены.

---

## 4. Структуры данных и ошибок (паники)

### 4.1 public struct CryptoError

Структура ошибки, используемая для сигнализации о некорректных входных параметрах (недопустимая длина ключа, вектора инициализации или отрицательный размер буфера).

```yukino
public struct CryptoError {
    message: string
    code: int32
}
```

- `message`: строковое описание возникшей ошибки.
- `code`: числовой код ошибки:
  - `1`: отрицательная длина в функции `zero_bytes`.
  - `2`: отрицательная длина в функции `random_bytes`.
  - `21`: неверная длина ключа Poly1305 (ожидается строго 32 байта).
  - `31`: неверная длина ключа ChaCha20-Poly1305 AEAD (ожидается строго 32 байта).
  - `32`: неверная длина nonce ChaCha20-Poly1305 AEAD (ожидается строго 12 байт).
  - `33`: ошибка проверки подлинности сообщения при дешифровании AEAD (недопустимый тег аутентификации).
  - `34`: зашифрованные данные короче минимального размера тега (16 байт).
  - `41`: неверная длина закрытого ключа X25519 (ожидается 32 байта).
  - `42`: неверная длина открытого ключа X25519 (ожидается 32 байта).
  - `51`: неверная длина исходного сида Ed25519 (ожидается 32 байта).
  - `52`: неверная длина секретного ключа Ed25519 при формировании подписи (ожидается 32 байта).
  - `61`: запрошенная длина OKM в функции HKDF превышает максимально допустимый предел 8160 байт (255 * 32).
  - `71`: число итераций PBKDF2 меньше или равно нулю.
  - `72`: запрошенная длина ключа PBKDF2 отрицательна.

---

## 5. Классы

### 5.1 public class HmacSha256

Класс для потокового вычисления кода аутентификации сообщений HMAC на основе хэш-функции SHA-256.

```yukino
public class HmacSha256
```

#### Методы класса HmacSha256

##### init

Подготавливает пустой экземпляр без контекста. Рабочий контекст с ключом создается фабрикой `hmac_sha256_init` (см. раздел 6.27): конструктор не имеет дестинейшн-региона и не может безопасно размещать внутренние хэшеры в полях класса.

```yukino
public fun init()
```

Возвращает `null` (процедура инициализации экземпляра). Метод никогда не вызывает панику.

##### update

Передает очередную порцию данных в вычисление HMAC.

```yukino
public fun update(data: bytes)
```

Возвращает `null`. Если экземпляр создан без фабрики `hmac_sha256_init`, вызывает панику со структурой `CryptoError` (код 13).

##### finish

Завершает вычисление HMAC и формирует итоговый 32-байтный код аутентификации.

```yukino
public fun finish() -> bytes
```

Возвращает 32-байтный срез `bytes`. Значение не может быть `null`. Если экземпляр создан без фабрики `hmac_sha256_init`, вызывает панику со структурой `CryptoError` (код 13).

##### finish_hex

Завершает вычисление HMAC и возвращает 64-символьную шестнадцатеричную строку в нижнем регистре.

```yukino
public fun finish_hex() -> string
```

Возвращает шестнадцатеричную строку типа `string`. Значение не может быть `null`. Если экземпляр создан без фабрики `hmac_sha256_init`, вызывает панику со структурой `CryptoError` (код 13).

Пример использования:

```yukino
load std.crypto.hmac: HmacSha256, hmac_sha256_init
load std.io: println

fun main() {
    set key: bytes = cast("api-secret", "bytes")
    set hmac: HmacSha256 = hmac_sha256_init(ref key)
    hmac.update(cast("part1-", "bytes"))
    hmac.update(cast("part2", "bytes"))
    println("HMAC-SHA256: " + hmac.finish_hex())
}
```

---

### 5.2 public class HmacSha512

Класс для потокового вычисления кода аутентификации сообщений HMAC на основе хэш-функции SHA-512.

```yukino
public class HmacSha512
```

#### Методы класса HmacSha512

##### init

Подготавливает пустой экземпляр без контекста. Рабочий контекст с ключом создается фабрикой `hmac_sha512_init` (см. раздел 6.28): конструктор не имеет дестинейшн-региона и не может безопасно размещать внутренние хэшеры в полях класса.

```yukino
public fun init()
```

Возвращает `null`. Метод никогда не вызывает панику.

##### update

Передает очередную порцию данных в вычисление HMAC-SHA512.

```yukino
public fun update(data: bytes)
```

Возвращает `null`. Если экземпляр создан без фабрики `hmac_sha512_init`, вызывает панику со структурой `CryptoError` (код 13).

##### finish

Завершает вычисление и формирует 64-байтный код аутентификации.

```yukino
public fun finish() -> bytes
```

Возвращает 64-байтный срез `bytes`. Значение не может быть `null`. Если экземпляр создан без фабрики `hmac_sha512_init`, вызывает панику со структурой `CryptoError` (код 13).

##### finish_hex

Завершает вычисление и возвращает 128-символьную шестнадцатеричную строку.

```yukino
public fun finish_hex() -> string
```

Возвращает строку типа `string`. Значение не может быть `null`. Если экземпляр создан без фабрики `hmac_sha512_init`, вызывает панику со структурой `CryptoError` (код 13).

---

### 5.3 public class ChaCha20

Класс потокового симметричного шифрования ChaCha20 с возможностью последовательного шифрования фрагментов произвольного размера.

```yukino
public class ChaCha20
```

#### Методы класса ChaCha20

##### init

Инициализирует состояние шифра 32-байтным ключом, 12-байтным nonce и начальным значением счетчика блоков.

```yukino
public fun init(key: bytes, nonce: bytes, counter: uint32 = 0u32)
```

Возвращает `null`. В случае несоответствия длины ключа 32 байтам или nonce 12 байтам вызывает панику со структурой `CryptoError`.

##### apply

Шифрует или дешифрует переданный байтовый массив путем побитового XOR с ключевым потоком, автоматически инкрементируя внутренний счетчик блоков.

```yukino
public fun apply(data: bytes) -> bytes
```

Возвращает срез зашифрованных или расшифрованных байтов `bytes` такой же длины, как входные данные. Метод никогда не вызывает панику.

Пример использования:

```yukino
load std.crypto.chacha: ChaCha20
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set key: bytes = random_bytes(32)
    set nonce: bytes = random_bytes(12)
    set cipher: ChaCha20 = init ChaCha20(key.cp(), nonce.cp(), 0u32)
    set ciphertext: bytes = cipher.apply(cast("Confidential", "bytes"))
    println("Encrypted: " + bytes_to_hex(ref ciphertext))
}
```

---

## 6. Справочник функций

### 6.1 constant_time_eq

Выполняет сравнение двух срезов байтов за постоянное время для предотвращения атак синхронизации по времени.

```yukino
public fun constant_time_eq(ref a: bytes, ref b: bytes) -> bool
```

Возвращает логическое значение `true`, если оба среза имеют равную длину и побайтно идентичны, иначе `false`.

Функция никогда не вызывает панику.

```yukino
load std.crypto.common: constant_time_eq
load std.io: println

fun main() {
    set tag1: bytes = cast("auth_tag_12345", "bytes")
    set tag2: bytes = cast("auth_tag_12345", "bytes")
    if constant_time_eq(ref tag1, ref tag2) {
        println("Tags match securely")
    }
}
```

---

### 6.2 zero_bytes

Создает срез байтов указанной длины, заполненный нулями.

```yukino
public fun zero_bytes(len: int64) -> bytes
```

Возвращает байтовый срез типа `bytes`. Значение не может быть `null`.

Если параметр `len` меньше нуля, вызывает панику со структурой `CryptoError` (код 1).

```yukino
load std.crypto.common: zero_bytes
load std.io: println

fun main() {
    set buf: bytes = zero_bytes(32)
    println("Allocated zeroed bytes: " + cast(buf.len(), "string"))
}
```

---

### 6.3 random_bytes

Генерирует криптографически стойкие случайные байты, полученные напрямую из системного генератора энтропии операционной системы.

```yukino
public fun random_bytes(len: int64) -> bytes
```

Возвращает байтовый срез типа `bytes`. Значение не может быть `null`.

Если параметр `len` меньше нуля, вызывает панику со структурой `CryptoError` (код 2).

```yukino
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set salt: bytes = random_bytes(16)
    println("Random salt: " + bytes_to_hex(ref salt))
}
```

---

### 6.4 hmac_sha256

Вычисляет 32-байтный код аутентификации сообщений HMAC на базе SHA-256.

```yukino
public fun hmac_sha256(ref key: bytes, ref data: bytes) -> bytes
```

Возвращает 32-байтный срез `bytes`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load std.crypto.hmac: hmac_sha256
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set key: bytes = cast("secret", "bytes")
    set msg: bytes = cast("message", "bytes")
    set mac: bytes = hmac_sha256(ref key, ref msg)
    println("HMAC: " + bytes_to_hex(ref mac))
}
```

---

### 6.5 hmac_sha256_hex

Вычисляет HMAC-SHA256 и возвращает результат в виде 64-символьной шестнадцатеричной строки.

```yukino
public fun hmac_sha256_hex(ref key: bytes, ref data: bytes) -> string
```

Возвращает строковое значение типа `string`. Значение не может быть `null`.

Функция никогда не вызывает панику.

---

### 6.6 hmac_sha256_verify

Проверяет подлинность кода аутентификации HMAC-SHA256 за постоянное время.

```yukino
public fun hmac_sha256_verify(ref key: bytes, ref data: bytes, ref expected_mac: bytes) -> bool
```

Возвращает `true`, если вычисленный HMAC точно совпадает с переданным `expected_mac`, иначе `false`.

Функция никогда не вызывает панику.

---

### 6.7 hmac_sha512

Вычисляет 64-байтный код аутентификации сообщений HMAC на базе SHA-512.

```yukino
public fun hmac_sha512(ref key: bytes, ref data: bytes) -> bytes
```

Возвращает 64-байтный срез `bytes`. Значение не может быть `null`.

Функция никогда не вызывает панику.

---

### 6.8 hmac_sha512_hex

Вычисляет HMAC-SHA512 и возвращает результат в виде 128-символьной шестнадцатеричной строки.

```yukino
public fun hmac_sha512_hex(ref key: bytes, ref data: bytes) -> string
```

Возвращает строковое значение типа `string`. Значение не может быть `null`.

Функция никогда не вызывает панику.

---

### 6.9 hmac_sha512_verify

Проверяет подлинность кода аутентификации HMAC-SHA512 за постоянное время.

```yukino
public fun hmac_sha512_verify(ref key: bytes, ref data: bytes, ref expected_mac: bytes) -> bool
```

Возвращает `true`, если вычисленный HMAC точно совпадает с ожидаемым, иначе `false`.

Функция никогда не вызывает панику.

---

### 6.10 chacha20_crypt

Шифрует или расшифровывает сообщение по стандарту ChaCha20 (RFC 8439). Поскольку шифр симметричный потоковый, повторное применение с теми же ключом, nonce и счетчиком восстанавливает исходный текст.

```yukino
public fun chacha20_crypt(ref key: bytes, ref nonce: bytes, counter: uint32, ref data: bytes) -> bytes
```

Возвращает байтовый срез `bytes` зашифрованных или расшифрованных данных. Значение не может быть `null`.

Если длина `key` не равна 32 байтам или длина `nonce` не равна 12 байтам, вызывает панику со структурой `CryptoError`.

```yukino
load std.crypto.chacha: chacha20_crypt
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set key: bytes = cast("01234567890123456789012345678901", "bytes")
    set nonce: bytes = cast("012345678901", "bytes")
    set msg: bytes = cast("Hello world", "bytes")
    set encrypted: bytes = chacha20_crypt(ref key, ref nonce, 0u32, ref msg)
    println("Encrypted: " + bytes_to_hex(ref encrypted))
}
```

---

### 6.11 chacha20_block

Генерирует один 64-байтный блок ключевого потока ChaCha20 для заданного ключа, nonce и номера блока.

```yukino
public fun chacha20_block(ref key: bytes, ref nonce: bytes, counter: uint32) -> bytes
```

Возвращает 64-байтный срез `bytes`. Значение не может быть `null`.

Если ключ или nonce имеют недопустимую длину, вызывает панику со структурой `CryptoError`.

---

### 6.12 poly1305_mac

Вычисляет 16-байтный аутентификационный тег Poly1305 (RFC 8439) для переданных данных с использованием одноразового 32-байтного ключа.

```yukino
public fun poly1305_mac(ref key: bytes, ref data: bytes) -> bytes
```

Возвращает 16-байтный срез `bytes`. Значение не может быть `null`.

Если длина ключа не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 21).

```yukino
load std.crypto.poly1305: poly1305_mac
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set key: bytes = cast("01234567890123456789012345678901", "bytes")
    set data: bytes = cast("Payload to authenticate", "bytes")
    set tag: bytes = poly1305_mac(ref key, ref data)
    println("Poly1305 tag: " + bytes_to_hex(ref tag))
}
```

---

### 6.13 poly1305_hex

Вычисляет Poly1305 MAC и возвращает результат в виде 32-символьной шестнадцатеричной строки.

```yukino
public fun poly1305_hex(ref key: bytes, ref data: bytes) -> string
```

Возвращает строковое значение типа `string`. Значение не может быть `null`.

Если длина ключа не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 21).

---

### 6.14 poly1305_verify

Проверяет аутентичность 16-байтного тега Poly1305 за постоянное время.

```yukino
public fun poly1305_verify(ref key: bytes, ref data: bytes, ref tag: bytes) -> bool
```

Возвращает `true`, если тег корректен и подлинность подтверждена, иначе `false`.

Если длина ключа не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 21).

---

### 6.15 chacha20_poly1305_encrypt

Выполняет аутентифицированное шифрование открытого текста с дополнительными аутентифицируемыми открытыми данными (AAD) по стандарту RFC 8439 Section 2.8. Итоговый шифротекст содержит исходный зашифрованный текст с добавленным в конец 16-байтным тегом Poly1305.

```yukino
public fun chacha20_poly1305_encrypt(ref key: bytes, ref nonce: bytes, ref aad: bytes, ref plaintext: bytes) -> bytes
```

Возвращает байтовый срез `bytes` длиной `plaintext.len() + 16`. Значение не может быть `null`.

Если ключ не равен 32 байтам или nonce не равен 12 байтам, вызывает панику со структурой `CryptoError`.

```yukino
load std.crypto.aead: chacha20_poly1305_encrypt, chacha20_poly1305_decrypt
load std.crypto.common: random_bytes
load std.io: println

fun main() {
    set key: bytes = random_bytes(32)
    set nonce: bytes = random_bytes(12)
    set aad: bytes = cast("header-metadata", "bytes")
    set msg: bytes = cast("Secret payload", "bytes")

    set encrypted: bytes = chacha20_poly1305_encrypt(ref key, ref nonce, ref aad, ref msg)
    set decrypted: bytes = chacha20_poly1305_decrypt(ref key, ref nonce, ref aad, ref encrypted)
    println("Decrypted: " + cast(decrypted, "string"))
}
```

---

### 6.16 chacha20_poly1305_decrypt

Проверяет подлинность тега аутентификации и дешифрует данные, зашифрованные ChaCha20-Poly1305 AEAD.

```yukino
public fun chacha20_poly1305_decrypt(ref key: bytes, ref nonce: bytes, ref aad: bytes, ref ciphertext_and_tag: bytes) -> bytes
```

Возвращает исходный расшифрованный открытый текст типа `bytes`. Значение не может быть `null`.

Если длина входных зашифрованных данных меньше 16 байт, либо если аутентификационный тег не прошел проверку подлинности, вызывает панику со структурой `CryptoError` (код 33 или 34).

---

### 6.17 x25519_public_key

Вычисляет 32-байтный открытый ключ протокола Диффи-Хеллмана X25519 (RFC 7748) из 32-байтного закрытого ключа путем умножения на базовую точку кривой Curve25519.

```yukino
public fun x25519_public_key(ref private_key: bytes) -> bytes
```

Возвращает 32-байтный открытый ключ `bytes`. Значение не может быть `null`.

Если длина `private_key` не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 41).

```yukino
load std.crypto.x25519: x25519_public_key
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set priv: bytes = random_bytes(32)
    set pub: bytes = x25519_public_key(ref priv)
    println("X25519 Public Key: " + bytes_to_hex(ref pub))
}
```

---

### 6.18 x25519_diffie_hellman

Вычисляет общий секрет (shared secret) протокола Диффи-Хеллмана между собственным закрытым ключом и открытым ключом удаленной стороны.

```yukino
public fun x25519_diffie_hellman(ref private_key: bytes, ref public_key: bytes) -> bytes
```

Возвращает 32-байтный общий секрет `bytes`. Значение не может быть `null`.

Если длина `private_key` или `public_key` не равна 32 байтам, вызывает панику со структурой `CryptoError`.

```yukino
load std.crypto.x25519: x25519_public_key, x25519_diffie_hellman
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set alice_priv: bytes = random_bytes(32)
    set alice_pub: bytes = x25519_public_key(ref alice_priv)

    set bob_priv: bytes = random_bytes(32)
    set bob_pub: bytes = x25519_public_key(ref bob_priv)

    set shared_alice: bytes = x25519_diffie_hellman(ref alice_priv, ref bob_pub)
    set shared_bob: bytes = x25519_diffie_hellman(ref bob_priv, ref alice_pub)
    println("Shared secret: " + bytes_to_hex(ref shared_alice))
}
```

---

### 6.19 x25519

Выполняет скалярное умножение на кривой Curve25519 произвольной координаты U на скаляр по алгоритму Монтгомери (RFC 7748).

```yukino
public fun x25519(ref scalar: bytes, ref point: bytes) -> bytes
```

Возвращает 32-байтный результат скалярного умножения `bytes`. Значение не может быть `null`.

Если скаляр или точка не равны 32 байтам, вызывает панику со структурой `CryptoError`.

---

### 6.20 ed25519_public_key

Формирует 32-байтный открытый ключ проверки цифровой подписи Ed25519 (RFC 8032) из 32-байтного секретного сида.

```yukino
public fun ed25519_public_key(ref seed: bytes) -> bytes
```

Возвращает 32-байтный открытый ключ `bytes`. Значение не может быть `null`.

Если длина сида не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 51).

```yukino
load std.crypto.ed25519: ed25519_public_key
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set seed: bytes = random_bytes(32)
    set pubkey: bytes = ed25519_public_key(ref seed)
    println("Ed25519 Public Key: " + bytes_to_hex(ref pubkey))
}
```

---

### 6.21 ed25519_sign

Формирует 64-байтную цифровую подпись Ed25519 (RFC 8032) для сообщения произвольной длины с использованием 32-байтного сида.

```yukino
public fun ed25519_sign(ref seed: bytes, ref message: bytes) -> bytes
```

Возвращает 64-байтную цифровую подпись типа `bytes`. Значение не может быть `null`.

Если длина сида не равна 32 байтам, вызывает панику со структурой `CryptoError` (код 52).

```yukino
load std.crypto.ed25519: ed25519_public_key, ed25519_sign, ed25519_verify
load std.crypto.common: random_bytes
load std.io: println

fun main() {
    set seed: bytes = random_bytes(32)
    set pubkey: bytes = ed25519_public_key(ref seed)
    set document: bytes = cast("Release 1.0 manifest", "bytes")

    set sig: bytes = ed25519_sign(ref seed, ref document)
    if ed25519_verify(ref pubkey, ref document, ref sig) {
        println("Signature verified successfully")
    }
}
```

---

### 6.22 ed25519_verify

Проверяет математическую корректность 64-байтной цифровой подписи Ed25519 для заданного сообщения и 32-байтного открытого ключа.

```yukino
public fun ed25519_verify(ref public_key: bytes, ref message: bytes, ref signature: bytes) -> bool
```

Возвращает `true`, если цифровая подпись подлинна и сообщение не подвергалось модификации, иначе `false`.

Функция никогда не вызывает панику: при передаче открытого ключа или подписи некорректной длины функция возвращает `false`.

---

### 6.23 hkdf_extract

Выполняет фазу Extract алгоритма деривации ключей HKDF (RFC 5869) на базе HMAC-SHA256, формируя псевдослучайный ключ (PRK).

```yukino
public fun hkdf_extract(ref salt: bytes, ref ikm: bytes) -> bytes
```

Возвращает 32-байтный псевдослучайный ключ `bytes`. Значение не может быть `null`.

Функция никогда не вызывает панику.

---

### 6.24 hkdf_expand

Выполняет фазу Expand алгоритма деривации ключей HKDF (RFC 5869), генерируя псевдослучайный ключ требуемой длины `okm_len`.

```yukino
public fun hkdf_expand(ref prk: bytes, ref info: bytes, okm_len: int64) -> bytes
```

Возвращает байтовый срез `bytes` длиной `okm_len`. Значение не может быть `null`.

Если `okm_len` превышает 8160 байт (255 * 32) или отрицателен, вызывает панику со структурой `CryptoError` (код 61).

---

### 6.25 hkdf

Выполняет полный цикл формирования ключей HKDF (Extract и Expand) по стандарту RFC 5869 с использованием HMAC-SHA256.

```yukino
public fun hkdf(ref salt: bytes, ref ikm: bytes, ref info: bytes, okm_len: int64) -> bytes
```

Возвращает производный ключевой материал `bytes` запрошенной длины. Значение не может быть `null`.

В случае превышения максимально допустимой длины `okm_len` вызывает панику со структурой `CryptoError`.

```yukino
load std.crypto.hkdf: hkdf
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set salt: bytes = cast("salt", "bytes")
    set ikm: bytes = cast("user-secret-material", "bytes")
    set info: bytes = cast("session-auth-key", "bytes")
    set key: bytes = hkdf(ref salt, ref ikm, ref info, 32)
    println("Derived key: " + bytes_to_hex(ref key))
}
```

---

### 6.26 pbkdf2_sha256

Формирует криптографический ключ заданной длины из пароля и соли методом PBKDF2 (RFC 2898 / PKCS #5 v2.0) с применением функции псевдослучайных перестановок HMAC-SHA256.

```yukino
public fun pbkdf2_sha256(ref password: bytes, ref salt: bytes, iterations: int32, key_len: int64) -> bytes
```

Возвращает производный байтовый ключ типа `bytes` длиной `key_len`. Значение не может быть `null`.

Если `iterations <= 0` или `key_len < 0`, вызывает панику со структурой `CryptoError` (код 71 или 72).

```yukino
load std.crypto.pbkdf2: pbkdf2_sha256
load std.crypto.common: random_bytes
load std.hash: bytes_to_hex
load std.io: println

fun main() {
    set password: bytes = cast("correct-horse-battery-staple", "bytes")
    set salt: bytes = random_bytes(16)
    set derived: bytes = pbkdf2_sha256(ref password, ref salt, 100000, 32)
    println("Derived key: " + bytes_to_hex(ref derived))
}
```

---

### 6.27 hmac_sha256_init

Создает готовый к использованию контекст HMAC-SHA256 с ключом произвольной длины согласно RFC 2104. Внутренние хэшеры размещаются в регионе вызывающего кода, поэтому фабрика является единственным безопасным способом создания экземпляра `HmacSha256`.

```yukino
public fun hmac_sha256_init(ref key: bytes) -> HmacSha256
```

Возвращает экземпляр `HmacSha256`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load std.crypto.hmac: hmac_sha256_init
load std.io: println

fun main() {
    set key: bytes = cast("api-secret", "bytes")
    set hmac = hmac_sha256_init(ref key)
    hmac.update(cast("data", "bytes"))
    println(hmac.finish_hex())
}
```

---

### 6.28 hmac_sha512_init

Создает готовый к использованию контекст HMAC-SHA512 с ключом произвольной длины. Внутренние хэшеры размещаются в регионе вызывающего кода, поэтому фабрика является единственным безопасным способом создания экземпляра `HmacSha512`.

```yukino
public fun hmac_sha512_init(ref key: bytes) -> HmacSha512
```

Возвращает экземпляр `HmacSha512`. Значение не может быть `null`.

Функция никогда не вызывает панику.

```yukino
load std.crypto.hmac: hmac_sha512_init
load std.io: println

fun main() {
    set key: bytes = cast("api-secret", "bytes")
    set hmac = hmac_sha512_init(ref key)
    hmac.update(cast("data", "bytes"))
    println(hmac.finish_hex())
}
```

---

[← Назад: Модуль std.hash](26-hash.md) | [Вперед: Модуль std.net →](28-net.md)
