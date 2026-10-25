use crate::panic::UnwindException;

pub const _URC_NO_REASON: i32 = 0;
pub const _URC_FOREIGN_EXCEPTION_CAUGHT: i32 = 1;
pub const _URC_FATAL_PHASE2_ERROR: i32 = 2;
pub const _URC_FATAL_PHASE1_ERROR: i32 = 3;
pub const _URC_NORMAL_STOP: i32 = 4;
pub const _URC_END_OF_STACK: i32 = 5;
pub const _URC_HANDLER_FOUND: i32 = 6;
pub const _URC_INSTALL_CONTEXT: i32 = 7;
pub const _URC_CONTINUE_UNWIND: i32 = 8;
pub const _URC_FAILURE: i32 = 9;

pub const _UA_SEARCH_PHASE: i32 = 1;
pub const _UA_CLEANUP_PHASE: i32 = 2;
pub const _UA_HANDLER_FRAME: i32 = 4;
pub const _UA_FORCE_UNWIND: i32 = 8;

pub const DW_EH_PE_ABS: u8 = 0x00;
pub const DW_EH_PE_OMIT: u8 = 0xFF;
pub const DW_EH_PE_ULEB128: u8 = 0x01;
pub const DW_EH_PE_UDATA2: u8 = 0x02;
pub const DW_EH_PE_UDATA4: u8 = 0x03;
pub const DW_EH_PE_UDATA8: u8 = 0x04;
pub const DW_EH_PE_SLEB128: u8 = 0x09;
pub const DW_EH_PE_SDATA2: u8 = 0x0A;
pub const DW_EH_PE_SDATA4: u8 = 0x0B;
pub const DW_EH_PE_SDATA8: u8 = 0x0C;
pub const DW_EH_PE_PCREL: u8 = 0x10;
pub const DW_EH_PE_INDIRECT: u8 = 0x80;

#[cfg(target_arch = "x86_64")]
pub const DWARF_REG_DATA0: i32 = 0;
#[cfg(target_arch = "x86_64")]
pub const DWARF_REG_DATA1: i32 = 1;

#[cfg(target_arch = "aarch64")]
pub const DWARF_REG_DATA0: i32 = 0;
#[cfg(target_arch = "aarch64")]
pub const DWARF_REG_DATA1: i32 = 1;

#[cfg(target_arch = "arm")]
pub const DWARF_REG_DATA0: i32 = 0;
#[cfg(target_arch = "arm")]
pub const DWARF_REG_DATA1: i32 = 1;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
pub const DWARF_REG_DATA0: i32 = 0;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
pub const DWARF_REG_DATA1: i32 = 1;

pub enum UnwindContext {}

#[cfg(not(all(windows, target_env = "msvc")))]
unsafe extern "C" {
    fn _Unwind_GetIP(context: *mut UnwindContext) -> usize;
    fn _Unwind_SetIP(context: *mut UnwindContext, new_value: usize) -> i32;
    fn _Unwind_GetLanguageSpecificData(context: *mut UnwindContext) -> *const u8;
    fn _Unwind_GetRegionStart(context: *mut UnwindContext) -> usize;
    fn _Unwind_SetGR(context: *mut UnwindContext, index: i32, new_value: usize) -> i32;
}

// the ptr must point to valid readable bytes containing a eleb128 seq
pub unsafe fn read_uleb128(ptr: &mut *const u8) -> u64 {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let byte = unsafe { **ptr };
        *ptr = unsafe { ptr.add(1) };
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 { break; }
        shift += 7;
    }
    result
}

// the ptr must point to valid readable bytes containing a eleb128 seq
pub unsafe fn read_sleb128(ptr: &mut *const u8) -> i64 {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    let mut byte: u8;
    loop {
        byte = unsafe { **ptr };
        *ptr = unsafe { ptr.add(1) };
        result |= ((byte & 0x7F) as i64) << shift;
        shift += 7;
        if (byte & 0x80) == 0 { break; }
    }
    if shift < 64 && (byte & 0x40) != 0 { result |= -1i64 << shift; }
    result
}

