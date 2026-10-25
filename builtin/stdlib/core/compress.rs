use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

thread_local! {
    static LAST_ERROR: RefCell<(i32, String)> = const { RefCell::new((0, String::new())) };
    static COMPRESS_BUF_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

fn set_last_error(code: i32, msg: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (code, msg);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (0, String::new());
    });
}

fn set_compress_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    COMPRESS_BUF_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 256 {
            ring.1.resize_with(256, Vec::new);
        }
        let idx = ring.0 % 256;
        ring.0 = ring.0.wrapping_add(1);
        let buf = &mut ring.1[idx];
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe {
                *out_len = buf.len() as i64;
            }
        }
        buf.as_ptr()
    })
}

unsafe fn slice_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a [u8]> {
    if len < 0 {
        return None;
    }
    if len == 0 {
        return Some(&[]);
    }
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a str> {
    let slice = unsafe { slice_from_raw(ptr, len)? };
    std::str::from_utf8(slice).ok()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompressFormat {
    Gzip,
    Deflate,
    Zlib,
    Zstd,
    Xz,
    Lzma,
    Bzip2,
    Lz4,
}

fn parse_format(fmt: &str) -> Option<CompressFormat> {
    match fmt.to_ascii_lowercase().as_str() {
        "gzip" | "gz" => Some(CompressFormat::Gzip),
        "deflate" => Some(CompressFormat::Deflate),
        "zlib" => Some(CompressFormat::Zlib),
        "zstd" | "zst" => Some(CompressFormat::Zstd),
        "xz" => Some(CompressFormat::Xz),
        "lzma" => Some(CompressFormat::Lzma),
        "bzip2" | "bz2" => Some(CompressFormat::Bzip2),
        "lz4" => Some(CompressFormat::Lz4),
        _ => None,
    }
}

fn compress_data(data: &[u8], format: CompressFormat, level: i32) -> Result<Vec<u8>, String> {
    match format {
        CompressFormat::Gzip => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::GzEncoder::new(Vec::new(), compression);
            encoder.write_all(data).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        }
        CompressFormat::Deflate => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), compression);
            encoder.write_all(data).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        }
        CompressFormat::Zlib => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), compression);
            encoder.write_all(data).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        }
        CompressFormat::Zstd => {
            let lvl = if level < 0 { 3 } else { level.clamp(1, 22) };
            zstd::encode_all(data, lvl).map_err(|e| e.to_string())
        }
        CompressFormat::Xz => {
            let mut out = Vec::new();
            lzma_rs::xz_compress(&mut &data[..], &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Lzma => {
            let mut out = Vec::new();
            lzma_rs::lzma_compress(&mut &data[..], &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Bzip2 => {
            let compression = if level < 0 {
                bzip2::Compression::default()
            } else {
                bzip2::Compression::new(level.clamp(1, 9) as u32)
            };
            let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), compression);
            encoder.write_all(data).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        }
        CompressFormat::Lz4 => {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
            encoder.write_all(data).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())
        }
    }
}

