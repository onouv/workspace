//! Prepare the `.ws` setup file before `ws up` creates a session (002).

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process;

use ws::config::DEFAULT_CONTENT;

use crate::lifecycle::prepared_setup::PreparedSetup;
use crate::lifecycle::setup_notice::SetupNotice;
use crate::lifecycle::setup_source::SetupSource;

/// The name of the setup file in the project folder.
const SETUP_FILE_NAME: &str = ".ws";

/// The state of `.ws` when `ws up` creates a session (002 research R2).
enum SetupFileState {
    /// No filesystem entry has the name `.ws`.
    Missing,
    /// A regular file with whitespace-only content, and its metadata at classification time.
    Empty(fs::Metadata),
    /// A symlink to a whitespace-only file. `ws` never writes through a symlink.
    EmptyThroughSymlink,
    /// Any other entry. The normal loader reads it and reports a problem (001 FR-015).
    Present,
}

/// Why an exclusive create did not make `.ws`.
enum CreateError {
    /// A second process created `.ws` first (FR-005).
    LostRace,
    /// The folder or file system refused the write (FR-009).
    Failed(io::Error),
}

/// Make sure that `project_dir/.ws` can give the workspace of a new session.
///
/// Only `ws up` calls this, after it found that the session does not exist (FR-006). Each call
/// makes at most one create and one overwrite attempt, and never retries (constitution IX).
pub fn prepare(project_dir: &Path) -> PreparedSetup {
    let path = project_dir.join(SETUP_FILE_NAME);
    match classify(&path) {
        SetupFileState::Missing => create_missing(project_dir, path),
        state => use_existing(path, state),
    }
}

/// Classify `.ws` without following a symlink, so that a broken symlink counts as present.
fn classify(path: &Path) -> SetupFileState {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return SetupFileState::Missing,
        Err(_) => return SetupFileState::Present,
    };
    let is_blank = || fs::read_to_string(path).is_ok_and(|content| content.trim().is_empty());
    if metadata.file_type().is_symlink() {
        if is_blank() {
            SetupFileState::EmptyThroughSymlink
        } else {
            SetupFileState::Present
        }
    } else if metadata.is_file() && is_blank() {
        SetupFileState::Empty(metadata)
    } else {
        SetupFileState::Present
    }
}

/// Act on a `.ws` that exists: overwrite it if it is empty, else load it as it is.
fn use_existing(path: PathBuf, state: SetupFileState) -> PreparedSetup {
    match state {
        SetupFileState::Empty(metadata) => overwrite_empty(path, &metadata),
        SetupFileState::EmptyThroughSymlink => {
            let notice = SetupNotice::EmptySymlink { path: path.clone() };
            in_memory_default(path, notice)
        }
        // A `.ws` that disappeared after a lost race also goes to the loader, which reports it.
        SetupFileState::Missing | SetupFileState::Present => from_file(path),
    }
}

/// Create a missing `.ws` with the default content (FR-001, FR-005, FR-009).
fn create_missing(project_dir: &Path, path: PathBuf) -> PreparedSetup {
    match create_exclusively(project_dir, &path) {
        Ok(()) => {
            let notice = SetupNotice::Created { path: path.clone() };
            PreparedSetup::new(SetupSource::File(path), Some(notice))
        }
        // The file of the second process wins. If it is empty, FR-008 applies (FR-005).
        Err(CreateError::LostRace) => {
            let state = classify(&path);
            use_existing(path, state)
        }
        Err(CreateError::Failed(error)) => write_failed(path, &error),
    }
}

/// Make `.ws` appear with its full content in one step (research R3).
///
/// The content goes to a temporary file first. `hard_link` then fails if `.ws` exists, also as a
/// broken symlink, so a second process never reads a partial file. The temporary file is removed
/// in all cases (FR-012). If the link fails for another reason, for example on a file system
/// without hard links, a direct exclusive create is the fallback.
fn create_exclusively(project_dir: &Path, path: &Path) -> Result<(), CreateError> {
    let temporary = project_dir.join(format!("{SETUP_FILE_NAME}.{}.tmp", process::id()));
    write_new_file(&temporary).map_err(CreateError::Failed)?;
    let linked = fs::hard_link(&temporary, path);
    let _ = fs::remove_file(&temporary);
    match linked {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(CreateError::LostRace),
        Err(_) => write_new_file(path).map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => CreateError::LostRace,
            _ => CreateError::Failed(error),
        }),
    }
}

/// Create `path`, which must not exist, with the default content.
fn write_new_file(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(DEFAULT_CONTENT.as_bytes())
}