unsafe fn read_encoded_value(ptr: &mut *const u8, format: u8) -> usize {
    match format {
        DW_EH_PE_ABS => {
            let val = unsafe { (*ptr as *const usize).read_unaligned() };
            *ptr = unsafe { ptr.add(std::mem::size_of::<usize>()) };
            val
        }
        DW_EH_PE_ULEB128 => unsafe { read_uleb128(ptr) as usize },
        DW_EH_PE_UDATA2 => {
            let val = unsafe { (*ptr as *const u16).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(2) };
            val
        }
        DW_EH_PE_UDATA4 => {
            let val = unsafe { (*ptr as *const u32).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(4) };
            val
        }
        DW_EH_PE_UDATA8 => {
            let val = unsafe { (*ptr as *const u64).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(8) };
            val
        }
        DW_EH_PE_SLEB128 => unsafe { read_sleb128(ptr) as usize },
        DW_EH_PE_SDATA2 => {
            let val = unsafe { (*ptr as *const i16).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(2) };
            val
        }
        DW_EH_PE_SDATA4 => {
            let val = unsafe { (*ptr as *const i32).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(4) };
            val
        }
        DW_EH_PE_SDATA8 => {
            let val = unsafe { (*ptr as *const i64).read_unaligned() as usize };
            *ptr = unsafe { ptr.add(8) };
            val
        }
        _ => 0,
    }
}

