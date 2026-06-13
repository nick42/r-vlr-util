//! Windows service-control-manager access.

use super::error::{HResult, Result};
use super::handle::OwnedServiceHandle;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows_sys::Win32::System::Services::{
    CreateServiceW, DeleteService, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW,
    QueryServiceConfigW, SC_MANAGER_CONNECT, SC_MANAGER_CREATE_SERVICE,
    SC_MANAGER_ENUMERATE_SERVICE, SERVICE_ALL_ACCESS, SERVICE_DEMAND_START, SERVICE_ERROR_NORMAL,
    SERVICE_QUERY_CONFIG, SERVICE_WIN32_OWN_PROCESS,
};

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub struct ServiceManager {
    handle: OwnedServiceHandle,
}

impl ServiceManager {
    pub fn connect(
        machine_name: Option<&str>,
        database_name: Option<&str>,
        access: u32,
    ) -> Result<Self> {
        let machine = machine_name.map(wide);
        let database = database_name.map(wide);
        // SAFETY: optional strings are NUL-terminated and live through the call.
        let handle = unsafe {
            OpenSCManagerW(
                machine.as_ref().map_or(null(), Vec::as_ptr),
                database.as_ref().map_or(null(), Vec::as_ptr),
                access,
            )
        };
        // SAFETY: successful API call returns an owned service handle.
        unsafe { OwnedServiceHandle::from_raw(handle) }
            .map(|handle| Self { handle })
            .ok_or_else(HResult::last_error)
    }

    pub fn open_service(&self, name: &str, access: u32) -> Result<Service> {
        let name = wide(name);
        // SAFETY: manager handle is valid and name is NUL-terminated.
        let handle = unsafe { OpenServiceW(self.handle.as_raw(), name.as_ptr(), access) };
        // SAFETY: successful API call returns an owned service handle.
        unsafe { OwnedServiceHandle::from_raw(handle) }
            .map(|handle| Service { handle })
            .ok_or_else(HResult::last_error)
    }

    pub fn create_service(&self, config: &CreateServiceConfig<'_>) -> Result<Service> {
        config.validate()?;
        let name = wide(config.name);
        let display_name = wide(config.display_name);
        let binary_path = wide(config.binary_path);
        let load_order_group = config.load_order_group.map(wide);
        let dependencies = config
            .dependencies
            .filter(|values| !values.is_empty())
            .map(|values| {
                let wide_values: Vec<Vec<u16>> = values
                    .iter()
                    .map(|value| value.encode_utf16().collect())
                    .collect();
                crate::data::encode_multi_sz(&wide_values)
                    .expect("service dependencies are nonempty")
            });
        let account_name = config.account_name.map(wide);
        let password = config.password.map(wide);
        let mut tag_id = 0;
        // SAFETY: all optional pointers refer to live, NUL-terminated buffers.
        let handle = unsafe {
            CreateServiceW(
                self.handle.as_raw(),
                name.as_ptr(),
                display_name.as_ptr(),
                config.desired_access,
                config.service_type,
                config.start_type,
                config.error_control,
                binary_path.as_ptr(),
                load_order_group.as_ref().map_or(null(), Vec::as_ptr),
                if config.return_tag_id {
                    &raw mut tag_id
                } else {
                    null_mut()
                },
                dependencies.as_ref().map_or(null(), Vec::as_ptr),
                account_name.as_ref().map_or(null(), Vec::as_ptr),
                password.as_ref().map_or(null(), Vec::as_ptr),
            )
        };
        // SAFETY: successful API call returns an owned service handle.
        unsafe { OwnedServiceHandle::from_raw(handle) }
            .map(|handle| Service { handle })
            .ok_or_else(HResult::last_error)
    }
}

#[derive(Clone, Debug)]
pub struct CreateServiceConfig<'a> {
    pub name: &'a str,
    pub display_name: &'a str,
    pub binary_path: &'a str,
    pub desired_access: u32,
    pub service_type: u32,
    pub start_type: u32,
    pub error_control: u32,
    pub load_order_group: Option<&'a str>,
    pub dependencies: Option<&'a [&'a str]>,
    pub account_name: Option<&'a str>,
    pub password: Option<&'a str>,
    pub return_tag_id: bool,
}

