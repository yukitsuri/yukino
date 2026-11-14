# Module std.fs.archives

The `std.fs.archives` module provides tools for working with standard archive formats TAR and ZIP, including archive creation, content listing, extracting files and directories, transparent compression (Gzip, Zstandard, XZ, Bzip2, Deflate), and password-based encryption (ZipCrypto, AES-CBC, AES-CTR, WinZip AES). Cryptographic primitives are available through the submodule `std.fs.archives.pass`.

---

## 1. Import: load std.fs.archives

Importing the entire package:

```yukino
load std.fs.archives
// For archive encryption and decryption primitives:
load std.fs.archives.pass
```

When importing the module, the following entities become available:

- **Data and Error Structures:** `TarEntry`, `TarError`, `ZipEntry`, `ZipError`, `PassError`, `WinZipAesResult`.
- **TAR Functions:** `tar_pack`, `tar_pack_dir`, `tar_unpack`, `tar_list`, `tar_pack_encrypted`, `tar_unpack_encrypted`.
- **ZIP Functions:** `zip_pack`, `zip_pack_dir`, `zip_pack_with_password`, `zip_unpack`, `zip_unpack_with_password`, `zip_list`.
- **PASS Functions:** `zipcrypto_encrypt`, `zipcrypto_decrypt`, `aes_ctr`, `aes_cbc_encrypt`, `aes_cbc_decrypt`, `winzip_aes_encrypt`, `winzip_aes_decrypt`.

TAR and ZIP entities also automatically become available when loading `load std.fs` or the whole standard library: `load std`.

---

## 2. Constants

The `std.fs.archives` module declares no global constants.

---

## 3. Interfaces

The `std.fs.archives` module declares no interfaces.

---

## 4. Data and Panic Error Structures

### 4.1 TarEntry

Represents an entry inside a TAR archive (file, directory, or symlink).

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

Error structure raised by TAR functions on missing files, malformed ustar headers, or I/O failures.

```yukino
public struct TarError {
    message: string
    code: int32
}
```

### 4.3 ZipEntry

Represents an entry inside a ZIP archive in the Central Directory.

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

Error structure raised by ZIP functions when encountering missing EOCD/local headers, corrupted compressed streams, CRC-32 checksum mismatches, or password errors.

```yukino
public struct ZipError {
    message: string
    code: int32
}
```

### 4.5 PassError

Error structure raised by crypto routines on invalid key/IV lengths, PKCS7 padding errors, or WinZip AES check value mismatches.

```yukino
public struct PassError {
    message: string
    code: int32
}
```

### 4.6 WinZipAesResult

Result of a WinZip AES encryption operation containing encrypted ciphertext and a two-byte password verification value.

```yukino
public struct WinZipAesResult {
    data: bytes
    check_value: uint16
}
```

---

## 5. Classes

The `std.fs.archives` module declares no classes.

---

## 6. Function Reference

### 6.1 tar_pack

Packs a list of paths (files and directories recursively) into a TAR archive at `archive_path`. The `compression` parameter defaults to `"auto"` and detects compression by file extension (`.tar`, `.tar.gz`/`.tgz`, `.tar.xz`/`.txz`, `.tar.zst`/`.tzst`, `.tar.bz2`/`.tbz2`), or can be explicitly specified as `"none"`, `"gz"`, `"xz"`, `"zstd"`, `"bz2"`.

```yukino
public fun tar_pack(archive_path: string, paths: string[], compression: string = "auto") -> bool
```

Returns boolean `true` on success. The return value is non-nullable.

Panics with a `TarError` structure (code `404`) if a path does not exist or I/O error occurs.

```yukino
load std.fs.archives.tar: tar_pack
load std.io: println

tar_pack("release.tar.gz", ["src", "README.md"])
println("TAR archive created")
```

### 6.2 tar_pack_dir

Convenience helper to pack a single directory `dir_path` into a TAR archive.

```yukino
public fun tar_pack_dir(archive_path: string, dir_path: string, compression: string = "auto") -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `TarError` structure if the directory does not exist.

```yukino
load std.fs.archives.tar: tar_pack_dir
load std.io: println

tar_pack_dir("backup.tar.zst", "my_project")
println("Directory archived")
```

### 6.3 tar_unpack

Extracts a TAR archive (with automatic compression format detection) into `dest_dir`, recreating directory hierarchy.

```yukino
public fun tar_unpack(archive_path: string, dest_dir: string) -> bool
```

Returns boolean `true` on successful extraction. The return value is non-nullable.

Panics with a `TarError` structure if the archive is missing or corrupted.

```yukino
load std.fs.archives.tar: tar_unpack
load std.io: println

