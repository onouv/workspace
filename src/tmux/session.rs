//! Tmux session probing and creation, using the [`TmuxClient`] argv boundary.

use std::path::Path;

use super::client::{TmuxClient, TmuxError};

/// Return whether a session named `name` currently exists.
///
/// A `has-session` failure is treated as "does not exist" rather than propagated, since that is
/// tmux's normal way of reporting a missing session; a failure to invoke tmux at all (for
/// example, because it is not installed) still propagates as [`TmuxError::Invocation`].
pub fn exists(tmux: &TmuxClient, name: &str) -> Result<bool, TmuxError> {
    match tmux.execute(["has-session", "-t", name]) {
        Ok(_) => Ok(true),
        Err(TmuxError::CommandFailed { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Create a new detached session named `name`, rooted at `path`, with one window named
/// `window_name`.
///
/// The session is created detached (`-d`): this call never attaches the current process, so the
/// caller decides how to connect afterward (direct attach outside tmux, or a separate terminal
/// launcher inside tmux).
pub fn create(
    tmux: &TmuxClient,
    name: &str,
    path: &Path,
    window_name: &str,
) -> Result<(), TmuxError> {
    tmux.execute([
        "new-session",
        "-d",
        "-s",
        name,
        "-c",
        &path.to_string_lossy(),
        "-n",
        window_name,
    ])?;
    Ok(())
}

/// Return whether `error` is tmux's report that a session by the requested name already exists.
///
/// Used to recover from the race described in the specification: if another invocation created
/// the target session between our existence probe and our creation attempt, the losing
/// invocation reconnects instead of failing.
pub fn is_duplicate_session_error(error: &TmuxError) -> bool {
    matches!(
        error,
        TmuxError::CommandFailed { stderr, .. } if stderr.contains("duplicate session")
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::Path;
    use std::process::{ExitStatus, Output};

    use super::{create, exists, is_duplicate_session_error};
    use crate::tmux::client::{CommandRunner, TmuxClient, TmuxError};

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    fn found_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn not_found_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
        Ok(Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: b"can't find session: target".to_vec(),
        })
    }

    fn duplicate_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
        Ok(Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: b"duplicate session: target".to_vec(),
        })
    }

    #[test]
    fn given_a_found_session_when_checked_then_it_exists() {
        let client = TmuxClient::with_runner(found_runner as CommandRunner);
        assert!(exists(&client, "target").expect("the probe should succeed"));
    }

    #[test]
    fn given_no_matching_session_when_checked_then_it_does_not_exist() {
        let client = TmuxClient::with_runner(not_found_runner as CommandRunner);
        assert!(!exists(&client, "target").expect("the probe should succeed"));
    }

    #[test]
    fn given_creation_arguments_when_created_then_the_argv_carries_name_path_and_window() {
        fn asserting_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            assert_eq!(
                args,
                [
                    OsString::from("new-session"),
                    OsString::from("-d"),
                    OsString::from("-s"),
                    OsString::from("target"),
                    OsString::from("-c"),
                    OsString::from("/tmp/project"),
                    OsString::from("-n"),
                    OsString::from("root"),
                ]
            );
            Ok(Output {
                status: success_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
        let client = TmuxClient::with_runner(asserting_runner as CommandRunner);
        create(&client, "target", Path::new("/tmp/project"), "root")
            .expect("creation should succeed");
    }

    #[test]
    fn given_a_duplicate_session_error_when_classified_then_it_is_recognized_as_a_race() {
        let client = TmuxClient::with_runner(duplicate_runner as CommandRunner);
        let error = create(&client, "target", Path::new("/tmp/project"), "root")
            .expect_err("the fake duplicate response should surface as an error");
        assert!(is_duplicate_session_error(&error));
    }

    #[test]
    fn given_an_unrelated_failure_when_classified_then_it_is_not_a_race() {
        let error = TmuxError::CommandFailed {
            status: "1".to_owned(),
            stderr: "no such file or directory".to_owned(),
        };
        assert!(!is_duplicate_session_error(&error));
    }
}