/// Replace the content of an empty `.ws` with the default content (FR-008, research R4).
///
/// `expected` is the metadata from classification. A different file entry or new content means
/// that a second writer was faster: `ws` then loads the file as it is, without a write.
fn overwrite_empty(path: PathBuf, expected: &fs::Metadata) -> PreparedSetup {
    match overwrite_if_still_empty(&path, expected) {
        Ok(true) => {
            let notice = SetupNotice::Overwritten { path: path.clone() };
            PreparedSetup::new(SetupSource::File(path), Some(notice))
        }
        Ok(false) => from_file(path),
        Err(error) => write_failed(path, &error),
    }
}

/// Write the default content into `path` only if it is still the same, still empty file.
///
/// The check reads through the same open file immediately before the write. POSIX has no atomic
/// "write if empty" operation, so a save in the few system calls between that read and the write
/// can still be lost. The spec accepts this window (002 FR-008, Assumptions).
fn overwrite_if_still_empty(path: &Path, expected: &fs::Metadata) -> io::Result<bool> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    if !is_same_entry(&file.metadata()?, expected) {
        return Ok(false);
    }
    let mut content = String::new();
    if file.read_to_string(&mut content).is_err() || !content.trim().is_empty() {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(0))?;
    file.write_all(DEFAULT_CONTENT.as_bytes())?;
    file.set_len(DEFAULT_CONTENT.len() as u64)?;
    Ok(true)
}

/// True when both metadata values describe the same file entry. `open` follows a symlink, so a
/// `.ws` that a second writer replaced with a symlink gives a different entry here.
#[cfg(unix)]
fn is_same_entry(actual: &fs::Metadata, expected: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    actual.dev() == expected.dev() && actual.ino() == expected.ino()
}

/// Without inode numbers, only the regular-file check of classification applies.
#[cfg(not(unix))]
fn is_same_entry(actual: &fs::Metadata, _expected: &fs::Metadata) -> bool {
    actual.is_file()
}

fn from_file(path: PathBuf) -> PreparedSetup {
    PreparedSetup::new(SetupSource::File(path), None)
}

fn in_memory_default(path: PathBuf, notice: SetupNotice) -> PreparedSetup {
    PreparedSetup::new(
        SetupSource::InMemoryDefault { source_path: path },
        Some(notice),
    )
}

