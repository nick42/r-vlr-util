//! HRESULT and Win32 error interoperability.

use std::fmt;
use windows_sys::Win32::Foundation::{GetLastError, WIN32_ERROR};
use windows_sys::core::HRESULT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct HResult(pub HRESULT);

impl HResult {
    pub const OK: Self = Self(0);
    pub const FALSE: Self = Self(1);
    pub const FAIL: Self = Self(0x8000_4005_u32.cast_signed());
    pub const INVALID_ARGUMENT: Self = Self(0x8007_0057_u32.cast_signed());
    pub const NOT_IMPLEMENTED: Self = Self(0x8000_4001_u32.cast_signed());

    #[must_use]
    pub const fn from_win32(code: WIN32_ERROR) -> Self {
        if code == 0 {
            return Self::OK;
        }
        Self(((code & 0xffff) | 0x8007_0000).cast_signed())
    }

    #[must_use]
    pub fn last_error() -> Self {
        // SAFETY: GetLastError has no preconditions.
        Self::from_win32(unsafe { GetLastError() })
    }

    #[must_use]
    pub const fn is_success(self) -> bool {
        self.0 >= 0
    }

    #[must_use]
    pub const fn is_failure(self) -> bool {
        self.0 < 0
    }

    #[must_use]
    pub const fn facility(self) -> u16 {
        ((self.0 as u32 & 0x07ff_0000) >> 16) as u16
    }

    #[must_use]
    pub const fn code(self) -> u16 {
        (self.0 as u32 & 0xffff) as u16
    }
}

impl fmt::Display for HResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:08X}", self.0 as u32)
    }
}

impl std::error::Error for HResult {}

pub type Result<T> = std::result::Result<T, HResult>;

#[cfg(test)]
mod tests {
    use super::HResult;

    #[test]
    fn win32_errors_become_failure_hresults() {
        assert_eq!(HResult::from_win32(0), HResult::OK);
        let error = HResult::from_win32(5);
        assert!(error.is_failure());
        assert_eq!(error.facility(), 7);
        assert_eq!(error.code(), 5);
        assert_eq!(error.to_string(), "0x80070005");
    }
}
