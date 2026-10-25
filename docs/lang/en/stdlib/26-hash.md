# Module std.hash

The `std.hash` module provides a comprehensive suite of checksum and cryptographic hashing algorithms implemented entirely in pure Yukino without external runtime dependencies. The module unifies ultra-fast non-cryptographic hashing algorithms with globally recognized cryptographic standards:

- **BitHash64:** an ultra-fast non-cryptographic 64-bit hash optimized for maximum throughput ([original repository](https://github.com/shareui/bithash)). Although not widely known across the industry, it is included in the standard library as a project created by the lead developer and founder of Yukino, and in practice BitHash frequently proves fast and helpful in application workloads. It is specifically designed for hash tables, in-memory caches, Bloom filters, and deduplication systems where single-cycle CPU performance and uniform avalanche bit distribution are paramount.
- **SHA-2 (SHA-256, SHA-512):** the industry-standard cryptographic hash suite (FIPS 180-4, RFC 6234). Essential for compatibility with established network protocols, TLS certificate verification, digital signatures, blockchain data structures, and enterprise compliance requirements.
- **SHA-3 (SHA3-256, SHA3-512):** the modern cryptographic hash standard (FIPS 202) based on the Keccak-f[1600] permutation-sponge construction. Featuring an internal structure fundamentally distinct from the MD/SHA families, SHA-3 is mathematically immune to length extension attacks and provides a robust cryptographic alternative to SHA-2.
- **BLAKE3:** a cutting-edge, high-speed cryptographic hash built on a Merkle tree structure and the ChaCha permutation core. Delivering 128-bit cryptographic security (equivalent to SHA-256), BLAKE3 outpaces traditional hash functions by multiples when processing large files and data streams.
- **Cryptographic Utilities:** hex encoding and decoding (`bytes_to_hex`, `hex_to_bytes`), constant-time comparison primitives resistant to side-channel timing attacks (`constant_time_eq`, `constant_time_eq_str`, `equal`), and streaming file equality verification (`file_eq`).

---

## 1. Import: load std.hash

Import the entire package:

```yukino
load std.hash
```

This brings the following entities into scope:

- **Error Structures:** `HashError`.
- **Streaming Hasher Classes:** `BitHash64`, `Sha256`, `Sha512`, `Sha3_256`, `Sha3_512`, `Blake3`.
- **Free Hashing Functions:** `bithash64`, `bithash64_str`, `bithash64_file`, `sha256`, `sha256_str`, `sha256_file`, `sha512`, `sha512_str`, `sha512_file`, `sha3_256`, `sha3_256_str`, `sha3_256_file`, `sha3_512`, `sha3_512_str`, `sha3_512_file`, `blake3`, `blake3_str`, `blake3_file`.
- **Streaming Class Factories:** `sha3_256_init`, `sha3_512_init`, `blake3_init`.
- **Helper Utilities:** `bytes_to_hex`, `hex_to_bytes`, `constant_time_eq`, `constant_time_eq_str`, `equal`, `file_eq`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

No global constants are declared in `std.hash`. Internal initialization vectors and permutation tables are encapsulated within their respective classes and functions.

---

## 3. Interfaces

No interfaces are declared in `std.hash`.

---

## 4. Data and Panic Structures

### 4.1 public struct HashError

An error structure thrown upon encountering invalid input arguments (for example, odd string length or illegal characters during hexadecimal decoding).

```yukino
public struct HashError {
    message: string
    code: int32
}
```

- `message`: human-readable explanation of the error.
- `code`: numeric error code:
  - `101`: odd string length provided to hexadecimal decoder.
  - `102`: invalid non-hexadecimal character encountered.

---

## 5. Classes

### 5.1 class BitHash64

Incremental streaming calculator for the 64-bit non-cryptographic BitHash64 algorithm.

```yukino
public class BitHash64 {
    // Internal BitHash64 state
}
```

#### 5.1.1 init

Initializes a new instance of `BitHash64` with the canonical seed state.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

```yukino
load std.hash.bithash64: BitHash64

fun main() {
    set h = init BitHash64()
}
```

#### 5.1.2 update

Updates the hasher state with the provided byte slice.

```yukino
public fun update(data: bytes)
```

The method returns no value.

The method never panics.

#### 5.1.3 digest

Finalizes the calculation and returns the resulting 64-bit integer.

```yukino
public fun digest() -> uint64
```

Returns a non-nullable integer of type `uint64`.

The method never panics.

#### 5.1.4 reset

Resets the hasher state to its initial seed for object reuse without reallocating memory.

```yukino
public fun reset()
```

The method returns no value.

The method never panics.

---

### 5.2 class Sha256

Streaming calculator for the cryptographic SHA-256 hash (256 bits / 32 bytes).

```yukino
public class Sha256 {
    // Internal SHA-256 state
}
```

#### 5.2.1 init

Initializes a new instance of `Sha256` with FIPS 180-4 standard initialization constants.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

#### 5.2.2 update

Feeds an arbitrary chunk of byte data into the hash stream.

```yukino
public fun update(data: bytes)
```

The method returns no value.

The method never panics.

#### 5.2.3 finalize

Applies length padding and returns the final 32-byte hash slice.

```yukino
public fun finalize() -> bytes
```

Returns a non-nullable byte slice of type `bytes` containing exactly 32 bytes.

The method never panics.

#### 5.2.4 reset

Resets the hasher state for repeated use.

```yukino
public fun reset()
```

The method returns no value.

The method never panics.

---

### 5.3 class Sha512

Streaming calculator for the cryptographic SHA-512 hash (512 bits / 64 bytes) operating on 64-bit words.

```yukino
public class Sha512 {
    // Internal SHA-512 state
}
```

#### 5.3.1 init

Initializes a new instance of `Sha512`.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

#### 5.3.2 update

Feeds data into the hash stream.

```yukino
public fun update(data: bytes)
```

The method returns no value.

The method never panics.

#### 5.3.3 finalize

Finalizes the calculation and returns the 64-byte hash slice.

```yukino
public fun finalize() -> bytes
```

Returns a non-nullable byte slice of type `bytes` containing exactly 64 bytes.

The method never panics.

#### 5.3.4 reset

Resets the hasher state to initial constants.

```yukino
public fun reset()
```

The method returns no value.

The method never panics.

---

### 5.4 class Sha3_256

Streaming calculator for SHA3-256 per FIPS 202 (Keccak-f[1600] with 512-bit capacity and 1088-bit absorption rate).

```yukino
public class Sha3_256 {
    // Internal SHA3-256 state
}
```

#### 5.4.1 init

Initializes only the scalar fields of the state. Working instances are created by the `sha3_256_init()` factory (section 6.25): the constructor has no destination region and cannot allocate the sponge state, so a directly created `init Sha3_256()` stays uninitialized.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

#### 5.4.2 update

Absorbs a chunk of bytes into the Keccak sponge state.

```yukino
public fun update(data: bytes)
```

The method returns no value.

If the instance was created without the `sha3_256_init` factory, panics with the `HashError` struct (code 105).

#### 5.4.3 finish

Applies the SHA-3 domain separator (`0x06`), performs final Pad10*1 padding, and squeezes 32 bytes of output.

```yukino
public fun finish() -> bytes
```

Returns a non-nullable byte slice of type `bytes` containing 32 bytes.

If the instance was created without the `sha3_256_init` factory, panics with the `HashError` struct (code 105).

#### 5.4.4 reset

Resets the Keccak sponge state for reuse.

```yukino
public fun reset()
```

The method returns no value.

If the instance was created without the `sha3_256_init` factory, panics with the `HashError` struct (code 105).

---

### 5.5 class Sha3_512

Streaming calculator for SHA3-512 per FIPS 202 (Keccak-f[1600] with 1024-bit capacity and 576-bit absorption rate).

```yukino
public class Sha3_512 {
    // Internal SHA3-512 state
}
```

#### 5.5.1 init

Initializes only the scalar fields of the state. Working instances are created by the `sha3_512_init()` factory (section 6.26): the constructor has no destination region and cannot allocate the sponge state.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

#### 5.5.2 update

Absorbs a chunk of bytes into the sponge state.

```yukino
public fun update(data: bytes)
```

The method returns no value.

If the instance was created without the `sha3_512_init` factory, panics with the `HashError` struct (code 105).

#### 5.5.3 finish

Finalizes the sponge state and squeezes 64 bytes of output.

```yukino
public fun finish() -> bytes
```

Returns a non-nullable byte slice of type `bytes` containing 64 bytes.

If the instance was created without the `sha3_512_init` factory, panics with the `HashError` struct (code 105).

#### 5.5.4 reset

Resets the hasher state to its initial zero state.

```yukino
public fun reset()
```

The method returns no value.

If the instance was created without the `sha3_512_init` factory, panics with the `HashError` struct (code 105).

---

### 5.6 class Blake3

Streaming calculator for the ultra-fast tree-based BLAKE3 cryptographic hash.

```yukino
public class Blake3 {
    // Internal BLAKE3 tree state
}
```

#### 5.6.1 init

Initializes only the scalar fields (the `IV` and the block buffer). Working instances are created by the `blake3_init()` factory (section 6.27): the constructor has no destination region and cannot allocate the tree node stack.

```yukino
public fun init()
```

The constructor returns no value.

The method never panics.

#### 5.6.2 update

Feeds an arbitrary chunk of data. Automatically buffers 1024-byte chunks and merges Merkle tree nodes.

```yukino
public fun update(data: bytes)
```

The method returns no value.

If the instance was created without the `blake3_init` factory, panics with the `HashError` struct (code 105).

#### 5.6.3 finish

Finalizes the Merkle tree and produces a 32-byte root hash.

```yukino
public fun finish() -> bytes
```

Returns a non-nullable byte slice of type `bytes` containing 32 bytes.

If the instance was created without the `blake3_init` factory, panics with the `HashError` struct (code 105).

#### 5.6.4 reset

Resets the tree state and node stack for a new calculation.

```yukino
public fun reset()
```

The method returns no value.

If the instance was created without the `blake3_init` factory, panics with the `HashError` struct (code 105).

---

## 6. Function Reference

### 6.1 bithash64

Calculates a 64-bit non-cryptographic hash from the given byte slice.

```yukino
public fun bithash64(ref data: bytes) -> uint64
```

Returns a non-nullable integer of type `uint64`.

The function never panics.

```yukino
load std.hash.bithash64: bithash64
load std.io: println

fun main() {
    set raw = cast("sample data", "bytes")
    set h = bithash64(ref raw)
    println("BitHash64: " + cast(h, "string"))
}
```

### 6.2 bithash64_str

Calculates a 64-bit non-cryptographic hash from the given string.

```yukino
public fun bithash64_str(s: string) -> uint64
```

Returns a non-nullable integer of type `uint64`.

The function never panics.

```yukino
load std.hash.bithash64: bithash64_str
load std.io: println

fun main() {
    set key = "session_token"
    set h = bithash64_str(key)
    println("Hash: " + cast(h, "string"))
}
```

### 6.3 bithash64_file

Calculates a 64-bit non-cryptographic hash of the file at the given path in streaming 64 KB blocks without buffering the entire file into memory.

```yukino
public fun bithash64_file(path: string) -> uint64
```

Returns a non-nullable integer of type `uint64`.

Panics with an `IoError` structure if the file cannot be opened or read.

```yukino
load std.hash.bithash64: bithash64_file
load std.io: println

fun main() {
    set h = bithash64_file("data.bin")
    println("File hash: " + cast(h, "string"))
}
```

### 6.4 sha256

Computes the cryptographic SHA-256 hash of the given byte slice and returns a lowercase 64-character hexadecimal string.

```yukino
public fun sha256(ref data: bytes) -> string
```

Returns a non-nullable hexadecimal string.

The function never panics.

```yukino
load std.hash.sha2: sha256
load std.io: println

fun main() {
    set raw = cast("hello", "bytes")
    println(sha256(ref raw)) // 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
}
```

### 6.5 sha256_str

Computes the cryptographic SHA-256 hash of the given string.

```yukino
public fun sha256_str(s: string) -> string
```

Returns a non-nullable hexadecimal string.

The function never panics.

```yukino
load std.hash.sha2: sha256_str
load std.io: println

fun main() {
    set msg = "hello"
    println(sha256_str(msg))
}
```

### 6.6 sha256_file

Calculates the SHA-256 hash of the file at the specified path in streaming 64 KB chunks.

```yukino
public fun sha256_file(path: string) -> string
```

Returns a non-nullable hexadecimal string.

Panics with an `IoError` structure if filesystem access fails.

```yukino
load std.hash.sha2: sha256_file
load std.io: println

fun main() {
    set sum = sha256_file("archive.tar")
    println("SHA-256: " + sum)
}
```

### 6.7 sha512

Computes the cryptographic SHA-512 hash of the given byte slice and returns a 128-character hexadecimal string.

```yukino
public fun sha512(ref data: bytes) -> string
```

Returns a non-nullable string.

The function never panics.

### 6.8 sha512_str

Computes the cryptographic SHA-512 hash of the given string.

```yukino
public fun sha512_str(s: string) -> string
```

Returns a non-nullable string.

The function never panics.

### 6.9 sha512_file

Computes the cryptographic SHA-512 hash of a file in streaming mode.

```yukino
public fun sha512_file(path: string) -> string
```

Returns a non-nullable string.

Panics with an `IoError` structure if the file cannot be read.

### 6.10 sha3_256

Computes the SHA3-256 hash per FIPS 202 of the given byte slice and returns a 64-character hexadecimal string.

```yukino
public fun sha3_256(ref data: bytes) -> string
```

Returns a non-nullable string.

The function never panics.

```yukino
load std.hash.sha3: sha3_256
load std.io: println

fun main() {
    set b = cast("hello", "bytes")
    println(sha3_256(ref b)) // 3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392
}
```

### 6.11 sha3_256_str

Computes the SHA3-256 hash of the given string.

```yukino
public fun sha3_256_str(s: string) -> string
```

Returns a non-nullable string.

The function never panics.

### 6.12 sha3_256_file

Computes the SHA3-256 hash of a file in streaming mode.

```yukino
public fun sha3_256_file(path: string) -> string
```

Returns a non-nullable string.

Panics with an `IoError` structure if filesystem access fails.

### 6.13 sha3_512

Computes the SHA3-512 hash per FIPS 202 of the given byte slice and returns a 128-character hexadecimal string.

```yukino
public fun sha3_512(ref data: bytes) -> string
```

Returns a non-nullable string.

The function never panics.

### 6.14 sha3_512_str

Computes the SHA3-512 hash of the given string.

```yukino
public fun sha3_512_str(s: string) -> string
```

Returns a non-nullable string.

The function never panics.

### 6.15 sha3_512_file

Computes the SHA3-512 hash of a file in streaming mode.

```yukino
public fun sha3_512_file(path: string) -> string
```

Returns a non-nullable string.

Panics with an `IoError` structure if filesystem access fails.

### 6.16 blake3

Computes the cryptographic BLAKE3 hash of the given byte slice and returns a 64-character hexadecimal string.

```yukino
public fun blake3(ref data: bytes) -> string
```

Returns a non-nullable string.

The function never panics.

```yukino
load std.hash.blake3: blake3
load std.io: println

fun main() {
    set b = cast("hello", "bytes")
    println(blake3(ref b)) // ea8f163db38682925e4491c5e58d4bb3506ef8c14eb78a86e908c5624a67200f
}
```

### 6.17 blake3_str

Computes the BLAKE3 hash of the given string.

```yukino
public fun blake3_str(s: string) -> string
```

Returns a non-nullable string.

The function never panics.

```yukino
load std.hash.blake3: blake3_str
load std.io: println

fun main() {
    set text = "The quick brown fox jumps over the lazy dog"
    println(blake3_str(text))
}
```

### 6.18 blake3_file

Computes the BLAKE3 hash of a file at the given path in streaming 64 KB chunks, building the Merkle tree dynamically.

```yukino
public fun blake3_file(path: string) -> string
```

Returns a non-nullable string.

Panics with an `IoError` structure if filesystem access fails.

```yukino
load std.hash.blake3: blake3_file
load std.io: println

fun main() {
    set h = blake3_file("/var/log/syslog")
    println("BLAKE3: " + h)
}
```

### 6.19 bytes_to_hex

Encodes an arbitrary byte slice into a lowercase hexadecimal string (2 characters per byte).

```yukino
public fun bytes_to_hex(ref data: bytes) -> string
```

Returns a non-nullable hexadecimal string.

The function never panics.

```yukino
load std.hash.common: bytes_to_hex
load std.io: println

fun main() {
    set raw = cast("abc", "bytes")
    println(bytes_to_hex(ref raw)) // "616263"
}
```

### 6.20 hex_to_bytes

Decodes a hexadecimal string into a byte slice. Supports both uppercase (`A-F`) and lowercase (`a-f`) characters.

```yukino
public fun hex_to_bytes(hex: string) -> bytes
```

Returns a non-nullable byte slice of type `bytes`.

Panics with `HashError` (code `101`) if the string length is odd. Panics with `HashError` (code `102`) if an invalid character is encountered.

```yukino
load std.hash.common: hex_to_bytes, bytes_to_hex
load std.io: println

fun main() {
    set hex = "deadbeef"
    set b = hex_to_bytes(hex)
    println(bytes_to_hex(ref b)) // "deadbeef"
}
```

### 6.21 constant_time_eq

Performs a byte-by-byte comparison of two `bytes` slices in constant time to prevent side-channel timing leaks.

```yukino
public fun constant_time_eq(ref a: bytes, ref b: bytes) -> bool
```

Returns `true` if both slices are identical in length and content, `false` otherwise.

The function never panics.

### 6.22 constant_time_eq_str

Compares two strings in constant time.

```yukino
public fun constant_time_eq_str(a: string, b: string) -> bool
```

Returns `true` if both strings are identical, `false` otherwise.

The function never panics.

```yukino
load std.hash.common: constant_time_eq_str
load std.io: println

fun main() {
    set token1 = "secret_auth_token"
    set token2 = "secret_auth_token"
    if constant_time_eq_str(token1, token2) {
        println("Tokens match securely")
    }
}
```

### 6.23 equal

Convenience constant-time string comparison alias for validating hash sums and tokens.

```yukino
public fun equal(a: string, b: string) -> bool
```

Returns `true` if strings are equal, `false` otherwise.

The function never panics.

### 6.24 file_eq

Compares the contents of two files on disk using streaming BLAKE3 hashing without buffering the entire files into memory.

```yukino
public fun file_eq(path1: string, path2: string) -> bool
```

Returns `true` if both files exist and possess identical checksums, `false` otherwise. If file sizes differ, the function immediately returns `false` without computing hashes.

Panics with an `IoError` structure if a file cannot be opened or read.

```yukino
load std.hash.common: file_eq
load std.io: println

fun main() {
    if file_eq("original.iso", "copy.iso") {
        println("Files are byte-for-byte identical")
    }
}
```

---

### 6.25 sha3_256_init

Creates a ready-to-use `Sha3_256` instance with a zeroed sponge state. The only safe way to construct the class: the factory places the state slices in the caller's region, which a constructor without a destination region cannot do.

```yukino
public fun sha3_256_init() -> Sha3_256
```

Returns a `Sha3_256` instance. The value is never `null`.

The function never panics.

```yukino
load std.hash.sha3: sha3_256_init
load std.io: println

fun main() {
    set st = sha3_256_init()
    st.update(cast("abc", "bytes"))
    set h = st.finish()
}
```

---

### 6.26 sha3_512_init

Creates a ready-to-use `Sha3_512` instance with a zeroed sponge state. The only safe way to construct the class.

```yukino
public fun sha3_512_init() -> Sha3_512
```

Returns a `Sha3_512` instance. The value is never `null`.

The function never panics.

```yukino
load std.hash.sha3: sha3_512_init
load std.io: println

fun main() {
    set st = sha3_512_init()
    st.update(cast("abc", "bytes"))
    set h = st.finish()
}
```

---

### 6.27 blake3_init

Creates a ready-to-use `Blake3` instance with the default `IV` and an allocated tree node stack. The only safe way to construct the class.

```yukino
public fun blake3_init() -> Blake3
```

Returns a `Blake3` instance. The value is never `null`.

The function never panics.

```yukino
load std.hash.blake3: blake3_init
load std.io: println

fun main() {
    set st = blake3_init()
    st.update(cast("abc", "bytes"))
    set h = st.finish()
}
```

---

[← Back: Module std.stdotool](25-stdotool.md) | [Next: Module std.crypto →](27-crypto.md)