/// The write failed for a reason that is not a second writer (FR-009). Only the error kind goes
/// into the warning, so it contains no sensitive data.
fn write_failed(path: PathBuf, error: &io::Error) -> PreparedSetup {
    let notice = SetupNotice::WriteFailed {
        path: path.clone(),
        cause: error.kind(),
    };
    in_memory_default(path, notice)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;
    use ws::config::DEFAULT_CONTENT;

    use super::{create_missing, overwrite_empty, prepare};
    use crate::lifecycle::setup_notice::SetupNotice;
    use crate::lifecycle::setup_source::SetupSource;

    #[test]
    fn given_no_ws_file_when_prepared_then_the_source_is_the_new_file() {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = project.path().join(".ws");

        let prepared = prepare(project.path());

        assert_eq!(prepared.source(), &SetupSource::File(path));
    }

    #[test]
    fn given_no_ws_file_when_prepared_then_the_notice_is_the_file_message() {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = project.path().join(".ws");

        let prepared = prepare(project.path());

        assert_eq!(prepared.notice(), Some(&SetupNotice::Created { path }));
    }

    #[test]
    fn given_no_ws_file_when_prepared_then_the_file_has_exactly_the_default_content() {
        let project = tempdir().expect("a temporary project folder should be available");

        prepare(project.path());

        let content = fs::read_to_string(project.path().join(".ws"))
            .expect("the created .ws should be readable");
        assert_eq!(content, DEFAULT_CONTENT);
    }

    #[test]
    fn given_no_ws_file_when_prepared_then_only_ws_is_left_in_the_folder() {
        let project = tempdir().expect("a temporary project folder should be available");

        prepare(project.path());

        let names: Vec<_> = fs::read_dir(project.path())
            .expect("the project folder should be readable")
            .map(|entry| {
                entry
                    .expect("a folder entry should be readable")
                    .file_name()
            })
            .collect();
        assert_eq!(names, [".ws"]);
    }

    fn write_ws(project: &Path, content: &str) -> PathBuf {
        let path = project.join(".ws");
        fs::write(&path, content).expect("the .ws fixture should be writable");
        path
    }

    /// True when the test runs with privileges that ignore mode bits (for example, as root).
    fn mode_bits_are_ignored(folder: &Path) -> bool {
        let probe = folder.join("probe");
        let ignored = fs::write(&probe, "").is_ok();
        let _ = fs::remove_file(probe);
        ignored
    }

    fn make_read_only(path: &Path) {
        let mut permissions = fs::metadata(path)
            .expect("the path should exist")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions).expect("the mode should be settable");
    }

    #[test]
    fn given_an_empty_ws_when_prepared_then_the_notice_is_the_overwrite_warning() {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "");

        let prepared = prepare(project.path());

        assert_eq!(prepared.notice(), Some(&SetupNotice::Overwritten { path }));
    }

    #[test]
    fn given_a_whitespace_only_ws_when_prepared_then_it_has_exactly_the_default_content() {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), " \n\t\n   ");

        prepare(project.path());

        assert_eq!(
            fs::read_to_string(path).expect(".ws should be readable"),
            DEFAULT_CONTENT
        );
    }

    #[test]
    fn given_an_empty_ws_that_gets_content_before_the_write_when_overwritten_then_the_content_is_kept()
     {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "");
        let metadata = fs::symlink_metadata(&path).expect("the .ws metadata should be readable");
        fs::write(&path, "version: 1\n").expect("the second writer should write");

        let prepared = overwrite_empty(path.clone(), &metadata);

        assert_eq!(
            (
                prepared.notice(),
                fs::read_to_string(&path).expect(".ws should be readable")
            ),
            (None, "version: 1\n".to_owned())
        );
    }

    #[test]
    fn given_an_empty_ws_whose_entry_is_replaced_before_the_write_when_overwritten_then_nothing_is_written()
     {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "");
        let metadata = fs::symlink_metadata(&path).expect("the .ws metadata should be readable");
        // Keep the old entry alive under another name, so the new `.ws` cannot reuse its inode.
        fs::rename(&path, project.path().join("old.ws")).expect("the old entry should move");
        write_ws(project.path(), "");

        let prepared = overwrite_empty(path.clone(), &metadata);

        assert_eq!(
            (
                prepared.notice(),
                fs::read_to_string(&path).expect(".ws should be readable")
            ),
            (None, String::new())
        );
    }

    #[test]
    fn given_another_writer_created_ws_when_create_missing_runs_then_the_other_content_is_kept() {
        // Scenario: US3-AS4
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "version: 1\nwindows: []\n");
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .unwrap();

        let prepared = create_missing(project.path(), path.clone());

        assert_eq!(
            (
                prepared.source(),
                prepared.notice(),
                fs::read_to_string(&path).expect(".ws should be readable"),
                fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .unwrap(),
            ),
            (
                &SetupSource::File(path.clone()),
                None,
                "version: 1\nwindows: []\n".to_owned(),
                modified
            )
        );
    }

    #[test]
    fn given_another_writer_created_an_empty_ws_when_create_missing_runs_then_ws_overwrites_it() {
        // 002 FR-005: "FR-008 applies if that file is empty".
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "");

        let prepared = create_missing(project.path(), path.clone());

        assert_eq!(prepared.notice(), Some(&SetupNotice::Overwritten { path }));
    }

    #[test]
    fn given_a_read_only_folder_and_no_ws_file_when_prepared_then_the_default_comes_from_memory_with_a_warning()
     {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = project.path().join(".ws");
        make_read_only(project.path());
        if mode_bits_are_ignored(project.path()) {
            eprintln!("skipped: mode bits are ignored for this user");
            return;
        }

        let prepared = prepare(project.path());

        assert_eq!(
            (prepared.source(), prepared.notice(), path.exists()),
            (
                &SetupSource::InMemoryDefault {
                    source_path: path.clone()
                },
                Some(&SetupNotice::WriteFailed {
                    path: path.clone(),
                    cause: io::ErrorKind::PermissionDenied
                }),
                false
            )
        );
    }

    #[test]
    fn given_a_read_only_empty_ws_when_prepared_then_the_file_does_not_change_and_the_default_comes_from_memory()
     {
        let project = tempdir().expect("a temporary project folder should be available");
        let path = write_ws(project.path(), "");
        make_read_only(&path);
        if fs::OpenOptions::new().write(true).open(&path).is_ok() {
            eprintln!("skipped: mode bits are ignored for this user");
            return;
        }

        let prepared = prepare(project.path());

        assert_eq!(
            (
                prepared.source(),
                fs::read_to_string(&path).expect(".ws should be readable")
            ),
            (
                &SetupSource::InMemoryDefault {
                    source_path: path.clone()
                },
                String::new()
            )
        );
    }

    #[test]
    fn given_ws_is_a_link_to_an_empty_file_when_prepared_then_the_target_does_not_change_and_a_warning_follows()
     {
        let project = tempdir().expect("a temporary project folder should be available");
        let target = project.path().join("empty.yaml");
        fs::write(&target, "").expect("the link target should be writable");
        let path = project.path().join(".ws");
        std::os::unix::fs::symlink(&target, &path).expect("the .ws link should be creatable");

        let prepared = prepare(project.path());

        assert_eq!(
            (
                prepared.source(),
                prepared.notice(),
                fs::read_to_string(&target).expect("the target should be readable")
            ),
            (
                &SetupSource::InMemoryDefault {
                    source_path: path.clone()
                },
                Some(&SetupNotice::EmptySymlink { path }),
                String::new()
            )
        );
    }
}
