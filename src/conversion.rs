//! Explicit conversions at UTF-8, UTF-16, and C-string boundaries.

use std::ffi::{CStr, CString, FromBytesWithNulError, NulError};
use std::string::FromUtf16Error;

/// Converts UTF-16 code units into Rust's UTF-8 `String`.
pub fn from_utf16(value: &[u16]) -> Result<String, FromUtf16Error> {
    String::from_utf16(value)
}

/// Converts UTF-16 code units into UTF-8, replacing malformed sequences.
#[must_use]
pub fn from_utf16_lossy(value: &[u16]) -> String {
    String::from_utf16_lossy(value)
}

/// Converts a Rust UTF-8 string into UTF-16 code units.
#[must_use]
pub fn to_utf16(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

/// Converts a Rust string into an owned NUL-terminated C string.
pub fn to_c_string(value: &str) -> Result<CString, NulError> {
    CString::new(value)
}

/// Validates bytes as a borrowed NUL-terminated C string.
pub fn as_c_str(value: &[u8]) -> Result<&CStr, FromBytesWithNulError> {
    CStr::from_bytes_with_nul(value)
}

#[cfg(test)]
mod tests {
    use super::{as_c_str, from_utf16, from_utf16_lossy, to_c_string, to_utf16};

    #[test]
    fn utf8_and_utf16_round_trip_non_ascii_text() {
        let original = "Hello, 世界";
        let utf16 = to_utf16(original);
        assert_eq!(from_utf16(&utf16).unwrap(), original);
        assert!(from_utf16_lossy(&[0xd800]).contains('\u{fffd}'));
    }

    #[test]
    fn c_string_boundaries_are_explicit() {
        let owned = to_c_string("value").unwrap();
        assert_eq!(owned.as_c_str().to_bytes(), b"value");
        assert_eq!(as_c_str(b"value\0").unwrap().to_bytes(), b"value");
        assert!(to_c_string("embedded\0nul").is_err());
    }
}