impl<'a> CreateServiceConfig<'a> {
    #[must_use]
    pub const fn new(name: &'a str, display_name: &'a str, binary_path: &'a str) -> Self {
        Self {
            name,
            display_name,
            binary_path,
            desired_access: SERVICE_ALL_ACCESS,
            service_type: SERVICE_WIN32_OWN_PROCESS,
            start_type: SERVICE_DEMAND_START,
            error_control: SERVICE_ERROR_NORMAL,
            load_order_group: None,
            dependencies: None,
            account_name: None,
            password: None,
            return_tag_id: false,
        }
    }

    fn validate(&self) -> Result<()> {
        let invalid_name = self.name.is_empty()
            || self.display_name.is_empty()
            || self.binary_path.is_empty()
            || self.name.len() > 256
            || self.display_name.len() > 256
            || self.name.contains(['/', '\\']);
        let unquoted_spaced_path = self.binary_path.contains(' ')
            && !(self.binary_path.starts_with('"') && self.binary_path.ends_with('"'));
        if invalid_name || unquoted_spaced_path {
            Err(HResult::INVALID_ARGUMENT)
        } else {
            Ok(())
        }
    }
}

pub struct Service {
    handle: OwnedServiceHandle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceConfig {
    pub service_type: u32,
    pub start_type: u32,
    pub error_control: u32,
    pub binary_path: String,
    pub display_name: String,
}

fn utf16_ptr_to_string(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    // SAFETY: SCM returns valid NUL-terminated strings inside the query buffer.
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length))
    }
}

impl Service {
    pub fn delete(&self) -> Result<()> {
        // SAFETY: handle is a live service handle.
        if unsafe { DeleteService(self.handle.as_raw()) } == 0 {
            Err(HResult::last_error())
        } else {
            Ok(())
        }
    }

    pub fn query_config(&self) -> Result<ServiceConfig> {
        let mut required = 0;
        // SAFETY: first call intentionally queries required size.
        unsafe {
            QueryServiceConfigW(
                self.handle.as_raw(),
                std::ptr::null_mut(),
                0,
                &raw mut required,
            )
        };
        if required == 0 {
            return Err(HResult::last_error());
        }
        let word_count = (required as usize).div_ceil(size_of::<usize>());
        let mut buffer = vec![0_usize; word_count];
        let config = buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>();
        // SAFETY: buffer is writable to the size requested by SCM.
        if unsafe { QueryServiceConfigW(self.handle.as_raw(), config, required, &raw mut required) }
            == 0
        {
            return Err(HResult::last_error());
        }
        // SAFETY: successful QueryServiceConfigW initialized config.
        let config = unsafe { &*config };
        Ok(ServiceConfig {
            service_type: config.dwServiceType,
            start_type: config.dwStartType,
            error_control: config.dwErrorControl,
            binary_path: utf16_ptr_to_string(config.lpBinaryPathName),
            display_name: utf16_ptr_to_string(config.lpDisplayName),
        })
    }
}

pub const CONNECT: u32 = SC_MANAGER_CONNECT;
pub const CREATE_SERVICE: u32 = SC_MANAGER_CREATE_SERVICE;
pub const ENUMERATE_SERVICE: u32 = SC_MANAGER_ENUMERATE_SERVICE;
pub const QUERY_CONFIG: u32 = SERVICE_QUERY_CONFIG;

#[cfg(test)]
mod tests {
    use super::{CONNECT, CreateServiceConfig, ServiceManager};

    #[test]
    fn connects_to_local_service_manager() {
        let _manager = ServiceManager::connect(None, None, CONNECT).unwrap();
    }

    #[test]
    fn service_creation_config_rejects_unquoted_spaced_path() {
        assert!(
            CreateServiceConfig::new("name", "display", "C:\\Program Files\\service.exe")
                .validate()
                .is_err()
        );
    }
}
