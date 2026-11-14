# Module std.crypto

The `std.crypto` module provides a comprehensive production-grade cryptographic toolkit implemented entirely in pure Yukino without external C/C++ runtime dependencies. The sole external system integration point is sourcing cryptographically secure operating system entropy (`getrandom` on Linux, `arc4random_buf` on BSD/macOS, `BCryptGenRandom` on Windows).

The module architecture brings together recognized international cryptographic standards designed with constant-time execution paths to safeguard against side-channel timing attacks:

- **HMAC (RFC 2104):** Message authentication codes based on SHA-256 and SHA-512 with single-shot and streaming context support.
- **ChaCha20 (RFC 8439):** High-speed symmetric stream cipher using a 256-bit key, 96-bit nonce, and 32-bit block counter.
- **Poly1305 (RFC 8439):** One-time message authentication code over the field 2^130 - 5.
- **ChaCha20-Poly1305 AEAD (RFC 8439 Section 2.8):** Authenticated Encryption with Associated Data.
- **X25519 (RFC 7748):** Montgomery ladder Diffie-Hellman key exchange over the Curve25519 elliptic curve.
- **Ed25519 (RFC 8032):** Edwards-curve Digital Signature Algorithm (EdDSA) using the twisted Edwards curve Ed25519 and SHA-512.
- **HKDF (RFC 5869):** HMAC-based Extract-and-Expand Key Derivation Function using SHA-256.
- **PBKDF2 (RFC 2898 / PKCS #5):** Password-Based Key Derivation Function using HMAC-SHA256 iterations.
- **Cryptographic Utilities:** System entropy retrieval, zeroing memory, constant-time equality checks, and Little-Endian/Big-Endian integer serialization.

---

## 1. Import: load std.crypto

Import the entire package:

```yukino
load std.crypto
```

This brings the following entities into scope:

- **Error structures:** `CryptoError`.
- **Streaming classes:** `HmacSha256`, `HmacSha512`, `ChaCha20`.
- **HMAC functions:** `hmac_sha256`, `hmac_sha256_hex`, `hmac_sha256_verify`, `hmac_sha512`, `hmac_sha512_hex`, `hmac_sha512_verify`, `hmac_sha256_init`, `hmac_sha512_init`.
- **Symmetric encryption functions:** `chacha20_crypt`, `chacha20_block`, `chacha20_poly1305_encrypt`, `chacha20_poly1305_decrypt`.
- **Poly1305 authentication functions:** `poly1305_mac`, `poly1305_hex`, `poly1305_verify`.
- **Curve25519 asymmetric functions:** `x25519_public_key`, `x25519_diffie_hellman`, `x25519`, `ed25519_public_key`, `ed25519_sign`, `ed25519_verify`.
- **Key derivation functions:** `hkdf_extract`, `hkdf_expand`, `hkdf`, `pbkdf2_sha256`.
- **Utilities:** `constant_time_eq`, `zero_bytes`, `random_bytes`, `load32_le`, `store32_le`, `load64_le`, `store64_le`, `load32_be`, `store32_be`, `load64_be`, `store64_be`.

All of these entities are also made available when importing the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.crypto`. All internal algorithm coefficients are encapsulated within the corresponding implementations.

---

## 3. Interfaces

No specialized interfaces are declared in `std.crypto`.

---

## 4. Error Structures

### 4.1 public struct CryptoError

The error structure used to signal invalid parameters (invalid key length, invalid nonce, negative buffer sizes).

```yukino
public struct CryptoError {
    message: string
    code: int32
}
```

- `message`: human-readable description of the error.
- `code`: numeric error code:
  - `1`: negative length in `zero_bytes`.
  - `2`: negative length in `random_bytes`.
  - `21`: invalid Poly1305 key length (must be exactly 32 bytes).
  - `31`: invalid ChaCha20-Poly1305 key length (must be 32 bytes).
  - `32`: invalid ChaCha20-Poly1305 nonce length (must be 12 bytes).
  - `33`: authentication tag mismatch during AEAD decryption.
  - `34`: ciphertext shorter than minimum tag length (16 bytes).
  - `41`: invalid private key length for X25519 (must be 32 bytes).
  - `42`: invalid public key length for X25519 (must be 32 bytes).
  - `51`: invalid seed length for Ed25519 (must be 32 bytes).
  - `52`: invalid secret key length for Ed25519 sign (must be 32 bytes).
  - `61`: requested HKDF OKM length exceeds 8160 bytes limit (255 * 32).
  - `71`: PBKDF2 iterations must be positive.
  - `72`: PBKDF2 key length cannot be negative.

---

## 5. Classes

### 5.1 public class HmacSha256

Streaming message authentication context using SHA-256.

```yukino
public class HmacSha256
```

#### Methods

- `public fun init()`: prepares an empty instance; the working context is created by the `hmac_sha256_init` factory (section 6.27) because a constructor has no destination region and cannot place inner hashers in class fields.
- `public fun update(data: bytes)`: feeds input data chunk. Panics with `CryptoError` (code 13) if the instance was created without the factory.
- `public fun finish() -> bytes`: returns 32-byte MAC tag. Panics with `CryptoError` (code 13) if the instance was created without the factory.
- `public fun finish_hex() -> string`: returns 64-character lowercase hex string. Panics with `CryptoError` (code 13) if the instance was created without the factory.

---

### 5.2 public class HmacSha512

Streaming message authentication context using SHA-512.

```yukino
public class HmacSha512
```

#### Methods

- `public fun init()`: prepares an empty instance; the working context is created by the `hmac_sha512_init` factory (section 6.28) because a constructor has no destination region and cannot place inner hashers in class fields.
- `public fun update(data: bytes)`: feeds input data chunk. Panics with `CryptoError` (code 13) if the instance was created without the factory.
- `public fun finish() -> bytes`: returns 64-byte MAC tag. Panics with `CryptoError` (code 13) if the instance was created without the factory.
- `public fun finish_hex() -> string`: returns 128-character hex string. Panics with `CryptoError` (code 13) if the instance was created without the factory.

---

### 5.3 public class ChaCha20

Streaming symmetric cipher state for ChaCha20.

```yukino
public class ChaCha20
```

#### Methods

- `public fun init(key: bytes, nonce: bytes, counter: uint32 = 0u32)`: initializes cipher with 32-byte key, 12-byte nonce, and block counter.
- `public fun apply(data: bytes) -> bytes`: encrypts or decrypts bytes in-place via XOR keystream.

---

## 6. Function Reference

### 6.1 constant_time_eq

Compares two byte slices in constant time to prevent side-channel timing leaks.

```yukino
public fun constant_time_eq(ref a: bytes, ref b: bytes) -> bool
```

Returns `true` if slices are identical in length and content, `false` otherwise. The function never panics.

---

### 6.2 zero_bytes

Creates a byte slice of given length filled with zeros.

```yukino
public fun zero_bytes(len: int64) -> bytes
```

Returns non-null `bytes`. Panics with `CryptoError` (code 1) if `len < 0`.

---

### 6.3 random_bytes

Generates cryptographically secure random bytes from operating system entropy.

```yukino
public fun random_bytes(len: int64) -> bytes
```

Returns non-null `bytes`. Panics with `CryptoError` (code 2) if `len < 0`.

---

### 6.4 hmac_sha256

Computes 32-byte HMAC-SHA256 for the given key and message.

```yukino
public fun hmac_sha256(ref key: bytes, ref data: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Never panics.

---

### 6.5 hmac_sha256_hex

Computes HMAC-SHA256 and formats it as a 64-character hex string.

```yukino
public fun hmac_sha256_hex(ref key: bytes, ref data: bytes) -> string
```

Returns non-null `string`. Never panics.

---

### 6.6 hmac_sha256_verify

Verifies HMAC-SHA256 in constant time.

```yukino
public fun hmac_sha256_verify(ref key: bytes, ref data: bytes, ref expected_mac: bytes) -> bool
```

Returns `true` if MAC matches, `false` otherwise. Never panics.

---

### 6.7 hmac_sha512

Computes 64-byte HMAC-SHA512.

```yukino
public fun hmac_sha512(ref key: bytes, ref data: bytes) -> bytes
```

Returns non-null 64-byte `bytes`. Never panics.

---

### 6.8 hmac_sha512_hex

Computes HMAC-SHA512 and returns a 128-character hex string.

```yukino
public fun hmac_sha512_hex(ref key: bytes, ref data: bytes) -> string
```

Returns non-null `string`. Never panics.

---

### 6.9 hmac_sha512_verify

Verifies HMAC-SHA512 in constant time.

```yukino
public fun hmac_sha512_verify(ref key: bytes, ref data: bytes, ref expected_mac: bytes) -> bool
```

Returns `true` if MAC matches, `false` otherwise. Never panics.

---

### 6.10 chacha20_crypt

Encrypts or decrypts data using ChaCha20 stream cipher per RFC 8439.

```yukino
public fun chacha20_crypt(ref key: bytes, ref nonce: bytes, counter: uint32, ref data: bytes) -> bytes
```

Returns non-null `bytes`. Panics with `CryptoError` if key length != 32 or nonce length != 12.

---

### 6.11 chacha20_block

Generates a 64-byte ChaCha20 keystream block.

```yukino
public fun chacha20_block(ref key: bytes, ref nonce: bytes, counter: uint32) -> bytes
```

Returns non-null 64-byte `bytes`. Panics with `CryptoError` on invalid lengths.

---

### 6.12 poly1305_mac

Computes a 16-byte Poly1305 MAC tag per RFC 8439 using a 32-byte one-time key.

```yukino
public fun poly1305_mac(ref key: bytes, ref data: bytes) -> bytes
```

Returns non-null 16-byte `bytes`. Panics with `CryptoError` (code 21) if key length != 32.

---

### 6.13 poly1305_hex

Computes Poly1305 MAC and returns a 32-character hex string.

```yukino
public fun poly1305_hex(ref key: bytes, ref data: bytes) -> string
```

Returns non-null `string`. Panics with `CryptoError` (code 21) if key length != 32.

---

### 6.14 poly1305_verify

Verifies a 16-byte Poly1305 tag in constant time.

```yukino
public fun poly1305_verify(ref key: bytes, ref data: bytes, ref tag: bytes) -> bool
```

Returns `true` if valid, `false` otherwise. Panics with `CryptoError` (code 21) if key length != 32.

---

### 6.15 chacha20_poly1305_encrypt

Performs ChaCha20-Poly1305 authenticated encryption with associated data per RFC 8439 Section 2.8.

```yukino
public fun chacha20_poly1305_encrypt(ref key: bytes, ref nonce: bytes, ref aad: bytes, ref plaintext: bytes) -> bytes
```

Returns ciphertext with appended 16-byte tag. Panics with `CryptoError` if key != 32 or nonce != 12.

---

### 6.16 chacha20_poly1305_decrypt

Authenticates and decrypts ChaCha20-Poly1305 AEAD payload.

```yukino
public fun chacha20_poly1305_decrypt(ref key: bytes, ref nonce: bytes, ref aad: bytes, ref ciphertext_and_tag: bytes) -> bytes
```

Returns decrypted plaintext `bytes`. Panics with `CryptoError` (code 33 or 34) if authentication fails or length < 16.

---

### 6.17 x25519_public_key

Derives a 32-byte X25519 public key from a 32-byte private key.

```yukino
public fun x25519_public_key(ref private_key: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Panics with `CryptoError` (code 41) if private key length != 32.

---

### 6.18 x25519_diffie_hellman

Calculates X25519 shared secret between local private key and remote public key.

```yukino
public fun x25519_diffie_hellman(ref private_key: bytes, ref public_key: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Panics with `CryptoError` on invalid key lengths.

---

### 6.19 x25519

Performs scalar multiplication on Curve25519 per RFC 7748.

```yukino
public fun x25519(ref scalar: bytes, ref point: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Panics with `CryptoError` on invalid lengths.

---

### 6.20 ed25519_public_key

Derives a 32-byte Ed25519 public verification key from a 32-byte seed.

```yukino
public fun ed25519_public_key(ref seed: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Panics with `CryptoError` (code 51) if seed length != 32.

---

### 6.21 ed25519_sign

Computes a 64-byte Ed25519 digital signature over message with a 32-byte seed.

```yukino
public fun ed25519_sign(ref seed: bytes, ref message: bytes) -> bytes
```

Returns non-null 64-byte `bytes`. Panics with `CryptoError` (code 52) if seed length != 32.

---

### 6.22 ed25519_verify

Verifies a 64-byte Ed25519 digital signature against a 32-byte public key.

```yukino
public fun ed25519_verify(ref public_key: bytes, ref message: bytes, ref signature: bytes) -> bool
```

Returns `true` if signature is valid, `false` otherwise. Never panics.

---

### 6.23 hkdf_extract

Extracts a pseudorandom key (PRK) from salt and input key material per RFC 5869.

```yukino
public fun hkdf_extract(ref salt: bytes, ref ikm: bytes) -> bytes
```

Returns non-null 32-byte `bytes`. Never panics.

---

### 6.24 hkdf_expand

Expands PRK into desired output key material length.

```yukino
public fun hkdf_expand(ref prk: bytes, ref info: bytes, okm_len: int64) -> bytes
```

Returns non-null `bytes`. Panics with `CryptoError` (code 61) if `okm_len > 8160` or `okm_len < 0`.

---

### 6.25 hkdf

Runs full HKDF extract-and-expand pipeline per RFC 5869.

```yukino
public fun hkdf(ref salt: bytes, ref ikm: bytes, ref info: bytes, okm_len: int64) -> bytes
```

Returns non-null derived key `bytes`. Panics with `CryptoError` if `okm_len` is invalid.

---

### 6.26 pbkdf2_sha256

Derives a key from password and salt using PBKDF2 with HMAC-SHA256 per RFC 2898.

```yukino
public fun pbkdf2_sha256(ref password: bytes, ref salt: bytes, iterations: int32, key_len: int64) -> bytes
```

Returns non-null derived `bytes`. Panics with `CryptoError` (code 71 or 72) if `iterations <= 0` or `key_len < 0`.

---

### 6.27 hmac_sha256_init

Creates a ready-to-use HMAC-SHA256 context keyed with an arbitrary-length key per RFC 2104. The inner hashers are placed in the caller's region, so the factory is the only safe way to construct `HmacSha256`.

```yukino
public fun hmac_sha256_init(ref key: bytes) -> HmacSha256
```

Returns a `HmacSha256` instance. The value is never `null`.

The function never panics.

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

Creates a ready-to-use HMAC-SHA512 context keyed with an arbitrary-length key. The inner hashers are placed in the caller's region, so the factory is the only safe way to construct `HmacSha512`.

```yukino
public fun hmac_sha512_init(ref key: bytes) -> HmacSha512
```

Returns a `HmacSha512` instance. The value is never `null`.

The function never panics.

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

[← Back: Module std.hash](26-hash.md) | [Next: Module std.net →](28-net.md)
