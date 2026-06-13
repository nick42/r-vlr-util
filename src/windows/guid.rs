//! Windows GUID creation, parsing, and formatting.

use super::error::{HResult, Result};
use std::fmt;
use std::hash::{Hash, Hasher};
use windows_sys::Win32::System::Com::{CLSIDFromString, CoCreateGuid, StringFromGUID2};
use windows_sys::core::GUID;

#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct Guid(pub GUID);

impl PartialEq for Guid {
    fn eq(&self, other: &Self) -> bool {
        self.0.data1 == other.0.data1
            && self.0.data2 == other.0.data2
            && self.0.data3 == other.0.data3
            && self.0.data4 == other.0.data4
    }
}

impl Eq for Guid {}

impl Hash for Guid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.data1.hash(state);
        self.0.data2.hash(state);
        self.0.data3.hash(state);
        self.0.data4.hash(state);
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl Guid {
    pub fn new() -> Result<Self> {
        let mut value = GUID::default();
        // SAFETY: value points to writable GUID storage.
        let result = unsafe { CoCreateGuid(&raw mut value) };
        if result < 0 {
            Err(HResult(result))
        } else {
            Ok(Self(value))
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        let value: Vec<_> = value.encode_utf16().chain(Some(0)).collect();
        let mut guid = GUID::default();
        // SAFETY: value is NUL-terminated and guid is writable.
        let result = unsafe { CLSIDFromString(value.as_ptr(), &raw mut guid) };
        if result < 0 {
            Err(HResult(result))
        } else {
            Ok(Self(guid))
        }
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = [0_u16; 39];
        // SAFETY: buffer is sufficiently sized for the documented GUID form.
        let count = unsafe { StringFromGUID2(&raw const self.0, buffer.as_mut_ptr(), 39) };
        if count == 0 {
            return Err(fmt::Error);
        }
        let string = String::from_utf16_lossy(&buffer[..count as usize - 1]);
        formatter.write_str(&string)
    }
}

#[cfg(test)]
mod tests {
    use super::Guid;

    #[test]
    fn guid_round_trips() {
        let guid = Guid::new().unwrap();
        let text = guid.to_string();
        assert_eq!(Guid::parse(&text).unwrap(), guid);
        assert!(Guid::parse("not-a-guid").is_err());
    }
}
