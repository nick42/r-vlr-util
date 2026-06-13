//! Windows SID conversion, account lookup, and token privileges.

use super::error::{HResult, Result};
use super::handle::{OwnedHandle, OwnedLocalAllocation};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError, HANDLE, LUID};
use windows_sys::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSidToSidW};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, GetLengthSid, IsValidSid, LUID_AND_ATTRIBUTES, LookupAccountSidW,
    LookupPrivilegeValueW, PSID, SE_PRIVILEGE_ENABLED, SID_NAME_USE, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn utf16_ptr_to_string(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    // SAFETY: pointer references a NUL-terminated Windows string.
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sid {
    bytes: Vec<u8>,
}

impl Sid {
    /// Copies a validated Win32 SID into owned storage.
    ///
    /// # Safety
    ///
    /// `sid` must point to readable memory containing a SID for the duration
    /// of this call.
    pub unsafe fn from_raw_copy(sid: PSID) -> Result<Self> {
        if sid.is_null() || unsafe { IsValidSid(sid) } == 0 {
            return Err(HResult::INVALID_ARGUMENT);
        }
        let length = unsafe { GetLengthSid(sid) } as usize;
        let bytes = unsafe { std::slice::from_raw_parts(sid.cast::<u8>(), length) }.to_vec();
        Ok(Self { bytes })
    }

    pub fn parse(value: &str) -> Result<Self> {
        let value = wide(value);
        let mut sid = null_mut();
        // SAFETY: value is NUL-terminated and output is writable.
        if unsafe { ConvertStringSidToSidW(value.as_ptr(), &raw mut sid) } == 0 {
            return Err(HResult::last_error());
        }
        // SAFETY: successful conversion returns LocalFree-owned memory.
        let allocation =
            unsafe { OwnedLocalAllocation::from_raw(sid) }.expect("SID allocation was null");
        // SAFETY: GetLengthSid accepts the valid SID returned by the API.
        // SAFETY: ConvertStringSidToSidW returned a valid SID allocation.
        let result = unsafe { Self::from_raw_copy(sid) };
        drop(allocation);
        result
    }

    pub fn to_string(&self) -> Result<String> {
        let mut value = null_mut();
        // SAFETY: bytes contain a validated SID and output is writable.
        if unsafe { ConvertSidToStringSidW(self.bytes.as_ptr().cast_mut().cast(), &raw mut value) }
            == 0
        {
            return Err(HResult::last_error());
        }
        // SAFETY: successful conversion returns LocalFree-owned memory.
        let allocation = unsafe { OwnedLocalAllocation::from_raw(value.cast()) }
            .expect("SID string allocation was null");
        let string = utf16_ptr_to_string(value);
        drop(allocation);
        Ok(string)
    }

    pub fn lookup_account(&self, system_name: Option<&str>) -> Result<AccountSidInfo> {
        let system = system_name.map(wide);
        let system_ptr = system.as_ref().map_or(null(), Vec::as_ptr);
        let mut name_len = 0;
        let mut domain_len = 0;
        let mut use_type: SID_NAME_USE = 0;
        // SAFETY: first call queries required buffer lengths.
        unsafe {
            LookupAccountSidW(
                system_ptr,
                self.bytes.as_ptr().cast_mut().cast(),
                null_mut(),
                &raw mut name_len,
                null_mut(),
                &raw mut domain_len,
                &raw mut use_type,
            )
        };
        // SAFETY: GetLastError has no preconditions.
        if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
            return Err(HResult::last_error());
        }
        let mut name = vec![0_u16; name_len as usize];
        let mut domain = vec![0_u16; domain_len as usize];
        // SAFETY: buffers are allocated to the requested sizes.
        if unsafe {
            LookupAccountSidW(
                system_ptr,
                self.bytes.as_ptr().cast_mut().cast(),
                name.as_mut_ptr(),
                &raw mut name_len,
                domain.as_mut_ptr(),
                &raw mut domain_len,
                &raw mut use_type,
            )
        } == 0
        {
            return Err(HResult::last_error());
        }
        name.truncate(name_len as usize);
        domain.truncate(domain_len as usize);
        Ok(AccountSidInfo {
            name: String::from_utf16_lossy(&name),
            domain: String::from_utf16_lossy(&domain),
            use_type,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountSidInfo {
    pub name: String,
    pub domain: String,
    pub use_type: SID_NAME_USE,
}

fn open_current_process_token() -> Result<OwnedHandle> {
    let mut token: HANDLE = null_mut();
    // SAFETY: current process pseudo-handle is valid and output is writable.
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &raw mut token,
        )
    } == 0
    {
        return Err(HResult::last_error());
    }
    // SAFETY: successful OpenProcessToken returned an owned token.
    Ok(unsafe { OwnedHandle::from_raw(token) }.expect("token handle was null"))
}

pub fn set_process_privilege(privilege_name: &str, enabled: bool) -> Result<()> {
    let privilege_name = wide(privilege_name);
    let mut luid = LUID::default();
    // SAFETY: privilege_name is NUL-terminated and luid is writable.
    if unsafe { LookupPrivilegeValueW(null(), privilege_name.as_ptr(), &raw mut luid) } == 0 {
        return Err(HResult::last_error());
    }
    let token = open_current_process_token()?;
    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: if enabled { SE_PRIVILEGE_ENABLED } else { 0 },
        }],
    };
    // SAFETY: token is valid and privileges points to initialized data.
    if unsafe {
        AdjustTokenPrivileges(
            token.as_raw(),
            0,
            &raw const privileges,
            0,
            null_mut(),
            null_mut(),
        )
    } == 0
    {
        return Err(HResult::last_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Sid;

    #[test]
    fn sid_round_trips_and_resolves_well_known_account() {
        let sid = Sid::parse("S-1-5-18").unwrap();
        assert_eq!(sid.to_string().unwrap(), "S-1-5-18");
        assert!(!sid.lookup_account(None).unwrap().name.is_empty());
    }
}
