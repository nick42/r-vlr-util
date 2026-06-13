//! Windows runtime information.

#[cfg(target_pointer_width = "32")]
use super::error::HResult;
use super::error::Result;
use std::ffi::OsString;
use windows_sys::Win32::System::Diagnostics::Debug::IsDebuggerPresent;
#[cfg(target_pointer_width = "32")]
use windows_sys::Win32::System::SystemInformation::IsWow64Process;
#[cfg(target_pointer_width = "32")]
use windows_sys::Win32::System::Threading::GetCurrentProcess;

#[must_use]
pub fn is_debugger_attached() -> bool {
    // SAFETY: IsDebuggerPresent has no preconditions.
    unsafe { IsDebuggerPresent() != 0 }
}

/// Returns the current process command line as parsed by Rust's Windows
/// standard-library implementation.
#[must_use]
pub fn command_line_arguments() -> Vec<OsString> {
    std::env::args_os().collect()
}

pub fn platform_is_64_bit() -> Result<bool> {
    #[cfg(target_pointer_width = "64")]
    {
        Ok(true)
    }
    #[cfg(target_pointer_width = "32")]
    {
        let mut is_wow64 = 0;
        // SAFETY: current-process pseudo-handle is valid and output is writable.
        if unsafe { IsWow64Process(GetCurrentProcess(), &raw mut is_wow64) } == 0 {
            Err(HResult::last_error())
        } else {
            Ok(is_wow64 != 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{command_line_arguments, platform_is_64_bit};

    #[test]
    fn current_command_line_has_executable() {
        assert!(!command_line_arguments().is_empty());
        assert!(platform_is_64_bit().is_ok());
    }
}
