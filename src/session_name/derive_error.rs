//! Reasons why the current folder name cannot become a session name.

use thiserror::Error;

/// The current folder name cannot become a session name (003 FR-004).
///
/// Each message tells the user to give an explicit name instead.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum DeriveError {
    #[error("{} (the current folder has no name){}", PREFIX, SUFFIX)]
    NoFolderName,
    #[error("{} (the folder name is not valid UTF-8 text){}", PREFIX, SUFFIX)]
    NotText,
    #[error("{} (the folder name is blank){}", PREFIX, SUFFIX)]
    Blank,
    #[error("{} (the folder name contains a control character){}", PREFIX, SUFFIX)]
    ControlCharacter,
    #[error("{} (the current folder cannot be read){}", PREFIX, SUFFIX)]
    CurrentFolderUnreadable,
    #[error("{} ({reason}){}", PREFIX, SUFFIX)]
    Rejected { reason: &'static str },
}

const PREFIX: &str = "cannot use the current folder name as a session name";
const SUFFIX: &str = "; run `ws up SESSION_NAME` with an explicit name";
