//! A session name made from the name of the current folder.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::derive_error::DeriveError;

/// A valid session name made from the last component of the project folder (003 FR-002).
///
/// Construction applies every rule, so each instance is a valid session name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedSessionName {
    name: String,
    project_folder: PathBuf,
}

impl DerivedSessionName {
    /// Make the session name from the last component of `project_folder`.
    pub fn derive(project_folder: &Path) -> Result<Self, DeriveError> {
        let folder_name = project_folder
            .file_name()
            .ok_or(DeriveError::NoFolderName)?;
        Ok(Self {
            name: name_from(folder_name)?,
            project_folder: project_folder.to_owned(),
        })
    }

    /// The derived session name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The project folder that the name comes from. The folder check compares the session
    /// folder with it (003 FR-011).
    pub fn project_folder(&self) -> &Path {
        &self.project_folder
    }
}

/// Apply the derivation rules to one folder name (003 research R3).
fn name_from(folder_name: &OsStr) -> Result<String, DeriveError> {
    let text = folder_name.to_str().ok_or(DeriveError::NotText)?;
    if text.trim().is_empty() {
        return Err(DeriveError::Blank);
    }
    if text.chars().any(char::is_control) {
        return Err(DeriveError::ControlCharacter);
    }
    // tmux reserves `.` and `:` in targets; tmux itself replaces them with `_` (003 FR-005).
    let name = text.replace(['.', ':'], "_");
    super::check(&name).map_err(|reason| DeriveError::Rejected { reason })?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DerivedSessionName;
    use crate::session_name::derive_error::DeriveError;

    fn derived_name(path: &str) -> Result<String, DeriveError> {
        DerivedSessionName::derive(Path::new(path)).map(|derived| derived.name().to_owned())
    }

    #[test]
    fn given_a_plain_folder_name_when_derived_then_the_name_is_the_folder_name() {
        assert_eq!(derived_name("/tmp/work/alpha"), Ok("alpha".to_owned()));
    }

    #[test]
    fn given_a_folder_name_with_a_dot_when_derived_then_the_dot_becomes_an_underscore() {
        assert_eq!(derived_name("/tmp/my.project"), Ok("my_project".to_owned()));
    }

    #[test]
    fn given_a_folder_name_with_a_colon_when_derived_then_the_colon_becomes_an_underscore() {
        assert_eq!(derived_name("/tmp/a:b"), Ok("a_b".to_owned()));
    }

    #[test]
    fn given_a_folder_name_with_spaces_when_derived_then_the_spaces_are_kept() {
        assert_eq!(
            derived_name("/tmp/ my project "),
            Ok(" my project ".to_owned())
        );
    }

    #[test]
    fn given_the_filesystem_root_when_derived_then_it_fails_with_no_folder_name() {
        assert_eq!(derived_name("/"), Err(DeriveError::NoFolderName));
    }

    #[test]
    fn given_a_blank_folder_name_when_derived_then_it_fails_as_blank() {
        assert_eq!(derived_name("/tmp/   "), Err(DeriveError::Blank));
    }

    #[test]
    fn given_a_folder_name_with_a_control_character_when_derived_then_it_fails() {
        assert_eq!(
            derived_name("/tmp/a\tb"),
            Err(DeriveError::ControlCharacter)
        );
    }

    #[test]
    fn given_a_folder_name_that_is_not_utf8_when_derived_then_it_fails_as_not_text() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new("/tmp").join(OsStr::from_bytes(b"bad\xff"));

        assert_eq!(DerivedSessionName::derive(&path), Err(DeriveError::NotText));
    }
}
