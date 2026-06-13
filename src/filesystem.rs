//! Cross-platform filesystem operations.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeleteOptions {
    pub allow_relative_path: bool,
}

pub fn delete_file(path: impl AsRef<Path>, options: DeleteOptions) -> io::Result<bool> {
    let path = path.as_ref();
    if !options.allow_relative_path && path.is_relative() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "relative file paths are disabled",
        ));
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[must_use]
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

#[must_use]
pub fn directory_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

#[must_use]
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

#[cfg(test)]
mod tests {
    use super::{DeleteOptions, delete_file, directory_exists, file_exists, temp_dir};
    use std::fs;

    #[test]
    fn checks_and_deletes_files() {
        let directory = temp_dir().join(format!("r-vlr-util-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let file = directory.join("test.txt");
        fs::write(&file, "data").unwrap();
        assert!(directory_exists(&directory));
        assert!(file_exists(&file));
        assert!(delete_file(&file, DeleteOptions::default()).unwrap());
        assert!(!delete_file(&file, DeleteOptions::default()).unwrap());
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn relative_deletion_requires_opt_in() {
        assert!(delete_file("relative-file", DeleteOptions::default()).is_err());
    }
}
