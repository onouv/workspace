//! Tmux session probing, using the [`TmuxClient`] argv boundary.
//!
//! Session creation lives in [`super::window::materialize`], which creates the session together
//! with its first window (`new-session` inherently creates exactly one window).

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

/// Terminate the session named `name`.
///
/// Used to roll back a session this invocation partially created when a later step in its
/// launch plan fails, so setup failures never leave a half-built session behind.
pub fn kill(tmux: &TmuxClient, name: &str) -> Result<(), TmuxError> {
    tmux.execute(["kill-session", "-t", name])?;
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
    use std::process::{ExitStatus, Output};

    use super::{exists, is_duplicate_session_error};
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
    fn given_a_duplicate_session_error_when_classified_then_it_is_recognized_as_a_race() {
        let error = TmuxError::CommandFailed {
            status: "1".to_owned(),
            stderr: "duplicate session: target".to_owned(),
        };
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