fn decompress_data(data: &[u8], format: CompressFormat) -> Result<Vec<u8>, String> {
    match format {
        CompressFormat::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Deflate => {
            let mut decoder = flate2::read::DeflateDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Zlib => {
            let mut decoder = flate2::read::ZlibDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Zstd => zstd::decode_all(data).map_err(|e| e.to_string()),
        CompressFormat::Xz => {
            let mut out = Vec::new();
            lzma_rs::xz_decompress(&mut &data[..], &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Lzma => {
            let mut out = Vec::new();
            lzma_rs::lzma_decompress(&mut &data[..], &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Bzip2 => {
            let mut decoder = bzip2::read::BzDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        CompressFormat::Lz4 => {
            let mut decoder = lz4_flex::frame::FrameDecoder::new(data);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_compress_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_compress_last_error_message(out_len: *mut i64) -> *const u8 {
    LAST_ERROR.with(|cell| {
        let err = cell.borrow();
        let bytes = err.1.as_bytes();
        set_compress_result(bytes, out_len)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_crc32(data_p: *const u8, data_len: i64) -> u32 {
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => return 0,
    };
    crc32fast::hash(data)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_compress(
    data_p: *const u8,
    data_len: i64,
    fmt_p: *const u8,
    fmt_len: i64,
    level: i32,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(101, "invalid data pointer or length".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let fmt_str = match unsafe { str_from_raw(fmt_p, fmt_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid format string".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let format = match parse_format(fmt_str) {
        Some(f) => f,
        None => {
            set_last_error(102, format!("unsupported compression format: {fmt_str}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    match compress_data(data, format, level) {
        Ok(res) => set_compress_result(&res, out_len),
        Err(e) => {
            set_last_error(103, format!("compression failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_decompress(
    data_p: *const u8,
    data_len: i64,
    fmt_p: *const u8,
    fmt_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(101, "invalid data pointer or length".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let fmt_str = match unsafe { str_from_raw(fmt_p, fmt_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid format string".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let format = match parse_format(fmt_str) {
        Some(f) => f,
        None => {
            set_last_error(102, format!("unsupported compression format: {fmt_str}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    match decompress_data(data, format) {
        Ok(res) => set_compress_result(&res, out_len),
        Err(e) => {
            set_last_error(104, format!("decompression failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_compress_file(
    src_p: *const u8,
    src_len: i64,
    dst_p: *const u8,
    dst_len: i64,
    fmt_p: *const u8,
    fmt_len: i64,
    level: i32,
) -> bool {
    clear_last_error();
    let src_str = match unsafe { str_from_raw(src_p, src_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid source path".to_string());
            return false;
        }
    };
    let dst_str = match unsafe { str_from_raw(dst_p, dst_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid destination path".to_string());
            return false;
        }
    };
    let fmt_str = match unsafe { str_from_raw(fmt_p, fmt_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid format string".to_string());
            return false;
        }
    };
    let format = match parse_format(fmt_str) {
        Some(f) => f,
        None => {
            set_last_error(102, format!("unsupported format: {fmt_str}"));
            return false;
        }
    };

    let mut in_file = match File::open(Path::new(src_str)) {
        Ok(f) => f,
        Err(e) => {
            set_last_error(105, format!("failed to open source file: {e}"));
            return false;
        }
    };

    let mut out_file = match File::create(Path::new(dst_str)) {
        Ok(f) => f,
        Err(e) => {
            set_last_error(105, format!("failed to create destination file: {e}"));
            return false;
        }
    };

    let res: Result<(), String> = (|| match format {
        CompressFormat::Gzip => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::GzEncoder::new(out_file, compression);
            std::io::copy(&mut in_file, &mut encoder).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Deflate => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::DeflateEncoder::new(out_file, compression);
            std::io::copy(&mut in_file, &mut encoder).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Zlib => {
            let compression = if level < 0 {
                flate2::Compression::default()
            } else {
                flate2::Compression::new(level.clamp(0, 9) as u32)
            };
            let mut encoder = flate2::write::ZlibEncoder::new(out_file, compression);
            std::io::copy(&mut in_file, &mut encoder).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Zstd => {
            let lvl = if level < 0 { 3 } else { level.clamp(1, 22) };
            zstd::stream::copy_encode(&mut in_file, &mut out_file, lvl).map_err(|e| e.to_string())
        }
        CompressFormat::Xz => {
            let mut reader = std::io::BufReader::new(in_file);
            lzma_rs::xz_compress(&mut reader, &mut out_file).map_err(|e| e.to_string())
        }
        CompressFormat::Lzma => {
            let mut reader = std::io::BufReader::new(in_file);
            lzma_rs::lzma_compress(&mut reader, &mut out_file).map_err(|e| e.to_string())
        }
        CompressFormat::Bzip2 => {
            let compression = if level < 0 {
                bzip2::Compression::default()
            } else {
                bzip2::Compression::new(level.clamp(1, 9) as u32)
            };
            let mut encoder = bzip2::write::BzEncoder::new(out_file, compression);
            std::io::copy(&mut in_file, &mut encoder).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Lz4 => {
            let mut encoder = lz4_flex::frame::FrameEncoder::new(out_file);
            std::io::copy(&mut in_file, &mut encoder).map_err(|e| e.to_string())?;
            encoder.finish().map_err(|e| e.to_string())?;
            Ok(())
        }
    })();

    match res {
        Ok(()) => true,
        Err(e) => {
            set_last_error(103, format!("file compression failed: {e}"));
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_decompress_file(
    src_p: *const u8,
    src_len: i64,
    dst_p: *const u8,
    dst_len: i64,
    fmt_p: *const u8,
    fmt_len: i64,
) -> bool {
    clear_last_error();
    let src_str = match unsafe { str_from_raw(src_p, src_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid source path".to_string());
            return false;
        }
    };
    let dst_str = match unsafe { str_from_raw(dst_p, dst_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid destination path".to_string());
            return false;
        }
    };
    let fmt_str = match unsafe { str_from_raw(fmt_p, fmt_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "invalid format string".to_string());
            return false;
        }
    };
    let format = match parse_format(fmt_str) {
        Some(f) => f,
        None => {
            set_last_error(102, format!("unsupported format: {fmt_str}"));
            return false;
        }
    };

    let mut in_file = match File::open(Path::new(src_str)) {
        Ok(f) => f,
        Err(e) => {
            set_last_error(105, format!("failed to open source file: {e}"));
            return false;
        }
    };

    let mut out_file = match File::create(Path::new(dst_str)) {
        Ok(f) => f,
        Err(e) => {
            set_last_error(105, format!("failed to create destination file: {e}"));
            return false;
        }
    };

    let res: Result<(), String> = (|| match format {
        CompressFormat::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(in_file);
            std::io::copy(&mut decoder, &mut out_file).map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Deflate => {
            let mut decoder = flate2::read::DeflateDecoder::new(in_file);
            std::io::copy(&mut decoder, &mut out_file).map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Zlib => {
            let mut decoder = flate2::read::ZlibDecoder::new(in_file);
            std::io::copy(&mut decoder, &mut out_file).map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Zstd => {
            zstd::stream::copy_decode(&mut in_file, &mut out_file).map_err(|e| e.to_string())
        }
        CompressFormat::Xz => {
            let mut reader = std::io::BufReader::new(in_file);
            lzma_rs::xz_decompress(&mut reader, &mut out_file).map_err(|e| e.to_string())
        }
        CompressFormat::Lzma => {
            let mut reader = std::io::BufReader::new(in_file);
            lzma_rs::lzma_decompress(&mut reader, &mut out_file).map_err(|e| e.to_string())
        }
        CompressFormat::Bzip2 => {
            let mut decoder = bzip2::read::BzDecoder::new(in_file);
            std::io::copy(&mut decoder, &mut out_file).map_err(|e| e.to_string())?;
            Ok(())
        }
        CompressFormat::Lz4 => {
            let mut decoder = lz4_flex::frame::FrameDecoder::new(in_file);
            std::io::copy(&mut decoder, &mut out_file).map_err(|e| e.to_string())?;
            Ok(())
        }
    })();

    match res {
        Ok(()) => true,
        Err(e) => {
            set_last_error(104, format!("file decompression failed: {e}"));
            false
        }
    }
}