tar_unpack("release.tar.gz", "extracted_files")
println("Files unpacked")
```

### 6.4 tar_list

Reads a TAR archive and returns an array of `TarEntry` structures without writing files to disk.

```yukino
public fun tar_list(archive_path: string) -> TarEntry[]
```

Returns an array of `TarEntry[]`. The return value is non-nullable.

Panics with a `TarError` structure if the archive does not exist.

```yukino
load std.fs.archives.tar: tar_list
load std.io: println

set entries = tar_list("release.tar.gz")
println("Entries count: ${entries.len()}")
```

### 6.5 tar_pack_encrypted

Packs paths into a TAR archive and encrypts the archive using AES-256-CBC with the supplied `password`.

```yukino
public fun tar_pack_encrypted(archive_path: string, paths: string[], password: string) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `TarError` or `PassError` structure on failure.

```yukino
load std.fs.archives.tar: tar_pack_encrypted
load std.io: println

tar_pack_encrypted("secure.tar.enc", ["secrets"], "StrongMasterKey123")
println("Encrypted TAR created")
```

### 6.6 tar_unpack_encrypted

Decrypts and unpacks an encrypted TAR archive into `dest_dir` using `password`.

```yukino
public fun tar_unpack_encrypted(archive_path: string, dest_dir: string, password: string) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `TarError` or `PassError` structure on invalid password or missing file.

```yukino
load std.fs.archives.tar: tar_unpack_encrypted
load std.io: println

tar_unpack_encrypted("secure.tar.enc", "unpacked_secrets", "StrongMasterKey123")
println("Encrypted TAR unpacked")
```

### 6.7 zip_pack

Packs an array of paths into a standard ZIP archive. The `method` parameter selects the compression algorithm: `8` for Deflate (default), `0` for Store (uncompressed).

```yukino
public fun zip_pack(archive_path: string, paths: string[], method: uint16 = 8) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `ZipError` structure (code `404`) if a path does not exist.

```yukino
load std.fs.archives.zip: zip_pack
load std.io: println

zip_pack("archive.zip", ["docs", "config.json"])
println("ZIP archive created")
```

### 6.8 zip_pack_dir

Packs a single directory `dir_path` into a ZIP archive.

```yukino
public fun zip_pack_dir(archive_path: string, dir_path: string, method: uint16 = 8) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `ZipError` structure on failure.

```yukino
load std.fs.archives.zip: zip_pack_dir
load std.io: println

zip_pack_dir("project.zip", "my_folder")
println("Folder packed into ZIP")
```

### 6.9 zip_pack_with_password

Packs files into a ZIP archive and encrypts each file payload using PKWARE ZipCrypto and the given `password`.

```yukino
public fun zip_pack_with_password(archive_path: string, paths: string[], password: string, method: uint16 = 8) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `ZipError` structure on error.

```yukino
load std.fs.archives.zip: zip_pack_with_password
load std.io: println

zip_pack_with_password("protected.zip", ["private_data"], "Secret123")
println("Password-protected ZIP created")
```

### 6.10 zip_unpack

Unpacks a ZIP archive into `dest_dir`.

```yukino
public fun zip_unpack(archive_path: string, dest_dir: string) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `ZipError` structure if the archive is missing, corrupted, or CRC-32 checksum fails. If the archive is password-protected, panics with code `302`.

```yukino
load std.fs.archives.zip: zip_unpack
load std.io: println

zip_unpack("archive.zip", "extracted_zip")
println("ZIP extracted successfully")
```

### 6.11 zip_unpack_with_password

Unpacks a ZIP archive using `password` to decrypt password-protected entries.

```yukino
public fun zip_unpack_with_password(archive_path: string, dest_dir: string, password: string) -> bool
```

Returns boolean `true`. The return value is non-nullable.

Panics with a `ZipError` structure on incorrect password, stream corruption, or CRC-32 mismatch.

```yukino
load std.fs.archives.zip: zip_unpack_with_password
load std.io: println

zip_unpack_with_password("protected.zip", "unpacked_zip", "Secret123")
println("Password-protected ZIP extracted")
```

### 6.12 zip_list

Parses the Central Directory of a ZIP archive and returns an array of `ZipEntry` structures without unpacking files.

```yukino
public fun zip_list(archive_path: string) -> ZipEntry[]
```

Returns an array of `ZipEntry[]`. The return value is non-nullable.

Panics with a `ZipError` structure if EOCD is missing or archive is invalid.

```yukino
load std.fs.archives.zip: zip_list
load std.io: println

