//! Decision flow for `ws exit`: detach the current tmux client without killing its session.

use crate::error::AppError;
use crate::terminal::TerminalContext;
use crate::tmux::client::{TmuxClient, TmuxError};

/// Detach the current tmux client (FR-022). Fails clearly when no client is attached.
pub fn execute(tmux: &TmuxClient, terminal: TerminalContext) -> Result<String, AppError> {
    if !terminal.inside_tmux() {
        return Err(AppError::OperationFailed {
            message: "ws exit requires an attached tmux client".to_owned(),
        });
    }
    tmux.detach_client().map_err(|error| match error {
        TmuxError::Invocation { source } => AppError::DependencyUnavailable {
            message: format!("could not start or contact tmux: {source}"),
        },
        TmuxError::CommandFailed { status, stderr } => AppError::OperationFailed {
            message: format!("tmux reported a failure (status {status}): {stderr}"),
        },
        TmuxError::InteractiveCommandFailed { status } => AppError::OperationFailed {
            message: format!("tmux reported a failure (status {status})"),
        },
    })?;
    Ok(String::new())
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

    fn detach_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        assert_eq!(args, [OsString::from("detach-client")]);
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    #[test]
    fn given_inside_tmux_when_exit_runs_then_it_detaches_the_client() {
        let tmux = TmuxClient::with_runner(detach_runner as CommandRunner);

        let result = execute(&tmux, TerminalContext::inside_tmux_for_test());

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_outside_tmux_when_exit_runs_then_it_fails_without_contacting_tmux() {
        fn panicking_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            panic!("tmux should not be contacted outside tmux: {args:?}")
        }
        let tmux = TmuxClient::with_runner(panicking_runner as CommandRunner);

        let error = execute(&tmux, TerminalContext::outside_tmux_for_test())
            .expect_err("exit outside tmux should fail clearly");

        assert!(matches!(error, AppError::OperationFailed { .. }));
    }
}
