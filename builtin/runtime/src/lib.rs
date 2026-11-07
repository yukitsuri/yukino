pub mod panic;
pub mod personality;
pub mod region;
pub mod rt_args;
pub mod rt_cast;
pub mod rt_slice;
pub mod test_runner;
pub mod utf8;

pub use panic::*;
pub use personality::*;
pub use region::*;
pub use rt_args::*;
pub use rt_cast::*;
pub use rt_slice::*;
pub use test_runner::*;
pub use utf8::*;

#[unsafe(no_mangle)]
pub extern "C" fn yukino_rt_version() -> *const u8 {
    c"yukino-rt 0.1.0".as_ptr().cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rt_version() {
        let ptr = yukino_rt_version();
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_rt_cast_helpers() {
        let mut reg = std::mem::MaybeUninit::<YukinoRegion>::uninit();
        unsafe {
            yukino_region_init(reg.as_mut_ptr(), 4096);
            let reg_ptr = reg.as_mut_ptr();

            // 1. int to string & string to int
            let mut s_int = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_int64_to_string(&mut s_int, reg_ptr, 12345);
            assert_eq!(s_int.len, 5);
            let parsed_int = yukino_rt_string_to_int64(s_int.ptr, s_int.len);
            assert_eq!(parsed_int, 12345);

            // 2. float to string & string to float
            let mut s_flt = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_float64_to_string(&mut s_flt, reg_ptr, 2.75);
            assert!(s_flt.len >= 4);
            let parsed_flt = yukino_rt_string_to_float64(s_flt.ptr, s_flt.len);
            assert!((parsed_flt - 2.75).abs() < 1e-6);

            // 3. char to string & string to char
            let mut s_ch = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_char_to_string(&mut s_ch, reg_ptr, 'A' as u32);
            assert_eq!(s_ch.len, 1);
            let parsed_ch = yukino_rt_string_to_char(s_ch.ptr, s_ch.len);
            assert_eq!(parsed_ch, 'A' as u32);

            // 4. ptr to string & string to ptr
            let dummy_ptr = 0x1000usize as *const u8;
            let mut s_ptr = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_ptr_to_string(&mut s_ptr, reg_ptr, dummy_ptr);
            let parsed_ptr = yukino_rt_string_to_ptr(s_ptr.ptr, s_ptr.len);
            assert_eq!(parsed_ptr, dummy_ptr);

            // 5. from_raw_parts for string and slice
            let mut raw_str = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_string_from_raw_parts(&mut raw_str, dummy_ptr, 10);
            assert_eq!(raw_str.ptr, dummy_ptr);
            assert_eq!(raw_str.len, 10);

            let mut raw_slice = YukinoSlice { ptr: std::ptr::null_mut(), len: 0, cap: 0 };
            yukino_rt_slice_from_raw_parts(&mut raw_slice, dummy_ptr as *mut u8, 5, 20);
            assert_eq!(raw_slice.ptr, dummy_ptr as *mut u8);
            assert_eq!(raw_slice.len, 5);
            assert_eq!(raw_slice.cap, 20);

            // 6. string_to_chars and chars_to_string
            let greeting = "Hello, мир!";
            let mut chars_slice = YukinoSlice { ptr: std::ptr::null_mut(), len: 0, cap: 0 };
            yukino_rt_string_to_chars(&mut chars_slice, reg_ptr, greeting.as_ptr(), greeting.len());
            assert_eq!(chars_slice.len, 11);
            let mut back_str = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_chars_to_string(&mut back_str, reg_ptr, chars_slice.ptr.cast(), chars_slice.len);
            assert_eq!(back_str.len, greeting.len());
            let back_text = std::str::from_utf8(std::slice::from_raw_parts(back_str.ptr, back_str.len)).unwrap();
            assert_eq!(back_text, greeting);

            // 7. cstr_to_string and string_to_cstr
            let cstr_ptr = yukino_rt_string_to_cstr(reg_ptr, greeting.as_ptr(), greeting.len());
            assert_eq!(*cstr_ptr.add(greeting.len()), 0);
            let mut roundtrip = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_cstr_to_string(&mut roundtrip, reg_ptr, cstr_ptr);
            assert_eq!(roundtrip.len, greeting.len());

            yukino_region_destroy(reg_ptr);
        }
    }

    #[test]
    fn test_rt_args_helpers() {
        let mut reg = std::mem::MaybeUninit::<YukinoRegion>::uninit();
        unsafe {
            yukino_region_init(reg.as_mut_ptr(), 4096);
            let reg_ptr = reg.as_mut_ptr();

            let arg0 = c"prog".as_ptr().cast();
            let arg1 = c"hello".as_ptr().cast();
            let arg2 = c"world".as_ptr().cast();
            let argv = [arg0, arg1, arg2];

            // args()
            let mut slice = YukinoSlice { ptr: std::ptr::null_mut(), len: 0, cap: 0 };
            yukino_rt_get_args(&mut slice, reg_ptr, 3, argv.as_ptr());
            assert_eq!(slice.len, 3);
            let items = std::slice::from_raw_parts(slice.ptr.cast::<YukinoString>(), 3);
            assert_eq!(items[0].len, 4);
            assert_eq!(items[1].len, 5);
            assert_eq!(items[2].len, 5);

            // raw_args()
            let mut raw = YukinoString { ptr: std::ptr::null(), len: 0 };
            yukino_rt_get_raw_args(&mut raw, reg_ptr, 3, argv.as_ptr());
            assert_eq!(raw.len, 16);
            let text = std::str::from_utf8(std::slice::from_raw_parts(raw.ptr, raw.len)).unwrap();
            assert_eq!(text, "prog hello world");

            yukino_region_destroy(reg_ptr);
        }
    }
}
