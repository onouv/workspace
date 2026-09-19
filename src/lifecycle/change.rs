//! Decision flow for `ws change SESSION_NAME`: switch the current tmux client.

use crate::error::AppError;
use crate::terminal::TerminalContext;
use crate::tmux::client::{TmuxClient, TmuxError};
use crate::tmux::session;

/// Switch the current tmux client to `session_name`, requiring both an existing target and an
/// active tmux client (FR-020, FR-021). Never reads or applies `.ws`.
pub fn execute(
    tmux: &TmuxClient,
    terminal: TerminalContext,
    session_name: &str,
) -> Result<String, AppError> {
    if !terminal.inside_tmux() {
        return Err(AppError::OperationFailed {
            message:
                "changing the current tmux client requires running inside tmux; use `ws up` instead"
                    .to_owned(),
        });
    }
    if !session::exists(tmux, session_name).map_err(map_tmux_error)? {
        return Err(AppError::OperationFailed {
            message: format!("no session named `{session_name}` exists"),
        });
    }
    tmux.switch_client(session_name).map_err(map_tmux_error)?;
    Ok(format!("switched to `{session_name}`\n"))
}

fn map_tmux_error(error: TmuxError) -> AppError {
    match error {
        TmuxError::Invocation { source } => AppError::DependencyUnavailable {
            message: format!("could not start or contact tmux: {source}"),
        },
        TmuxError::CommandFailed { status, stderr } => AppError::OperationFailed {
            message: format!("tmux reported a failure (status {status}): {stderr}"),
        },
        TmuxError::InteractiveCommandFailed { status } => AppError::OperationFailed {
            message: format!("tmux reported a failure (status {status})"),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};

    use super::execute;
    use crate::error::AppError;
    use crate::terminal::TerminalContext;
    use crate::tmux::client::{CommandRunner, TmuxClient};

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    fn found_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session") => Ok(Output {
                status: success_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            Some("switch-client") => Ok(Output {
                status: success_status(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            other => panic!("unexpected tmux invocation: {other:?}"),
        }
    }

    fn not_found_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
        Ok(Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: b"can't find session".to_vec(),
        })
    }

    #[test]
    fn given_an_existing_target_when_changed_inside_tmux_then_it_switches_the_client() {
        let tmux = TmuxClient::with_runner(found_runner as CommandRunner);

        let result = execute(&tmux, TerminalContext::inside_tmux_for_test(), "target");

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_a_missing_target_when_changed_then_it_returns_a_recoverable_error() {
        let tmux = TmuxClient::with_runner(not_found_runner as CommandRunner);

        let error = execute(&tmux, TerminalContext::inside_tmux_for_test(), "target")
            .expect_err("a missing target should be a recoverable error");

        assert!(matches!(error, AppError::OperationFailed { .. }));
    }

    #[test]
    fn given_outside_tmux_when_changed_then_it_returns_a_recoverable_error_without_a_probe() {
        fn panicking_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            panic!("tmux should not be contacted outside tmux: {args:?}")
        }
        let tmux = TmuxClient::with_runner(panicking_runner as CommandRunner);

        let error = execute(&tmux, TerminalContext::outside_tmux_for_test(), "target")
            .expect_err("changing outside tmux should be a recoverable error");

        assert!(matches!(error, AppError::OperationFailed { .. }));
    }
}
