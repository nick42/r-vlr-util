//! COM apartment initialization scoped to the current thread.

use super::error::{HResult, Result};
use windows_sys::Win32::System::Com::{
    COINIT, CoInitializeEx, CoInitializeSecurity, CoUninitialize, EOAC_DEFAULT, RPC_C_AUTHN_LEVEL,
    RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL, RPC_C_IMP_LEVEL_DEFAULT,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComSecuritySettings {
    pub authentication_level: RPC_C_AUTHN_LEVEL,
    pub impersonation_level: RPC_C_IMP_LEVEL,
    pub capabilities: u32,
}

impl Default for ComSecuritySettings {
    fn default() -> Self {
        Self {
            authentication_level: RPC_C_AUTHN_LEVEL_DEFAULT,
            impersonation_level: RPC_C_IMP_LEVEL_DEFAULT,
            capabilities: EOAC_DEFAULT as u32,
        }
    }
}

pub fn initialize_security(settings: ComSecuritySettings) -> Result<()> {
    // SAFETY: null optional parameters request process defaults.
    let result = unsafe {
        CoInitializeSecurity(
            std::ptr::null_mut(),
            -1,
            std::ptr::null(),
            std::ptr::null(),
            settings.authentication_level,
            settings.impersonation_level,
            std::ptr::null(),
            settings.capabilities,
            std::ptr::null(),
        )
    };
    if result < 0 {
        Err(HResult(result))
    } else {
        Ok(())
    }
}

pub struct ComApartment {
    initialized: bool,
}

impl ComApartment {
    pub fn initialize(flags: COINIT) -> Result<Self> {
        // SAFETY: null reserved pointer is required; lifetime is represented by Self.
        let result = unsafe { CoInitializeEx(std::ptr::null(), flags as u32) };
        if result < 0 {
            Err(HResult(result))
        } else {
            Ok(Self { initialized: true })
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: paired with successful CoInitializeEx on this thread.
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ComApartment;
    use windows_sys::Win32::System::Com::COINIT_MULTITHREADED;

    #[test]
    fn initializes_com_for_scope() {
        let _apartment = ComApartment::initialize(COINIT_MULTITHREADED).unwrap();
    }
}