set items = zip_list("archive.zip")
println("ZIP entry count: ${items.len()}")
```

### 6.13 zipcrypto_encrypt

Encrypts a byte buffer using standard PKWARE ZipCrypto stream cipher.

```yukino
public fun zipcrypto_encrypt(password: string, data: bytes) -> bytes
```

Returns the encrypted byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure on failure.

```yukino
load std.fs.archives.pass: zipcrypto_encrypt
load std.io: println

set enc = zipcrypto_encrypt("key", cast("secret data", "bytes"))
println("Encrypted bytes: ${enc.len()}")
```

### 6.14 zipcrypto_decrypt

Decrypts a byte buffer using PKWARE ZipCrypto stream cipher.

```yukino
public fun zipcrypto_decrypt(password: string, data: bytes) -> bytes
```

Returns the decrypted byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure on failure.

```yukino
load std.fs.archives.pass: zipcrypto_decrypt
load std.io: println

set dec = zipcrypto_decrypt("key", enc_bytes)
println(cast(dec, "string"))
```

### 6.15 aes_ctr

Performs symmetric encryption or decryption using AES in Counter (CTR) mode. Key `key` can be 16, 24, or 32 bytes (AES-128/192/256), `iv` must be 16 bytes.

```yukino
public fun aes_ctr(key: bytes, iv: bytes, data: bytes) -> bytes
```

Returns the transformed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure (code `202`) on invalid IV length, and with code `203` on invalid key length.

```yukino
load std.fs.archives.pass: aes_ctr
load std.io: println

set key = cast("0123456789abcdef0123456789abcdef", "bytes")
set iv = cast("0123456789abcdef", "bytes")
set enc = aes_ctr(key.cp(), iv.cp(), cast("payload", "bytes"))
println("AES-CTR size: ${enc.len()}")
```

### 6.16 aes_cbc_encrypt

Encrypts a byte buffer using AES in Cipher Block Chaining (CBC) mode with PKCS7 padding.

```yukino
public fun aes_cbc_encrypt(key: bytes, iv: bytes, data: bytes) -> bytes
```

Returns the encrypted byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure on invalid parameter sizes.

```yukino
load std.fs.archives.pass: aes_cbc_encrypt
load std.io: println

set enc = aes_cbc_encrypt(key, iv, cast("payload", "bytes"))
println("AES-CBC size: ${enc.len()}")
```

### 6.17 aes_cbc_decrypt

Decrypts a byte buffer using AES in CBC mode, validating and stripping PKCS7 padding.

```yukino
public fun aes_cbc_decrypt(key: bytes, iv: bytes, data: bytes) -> bytes
```

Returns the decrypted byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure on padding error or invalid keys.

```yukino
load std.fs.archives.pass: aes_cbc_decrypt
load std.io: println

set dec = aes_cbc_decrypt(key, iv, enc)
println(cast(dec, "string"))
```

### 6.18 winzip_aes_encrypt

Encrypts data according to the WinZip AES specification (PBKDF2-HMAC-SHA1 key derivation with AES-CTR encryption). Accepts `aes_bits` (`128`, `192`, or `256`) and random salt `salt` (sizes recommended by the WinZip specification: 8 bytes for 128-bit, 16 bytes for 256-bit; the salt length is not validated by the code).

```yukino
public fun winzip_aes_encrypt(password: string, aes_bits: int32, salt: bytes, data: bytes) -> WinZipAesResult
```

Returns a `WinZipAesResult` containing the ciphertext buffer `data` and a 2-byte verification value `check_value`. The return value is non-nullable.

Panics with a `PassError` structure on key derivation failure.

```yukino
load std.fs.archives.pass: winzip_aes_encrypt
load std.io: println

set res = winzip_aes_encrypt("Pass", 256, salt, cast("secret", "bytes"))
println("Verification value: ${res.check_value}")
```

### 6.19 winzip_aes_decrypt

Decrypts WinZip AES data, verifying the password against `check_val`.

```yukino
public fun winzip_aes_decrypt(password: string, aes_bits: int32, salt: bytes, data: bytes, check_val: uint16) -> bytes
```

Returns the decrypted byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `PassError` structure (code `204`) when the verification value does not match (wrong password), and with code `203` on other decryption errors.

```yukino
load std.fs.archives.pass: winzip_aes_decrypt
load std.io: println

set plain = winzip_aes_decrypt("Pass", 256, salt, res.data, res.check_value)
println(cast(plain, "string"))
```

---

[← Back: Module std.compress](15-compress.md) | [Next: Module std.locale →](17-locale.md)
