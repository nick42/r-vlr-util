//! Dynamically loaded Windows libraries and symbols.

use super::error::{HResult, Result};
use std::ffi::{CStr, OsStr, c_void};
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

pub struct DynamicLibrary(HMODULE);

impl DynamicLibrary {
    pub fn load(name: impl AsRef<OsStr>) -> Result<Self> {
        let name: Vec<_> = name.as_ref().encode_wide().chain(Some(0)).collect();
        // SAFETY: name is a valid NUL-terminated UTF-16 string.
        let module = unsafe { LoadLibraryW(name.as_ptr()) };
        if module.is_null() {
            Err(HResult::last_error())
        } else {
            Ok(Self(module))
        }
    }

    /// Finds a raw exported symbol.
    ///
    /// The returned pointer is only valid while this library remains loaded.
    pub fn symbol(&self, name: &CStr) -> Result<*const c_void> {
        // SAFETY: module is valid and name is NUL-terminated.
        let function = unsafe { GetProcAddress(self.0, name.as_ptr().cast()) };
        function
            .map(|value| (value as *const ()).cast::<c_void>())
            .ok_or_else(HResult::last_error)
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: this value owns a valid loaded module.
        unsafe { FreeLibrary(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicLibrary;
    #[test]
    fn loads_library_and_symbol() {
        let library = DynamicLibrary::load("kernel32.dll").unwrap();
        assert!(library.symbol(c"GetCurrentProcessId").is_ok());
        assert!(library.symbol(c"not_a_symbol").is_err());
    }
}
