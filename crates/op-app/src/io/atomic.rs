//! Stage a native save beside its destination before replacing the old file.
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace_with(path, |file| file.write_all(bytes))
}

fn replace_with(path: &Path, write: impl FnOnce(&mut fs::File) -> io::Result<()>) -> io::Result<()> {
    // Preserve the target of a user-selected symlink instead of replacing the link.
    let resolved;
    let path = if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        resolved = path.canonicalize()?;
        resolved.as_path()
    } else {
        path
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = match fs::metadata(path) {
        Ok(m) if !m.is_file() || m.permissions().readonly() => {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                op_core::i18n::tr("file.not_writable"),
            ));
        }
        Ok(m) => Some(m),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    if let Some(metadata) = metadata {
        temporary.as_file().set_permissions(metadata.permissions())?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_write_preserves_original_and_removes_staging_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.opx");
        fs::write(&path, b"original").unwrap();
        let result = replace_with(&path, |file| {
            file.write_all(b"incomplete")?;
            Err(io::Error::other("simulated write interruption"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        write(&path, b"replacement").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        write(&directory.path().join("new.opx"), b"new document").unwrap();
    }

    #[test]
    fn failed_replacement_preserves_destination_and_cleans_up() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.opx");
        fs::write(&path, b"original").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        assert!(write(&path, b"replacement").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert!(write(directory.path(), b"replacement").is_err());
        assert!(write(&directory.path().join("missing/file.opx"), b"replacement").is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        // Windows cannot remove a read-only file during temporary-directory cleanup.
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o600);
        }
        #[cfg(not(unix))]
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_permissions_and_symlink_target() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.opx");
        let link = directory.path().join("link.opx");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&path, &link).unwrap();
        write(&link, b"replacement").unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o640);
    }
}
