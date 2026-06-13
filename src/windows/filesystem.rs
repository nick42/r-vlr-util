//! Windows filesystem enumeration and volume discovery.

use super::error::{HResult, Result};
use std::ffi::OsString;
use std::fs;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetVolumePathNamesForVolumeNameW,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: OsString,
    pub attributes: u32,
    pub size: u64,
}

impl FileEntry {
    #[must_use]
    pub fn is_directory(&self) -> bool {
        self.path.is_dir()
    }
}

/// Enumerates a directory with Windows file attributes.
pub fn enumerate_files(
    path: impl AsRef<Path>,
    skip_pseudo_entries: bool,
) -> std::io::Result<Vec<FileEntry>> {
    fs::read_dir(path)?
        .filter_map(|entry| match entry {
            Ok(entry)
                if skip_pseudo_entries
                    && matches!(entry.file_name().to_string_lossy().as_ref(), "." | "..") =>
            {
                None
            }
            Ok(entry) => Some(entry.metadata().map(|metadata| FileEntry {
                path: entry.path(),
                name: entry.file_name(),
                attributes: metadata.file_attributes(),
                size: metadata.file_size(),
            })),
            Err(error) => Some(Err(error)),
        })
        .collect()
}

struct VolumeSearch(HANDLE);

impl Drop for VolumeSearch {
    fn drop(&mut self) {
        // SAFETY: this value owns a valid FindFirstVolumeW search handle.
        unsafe { FindVolumeClose(self.0) };
    }
}

fn utf16_buffer_to_string(buffer: &[u16]) -> String {
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}

/// Enumerates Windows volume GUID paths.
pub fn enumerate_volumes() -> Result<Vec<String>> {
    let mut buffer = vec![0_u16; 1024];
    // SAFETY: buffer is writable and its length is supplied.
    let first = unsafe { FindFirstVolumeW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if first == INVALID_HANDLE_VALUE {
        return Err(HResult::last_error());
    }
    let search = VolumeSearch(first);
    let mut volumes = vec![utf16_buffer_to_string(&buffer)];
    loop {
        buffer.fill(0);
        // SAFETY: search is valid and buffer is writable.
        if unsafe { FindNextVolumeW(search.0, buffer.as_mut_ptr(), buffer.len() as u32) } == 0 {
            let error = HResult::last_error();
            if u32::from(error.code()) == ERROR_NO_MORE_FILES {
                break;
            }
            return Err(error);
        }
        volumes.push(utf16_buffer_to_string(&buffer));
    }
    Ok(volumes)
}

/// Returns mount paths associated with a volume GUID path.
pub fn volume_path_names(volume_name: &str) -> Result<Vec<String>> {
    let volume_name: Vec<_> = volume_name.encode_utf16().chain(Some(0)).collect();
    let mut required = 0_u32;
    // SAFETY: first call intentionally supplies no output to request size.
    unsafe {
        GetVolumePathNamesForVolumeNameW(
            volume_name.as_ptr(),
            std::ptr::null_mut(),
            0,
            &raw mut required,
        )
    };
    if required == 0 {
        return Err(HResult::last_error());
    }
    let mut buffer = vec![0_u16; required as usize];
    // SAFETY: buffer has the requested size and volume_name is NUL-terminated.
    if unsafe {
        GetVolumePathNamesForVolumeNameW(
            volume_name.as_ptr(),
            buffer.as_mut_ptr(),
            required,
            &raw mut required,
        )
    } == 0
    {
        return Err(HResult::last_error());
    }
    crate::data::parse_multi_sz(&buffer)
        .map(|values| values.into_iter().map(String::from_utf16_lossy).collect())
        .map_err(|_| HResult::FAIL)
}

#[cfg(test)]
mod tests {
    use super::{enumerate_files, enumerate_volumes, volume_path_names};

    #[test]
    fn enumerates_current_directory() {
        let values = enumerate_files(".", true).unwrap();
        assert!(values.iter().any(|value| value.name == "Cargo.toml"));
    }

    #[test]
    fn enumerates_volumes_and_mount_paths() {
        let volumes = enumerate_volumes().unwrap();
        assert!(!volumes.is_empty());
        let _ = volume_path_names(&volumes[0]).unwrap();
    }
}
