//! The current folder as the user sees it in the shell.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::derive_error::DeriveError;

/// Return the logical path of the current folder.
///
/// The shell keeps the logical path (the path through symlinks that the user typed) in `PWD`.
/// `pwd` is used only when it is absolute and names the same folder as `current_dir`, so a stale
/// or false `PWD` falls back to the physical `current_dir` (003 research R2).
pub fn resolve(
    pwd: Option<&OsStr>,
    current_dir: io::Result<PathBuf>,
) -> Result<PathBuf, DeriveError> {
    let current_dir = current_dir.map_err(|_| DeriveError::CurrentFolderUnreadable)?;
    match pwd.map(Path::new) {
        Some(pwd) if pwd.is_absolute() && same_folder(pwd, &current_dir) => Ok(pwd.to_owned()),
        _ => Ok(current_dir),
    }
}

fn same_folder(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::os::unix::fs::symlink;

    use super::resolve;
    use crate::session_name::derive_error::DeriveError;

    #[test]
    fn given_pwd_is_a_symlink_to_the_current_folder_when_resolved_then_the_symlink_path_is_used() {
        let root = tempfile::tempdir().expect("a temporary folder");
        let target = root.path().join("target");
        let link = root.path().join("link");
        std::fs::create_dir(&target).expect("the target folder");
        symlink(&target, &link).expect("the symlink");

        let resolved = resolve(Some(link.as_os_str()), Ok(target.clone()));

        assert_eq!(resolved, Ok(link));
    }

    #[test]
    fn given_pwd_names_a_different_folder_when_resolved_then_the_current_folder_is_used() {
        let root = tempfile::tempdir().expect("a temporary folder");
        let current = root.path().join("current");
        let stale = root.path().join("stale");
        std::fs::create_dir(&current).expect("the current folder");
        std::fs::create_dir(&stale).expect("the stale folder");

        let resolved = resolve(Some(stale.as_os_str()), Ok(current.clone()));

        assert_eq!(resolved, Ok(current));
    }

    #[test]
    fn given_pwd_is_relative_when_resolved_then_the_current_folder_is_used() {
        let root = tempfile::tempdir().expect("a temporary folder");

        let resolved = resolve(Some("relative".as_ref()), Ok(root.path().to_owned()));

        assert_eq!(resolved, Ok(root.path().to_owned()));
    }

    #[test]
    fn given_the_current_folder_cannot_be_read_when_resolved_then_it_fails() {
        let resolved = resolve(None, Err(io::Error::other("gone")));

        assert_eq!(resolved, Err(DeriveError::CurrentFolderUnreadable));
    }
}
