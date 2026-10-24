//! One-line stderr texts of the setup-file step (002 FR-007, FR-008, FR-009).

use std::fmt;
use std::io;
use std::path::PathBuf;

/// The file message or warning that `ws up` shows after the setup-file step.
///
/// The texts are the contract in `specs/002-default/contracts/cli.md`, without the `ws: ` prefix
/// that the notifier adds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetupNotice {
    /// `ws` created a missing `.ws` (the file message, FR-007).
    Created { path: PathBuf },
    /// `ws` replaced the content of an empty `.ws` (FR-008).
    Overwritten { path: PathBuf },
    /// `ws` could not create or overwrite `.ws` (FR-009). Only the error kind is shown, so the
    /// line never contains file content.
    WriteFailed { path: PathBuf, cause: io::ErrorKind },
    /// `.ws` is a symlink to an empty file, which `ws` never writes through.
    EmptySymlink { path: PathBuf },
}

impl fmt::Display for SetupNotice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { path } => write!(
                formatter,
                "created {} with the default workspace",
                path.display()
            ),
            Self::Overwritten { path } => write!(
                formatter,
                "warning: replaced the empty content of {} with the default workspace",
                path.display()
            ),
            Self::WriteFailed { path, cause } => write!(
                formatter,
                "warning: could not write {} ({cause}); using the default workspace without a file",
                path.display()
            ),
            Self::EmptySymlink { path } => write!(
                formatter,
                "warning: {} is a link to an empty file; using the default workspace without a change",
                path.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use super::SetupNotice;

    fn path() -> PathBuf {
        PathBuf::from("/tmp/alpha/.ws")
    }

    #[test]
    fn given_a_created_file_when_displayed_then_it_is_the_file_message() {
        let notice = SetupNotice::Created { path: path() };

        assert_eq!(
            notice.to_string(),
            "created /tmp/alpha/.ws with the default workspace"
        );
    }

    #[test]
    fn given_an_overwritten_file_when_displayed_then_it_is_the_overwrite_warning() {
        let notice = SetupNotice::Overwritten { path: path() };

        assert_eq!(
            notice.to_string(),
            "warning: replaced the empty content of /tmp/alpha/.ws with the default workspace"
        );
    }

    #[test]
    fn given_a_failed_write_when_displayed_then_it_is_the_warning_with_the_error_kind() {
        let notice = SetupNotice::WriteFailed {
            path: path(),
            cause: io::ErrorKind::PermissionDenied,
        };

        assert_eq!(
            notice.to_string(),
            "warning: could not write /tmp/alpha/.ws (permission denied); using the default workspace without a file"
        );
    }

    #[test]
    fn given_a_link_to_an_empty_file_when_displayed_then_it_is_the_symlink_warning() {
        let notice = SetupNotice::EmptySymlink { path: path() };

        assert_eq!(
            notice.to_string(),
            "warning: /tmp/alpha/.ws is a link to an empty file; using the default workspace without a change"
        );
    }
}
