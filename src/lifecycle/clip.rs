//! Decision flow for `ws clip NAMESPACE ITEM`: resolve one configured credential and send it to
//! the clipboard, on demand and without ever printing it (FR-025 through FR-028, FR-039 through
//! FR-044).

use crate::credentials::clip_mapping::{ClipEntry, ClipMapping};
use crate::credentials::clipboard::{ClipboardError, ClipboardProvider};
use crate::credentials::pass_provider::{CredentialError, PassProvider};
use crate::error::AppError;

/// Resolve `namespace`/`item` against the already-loaded, merged mapping and send the selected
/// credential to the clipboard.
///
/// An entry absent from `mapping` is rejected before either `pass` or the clipboard provider is
/// contacted (FR-041, US8-AS2, US9-AS4). A `pass`-backed entry requests exactly that one entry; a
/// `literal` entry never contacts `pass` at all (US9-AS1, US9-AS2).
pub fn execute(
    pass: &PassProvider,
    clipboard: &dyn ClipboardProvider,
    mapping: &ClipMapping,
    namespace: &str,
    item: &str,
) -> Result<String, AppError> {
    let entry = mapping
        .resolve(namespace, item)
        .ok_or_else(|| AppError::OperationFailed {
            message: format!(
                "no clip entry configured for `{namespace} {item}`; see `ws help clip`"
            ),
        })?;

    let value = match entry {
        ClipEntry::Literal(value) => value.clone(),
        ClipEntry::Pass(path) => pass
            .reveal(path)
            .map_err(map_credential_error)?
            .expose()
            .to_owned(),
    };

    clipboard.copy(&value).map_err(map_clipboard_error)?;
    Ok(format!("copied `{namespace} {item}` to the clipboard\n"))
}

fn map_credential_error(error: CredentialError) -> AppError {
    match error {
        CredentialError::Invocation { source } => AppError::DependencyUnavailable {
            message: format!("could not start or contact pass: {source}"),
        },
        CredentialError::Unavailable => AppError::CredentialFailure {
            message: "pass could not provide the requested entry; is the store unlocked?"
                .to_owned(),
        },
    }
}

fn map_clipboard_error(error: ClipboardError) -> AppError {
    match error {
        ClipboardError::Invocation { source } => AppError::DependencyUnavailable {
            message: format!("could not start the clipboard provider: {source}"),
        },
        ClipboardError::Failed => AppError::OperationFailed {
            message: "the clipboard provider did not accept the value".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};

    use super::execute;
    use crate::credentials::clip_mapping::{ClipEntry, ClipMapping};
    use crate::credentials::clipboard::{ClipboardError, ClipboardProvider};
    use crate::credentials::pass_provider::{CommandRunner, PassProvider};
    use crate::error::AppError;

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    struct RecordingClipboard {
        copied: std::sync::Mutex<Option<String>>,
    }

    impl RecordingClipboard {
        fn new() -> Self {
            Self {
                copied: std::sync::Mutex::new(None),
            }
        }

        fn copied_value(&self) -> Option<String> {
            self.copied
                .lock()
                .expect("the recording mutex should not be poisoned")
                .clone()
        }
    }

    impl ClipboardProvider for RecordingClipboard {
        fn copy(&self, value: &str) -> Result<(), ClipboardError> {
            *self
                .copied
                .lock()
                .expect("the recording mutex should not be poisoned") = Some(value.to_owned());
            Ok(())
        }
    }

    struct RefusingClipboard;
    impl ClipboardProvider for RefusingClipboard {
        fn copy(&self, _: &str) -> Result<(), ClipboardError> {
            Err(ClipboardError::Failed)
        }
    }

    fn panicking_pass_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        panic!("pass should not be contacted for a literal entry: {args:?}")
    }

    #[test]
    fn given_a_pass_backed_entry_when_clip_runs_then_only_that_entry_is_requested_and_copied() {
        // US8-AS1 and US9-AS1 are owned end-to-end by tests/clip.rs; this unit test isolates the
        // same behavior from process/filesystem concerns.
        fn asserting_pass_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            assert_eq!(
                args,
                [OsString::from("show"), OsString::from("repos/github/ssh")]
            );
            Ok(Output {
                status: success_status(),
                stdout: b"the-ssh-passphrase\n".to_vec(),
                stderr: Vec::new(),
            })
        }
        let pass = PassProvider::with_runner(asserting_pass_runner as CommandRunner);
        let clipboard = RecordingClipboard::new();
        let mapping = ClipMapping::from_entries([(
            ("git".to_owned(), "ssh".to_owned()),
            ClipEntry::Pass("repos/github/ssh".to_owned()),
        )]);

        execute(&pass, &clipboard, &mapping, "git", "ssh")
            .expect("a configured entry should succeed");

        assert_eq!(
            clipboard.copied_value().as_deref(),
            Some("the-ssh-passphrase")
        );
    }

    #[test]
    fn given_a_literal_entry_when_clip_runs_then_pass_is_never_contacted() {
        // US9-AS2 is owned end-to-end by tests/clip.rs; this unit test isolates the same
        // behavior from process/filesystem concerns.
        let pass = PassProvider::with_runner(panicking_pass_runner as CommandRunner);
        let clipboard = RecordingClipboard::new();
        let mapping = ClipMapping::from_entries([(
            ("git".to_owned(), "user".to_owned()),
            ClipEntry::Literal("octocat".to_owned()),
        )]);

        execute(&pass, &clipboard, &mapping, "git", "user")
            .expect("a literal entry should succeed");

        assert_eq!(clipboard.copied_value().as_deref(), Some("octocat"));
    }

    #[test]
    fn given_an_unconfigured_entry_when_clip_runs_then_it_is_rejected_without_contacting_dependencies()
     {
        // US8-AS2 and US9-AS4 are owned end-to-end by tests/clip.rs; this unit test isolates
        // the same behavior from process/filesystem concerns.
        let pass = PassProvider::with_runner(panicking_pass_runner as CommandRunner);
        let clipboard = RefusingClipboard;
        let mapping = ClipMapping::from_entries([]);

        let error = execute(&pass, &clipboard, &mapping, "git", "ssh")
            .expect_err("an unconfigured entry should be rejected");

        assert!(matches!(error, AppError::OperationFailed { .. }));
    }

    #[test]
    fn given_pass_is_locked_when_clip_runs_then_it_reports_a_credential_failure_without_secret_detail()
     {
        // US8-AS3 is owned by src/credentials/pass_provider.rs's own test; this exercises the
        // same behavior end-to-end through `ws clip`'s error mapping.
        fn locked_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
            Ok(Output {
                status: failure_status(),
                stdout: Vec::new(),
                stderr: b"gpg: decryption failed: No secret key".to_vec(),
            })
        }
        let pass = PassProvider::with_runner(locked_runner as CommandRunner);
        let clipboard = RecordingClipboard::new();
        let mapping = ClipMapping::from_entries([(
            ("git".to_owned(), "ssh".to_owned()),
            ClipEntry::Pass("repos/github/ssh".to_owned()),
        )]);

        let error = execute(&pass, &clipboard, &mapping, "git", "ssh")
            .expect_err("a locked store should fail");

        assert!(matches!(error, AppError::CredentialFailure { .. }));
        assert!(!error.to_string().contains("secret key"));
        assert!(clipboard.copied_value().is_none());
    }
}
