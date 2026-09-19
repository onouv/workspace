//! Decision flow for `ws up SESSION_NAME`: create or reconnect to a named workspace.

use std::path::Path;

use crate::error::AppError;
use crate::terminal::TerminalContext;
use crate::terminal::launcher::{TerminalLauncher, TerminalLauncherError};
use crate::tmux::client::{TmuxClient, TmuxError};
use crate::tmux::launch_plan::LaunchPlan;
use crate::tmux::{session, window};

/// Create or reconnect to `session_name`, following the specification's create-or-reconnect
/// contract (FR-006 through FR-019).
///
/// An existing session is always connected to as-is; the local `.ws` file is read and applied
/// only when the target session does not yet exist. Outside tmux, the process attaches directly.
/// Inside tmux, the current client is preserved and the target is opened through a separate
/// terminal launcher instead of being nested or switched to.
pub fn execute(
    tmux: &TmuxClient,
    terminal_launcher: &dyn TerminalLauncher,
    terminal: TerminalContext,
    project_dir: &Path,
    session_name: &str,
) -> Result<String, AppError> {
    if session::exists(tmux, session_name).map_err(map_tmux_error)? {
        return connect(tmux, terminal_launcher, terminal, session_name);
    }

    let definition = load_definition(project_dir)?;
    let plan = LaunchPlan::new(session_name.to_owned(), definition);

    match apply_launch_plan(tmux, &plan) {
        Ok(()) => {}
        // Another invocation won the race and created the target first; reconnect to it instead
        // of failing (US3-AS6). This only applies to the first window's `new-session` call, the
        // only one that can fail this way.
        Err((_, error)) if session::is_duplicate_session_error(&error) => {}
        Err((window_name, error)) => {
            // The first window's own creation call (`new-session`) is what brings the session
            // into existence; if a later window or pane failed instead, roll back the session
            // this invocation started rather than leaving a half-built workspace (US6-AS4). A
            // pre-existing session with the same name was already handled by the existing-session
            // fast path above and is never reached here.
            rollback_partial_session(tmux, session_name);
            return Err(map_tmux_error_with_window(&window_name, error));
        }
    }

    connect(tmux, terminal_launcher, terminal, session_name)
}

/// Materialize every window (and its recursive pane tree) in declaration order (FR-011),
/// identifying which window a failure occurred in.
fn apply_launch_plan(tmux: &TmuxClient, plan: &LaunchPlan) -> Result<(), (String, TmuxError)> {
    for (index, definition) in plan.windows().iter().enumerate() {
        window::materialize(tmux, plan.session_name(), definition, index == 0)
            .map_err(|error| (definition.name.clone(), error))?;
    }
    Ok(())
}

/// Remove `session_name` if this invocation's launch plan managed to create it before a later
/// step failed. Best-effort: the original setup failure is what gets reported to the user either
/// way, so a failure here is not surfaced separately.
fn rollback_partial_session(tmux: &TmuxClient, session_name: &str) {
    if matches!(session::exists(tmux, session_name), Ok(true)) {
        let _ = session::kill(tmux, session_name);
    }
}

/// Load and validate the project's `.ws` file.
///
/// A missing or empty file resolves to the documented default single-window workspace rather
/// than an error.
fn load_definition(
    project_dir: &Path,
) -> Result<ws::config::ValidatedWorkspaceDefinition, AppError> {
    let config_path = project_dir.join(".ws");
    ws::config::load(&config_path).map_err(map_config_error)
}

fn connect(
    tmux: &TmuxClient,
    terminal_launcher: &dyn TerminalLauncher,
    terminal: TerminalContext,
    session_name: &str,
) -> Result<String, AppError> {
    if terminal.inside_tmux() {
        terminal_launcher
            .launch(session_name)
            .map_err(map_launcher_error)?;
        Ok(format!("opened `{session_name}` in a separate terminal\n"))
    } else {
        tmux.attach(session_name).map_err(map_tmux_error)?;
        Ok(String::new())
    }
}

