# Module std.compress

The `std.compress` module provides tools for in-memory and streaming file compression and decompression. Supported algorithms and formats include Gzip, Zstandard (zstd), XZ/LZMA, Deflate, Zlib, Bzip2, LZ4, as well as CRC-32 checksum calculation.

---

## 1. Import: load std.compress

Importing the entire package:

```yukino
load std.compress
```

When importing the module, the following entities become available:

- **Data and Error Structures:** `CompressError`.
- **Functions:** `compress`, `decompress`, `compress_file`, `decompress_file`, `gzip`, `gunzip`, `zstd_compress`, `zstd_decompress`, `xz_compress`, `xz_decompress`, `deflate_compress`, `deflate_decompress`, `zlib_compress`, `zlib_decompress`, `bzip2_compress`, `bzip2_decompress`, `lz4_compress`, `lz4_decompress`, `crc32`.

All listed entities also automatically become available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.compress` module declares no global constants.

---

## 3. Interfaces

The `std.compress` module declares no interfaces.

---

## 4. Data and Panic Error Structures

### 4.1 CompressError

Error structure raised by functions in this module when encountering invalid input, corrupted compressed streams, or codec failures.

```yukino
public struct CompressError {
    message: string
    code: int32
}
```

The `message` field contains a detailed error description, while `code` contains a numeric error code: `101` for invalid input data, `102` for an unsupported compression format, `103` for compression failures, `104` for decompression failures (including corrupted data), and `105` for file open or create failures.

---

## 5. Classes

The `std.compress` module declares no classes.

---

## 6. Function Reference

### 6.1 compress

Compresses a byte buffer using the specified format and an optional compression level. Accepted `format` values are `"gzip"` / `"gz"`, `"zstd"` / `"zst"`, `"xz"`, `"lzma"`, `"deflate"`, `"zlib"`, `"bz2"` / `"bzip2"`, `"lz4"`. Level `-1` selects the default compression level for the chosen format.

```yukino
public fun compress(data: bytes, format: string, level: int32 = -1) -> bytes
```

Returns a new byte buffer of type `bytes`. The return value is non-nullable.

In case of an unsupported format or compression failure, panics with a `CompressError` structure.

```yukino
load std.compress: compress
load std.io: println

set raw = cast("Data to compress", "bytes")
set compressed = compress(raw, "zstd", 3)
println("Compressed bytes: ${compressed.len()}")
```

### 6.2 decompress

Decompresses a compressed byte buffer using the specified format. Supports formats `"gzip"` / `"gz"`, `"zstd"` / `"zst"`, `"xz"` / `"lzma"`, `"deflate"`, `"zlib"`, `"bz2"` / `"bzip2"`, `"lz4"`.

```yukino
public fun decompress(data: bytes, format: string) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

In case of corrupted stream or unsupported format, panics with a `CompressError` structure.

```yukino
load std.compress: compress, decompress
load std.io: println

set original = cast("Hello world", "bytes")
set packed = compress(original.cp(), "gzip")
set unpacked = decompress(packed, "gzip")
println(cast(unpacked, "string"))
```

### 6.3 compress_file

Streams input from `src_path`, compresses it with the given format and level, and writes the output to `dst_path`.

```yukino
public fun compress_file(src_path: string, dst_path: string, format: string, level: int32 = -1) -> bool
```

Returns a boolean `true` on success. The return value is non-nullable.

In case of file open failures or compression errors, panics with a `CompressError` structure.

```yukino
load std.compress: compress_file
load std.io: println

compress_file("input.txt", "input.txt.gz", "gzip")
println("File compressed successfully")
```

### 6.4 decompress_file

Streams compressed data from `src_path`, decompresses it with the specified format, and writes the decompressed content to `dst_path`.

```yukino
public fun decompress_file(src_path: string, dst_path: string, format: string) -> bool
```

Returns a boolean `true` on success. The return value is non-nullable.

In case of corrupted input or file write errors, panics with a `CompressError` structure.

```yukino
load std.compress: decompress_file
load std.io: println

decompress_file("input.txt.gz", "restored.txt", "gzip")
println("File decompressed successfully")
```

### 6.5 gzip

Compresses a byte buffer into Gzip format. Accepts an optional compression level `level` between `0` and `9`; the default value `-1` selects the codec's default level.

```yukino
public fun gzip(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: gzip
load std.io: println

set data = cast("text for gzip", "bytes")
set gz = gzip(data)
println("Gzip size: ${gz.len()}")
```

### 6.6 gunzip

Decompresses a Gzip byte buffer.

```yukino
public fun gunzip(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if the stream is corrupted or checksum validation fails.

```yukino
load std.compress: gzip, gunzip
load std.io: println

set raw = cast("gunzip verification", "bytes")
set gz = gzip(raw)
set un = gunzip(gz)
println(cast(un, "string"))
```

### 6.7 zstd_compress

Compresses a byte buffer using Zstandard algorithm. Accepts an optional compression level `level` between `1` and `22`; the default value `-1` corresponds to the codec's default level (`3`).

```yukino
public fun zstd_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: zstd_compress
load std.io: println

