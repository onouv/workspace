//! The folder that a tmux session belongs to, and the check against the project folder.

use std::fs;
use std::path::{Path, PathBuf};

/// The folder of one tmux session (003 term "session folder").
///
/// For a session that `ws` made, it is the project folder that `ws` recorded in
/// [`super::session::PROJECT_FOLDER_OPTION`]. For any other session, it is the start directory
/// that tmux records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionFolder {
    path: PathBuf,
}

/// The result of the folder check (003 FR-011).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderCheck {
    /// The session belongs to the project folder.
    Match,
    /// The session belongs to a different folder.
    Mismatch,
    /// A folder cannot be resolved, so ownership cannot be proven.
    Unreadable,
}

impl SessionFolder {
    /// Choose the session folder: the recorded project folder when it is set, otherwise the
    /// tmux start directory. Returns `None` when both are empty.
    pub fn choose(recorded: &str, start: &str) -> Option<Self> {
        let path = if recorded.is_empty() { start } else { recorded };
        (!path.is_empty()).then(|| Self { path: path.into() })
    }

    /// The session folder as tmux reports it.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Compare the session folder with `project_folder` after symlinks are resolved in both.
    pub fn check(&self, project_folder: &Path) -> FolderCheck {
        match (
            fs::canonicalize(&self.path),
            fs::canonicalize(project_folder),
        ) {
            (Ok(session), Ok(project)) if session == project => FolderCheck::Match,
            (Ok(_), Ok(_)) => FolderCheck::Mismatch,
            _ => FolderCheck::Unreadable,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::{FolderCheck, SessionFolder};

    fn folder(path: &std::path::Path) -> SessionFolder {
        SessionFolder::choose(&path.display().to_string(), "").expect("a non-empty folder")
    }

    #[test]
    fn given_the_same_folder_when_checked_then_it_matches() {
        let project = tempfile::tempdir().expect("a temporary folder");

        assert_eq!(
            folder(project.path()).check(project.path()),
            FolderCheck::Match
        );
    }

    #[test]
    fn given_a_different_folder_when_checked_then_it_does_not_match() {
        let project = tempfile::tempdir().expect("a temporary folder");
        let other = tempfile::tempdir().expect("a temporary folder");

        assert_eq!(
            folder(other.path()).check(project.path()),
            FolderCheck::Mismatch
        );
    }

    #[test]
    fn given_a_session_folder_that_does_not_exist_when_checked_then_it_is_unreadable() {
        let project = tempfile::tempdir().expect("a temporary folder");
        let gone = project.path().join("gone");

        assert_eq!(folder(&gone).check(project.path()), FolderCheck::Unreadable);
    }

    #[test]
    fn given_a_symlink_to_the_project_folder_when_checked_then_it_matches() {
        let root = tempfile::tempdir().expect("a temporary folder");
        let target = root.path().join("target");
        let link = root.path().join("link");
        std::fs::create_dir(&target).expect("the target folder");
        symlink(&target, &link).expect("the symlink");

        assert_eq!(folder(&target).check(&link), FolderCheck::Match);
    }

    #[test]
    fn given_a_recorded_folder_and_a_start_directory_when_chosen_then_the_recorded_folder_wins() {
        let chosen = SessionFolder::choose("/project", "/project/sub").expect("a folder");

        assert_eq!(chosen.path(), std::path::Path::new("/project"));
    }

    #[test]
    fn given_no_recorded_folder_when_chosen_then_the_start_directory_is_used() {
        let chosen = SessionFolder::choose("", "/project").expect("a folder");

        assert_eq!(chosen.path(), std::path::Path::new("/project"));
    }

    #[test]
    fn given_no_folder_at_all_when_chosen_then_there_is_none() {
        assert_eq!(SessionFolder::choose("", ""), None);
    }
}
