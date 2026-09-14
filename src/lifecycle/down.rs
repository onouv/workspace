//! Decision flow for `ws down [SESSION_NAME] [--yes]`: terminate a workspace session after
//! confirmation.

use crate::error::AppError;
use crate::terminal::TerminalContext;
use crate::terminal::confirm::Confirm;
use crate::tmux::client::{TmuxClient, TmuxError};
use crate::tmux::session;

/// Terminate a named or (inside tmux only) current session, after interactive confirmation or an
/// explicit `--yes` (FR-023, FR-024).
pub fn execute(
    tmux: &TmuxClient,
    terminal: TerminalContext,
    confirm: &dyn Confirm,
    session_name: Option<&str>,
    yes: bool,
) -> Result<String, AppError> {
    let target = resolve_target(tmux, terminal, session_name)?;

    if !session::exists(tmux, &target).map_err(map_tmux_error)? {
        return Err(AppError::OperationFailed {
            message: format!("no session named `{target}` exists"),
        });
    }

    if !yes {
        if !terminal.is_interactive() {
            return Err(AppError::DestructiveActionRefused {
                message: format!(
                    "refusing to kill `{target}` without a TTY; pass --yes for explicit non-interactive consent"
                ),
            });
        }
        if !confirm.confirm(&format!("Kill session `{target}`?")) {
            return Err(AppError::DestructiveActionRefused {
                message: format!("kill of `{target}` was not confirmed"),
            });
        }
    }

    session::kill(tmux, &target).map_err(map_tmux_error)?;
    Ok(format!("killed `{target}`\n"))
}

/// A named target is used as-is. Without a name, only the current tmux session may be selected,
/// and only when inside tmux (FR-024): `--yes` never enables nameless destructive targeting
/// outside tmux, since there would be no well-defined target to guess.
fn resolve_target(
    tmux: &TmuxClient,
    terminal: TerminalContext,
    session_name: Option<&str>,
) -> Result<String, AppError> {
    match session_name {
        Some(name) => Ok(name.to_owned()),
        None if terminal.inside_tmux() => tmux.current_session().map_err(map_tmux_error),
        None => Err(AppError::InvalidInvocation {
            message: "a session name is required outside tmux".to_owned(),
        }),
    }
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
    use crate::terminal::confirm::Confirm;
    use crate::tmux::client::{CommandRunner, TmuxClient};

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    fn output(status: ExitStatus, stderr: &[u8]) -> io::Result<Output> {
        Ok(Output {
            status,
            stdout: Vec::new(),
            stderr: stderr.to_vec(),
        })
    }

    fn found_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session" | "kill-session") => output(success_status(), b""),
            other => panic!("unexpected tmux invocation: {other:?}"),
        }
    }

    struct FixedConfirm(bool);
    impl Confirm for FixedConfirm {
        fn confirm(&self, _: &str) -> bool {
            self.0
        }
    }

    #[test]
    fn given_yes_when_down_runs_then_it_kills_without_prompting() {
        let tmux = TmuxClient::with_runner(found_runner as CommandRunner);
        let confirm = FixedConfirm(false); // must not even be consulted

        let result = execute(
            &tmux,
            TerminalContext::outside_tmux_for_test(),
            &confirm,
            Some("target"),
            true,
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_interactive_confirmation_accepted_when_down_runs_then_it_kills() {
        // Scenario: US7-AS2
        let tmux = TmuxClient::with_runner(found_runner as CommandRunner);
        let confirm = FixedConfirm(true);

        let result = execute(
            &tmux,
            TerminalContext::inside_tmux_for_test(),
            &confirm,
            Some("target"),
            false,
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_no_name_inside_tmux_when_down_runs_interactively_then_it_targets_the_current_session()
    {
        // Scenario: US7-AS6
        fn current_session_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            match args.first().and_then(|arg| arg.to_str()) {
                Some("display-message") => Ok(Output {
                    status: success_status(),
                    stdout: b"CURRENT\n".to_vec(),
                    stderr: Vec::new(),
                }),
                Some("has-session") => {
                    assert_eq!(
                        args,
                        [
                            OsString::from("has-session"),
                            OsString::from("-t"),
                            OsString::from("CURRENT")
                        ]
                    );
                    output(success_status(), b"")
                }
                Some("kill-session") => {
                    assert_eq!(
                        args,
                        [
                            OsString::from("kill-session"),
                            OsString::from("-t"),
                            OsString::from("CURRENT")
                        ]
                    );
                    output(success_status(), b"")
                }
                other => panic!("unexpected tmux invocation: {other:?}"),
            }
        }
        let tmux = TmuxClient::with_runner(current_session_runner as CommandRunner);
        let confirm = FixedConfirm(true);

        let result = execute(
            &tmux,
            TerminalContext::inside_tmux_for_test(),
            &confirm,
            None,
            false,
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_interactive_confirmation_declined_when_down_runs_then_it_refuses() {
        let tmux = TmuxClient::with_runner(found_runner as CommandRunner);
        let confirm = FixedConfirm(false);

        let error = execute(
            &tmux,
            TerminalContext::inside_tmux_for_test(),
            &confirm,
            Some("target"),
            false,
        )
        .expect_err("a declined confirmation should refuse the destructive action");

        assert!(matches!(error, AppError::DestructiveActionRefused { .. }));
    }

    #[test]
    fn given_a_missing_target_when_down_runs_then_it_returns_a_recoverable_error() {
        fn not_found_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
            output(failure_status(), b"can't find session")
        }
        let tmux = TmuxClient::with_runner(not_found_runner as CommandRunner);
        let confirm = FixedConfirm(true);

        let error = execute(
            &tmux,
            TerminalContext::outside_tmux_for_test(),
            &confirm,
            Some("target"),
            true,
        )
        .expect_err("a missing target should be a recoverable error");

        assert!(matches!(error, AppError::OperationFailed { .. }));
    }

    #[test]
    fn given_no_name_outside_tmux_when_down_runs_then_it_does_not_guess_a_target() {
        fn panicking_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
            panic!("tmux should not be contacted without a resolved target: {args:?}")
        }
        let tmux = TmuxClient::with_runner(panicking_runner as CommandRunner);
        let confirm = FixedConfirm(true);

        let error = execute(
            &tmux,
            TerminalContext::outside_tmux_for_test(),
            &confirm,
            None,
            true,
        )
        .expect_err("a nameless target outside tmux should be rejected");

        assert!(matches!(error, AppError::InvalidInvocation { .. }));
    }
}