set zst = zstd_compress(cast("Data for Zstandard", "bytes"))
println("Zstd size: ${zst.len()}")
```

### 6.8 zstd_decompress

Decompresses a Zstandard-compressed byte buffer.

```yukino
public fun zstd_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if the data stream is invalid.

```yukino
load std.compress: zstd_compress, zstd_decompress
load std.io: println

set packed = zstd_compress(cast("Sample string", "bytes"))
set restored = zstd_decompress(packed)
println(cast(restored, "string"))
```

### 6.9 xz_compress

Compresses a byte buffer using XZ/LZMA2 format. The `level` parameter is present in the signature but is not used by the XZ codec.

```yukino
public fun xz_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: xz_compress
load std.io: println

set xz_data = xz_compress(cast("High compression ratio LZMA2", "bytes"))
println("XZ size: ${xz_data.len()}")
```

### 6.10 xz_decompress

Decompresses an XZ or LZMA byte buffer.

```yukino
public fun xz_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if the stream is corrupted.

```yukino
load std.compress: xz_compress, xz_decompress
load std.io: println

set packed = xz_compress(cast("XZ unpacking", "bytes"))
set unpacked = xz_decompress(packed)
println(cast(unpacked, "string"))
```

### 6.11 deflate_compress

Compresses a byte buffer using raw Deflate compression without container headers. Accepts an optional compression level `level` between `0` and `9`; the default value `-1` selects the codec's default level.

```yukino
public fun deflate_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: deflate_compress
load std.io: println

set def = deflate_compress(cast("raw deflate stream", "bytes"))
println("Deflate size: ${def.len()}")
```

### 6.12 deflate_decompress

Decompresses a raw Deflate byte buffer.

```yukino
public fun deflate_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if decompression fails.

```yukino
load std.compress: deflate_compress, deflate_decompress
load std.io: println

set def = deflate_compress(cast("raw stream", "bytes"))
set un = deflate_decompress(def)
println(cast(un, "string"))
```

### 6.13 zlib_compress

Compresses a byte buffer into Zlib format (RFC 1950 header with Adler-32 checksum). Accepts an optional compression level `level` between `0` and `9`; the default value `-1` selects the codec's default level.

```yukino
public fun zlib_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: zlib_compress
load std.io: println

set zl = zlib_compress(cast("zlib data", "bytes"))
println("Zlib size: ${zl.len()}")
```

### 6.14 zlib_decompress

Decompresses a Zlib-formatted byte buffer.

```yukino
public fun zlib_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if the stream is corrupted or Adler-32 checksum mismatch occurs.

```yukino
load std.compress: zlib_compress, zlib_decompress
load std.io: println

set zl = zlib_compress(cast("zlib unpack", "bytes"))
set un = zlib_decompress(zl)
println(cast(un, "string"))
```

### 6.15 bzip2_compress

Compresses a byte buffer using Bzip2 algorithm. Accepts an optional compression level `level` from `1` to `9`; the default value `-1` selects the codec's default level.

```yukino
public fun bzip2_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: bzip2_compress
load std.io: println

set bz = bzip2_compress(cast("bzip2 data", "bytes"))
println("Bzip2 size: ${bz.len()}")
```

### 6.16 bzip2_decompress

Decompresses a Bzip2-compressed byte buffer.

```yukino
public fun bzip2_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if data blocks are corrupted.

```yukino
load std.compress: bzip2_compress, bzip2_decompress
load std.io: println

set bz = bzip2_compress(cast("bzip2 unpack", "bytes"))
set un = bzip2_decompress(bz)
println(cast(un, "string"))
```

### 6.17 lz4_compress

Compresses a byte buffer using ultra-fast LZ4 algorithm into standard LZ4 frame container. The `level` parameter is present in the signature but is not used by the LZ4 codec.

```yukino
public fun lz4_compress(data: bytes, level: int32 = -1) -> bytes
```

Returns a compressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if compression fails.

```yukino
load std.compress: lz4_compress
load std.io: println

set lz = lz4_compress(cast("fast lz4 compression", "bytes"))
println("LZ4 size: ${lz.len()}")
```

### 6.18 lz4_decompress

Decompresses an LZ4 frame container byte buffer.

```yukino
public fun lz4_decompress(data: bytes) -> bytes
```

Returns the decompressed byte buffer of type `bytes`. The return value is non-nullable.

Panics with a `CompressError` structure if the header is invalid or the frame is corrupted.

```yukino
load std.compress: lz4_compress, lz4_decompress
load std.io: println

set lz = lz4_compress(cast("lz4 unpack", "bytes"))
set un = lz4_decompress(lz)
println(cast(un, "string"))
```

### 6.19 crc32

Calculates the 32-bit cyclic redundancy check checksum CRC-32 (IEEE 802.3 polynomial, `0xEDB88320`) for the provided byte buffer.

```yukino
public fun crc32(data: bytes) -> uint32
```

Returns a 32-bit unsigned integer of type `uint32`. The return value is non-nullable.

This function never panics.

```yukino
load std.compress: crc32
load std.io: println

set sum = crc32(cast("123456789", "bytes"))
println("CRC32: ${sum}")
```

---

[← Back: Module std.simpleurl](14-simpleurl.md) | [Next: Module std.fs.archives →](16-archives.md)
