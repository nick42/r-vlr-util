//! Stable C ABI for C and C++ consumers.

#![allow(unsafe_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::slice;

pub const ABI_VERSION: u32 = 1;
pub const STATUS_OK: i32 = 0;
pub const STATUS_INSUFFICIENT_BUFFER: i32 = 1;
pub const STATUS_INVALID_ARGUMENT: i32 = -1;
pub const STATUS_CONVERSION_FAILED: i32 = -2;
pub const STATUS_NOT_IMPLEMENTED: i32 = -3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VruGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

fn read_slice<'a, T>(pointer: *const T, length: usize) -> Option<&'a [T]> {
    if length == 0 {
        return Some(&[]);
    }
    if pointer.is_null() {
        return None;
    }
    // SAFETY: callers of the C ABI promise pointer validity for length items.
    Some(unsafe { slice::from_raw_parts(pointer, length) })
}

fn write_slice<T: Copy>(value: &[T], output: *mut T, capacity: usize, required: *mut usize) -> i32 {
    if required.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    // SAFETY: required was validated non-null.
    unsafe { required.write(value.len()) };
    if capacity < value.len() || output.is_null() {
        return STATUS_INSUFFICIENT_BUFFER;
    }
    // SAFETY: caller promises output is writable for capacity elements.
    unsafe { output.copy_from_nonoverlapping(value.as_ptr(), value.len()) };
    STATUS_OK
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_crc32(data: *const u8, length: usize) -> u32 {
    read_slice(data, length).map_or(0, crate::numeric::crc32)
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_string_equal_utf8(
    left: *const u8,
    left_length: usize,
    right: *const u8,
    right_length: usize,
    case_insensitive: u8,
) -> u8 {
    let Some(left) = read_slice(left, left_length) else {
        return 0;
    };
    let Some(right) = read_slice(right, right_length) else {
        return 0;
    };
    let (Ok(left), Ok(right)) = (std::str::from_utf8(left), std::str::from_utf8(right)) else {
        return 0;
    };
    let equal = if case_insensitive != 0 {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    };
    u8::from(equal)
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_utf8_to_utf16(
    input: *const u8,
    input_length: usize,
    output: *mut u16,
    output_capacity: usize,
    required: *mut usize,
) -> i32 {
    let Some(input) = read_slice(input, input_length) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(input) = std::str::from_utf8(input) else {
        return STATUS_CONVERSION_FAILED;
    };
    write_slice(
        &input.encode_utf16().collect::<Vec<_>>(),
        output,
        output_capacity,
        required,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_utf16_to_utf8(
    input: *const u16,
    input_length: usize,
    output: *mut u8,
    output_capacity: usize,
    required: *mut usize,
) -> i32 {
    let Some(input) = read_slice(input, input_length) else {
        return STATUS_INVALID_ARGUMENT;
    };
    let Ok(value) = String::from_utf16(input) else {
        return STATUS_CONVERSION_FAILED;
    };
    write_slice(value.as_bytes(), output, output_capacity, required)
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_file_exists_utf16(path: *const u16, path_length: usize) -> u8 {
    let Some(path) = read_slice(path, path_length) else {
        return 0;
    };
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        u8::from(std::path::PathBuf::from(std::ffi::OsString::from_wide(path)).is_file())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_directory_exists_utf16(path: *const u16, path_length: usize) -> u8 {
    let Some(path) = read_slice(path, path_length) else {
        return 0;
    };
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        u8::from(std::path::PathBuf::from(std::ffi::OsString::from_wide(path)).is_dir())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_is_debugger_attached() -> u8 {
    #[cfg(windows)]
    {
        u8::from(crate::windows::runtime::is_debugger_attached())
    }
    #[cfg(not(windows))]
    {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vru_guid_create(output: *mut VruGuid) -> i32 {
    if output.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    #[cfg(windows)]
    {
        match crate::windows::guid::Guid::new() {
            Ok(value) => {
                let value = value.0;
                // SAFETY: output was validated non-null.
                unsafe {
                    output.write(VruGuid {
                        data1: value.data1,
                        data2: value.data2,
                        data3: value.data3,
                        data4: value.data4,
                    });
                }
                STATUS_OK
            }
            Err(error) => error.0,
        }
    }
    #[cfg(not(windows))]
    {
        STATUS_NOT_IMPLEMENTED
    }
}

#[cfg(test)]
mod tests {
    use super::{
        STATUS_INSUFFICIENT_BUFFER, STATUS_OK, VruGuid, vru_abi_version, vru_crc32,
        vru_guid_create, vru_string_equal_utf8, vru_utf8_to_utf16, vru_utf16_to_utf8,
    };

    #[test]
    fn portable_abi_functions_work() {
        assert_eq!(vru_abi_version(), 1);
        assert_eq!(vru_crc32(b"123456789".as_ptr(), 9), 0xcbf4_3926);
        assert_ne!(
            vru_string_equal_utf8(b"Value".as_ptr(), 5, b"value".as_ptr(), 5, 1),
            0
        );
    }

    #[test]
    fn conversion_abi_uses_two_call_buffer_protocol() {
        let mut required = 0;
        assert_eq!(
            vru_utf8_to_utf16(
                b"value".as_ptr(),
                5,
                std::ptr::null_mut(),
                0,
                &raw mut required
            ),
            STATUS_INSUFFICIENT_BUFFER
        );
        let mut wide = vec![0_u16; required];
        assert_eq!(
            vru_utf8_to_utf16(
                b"value".as_ptr(),
                5,
                wide.as_mut_ptr(),
                wide.len(),
                &raw mut required
            ),
            STATUS_OK
        );
        let mut required_utf8 = 0;
        assert_eq!(
            vru_utf16_to_utf8(
                wide.as_ptr(),
                wide.len(),
                std::ptr::null_mut(),
                0,
                &raw mut required_utf8
            ),
            STATUS_INSUFFICIENT_BUFFER
        );
    }

    #[cfg(windows)]
    #[test]
    fn creates_guid_through_abi() {
        let mut guid = VruGuid::default();
        assert_eq!(vru_guid_create(&raw mut guid), STATUS_OK);
        assert_ne!(guid, VruGuid::default());
    }
}
