use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit, KeyIvInit, StreamCipher};
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<(i32, String)> = const { RefCell::new((0, String::new())) };
    static PASS_BUF_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
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

fn set_pass_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    PASS_BUF_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 256 { ring.1.resize_with(256, Vec::new); }
        let idx = ring.0 % 256;
        ring.0 = ring.0.wrapping_add(1);
        let buf = &mut ring.1[idx];
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

unsafe fn slice_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a [u8]> {
    if len < 0 { return None; }
    if len == 0 { return Some(&[]); }
    if ptr.is_null() { return None; }
    Some(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

struct ZipCrypto {
    key0: u32,
    key1: u32,
    key2: u32,
}

impl ZipCrypto {
    fn new(password: &[u8]) -> Self {
        let mut zc = Self {
            key0: 0x12345678,
            key1: 0x23456789,
            key2: 0x34567890,
        };
        for &b in password {
            zc.update_keys(b);
        }
        zc
    }

    fn update_keys(&mut self, b: u8) {
        self.key0 = crc32_byte(self.key0, b);
        self.key1 = self
            .key1
            .wrapping_add(self.key0 & 0xff)
            .wrapping_mul(134775813)
            .wrapping_add(1);
        let b2 = (self.key1 >> 24) as u8;
        self.key2 = crc32_byte(self.key2, b2);
    }

    fn decrypt_byte(&mut self, c: u8) -> u8 {
        let temp = (self.key2 | 2) as u16;
        let k = ((temp.wrapping_mul(temp ^ 1)) >> 8) as u8;
        let p = c ^ k;
        self.update_keys(p);
        p
    }

    fn encrypt_byte(&mut self, p: u8) -> u8 {
        let temp = (self.key2 | 2) as u16;
        let k = ((temp.wrapping_mul(temp ^ 1)) >> 8) as u8;
        self.update_keys(p);
        p ^ k
    }
}

fn crc32_byte(crc: u32, b: u8) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256 {
        let mut c = i as u32;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xedb88320 ^ (c >> 1);
            } else { c >>= 1; }
        }
        table[i] = c;
    }
    table[((crc ^ (b as u32)) & 0xff) as usize] ^ (crc >> 8)
}

fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut out = Vec::with_capacity(data.len() + pad_len);
    out.extend_from_slice(data);
    out.resize(out.len() + pad_len, pad_len as u8);
    out
}

fn pkcs7_unpad(data: &[u8]) -> Result<&[u8], &'static str> {
    if data.is_empty() { return Err("empty data"); }
    let pad_len = data[data.len() - 1] as usize;
    if pad_len == 0 || pad_len > data.len() || pad_len > 16 {
        return Err("invalid padding length");
    }
    for &b in &data[data.len() - pad_len..] {
        if b as usize != pad_len {
            return Err("invalid padding byte");
        }
    }
    Ok(&data[..data.len() - pad_len])
}

fn aes_cbc_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let padded = pkcs7_pad(data, 16);
    let mut out = Vec::with_capacity(padded.len());
    let mut prev_block = [0u8; 16];
    prev_block.copy_from_slice(iv);

    match key.len() {
        16 => {
            let cipher = aes::Aes128::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in padded.chunks_exact(16) {
                let mut block = [0u8; 16];
                for i in 0..16 {
                    block[i] = chunk[i] ^ prev_block[i];
                }
                let mut b = aes::cipher::generic_array::GenericArray::from(block);
                cipher.encrypt_block(&mut b);
                prev_block.copy_from_slice(b.as_slice());
                out.extend_from_slice(b.as_slice());
            }
        }
        24 => {
            let cipher = aes::Aes192::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in padded.chunks_exact(16) {
                let mut block = [0u8; 16];
                for i in 0..16 {
                    block[i] = chunk[i] ^ prev_block[i];
                }
                let mut b = aes::cipher::generic_array::GenericArray::from(block);
                cipher.encrypt_block(&mut b);
                prev_block.copy_from_slice(b.as_slice());
                out.extend_from_slice(b.as_slice());
            }
        }
        32 => {
            let cipher = aes::Aes256::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in padded.chunks_exact(16) {
                let mut block = [0u8; 16];
                for i in 0..16 {
                    block[i] = chunk[i] ^ prev_block[i];
                }
                let mut b = aes::cipher::generic_array::GenericArray::from(block);
                cipher.encrypt_block(&mut b);
                prev_block.copy_from_slice(b.as_slice());
                out.extend_from_slice(b.as_slice());
            }
        }
        _ => return Err(format!("AES key length must be 16, 24, or 32 bytes")),
    }
    Ok(out)
}

