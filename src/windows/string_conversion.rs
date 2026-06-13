//! Windows code-page-aware string conversion.

use super::error::{HResult, Result};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Globalization::{MultiByteToWideChar, WideCharToMultiByte};

pub const CODE_PAGE_ANSI: u32 = 0;
pub const CODE_PAGE_OEM: u32 = 1;
pub const CODE_PAGE_UTF7: u32 = 65_000;
pub const CODE_PAGE_UTF8: u32 = 65_001;

pub fn multi_byte_to_utf16(value: &[u8], code_page: u32, flags: u32) -> Result<Vec<u16>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let input_len = i32::try_from(value.len()).map_err(|_| HResult::FAIL)?;
    // SAFETY: input is a valid byte slice; null output requests required size.
    let required =
        unsafe { MultiByteToWideChar(code_page, flags, value.as_ptr(), input_len, null_mut(), 0) };
    if required == 0 {
        return Err(HResult::last_error());
    }
    let mut result = vec![0_u16; required as usize];
    // SAFETY: result has the exact capacity reported by the preceding API call.
    let written = unsafe {
        MultiByteToWideChar(
            code_page,
            flags,
            value.as_ptr(),
            input_len,
            result.as_mut_ptr(),
            required,
        )
    };
    if written == 0 {
        return Err(HResult::last_error());
    }
    result.truncate(written as usize);
    Ok(result)
}

pub fn utf16_to_multi_byte(value: &[u16], code_page: u32, flags: u32) -> Result<Vec<u8>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let input_len = i32::try_from(value.len()).map_err(|_| HResult::FAIL)?;
    // SAFETY: input is a valid UTF-16 code-unit slice; null output requests size.
    let required = unsafe {
        WideCharToMultiByte(
            code_page,
            flags,
            value.as_ptr(),
            input_len,
            null_mut(),
            0,
            null(),
            null_mut(),
        )
    };
    if required == 0 {
        return Err(HResult::last_error());
    }
    let mut result = vec![0_u8; required as usize];
    // SAFETY: result has the exact capacity reported by the preceding API call.
    let written = unsafe {
        WideCharToMultiByte(
            code_page,
            flags,
            value.as_ptr(),
            input_len,
            result.as_mut_ptr(),
            required,
            null(),
            null_mut(),
        )
    };
    if written == 0 {
        return Err(HResult::last_error());
    }
    result.truncate(written as usize);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{CODE_PAGE_UTF8, multi_byte_to_utf16, utf16_to_multi_byte};

    #[test]
    fn utf8_round_trips_through_windows_api() {
        let value = "Hello, 世界".as_bytes();
        let wide = multi_byte_to_utf16(value, CODE_PAGE_UTF8, 0).unwrap();
        assert_eq!(
            utf16_to_multi_byte(&wide, CODE_PAGE_UTF8, 0).unwrap(),
            value
        );
    }
}