/// Wrap a launch-plan failure with the name of the window it occurred in, so the user can locate
/// the affected declaration (FR-015) without exposing raw internal state.
fn map_tmux_error_with_window(window_name: &str, error: TmuxError) -> AppError {
    match map_tmux_error(error) {
        AppError::OperationFailed { message } => AppError::OperationFailed {
            message: format!("window `{window_name}`: {message}"),
        },
        other => other,
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

fn map_launcher_error(error: TerminalLauncherError) -> AppError {
    match error {
        TerminalLauncherError::Unavailable => AppError::DependencyUnavailable {
            message: format!(
                "no separate terminal launcher is configured; set {} to open a session from inside tmux",
                crate::terminal::launcher::LAUNCHER_ENV_VAR
            ),
        },
        TerminalLauncherError::Failed { message } => AppError::DependencyUnavailable { message },
    }
}

fn map_config_error(error: ws::config::ConfigError) -> AppError {
    AppError::InvalidConfiguration {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tempfile::tempdir;

    use super::execute;
    use crate::error::AppError;
    use crate::terminal::TerminalContext;
    use crate::terminal::launcher::{TerminalLauncher, TerminalLauncherError};
    use crate::tmux::client::{CommandRunner, InteractiveRunner, TmuxClient};

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

    /// A fake interactive runner used wherever a test's outside-tmux path reaches
    /// [`crate::tmux::client::TmuxClient::attach`]; `with_runner` alone leaves the interactive
    /// runner at its real system default, which would spawn a real `tmux`.
    fn interactive_success(_: &OsStr, _: &[OsString]) -> io::Result<ExitStatus> {
        Ok(success_status())
    }

    /// A fake `TerminalLauncher` that records whether it was invoked and with what session name.
    struct RecordingLauncher {
        calls: AtomicUsize,
        result: fn() -> Result<(), TerminalLauncherError>,
    }

    impl RecordingLauncher {
        fn succeeding() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: || Ok(()),
            }
        }

        fn unavailable() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                result: || Err(TerminalLauncherError::Unavailable),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl TerminalLauncher for RecordingLauncher {
        fn launch(&self, _: &str) -> Result<(), TerminalLauncherError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            (self.result)()
        }
    }

    fn session_found_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session") => output(success_status(), b""),
            Some("attach-session") => output(success_status(), b""),
            _ => panic!("unexpected tmux invocation: {args:?}"),
        }
    }

    #[test]
    fn given_an_existing_session_when_up_runs_outside_tmux_then_it_attaches_without_reading_config()
    {
        let tmux = TmuxClient::with_runners(
            "fake-tmux",
            session_found_runner as CommandRunner,
            interactive_success as InteractiveRunner,
        );
        let launcher = RecordingLauncher::succeeding();
        // A directory with no `.ws` file: if the existing-session path read configuration, it
        // would fail to find the file and this call would return an error instead of succeeding.
        let project = tempdir().expect("a temporary project directory should be available");

        let result = execute(
            &tmux,
            &launcher,
            TerminalContext::outside_tmux_for_test(),
            project.path(),
            "target",
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(launcher.call_count(), 0);
    }

    #[test]
    fn given_an_existing_session_when_up_runs_inside_tmux_then_it_uses_the_separate_terminal_launcher()
     {
        let tmux = TmuxClient::with_runner(session_found_runner as CommandRunner);
        let launcher = RecordingLauncher::succeeding();
        let project = tempdir().expect("a temporary project directory should be available");

        let result = execute(
            &tmux,
            &launcher,
            TerminalContext::inside_tmux_for_test(),
            project.path(),
            "target",
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(launcher.call_count(), 1);
    }

    fn missing_then_created_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session") => output(failure_status(), b"can't find session"),
            Some("new-session") => output(success_status(), b""),
            Some("attach-session") => output(success_status(), b""),
            _ => panic!("unexpected tmux invocation: {args:?}"),
        }
    }

    #[test]
    fn given_a_missing_session_when_up_runs_outside_tmux_then_it_creates_and_attaches_to_a_fresh_session()
     {
        let tmux = TmuxClient::with_runners(
            "fake-tmux",
            missing_then_created_runner as CommandRunner,
            interactive_success as InteractiveRunner,
        );
        let launcher = RecordingLauncher::succeeding();
        let project = tempdir().expect("a temporary project directory should be available");
        std::fs::write(
            project.path().join(".ws"),
            "version: 1\nwindows:\n  - name: root\n    path: .\n",
        )
        .expect("the .ws fixture should be writable");

        let result = execute(
            &tmux,
            &launcher,
            TerminalContext::outside_tmux_for_test(),
            project.path(),
            "target",
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(launcher.call_count(), 0);
    }

    fn missing_then_duplicate_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session") => output(failure_status(), b"can't find session"),
            Some("new-session") => output(failure_status(), b"duplicate session: target"),
            Some("attach-session") => output(success_status(), b""),
            _ => panic!("unexpected tmux invocation: {args:?}"),
        }
    }

    #[test]
    fn given_the_session_race_is_lost_when_up_runs_then_it_reconnects_instead_of_failing() {
        let tmux = TmuxClient::with_runners(
            "fake-tmux",
            missing_then_duplicate_runner as CommandRunner,
            interactive_success as InteractiveRunner,
        );
        let launcher = RecordingLauncher::succeeding();
        let project = tempdir().expect("a temporary project directory should be available");
        std::fs::write(
            project.path().join(".ws"),
            "version: 1\nwindows:\n  - name: root\n    path: .\n",
        )
        .expect("the .ws fixture should be writable");

        let result = execute(
            &tmux,
            &launcher,
            TerminalContext::outside_tmux_for_test(),
            project.path(),
            "target",
        );

        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn given_no_terminal_launcher_when_up_creates_a_missing_session_inside_tmux_then_it_fails_recoverably()
     {
        let tmux = TmuxClient::with_runner(missing_then_created_runner as CommandRunner);
        let launcher = RecordingLauncher::unavailable();
        let project = tempdir().expect("a temporary project directory should be available");
        std::fs::write(
            project.path().join(".ws"),
            "version: 1\nwindows:\n  - name: root\n    path: .\n",
        )
        .expect("the .ws fixture should be writable");

        let error = execute(
            &tmux,
            &launcher,
            TerminalContext::inside_tmux_for_test(),
            project.path(),
            "target",
        )
        .expect_err("an unavailable launcher should be a recoverable error");

        assert!(matches!(error, AppError::DependencyUnavailable { .. }));
    }
}
