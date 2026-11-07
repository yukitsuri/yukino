use serde::{Deserialize, Serialize};
use std::io;
use std::ptr;
use std::slice;
use yukic_lexer::tokenize;
use yukic_span::FileId;
use yukino_macroproto::{MacroDiagnostic, MacroRequest, MacroResponse, read_request, write_response};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireToken {
    pub kind: String,
    pub text: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireTokenList {
    pub tokens: Vec<WireToken>,
}

fn allocate_byte_buffer(bytes: Vec<u8>, out_len: *mut i64) -> *mut u8 {
    let len = bytes.len() as i64;
    unsafe {
        *out_len = len;
    }
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukimacrosdk_read_request_json(out_len: *mut i64) -> *mut u8 {
    if out_len.is_null() {
        return ptr::null_mut();
    }
    let mut stdin = io::stdin().lock();
    let req: MacroRequest = match read_request(&mut stdin) {
        Ok(r) => r,
        Err(_) => {
            unsafe { *out_len = 0 };
            return ptr::null_mut();
        }
    };
    let json_bytes = match serde_json::to_vec(&req) {
        Ok(b) => b,
        Err(_) => {
            unsafe { *out_len = 0 };
            return ptr::null_mut();
        }
    };
    allocate_byte_buffer(json_bytes, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukimacrosdk_write_response_json(
    success: bool,
    output_tokens_ptr: *const u8,
    output_tokens_len: i64,
    tracked_files_json_ptr: *const u8,
    tracked_files_json_len: i64,
    diagnostics_json_ptr: *const u8,
    diagnostics_json_len: i64,
    error_message_ptr: *const u8,
    error_message_len: i64,
) -> i32 {
    let output_tokens = if output_tokens_ptr.is_null() || output_tokens_len <= 0 {
        None
    } else {
        let s = unsafe { slice::from_raw_parts(output_tokens_ptr, output_tokens_len as usize) };
        Some(String::from_utf8_lossy(s).to_string())
    };

    let error_message = if error_message_ptr.is_null() || error_message_len <= 0 {
        None
    } else {
        let s = unsafe { slice::from_raw_parts(error_message_ptr, error_message_len as usize) };
        Some(String::from_utf8_lossy(s).to_string())
    };

    let tracked_files: Vec<String> = if tracked_files_json_ptr.is_null() || tracked_files_json_len <= 0 {
        Vec::new()
    } else {
        let s = unsafe { slice::from_raw_parts(tracked_files_json_ptr, tracked_files_json_len as usize) };
        serde_json::from_slice(s).unwrap_or_default()
    };

    let diagnostics: Vec<MacroDiagnostic> = if diagnostics_json_ptr.is_null() || diagnostics_json_len <= 0 {
        Vec::new()
    } else {
        let s = unsafe { slice::from_raw_parts(diagnostics_json_ptr, diagnostics_json_len as usize) };
        serde_json::from_slice(s).unwrap_or_default()
    };

    let response = MacroResponse {
        success,
        output_tokens,
        tracked_files,
        diagnostics,
        error_message,
    };

    let mut stdout = io::stdout().lock();
    if write_response(&mut stdout, &response).is_ok() {
        0
    } else {
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukimacrosdk_free_buffer(ptr: *mut u8, len: i64) {
    if !ptr.is_null() && len > 0 {
        let _ = unsafe { Box::from_raw(slice::from_raw_parts_mut(ptr, len as usize)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukimacrosdk_tokenize_json(
    src_ptr: *const u8,
    src_len: i64,
    out_len: *mut i64,
) -> *mut u8 {
    if src_ptr.is_null() || out_len.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0 };
        }
        return ptr::null_mut();
    }
    let src_slice = unsafe { slice::from_raw_parts(src_ptr, src_len as usize) };
    let source = String::from_utf8_lossy(src_slice);

    let (tokens, _) = tokenize(FileId(0), &source);
    let mut wire_tokens = Vec::with_capacity(tokens.len());
    for t in tokens {
        if t.is_eof() {
            continue;
        }
        let start = t.span.start.0 as usize;
        let end = (t.span.end.0 as usize).min(source.len());
        let text = if start <= end && end <= source.len() {
            source[start..end].to_string()
        } else {
            String::new()
        };
        wire_tokens.push(WireToken {
            kind: format!("{:?}", t.kind),
            text,
            start: t.span.start.0,
            end: t.span.end.0,
        });
    }

    let wire_list = WireTokenList { tokens: wire_tokens };
    let json_bytes = serde_json::to_vec(&wire_list).unwrap_or_default();
    allocate_byte_buffer(json_bytes, out_len)
}
