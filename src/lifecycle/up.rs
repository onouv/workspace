//! Decision flow for `ws up [SESSION_NAME]`: create or reconnect to a named workspace.

use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::lifecycle::session_name_source::SessionNameSource;
use crate::session_name::derived_session_name::DerivedSessionName;
use crate::terminal::TerminalContext;
use crate::terminal::launcher::{TerminalLauncher, TerminalLauncherError};
use crate::terminal::notifier::Notifier;
use crate::tmux::client::{TmuxClient, TmuxError};
use crate::tmux::launch_plan::LaunchPlan;
use crate::tmux::session_folder::FolderCheck;
use crate::tmux::window_role::WindowRole;
use crate::tmux::{session, window};

/// Create or reconnect to `session_name`, following the specification's create-or-reconnect
/// contract (FR-006 through FR-019).
///
/// An existing session is always connected to as-is; the local `.ws` file is read and applied
/// only when the target session does not yet exist. Outside tmux, the process attaches directly.
/// Inside tmux, the current client is preserved and the target is opened through a separate
/// terminal launcher instead of being nested or switched to.
///
/// For a name derived from the current folder, the notifier shows the name before any tmux
/// contact, because an attach outside tmux blocks until the user detaches (003 FR-006). Before it
/// connects to an existing session, the session must belong to the project folder (003 FR-011).
/// This also applies when another invocation won the creation race.
pub fn execute(
    tmux: &TmuxClient,
    terminal_launcher: &dyn TerminalLauncher,
    terminal: TerminalContext,
    notifier: &dyn Notifier,
    project_dir: &Path,
    source: &SessionNameSource,
) -> Result<String, AppError> {
    let session_name = source.name();
    if let SessionNameSource::Derived(_) = source {
        notifier.notify(&format!(
            "using session name `{session_name}` from the current folder"
        ));
    }

    if session::exists(tmux, session_name).map_err(map_tmux_error)? {
        ensure_session_belongs_to_folder(tmux, source)?;
        return connect(tmux, terminal_launcher, terminal, session_name);
    }

    let definition = load_definition(project_dir)?;
    let plan = LaunchPlan::new(session_name.to_owned(), definition);

    match apply_launch_plan(tmux, &plan, project_dir) {
        Ok(()) => {}
        // Another invocation won the race and created the target first; reconnect to it instead
        // of failing (US3-AS6). This only applies to the first window's `new-session` call, the
        // only one that can fail this way.
        Err((_, error)) if session::is_duplicate_session_error(&error) => {
            ensure_session_belongs_to_folder(tmux, source)?;
        }
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

/// For a derived name, fail unless the existing session belongs to the project folder
/// (003 FR-011). An explicit name connects without this check.
fn ensure_session_belongs_to_folder(
    tmux: &TmuxClient,
    source: &SessionNameSource,
) -> Result<(), AppError> {
    let SessionNameSource::Derived(derived) = source else {
        return Ok(());
    };
    let folder = session::folder(tmux, derived.name()).map_err(map_tmux_error)?;
    match folder
        .as_ref()
        .map(|folder| (folder.check(derived.project_folder()), folder))
    {
        Some((FolderCheck::Match, _)) => Ok(()),
        Some((FolderCheck::Mismatch, folder)) => Err(folder_conflict(
            derived,
            &format!("belongs to {}", folder.path().display()),
        )),
        Some((FolderCheck::Unreadable, _)) | None => {
            Err(folder_conflict(derived, "has a folder that cannot be read"))
        }
    }
}

fn folder_conflict(derived: &DerivedSessionName, cause: &str) -> AppError {
    AppError::OperationFailed {
        message: format!(
            "session `{}` {cause}; run `ws up SESSION_NAME` with a different name",
            derived.name()
        ),
    }
}

/// Materialize every window (and its recursive pane tree) in declaration order (FR-011),
/// identifying which window a failure occurred in.
///
/// The first window records the canonical project folder in the new session, so a later
/// `ws up` without a name can check which folder the session belongs to (003 FR-011).
fn apply_launch_plan(
    tmux: &TmuxClient,
    plan: &LaunchPlan,
    project_dir: &Path,
) -> Result<(), (String, TmuxError)> {
    let project_folder = fs::canonicalize(project_dir).unwrap_or_else(|_| project_dir.to_owned());
    for (index, definition) in plan.windows().iter().enumerate() {
        let role = if index == 0 {
            WindowRole::First {
                project_folder: &project_folder,
            }
        } else {
            WindowRole::Additional
        };
        window::materialize(tmux, plan.session_name(), definition, role)
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
    use crate::lifecycle::session_name_source::SessionNameSource;
    use crate::session_name::derived_session_name::DerivedSessionName;
    use crate::terminal::TerminalContext;
    use crate::terminal::launcher::{TerminalLauncher, TerminalLauncherError};
    use crate::terminal::notifier::RecordingNotifier;
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
            &RecordingNotifier::default(),
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(launcher.call_count(), 0);
    }

    #[test]
    fn given_an_explicit_name_and_an_existing_session_when_up_runs_then_it_connects_without_a_notice_or_folder_check()
     {
        // `session_found_runner` panics on `list-sessions`, so a folder check would fail here.
        let tmux = TmuxClient::with_runners(
            "fake-tmux",
            session_found_runner as CommandRunner,
            interactive_success as InteractiveRunner,
        );
        let launcher = RecordingLauncher::succeeding();
        let notifier = RecordingNotifier::default();
        let project = tempdir().expect("a temporary project directory should be available");

        execute(
            &tmux,
            &launcher,
            TerminalContext::outside_tmux_for_test(),
            &notifier,
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
        )
        .expect("the existing session should be connected");

        assert!(notifier.lines().is_empty(), "{:?}", notifier.lines());
    }

    #[test]
    fn given_a_derived_name_when_up_runs_then_the_notifier_shows_the_name_once() {
        let tmux = TmuxClient::with_runners(
            "fake-tmux",
            missing_then_created_runner as CommandRunner,
            interactive_success as InteractiveRunner,
        );
        let launcher = RecordingLauncher::succeeding();
        let notifier = RecordingNotifier::default();
        let project = tempdir().expect("a temporary project directory should be available");
        let derived = DerivedSessionName::derive(std::path::Path::new("/tmp/alpha"))
            .expect("`alpha` is a usable folder name");

        execute(
            &tmux,
            &launcher,
            TerminalContext::outside_tmux_for_test(),
            &notifier,
            project.path(),
            &SessionNameSource::Derived(derived),
        )
        .expect("the missing session should be created");

        assert_eq!(
            notifier.lines(),
            ["using session name `alpha` from the current folder"]
        );
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
            &RecordingNotifier::default(),
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
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
            &RecordingNotifier::default(),
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
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

    fn race_lost_to_another_folder_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        match args.first().and_then(|arg| arg.to_str()) {
            Some("has-session") => output(failure_status(), b"can't find session"),
            Some("new-session") => output(failure_status(), b"duplicate session: alpha"),
            Some("list-sessions") => Ok(Output {
                status: success_status(),
                stdout: b"alpha\t/\t/\n".to_vec(),
                stderr: Vec::new(),
            }),
            _ => panic!("unexpected tmux invocation: {args:?}"),
        }
    }

    #[test]
    fn given_a_derived_name_and_the_race_is_lost_to_another_folder_when_up_runs_then_it_fails_without_connecting()
     {
        let tmux = TmuxClient::with_runner(race_lost_to_another_folder_runner as CommandRunner);
        let launcher = RecordingLauncher::succeeding();
        let parent = tempdir().expect("a temporary parent folder should be available");
        let project = parent.path().join("alpha");
        std::fs::create_dir(&project).expect("the project folder should be creatable");
        let derived = DerivedSessionName::derive(&project).expect("`alpha` is a usable name");

        let error = execute(
            &tmux,
            &launcher,
            TerminalContext::inside_tmux_for_test(),
            &RecordingNotifier::default(),
            &project,
            &SessionNameSource::Derived(derived),
        )
        .expect_err("a session of another folder must not be connected");

        assert!(
            matches!(error, AppError::OperationFailed { .. }),
            "{error:?}"
        );
        assert_eq!(launcher.call_count(), 0);
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
            &RecordingNotifier::default(),
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
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
            &RecordingNotifier::default(),
            project.path(),
            &SessionNameSource::Explicit("target".to_owned()),
        )
        .expect_err("an unavailable launcher should be a recoverable error");

        assert!(matches!(error, AppError::DependencyUnavailable { .. }));
    }
}
