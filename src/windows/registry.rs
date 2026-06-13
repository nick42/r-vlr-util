//! Owned Windows registry access.

use super::error::{HResult, Result};
use super::handle::OwnedRegistryKey;
use crate::options::{AppOptions, OptionSource, OptionValue, SpecifiedValue};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    HKEY_USERS, KEY_READ, KEY_WRITE, REG_BINARY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_NONE,
    REG_OPTION_NON_VOLATILE, REG_QWORD, REG_SAM_FLAGS, REG_SZ, RegCreateKeyExW, RegDeleteTreeW,
    RegDeleteValueW, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryRoot {
    ClassesRoot,
    CurrentUser,
    LocalMachine,
    Users,
    CurrentConfig,
}

impl RegistryRoot {
    #[must_use]
    pub const fn as_raw(self) -> HKEY {
        match self {
            Self::ClassesRoot => HKEY_CLASSES_ROOT,
            Self::CurrentUser => HKEY_CURRENT_USER,
            Self::LocalMachine => HKEY_LOCAL_MACHINE,
            Self::Users => HKEY_USERS,
            Self::CurrentConfig => HKEY_CURRENT_CONFIG,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryValue {
    None,
    String(String),
    ExpandString(String),
    MultiString(Vec<String>),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    Unknown { value_type: u32, data: Vec<u8> },
}

#[derive(Debug)]
pub struct RegistryKey {
    key: OwnedRegistryKey,
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn decode_utf16_bytes(data: &[u8]) -> String {
    let values: Vec<u16> = data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|value| *value != 0)
        .collect();
    String::from_utf16_lossy(&values)
}

impl RegistryKey {
    pub fn open(root: RegistryRoot, path: &str, access: REG_SAM_FLAGS) -> Result<Self> {
        Self::open_raw(root.as_raw(), path, access)
    }

    fn open_raw(parent: HKEY, path: &str, access: REG_SAM_FLAGS) -> Result<Self> {
        let path = wide(path);
        let mut key = null_mut();
        // SAFETY: path is NUL-terminated and key points to writable storage.
        let result = unsafe { RegOpenKeyExW(parent, path.as_ptr(), 0, access, &raw mut key) };
        if result != ERROR_SUCCESS {
            return Err(HResult::from_win32(result));
        }
        // SAFETY: successful RegOpenKeyExW returned an owned closeable key.
        Ok(Self {
            key: unsafe { OwnedRegistryKey::from_raw(key).expect("RegOpenKeyExW returned null") },
        })
    }

    pub fn create(root: RegistryRoot, path: &str, access: REG_SAM_FLAGS) -> Result<Self> {
        let path = wide(path);
        let mut key = null_mut();
        let mut disposition = 0;
        // SAFETY: path is NUL-terminated and outputs are writable.
        let result = unsafe {
            RegCreateKeyExW(
                root.as_raw(),
                path.as_ptr(),
                0,
                null_mut(),
                REG_OPTION_NON_VOLATILE,
                access,
                null(),
                &raw mut key,
                &raw mut disposition,
            )
        };
        if result != ERROR_SUCCESS {
            return Err(HResult::from_win32(result));
        }
        // SAFETY: successful RegCreateKeyExW returned an owned closeable key.
        Ok(Self {
            key: unsafe { OwnedRegistryKey::from_raw(key).expect("RegCreateKeyExW returned null") },
        })
    }

    #[must_use]
    pub const fn as_raw(&self) -> HKEY {
        self.key.as_raw()
    }

    pub fn get_value(&self, name: &str) -> Result<Option<RegistryValue>> {
        let name = wide(name);
        let mut value_type = 0_u32;
        let mut size = 0_u32;
        // SAFETY: name is NUL-terminated; null data queries metadata.
        let query = unsafe {
            RegQueryValueExW(
                self.as_raw(),
                name.as_ptr(),
                null_mut(),
                &raw mut value_type,
                null_mut(),
                &raw mut size,
            )
        };
        if query == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if query != ERROR_SUCCESS && query != ERROR_MORE_DATA {
            return Err(HResult::from_win32(query));
        }
        let mut data = vec![0_u8; size as usize];
        // SAFETY: data is writable to the size supplied.
        let query = unsafe {
            RegQueryValueExW(
                self.as_raw(),
                name.as_ptr(),
                null_mut(),
                &raw mut value_type,
                data.as_mut_ptr(),
                &raw mut size,
            )
        };
        if query != ERROR_SUCCESS {
            return Err(HResult::from_win32(query));
        }
        data.truncate(size as usize);
        let value = match value_type {
            REG_NONE => RegistryValue::None,
            REG_SZ => RegistryValue::String(decode_utf16_bytes(&data)),
            REG_EXPAND_SZ => RegistryValue::ExpandString(decode_utf16_bytes(&data)),
            REG_DWORD if data.len() >= 4 => {
                RegistryValue::Dword(u32::from_le_bytes(data[..4].try_into().unwrap()))
            }
            REG_QWORD if data.len() >= 8 => {
                RegistryValue::Qword(u64::from_le_bytes(data[..8].try_into().unwrap()))
            }
            REG_MULTI_SZ => {
                let values: Vec<u16> = data
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .collect();
                RegistryValue::MultiString(
                    crate::data::parse_multi_sz(&values)
                        .map_err(|_| HResult::FAIL)?
                        .into_iter()
                        .map(String::from_utf16_lossy)
                        .collect(),
                )
            }
            REG_BINARY => RegistryValue::Binary(data),
            _ => RegistryValue::Unknown { value_type, data },
        };
        Ok(Some(value))
    }

    pub fn set_value(&self, name: &str, value: &RegistryValue) -> Result<()> {
        let name = wide(name);
        let (value_type, data) = value.to_raw();
        // SAFETY: name is NUL-terminated and data is valid for its supplied length.
        let result = unsafe {
            RegSetValueExW(
                self.as_raw(),
                name.as_ptr(),
                0,
                value_type,
                data.as_ptr(),
                data.len() as u32,
            )
        };
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(HResult::from_win32(result))
        }
    }

    pub fn delete_value(&self, name: &str) -> Result<bool> {
        let name = wide(name);
        // SAFETY: name is NUL-terminated.
        let result = unsafe { RegDeleteValueW(self.as_raw(), name.as_ptr()) };
        match result {
            ERROR_SUCCESS => Ok(true),
            ERROR_FILE_NOT_FOUND => Ok(false),
            _ => Err(HResult::from_win32(result)),
        }
    }

    pub fn delete_tree(&self, subkey: &str) -> Result<bool> {
        let subkey = wide(subkey);
        // SAFETY: subkey is NUL-terminated.
        let result = unsafe { RegDeleteTreeW(self.as_raw(), subkey.as_ptr()) };
        match result {
            ERROR_SUCCESS => Ok(true),
            ERROR_FILE_NOT_FOUND => Ok(false),
            _ => Err(HResult::from_win32(result)),
        }
    }

    pub fn subkey_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for index in 0.. {
            let mut buffer = vec![0_u16; 16_384];
            let mut length = buffer.len() as u32;
            // SAFETY: buffer and length are writable.
            let result = unsafe {
                RegEnumKeyExW(
                    self.as_raw(),
                    index,
                    buffer.as_mut_ptr(),
                    &raw mut length,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            if result == ERROR_NO_MORE_ITEMS {
                break;
            }
            if result != ERROR_SUCCESS {
                return Err(HResult::from_win32(result));
            }
            names.push(String::from_utf16_lossy(&buffer[..length as usize]));
        }
        Ok(names)
    }

    pub fn value_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for index in 0.. {
            let mut buffer = vec![0_u16; 16_384];
            let mut length = buffer.len() as u32;
            // SAFETY: name buffer and length are writable; data is not requested.
            let result = unsafe {
                RegEnumValueW(
                    self.as_raw(),
                    index,
                    buffer.as_mut_ptr(),
                    &raw mut length,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            if result == ERROR_NO_MORE_ITEMS {
                break;
            }
            if result != ERROR_SUCCESS {
                return Err(HResult::from_win32(result));
            }
            names.push(String::from_utf16_lossy(&buffer[..length as usize]));
        }
        Ok(names)
    }

    pub fn read_values_as_options(&self, options: &mut AppOptions) -> Result<usize> {
        let mut count = 0;
        for name in self.value_names()? {
            let Some(value) = self.get_value(&name)? else {
                continue;
            };
            let value = match value {
                RegistryValue::String(value) | RegistryValue::ExpandString(value) => {
                    OptionValue::String(value)
                }
                RegistryValue::Dword(value) => OptionValue::U64(u64::from(value)),
                RegistryValue::Qword(value) => OptionValue::U64(value),
                RegistryValue::Binary(value) => OptionValue::Bytes(value),
                RegistryValue::MultiString(value) => OptionValue::Strings(value),
                RegistryValue::None | RegistryValue::Unknown { .. } => continue,
            };
            options.add(SpecifiedValue::new(
                OptionSource::SystemConfigRepository,
                name,
                value,
            ));
            count += 1;
        }
        Ok(count)
    }
}

impl RegistryValue {
    fn to_raw(&self) -> (u32, Vec<u8>) {
        fn utf16_bytes(value: &str) -> Vec<u8> {
            value
                .encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_le_bytes)
                .collect()
        }
        match self {
            Self::None => (REG_NONE, Vec::new()),
            Self::String(value) => (REG_SZ, utf16_bytes(value)),
            Self::ExpandString(value) => (REG_EXPAND_SZ, utf16_bytes(value)),
            Self::MultiString(values) => {
                let wide: Vec<Vec<u16>> = values
                    .iter()
                    .map(|value| value.encode_utf16().collect())
                    .collect();
                let encoded = crate::data::encode_multi_sz(&wide)
                    .expect("registry multi-string values are nonempty");
                (
                    REG_MULTI_SZ,
                    encoded.into_iter().flat_map(u16::to_le_bytes).collect(),
                )
            }
            Self::Dword(value) => (REG_DWORD, value.to_le_bytes().to_vec()),
            Self::Qword(value) => (REG_QWORD, value.to_le_bytes().to_vec()),
            Self::Binary(value) => (REG_BINARY, value.clone()),
            Self::Unknown { value_type, data } => (*value_type, data.clone()),
        }
    }
}

pub const READ: REG_SAM_FLAGS = KEY_READ;
pub const WRITE: REG_SAM_FLAGS = KEY_WRITE;

#[cfg(test)]
mod tests {
    use super::{READ, RegistryKey, RegistryRoot};

    #[test]
    fn reads_and_enumerates_known_registry_key() {
        let key = RegistryKey::open(RegistryRoot::LocalMachine, "SOFTWARE", READ).unwrap();
        assert!(!key.subkey_names().unwrap().is_empty());
    }
}
