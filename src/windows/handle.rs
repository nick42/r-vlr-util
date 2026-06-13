//! Owned wrappers for common Windows handles.

use std::fmt;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows_sys::Win32::System::Registry::{HKEY, RegCloseKey};
use windows_sys::Win32::System::Services::{CloseServiceHandle, SC_HANDLE};

pub struct OwnedHandle(HANDLE);

impl OwnedHandle {
    /// Takes ownership of a valid handle.
    ///
    /// # Safety
    /// The caller must transfer unique ownership of a handle closed by
    /// `CloseHandle`.
    pub unsafe fn from_raw(handle: HANDLE) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }

    #[must_use]
    pub const fn as_raw(&self) -> HANDLE {
        self.0
    }

    #[must_use]
    pub fn into_raw(mut self) -> HANDLE {
        let handle = self.0;
        self.0 = null_mut();
        handle
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this type uniquely owns a valid CloseHandle handle.
            unsafe { CloseHandle(self.0) };
        }
    }
}

impl fmt::Debug for OwnedHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("OwnedHandle").field(&self.0).finish()
    }
}

pub struct OwnedRegistryKey(HKEY);

impl OwnedRegistryKey {
    /// Takes ownership of a registry key handle.
    ///
    /// # Safety
    /// The caller must transfer unique ownership of a handle closed by
    /// `RegCloseKey`. Predefined root keys must not be wrapped.
    pub unsafe fn from_raw(key: HKEY) -> Option<Self> {
        (!key.is_null()).then_some(Self(key))
    }

    #[must_use]
    pub const fn as_raw(&self) -> HKEY {
        self.0
    }
}

impl Drop for OwnedRegistryKey {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this type uniquely owns a RegCloseKey-compatible handle.
            unsafe { RegCloseKey(self.0) };
        }
    }
}

impl fmt::Debug for OwnedRegistryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OwnedRegistryKey")
            .field(&self.0)
            .finish()
    }
}

pub struct OwnedServiceHandle(SC_HANDLE);

impl OwnedServiceHandle {
    /// Takes ownership of a service control manager or service handle.
    ///
    /// # Safety
    /// The caller must transfer unique ownership of a handle closed by
    /// `CloseServiceHandle`.
    pub unsafe fn from_raw(handle: SC_HANDLE) -> Option<Self> {
        (!handle.is_null()).then_some(Self(handle))
    }

    #[must_use]
    pub const fn as_raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for OwnedServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this type uniquely owns a service handle.
            unsafe { CloseServiceHandle(self.0) };
        }
    }
}

pub struct OwnedLocalAllocation(HLOCAL);

impl OwnedLocalAllocation {
    /// Takes ownership of a LocalAlloc-compatible allocation.
    ///
    /// # Safety
    /// The caller must transfer unique ownership of memory freed by `LocalFree`.
    pub unsafe fn from_raw(value: HLOCAL) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }

    #[must_use]
    pub const fn as_raw(&self) -> HLOCAL {
        self.0
    }
}

impl Drop for OwnedLocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this type uniquely owns a LocalFree-compatible pointer.
            unsafe { LocalFree(self.0) };
        }
    }
}