// the ptr must point to valid readable bytes containing a dwarf enc
pub unsafe fn read_encoded_ptr(ptr: &mut *const u8, encoding: u8, base: usize) -> usize {
    if encoding == DW_EH_PE_OMIT { return 0; }
    let p_addr = *ptr as usize;
    let raw_val = unsafe { read_encoded_value(ptr, encoding & 0x0F) };
    let mut result = match encoding & 0x70 {
        DW_EH_PE_PCREL => p_addr.wrapping_add(raw_val),
        _ => base.wrapping_add(raw_val),
    };
    if (encoding & DW_EH_PE_INDIRECT) != 0 && result != 0 {
        result = unsafe { (result as *const usize).read_unaligned() };
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallSite { pub landing_pad: usize, pub action_offset: u64, }

#[derive(Clone, Debug)]
pub struct ParsedLsda {
    pub lp_start: usize,
    pub type_table: *const u8,
    pub type_encoding: u8,
    pub call_site_encoding: u8,
    pub call_site_table: *const u8,
    pub call_site_table_end: *const u8,
    pub action_table_start: *const u8,
}

// lsda must point to a valid DWARF LSDA buffer or be null
pub unsafe fn parse_lsda_header(lsda: *const u8, region_start: usize) -> Option<ParsedLsda> {
    if lsda.is_null() { return None; }
    let mut p = lsda;
    let lp_start_encoding = unsafe { *p };
    p = unsafe { p.add(1) };
    let p_addr = p as usize;
    let lp_start = if lp_start_encoding != DW_EH_PE_OMIT { unsafe { read_encoded_ptr(&mut p, lp_start_encoding, p_addr) }} 
    else { region_start };

    let type_encoding = unsafe { *p };
    p = unsafe { p.add(1) };
    let type_table = if type_encoding != DW_EH_PE_OMIT {
        let type_table_offset = unsafe { read_uleb128(&mut p) as usize };
        unsafe { p.add(type_table_offset) }
    } else { std::ptr::null() };

    let call_site_encoding = unsafe { *p };
    p = unsafe { p.add(1) };
    let call_site_table_len = unsafe { read_uleb128(&mut p) as usize };
    let call_site_table = p;
    let call_site_table_end = unsafe { p.add(call_site_table_len) };
    let action_table_start = call_site_table_end;

    Some(ParsedLsda {
        lp_start,
        type_table,
        type_encoding,
        call_site_encoding,
        call_site_table,
        call_site_table_end,
        action_table_start,
    })
}

// lsda must contain valid pointers to the call site table
pub unsafe fn find_call_site(lsda: &ParsedLsda, ip: usize) -> Option<CallSite> {
    if ip <= lsda.lp_start { return None; }
    let ip_offset = (ip - 1) - lsda.lp_start;
    let mut p = lsda.call_site_table;
    while p < lsda.call_site_table_end {
        let cs_start = unsafe { read_encoded_ptr(&mut p, lsda.call_site_encoding, 0) };
        let cs_len = unsafe { read_encoded_ptr(&mut p, lsda.call_site_encoding, 0) };
        let cs_lp = unsafe { read_encoded_ptr(&mut p, lsda.call_site_encoding, 0) };
        let cs_action = unsafe { read_uleb128(&mut p) };

        if ip_offset >= cs_start && ip_offset < cs_start + cs_len {
            return Some(CallSite {
                landing_pad: if cs_lp == 0 { 0 } else { lsda.lp_start + cs_lp },
                action_offset: cs_action,
            });
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDecision {
    NoAction,
    Cleanup,
    Catch(i64),
}

// lsda must contain valid pointers to the action table
pub unsafe fn resolve_action(lsda: &ParsedLsda, action_offset: u64) -> ActionDecision {
    if action_offset == 0 { return ActionDecision::Cleanup; }
    let mut p = unsafe { lsda.action_table_start.add((action_offset - 1) as usize) };
    loop {
        let type_filter = unsafe { read_sleb128(&mut p) };
        let next_offset = unsafe { read_sleb128(&mut p) };
        if type_filter == 0 {
            if next_offset == 0 { return ActionDecision::Cleanup; }
        } else { return ActionDecision::Catch(type_filter); }
        if next_offset == 0 { break; }
        p = unsafe { p.add(next_offset as usize) };
    }
    ActionDecision::Cleanup
}

// invoked by system unwinder with valid exception object and unwind context
#[cfg(not(all(windows, target_env = "msvc")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_personality(
    version: i32,
    actions: i32,
    _exception_class: u64,
    exception_object: *mut UnwindException,
    context: *mut UnwindContext,
) -> i32 {
    if version != 1 || context.is_null() { return _URC_FATAL_PHASE1_ERROR; }

    let lsda_ptr = unsafe { _Unwind_GetLanguageSpecificData(context) };
    if lsda_ptr.is_null() { return _URC_CONTINUE_UNWIND; }

    let region_start = unsafe { _Unwind_GetRegionStart(context) };
    let Some(lsda) = (unsafe { parse_lsda_header(lsda_ptr, region_start) }) else {
        return _URC_CONTINUE_UNWIND;
    };

    let ip = unsafe { _Unwind_GetIP(context) };
    let Some(cs) = (unsafe { find_call_site(&lsda, ip) }) else {
        return _URC_CONTINUE_UNWIND;
    };

    if cs.landing_pad == 0 { return _URC_CONTINUE_UNWIND; }
    let decision = unsafe { resolve_action(&lsda, cs.action_offset) };

    if (actions & _UA_SEARCH_PHASE) != 0 {
        match decision {
            ActionDecision::Catch(_) => _URC_HANDLER_FOUND,
            _ => _URC_CONTINUE_UNWIND,
        }
    } else if (actions & _UA_CLEANUP_PHASE) != 0 {
        let selector = match decision {
            ActionDecision::Catch(filter) => filter as usize,
            _ => 0,
        };

        unsafe {
            _Unwind_SetIP(context, cs.landing_pad);
            _Unwind_SetGR(context, DWARF_REG_DATA0, exception_object as usize);
            _Unwind_SetGR(context, DWARF_REG_DATA1, selector);
        }

        _URC_INSTALL_CONTEXT
    } else { _URC_CONTINUE_UNWIND }
}

// invoked by Windows SEH dispatcher
#[cfg(all(windows, target_env = "msvc"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_personality(
    _exception_record: *mut u8,
    _establisher_frame: *mut u8,
    _context_record: *mut u8,
    _dispatcher_context: *mut u8,
) -> i32 { 1 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uleb128_roundtrip() {
        let bytes = [0xE5, 0x8E, 0x26]; // 624485
        let mut p = bytes.as_ptr();
        let val = unsafe { read_uleb128(&mut p) };
        assert_eq!(val, 624485);
        assert_eq!(p, unsafe { bytes.as_ptr().add(3) });
    }

    #[test]
    fn test_sleb128_negative() {
        let bytes = [0x9B, 0xF1, 0x59]; // -624485
        let mut p = bytes.as_ptr();
        let val = unsafe { read_sleb128(&mut p) };
        assert_eq!(val, -624485);
        assert_eq!(p, unsafe { bytes.as_ptr().add(3) });
    }

    #[test]
    fn test_synthetic_lsda_call_site_and_action() {
        // synthesize LSDA buffer
        // header:
        // lp_start_encoding: DW_EH_PE_OMIT (0xFF) -> lp_start = region_start (0x1000)
        // type_encoding: DW_EH_PE_OMIT (0xFF)
        // call_site_encoding: DW_EH_PE_UDATA4 (0x03)
        // call_site_length: uleb128(13) (one call site entry: start 4b, len 4b, lp 4b, action uleb128)
        let mut buf = vec![DW_EH_PE_OMIT, DW_EH_PE_OMIT, DW_EH_PE_UDATA4, 13u8];

        // call site 1: offset 0x20..0x40, landing pad offset 0x80, action 1
        buf.extend_from_slice(&0x20u32.to_ne_bytes());
        buf.extend_from_slice(&0x20u32.to_ne_bytes());
        buf.extend_from_slice(&0x80u32.to_ne_bytes());
        buf.push(1u8); // action 1 (1-based offset = index 0)

        // action table:
        // action 1: type_filter = 1 (catch clause), next = 0
        buf.push(1u8); // sleb128(1)
        buf.push(0u8); // sleb128(0)

        unsafe {
            let parsed = parse_lsda_header(buf.as_ptr(), 0x1000).expect("parsed header");
            assert_eq!(parsed.lp_start, 0x1000);

            // test inside call site range: ip = 0x1000 + 0x25 + 1 = 0x1026
            let cs = find_call_site(&parsed, 0x1026).expect("found call site");
            assert_eq!(cs.landing_pad, 0x1080);
            assert_eq!(cs.action_offset, 1);

            let action = resolve_action(&parsed, cs.action_offset);
            assert_eq!(action, ActionDecision::Catch(1));

            // test outside call site range: ip = 0x1050
            let outside = find_call_site(&parsed, 0x1050);
            assert!(outside.is_none());
        }
    }

    #[test]
    fn test_synthetic_lsda_cleanup_action() {
        let mut buf = vec![DW_EH_PE_OMIT, DW_EH_PE_OMIT, DW_EH_PE_UDATA4, 13u8];

        // call site with action 0 (cleanup only)
        buf.extend_from_slice(&0x10u32.to_ne_bytes());
        buf.extend_from_slice(&0x10u32.to_ne_bytes());
        buf.extend_from_slice(&0x50u32.to_ne_bytes());
        buf.push(0u8); // action 0

        unsafe {
            let parsed = parse_lsda_header(buf.as_ptr(), 0x2000).expect("parsed header");
            let cs = find_call_site(&parsed, 0x2015).expect("found call site");
            assert_eq!(cs.landing_pad, 0x2050);
            assert_eq!(cs.action_offset, 0);

            let action = resolve_action(&parsed, cs.action_offset);
            assert_eq!(action, ActionDecision::Cleanup);
        }
    }
}