fn aes_cbc_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() || data.len() % 16 != 0 {
        return Err("data length must be a non-empty multiple of 16".to_string());
    }
    let mut out = Vec::with_capacity(data.len());
    let mut prev_block = [0u8; 16];
    prev_block.copy_from_slice(iv);

    match key.len() {
        16 => {
            let cipher = aes::Aes128::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in data.chunks_exact(16) {
                let mut b = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                cipher.decrypt_block(&mut b);
                let mut plain = [0u8; 16];
                for i in 0..16 { plain[i] = b[i] ^ prev_block[i]; }
                prev_block.copy_from_slice(chunk);
                out.extend_from_slice(&plain);
            }
        }
        24 => {
            let cipher = aes::Aes192::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in data.chunks_exact(16) {
                let mut b = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                cipher.decrypt_block(&mut b);
                let mut plain = [0u8; 16];
                for i in 0..16 { plain[i] = b[i] ^ prev_block[i]; }
                prev_block.copy_from_slice(chunk);
                out.extend_from_slice(&plain);
            }
        }
        32 => {
            let cipher = aes::Aes256::new_from_slice(key).map_err(|e| e.to_string())?;
            for chunk in data.chunks_exact(16) {
                let mut b = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
                cipher.decrypt_block(&mut b);
                let mut plain = [0u8; 16];
                for i in 0..16 { plain[i] = b[i] ^ prev_block[i]; }
                prev_block.copy_from_slice(chunk);
                out.extend_from_slice(&plain);
            }
        }
        _ => return Err(format!("AES key length must be 16, 24, or 32 bytes")),
    }
    let unpadded = pkcs7_unpad(&out).map_err(|e| e.to_string())?;
    Ok(unpadded.to_vec())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_last_error_message(out_len: *mut i64) -> *const u8 {
    LAST_ERROR.with(|cell| {
        let err = cell.borrow();
        let bytes = err.1.as_bytes();
        set_pass_result(bytes, out_len)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_zipcrypto_decrypt(
    pass_p: *const u8,
    pass_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let pass = match unsafe { slice_from_raw(pass_p, pass_len) } {
        Some(p) => p,
        None => {
            set_last_error(201, "invalid password pointer or length".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data pointer or length".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            return std::ptr::null();
        }
    };

    let mut zc = ZipCrypto::new(pass);
    let mut out = Vec::with_capacity(data.len());
    for &b in data {
        out.push(zc.decrypt_byte(b));
    }
    set_pass_result(&out, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_zipcrypto_encrypt(
    pass_p: *const u8,
    pass_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let pass = match unsafe { slice_from_raw(pass_p, pass_len) } {
        Some(p) => p,
        None => {
            set_last_error(201, "invalid password pointer or length".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data pointer or length".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            return std::ptr::null();
        }
    };

    let mut zc = ZipCrypto::new(pass);
    let mut out = Vec::with_capacity(data.len());
    for &b in data { out.push(zc.encrypt_byte(b)); }
    set_pass_result(&out, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_aes_ctr(
    key_p: *const u8,
    key_len: i64,
    iv_p: *const u8,
    iv_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let key = match unsafe { slice_from_raw(key_p, key_len) } {
        Some(k) => k,
        None => {
            set_last_error(201, "invalid key".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            return std::ptr::null();
        }
    };
    let iv = match unsafe { slice_from_raw(iv_p, iv_len) } {
        Some(v) => v,
        None => {
            set_last_error(201, "invalid iv".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    if iv.len() != 16 {
        set_last_error(202, "IV must be 16 bytes for AES-CTR".to_string());
        if !out_len.is_null() {
            unsafe { *out_len = -1 };
        }
        return std::ptr::null();
    }

    let mut out = data.to_vec();
    let res = match key.len() {
        16 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes128>::new_from_slices(key, iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        24 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes192>::new_from_slices(key, iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        32 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes256>::new_from_slices(key, iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        _ => Err(format!(
            "AES key length must be 16, 24, or 32 bytes (got {})",
            key.len()
        )),
    };

    match res {
        Ok(()) => set_pass_result(&out, out_len),
        Err(e) => {
            set_last_error(203, format!("AES-CTR failed: {e}"));
            if !out_len.is_null() { unsafe { *out_len = -1 }; }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_aes_cbc_encrypt(
    key_p: *const u8,
    key_len: i64,
    iv_p: *const u8,
    iv_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let key = match unsafe { slice_from_raw(key_p, key_len) } {
        Some(k) => k,
        None => {
            set_last_error(201, "invalid key".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let iv = match unsafe { slice_from_raw(iv_p, iv_len) } {
        Some(v) => v,
        None => {
            set_last_error(201, "invalid iv".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    if iv.len() != 16 {
        set_last_error(202, "IV must be 16 bytes for AES-CBC".to_string());
        if !out_len.is_null() {
            unsafe { *out_len = -1 };
        }
        return std::ptr::null();
    }

    match aes_cbc_encrypt(key, iv, data) {
        Ok(out) => set_pass_result(&out, out_len),
        Err(e) => {
            set_last_error(203, format!("AES-CBC encrypt failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_aes_cbc_decrypt(
    key_p: *const u8,
    key_len: i64,
    iv_p: *const u8,
    iv_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let key = match unsafe { slice_from_raw(key_p, key_len) } {
        Some(k) => k,
        None => {
            set_last_error(201, "invalid key".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let iv = match unsafe { slice_from_raw(iv_p, iv_len) } {
        Some(v) => v,
        None => {
            set_last_error(201, "invalid iv".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    if iv.len() != 16 {
        set_last_error(202, "IV must be 16 bytes for AES-CBC".to_string());
        if !out_len.is_null() {
            unsafe { *out_len = -1 };
        }
        return std::ptr::null();
    }

    match aes_cbc_decrypt(key, iv, data) {
        Ok(out) => set_pass_result(&out, out_len),
        Err(e) => {
            set_last_error(203, format!("AES-CBC decrypt failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_winzip_aes_decrypt(
    pass_p: *const u8,
    pass_len: i64,
    aes_bits: i32,
    salt_p: *const u8,
    salt_len: i64,
    data_p: *const u8,
    data_len: i64,
    check_val: u16,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let pass = match unsafe { slice_from_raw(pass_p, pass_len) } {
        Some(p) => p,
        None => {
            set_last_error(201, "invalid password".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let salt = match unsafe { slice_from_raw(salt_p, salt_len) } {
        Some(s) => s,
        None => {
            set_last_error(201, "invalid salt".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    let key_size = match aes_bits {
        128 => 16,
        192 => 24,
        256 => 32,
        _ => {
            set_last_error(202, format!("unsupported AES bit size: {aes_bits}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    let total_key_material = key_size * 2 + 2;
    let mut key_buf = vec![0u8; total_key_material];
    pbkdf2_hmac::<Sha1>(pass, salt, 1000, &mut key_buf);

    let derived_check = u16::from_le_bytes([key_buf[key_size * 2], key_buf[key_size * 2 + 1]]);
    if check_val != 0 && derived_check != check_val {
        set_last_error(204, "password verification failed".to_string());
        if !out_len.is_null() {
            unsafe { *out_len = -1 };
        }
        return std::ptr::null();
    }

    let enc_key = &key_buf[0..key_size];
    let iv = [1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut out = data.to_vec();

    let res = match key_size {
        16 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes128>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        24 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes192>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        32 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes256>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        _ => unreachable!(),
    };

    match res {
        Ok(()) => set_pass_result(&out, out_len),
        Err(e) => {
            set_last_error(203, format!("WinZip AES decryption failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_winzip_aes_encrypt(
    pass_p: *const u8,
    pass_len: i64,
    aes_bits: i32,
    salt_p: *const u8,
    salt_len: i64,
    data_p: *const u8,
    data_len: i64,
    out_check: *mut u16,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let pass = match unsafe { slice_from_raw(pass_p, pass_len) } {
        Some(p) => p,
        None => {
            set_last_error(201, "invalid password".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let salt = match unsafe { slice_from_raw(salt_p, salt_len) } {
        Some(s) => s,
        None => {
            set_last_error(201, "invalid salt".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };
    let data = match unsafe { slice_from_raw(data_p, data_len) } {
        Some(d) => d,
        None => {
            set_last_error(201, "invalid data".to_string());
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    let key_size = match aes_bits {
        128 => 16,
        192 => 24,
        256 => 32,
        _ => {
            set_last_error(202, format!("unsupported AES bit size: {aes_bits}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            return std::ptr::null();
        }
    };

    let total_key_material = key_size * 2 + 2;
    let mut key_buf = vec![0u8; total_key_material];
    pbkdf2_hmac::<Sha1>(pass, salt, 1000, &mut key_buf);

    let check = u16::from_le_bytes([key_buf[key_size * 2], key_buf[key_size * 2 + 1]]);
    if !out_check.is_null() {
        unsafe {
            *out_check = check;
        }
    }

    let enc_key = &key_buf[0..key_size];
    let iv = [1u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut out = data.to_vec();

    let res = match key_size {
        16 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes128>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        24 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes192>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        32 => {
            let mut cipher = ctr::Ctr128LE::<aes::Aes256>::new_from_slices(enc_key, &iv)
                .map_err(|e| e.to_string());
            match cipher {
                Ok(ref mut c) => {
                    c.apply_keystream(&mut out);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        _ => unreachable!(),
    };

    match res {
        Ok(()) => set_pass_result(&out, out_len),
        Err(e) => {
            set_last_error(203, format!("WinZip AES encryption failed: {e}"));
            if !out_len.is_null() {
                unsafe { *out_len = -1 };
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_pass_pbkdf2_sha1(
    pass_p: *const u8,
    pass_len: i64,
    salt_p: *const u8,
    salt_len: i64,
    iterations: u32,
    out_key_p: *mut u8,
    out_key_len: i64,
) -> bool {
    clear_last_error();
    let pass = match unsafe { slice_from_raw(pass_p, pass_len) } {
        Some(p) => p,
        None => {
            set_last_error(201, "invalid password".to_string());
            return false;
        }
    };
    let salt = match unsafe { slice_from_raw(salt_p, salt_len) } {
        Some(s) => s,
        None => {
            set_last_error(201, "invalid salt".to_string());
            return false;
        }
    };
    if out_key_p.is_null() || out_key_len <= 0 {
        set_last_error(201, "invalid output buffer".to_string());
        return false;
    }
    let out_slice = unsafe { std::slice::from_raw_parts_mut(out_key_p, out_key_len as usize) };
    let iters = if iterations == 0 { 1000 } else { iterations };
    pbkdf2_hmac::<Sha1>(pass, salt, iters, out_slice);
    true
}
