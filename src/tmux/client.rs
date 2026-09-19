use std::ffi::{OsStr, OsString};
use std::io;
use std::process::{Command, ExitStatus, Output};

use thiserror::Error;

/// Function signature used to execute an external command and capture its output.
///
/// Keeping command execution injectable lets tests verify argv boundaries without starting a
/// real tmux server.
pub type CommandRunner = fn(&OsStr, &[OsString]) -> io::Result<Output>;

/// Function signature used to run an external command with inherited stdio.
///
/// Attaching a tmux client takes over the invoking process's terminal, so its output cannot be
/// captured; kept injectable so tests can observe the invocation without a real terminal.
pub type InteractiveRunner = fn(&OsStr, &[OsString]) -> io::Result<ExitStatus>;

/// Errors returned by the tmux process boundary.
#[derive(Debug, Error)]
pub enum TmuxError {
    #[error("could not invoke tmux: {source}")]
    Invocation {
        #[source]
        source: io::Error,
    },
    #[error("tmux command failed with status {status}: {stderr}")]
    CommandFailed { status: String, stderr: String },
    #[error("tmux command failed with status {status}")]
    InteractiveCommandFailed { status: String },
}

/// A small argv-safe client for the tmux executable.
#[derive(Debug)]
pub struct TmuxClient {
    runner: CommandRunner,
    interactive_runner: InteractiveRunner,
    executable: OsString,
}

impl TmuxClient {
    /// Construct a client that invokes `tmux` through the operating system.
    pub fn new() -> Self {
        Self::with_runner(run_system_command)
    }

    /// Construct a client with an injected command runner for tests.
    ///
    /// The interactive runner defaults to the real system implementation; tests that need to
    /// fake attach/switch behavior should use [`Self::with_runners`] instead.
    pub const fn with_runner(runner: CommandRunner) -> Self {
        Self {
            runner,
            interactive_runner: run_system_interactive_command,
            executable: OsString::new(),
        }
    }

    /// Construct a client with an explicit executable path and command runner, for tests.
    #[cfg(test)]
    pub(crate) fn with_executable(executable: impl Into<OsString>, runner: CommandRunner) -> Self {
        Self {
            runner,
            interactive_runner: run_system_interactive_command,
            executable: executable.into(),
        }
    }

    /// Construct a client with an explicit executable path and both injected runners, for tests
    /// that exercise [`Self::attach`].
    #[cfg(test)]
    pub(crate) fn with_runners(
        executable: impl Into<OsString>,
        runner: CommandRunner,
        interactive_runner: InteractiveRunner,
    ) -> Self {
        Self {
            runner,
            interactive_runner,
            executable: executable.into(),
        }
    }

    /// Execute one tmux command using separate argv values and capture its output.
    ///
    /// Session names, paths, and other generated values are never assembled into a shell string.
    pub fn execute<I, S>(&self, args: I) -> Result<Output, TmuxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let output = (self.runner)(self.resolved_executable(), &args)
            .map_err(|source| TmuxError::Invocation { source })?;

        if output.status.success() {
            Ok(output)
        } else {
            let status = exit_status_text(output.status.code());
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(TmuxError::CommandFailed { status, stderr })
        }
    }

    /// Attach the current process's terminal to `session_name` as a tmux client.
    ///
    /// This blocks until the user detaches, since it takes over the invoking process's stdio.
    pub fn attach(&self, session_name: &str) -> Result<(), TmuxError> {
        self.run_interactive(["attach-session", "-t", session_name])
    }

    /// Switch the current tmux client to `session_name` without changing any other client.
    ///
    /// Unlike [`Self::attach`], this only makes sense when the invoking process is already a
    /// tmux client (inside a pane); it returns immediately and does not take over stdio.
    pub fn switch_client(&self, session_name: &str) -> Result<(), TmuxError> {
        self.execute(["switch-client", "-t", session_name])?;
        Ok(())
    }

    /// Detach the current tmux client without killing its session.
    ///
    /// Only meaningful when the invoking process is already a tmux client.
    pub fn detach_client(&self) -> Result<(), TmuxError> {
        self.execute(["detach-client"])?;
        Ok(())
    }

    /// Return the name of the session attached to the current tmux client.
    ///
    /// Only meaningful when the invoking process is already a tmux client.
    pub fn current_session(&self) -> Result<String, TmuxError> {
        let output = self.execute(["display-message", "-p", "#S"])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn run_interactive<I, S>(&self, args: I) -> Result<(), TmuxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let status = (self.interactive_runner)(self.resolved_executable(), &args)
            .map_err(|source| TmuxError::Invocation { source })?;

        if status.success() {
            Ok(())
        } else {
            Err(TmuxError::InteractiveCommandFailed {
                status: exit_status_text(status.code()),
            })
        }
    }

    fn resolved_executable(&self) -> &OsStr {
        if self.executable.is_empty() {
            OsStr::new("tmux")
        } else {
            &self.executable
        }
    }
}

impl Default for TmuxClient {
    fn default() -> Self {
        Self::new()
    }
}

fn exit_status_text(code: Option<i32>) -> String {
    code.map_or_else(
        || "terminated by signal".to_owned(),
        |code| code.to_string(),
    )
}

fn run_system_command(program: &OsStr, args: &[OsString]) -> io::Result<Output> {
    Command::new(program).args(args).output()
}

fn run_system_interactive_command(program: &OsStr, args: &[OsString]) -> io::Result<ExitStatus> {
    Command::new(program).args(args).status()
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};

    use super::{CommandRunner, TmuxClient, TmuxError};

    #[cfg(unix)]
    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    #[cfg(unix)]
    fn failure_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(256)
    }

    #[cfg(unix)]
    fn successful_runner(program: &OsStr, args: &[OsString]) -> io::Result<Output> {
        assert_eq!(program, OsStr::new("fake-tmux"));
        assert_eq!(
            args,
            [
                OsString::from("has-session"),
                OsString::from("-t"),
                OsString::from("name with spaces")
            ]
        );
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    #[cfg(unix)]
    fn failing_runner(_: &OsStr, _: &[OsString]) -> io::Result<Output> {
        Ok(Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: b"session not found".to_vec(),
        })
    }

    #[cfg(unix)]
    #[test]
    fn preserves_argument_boundaries() {
        let client = TmuxClient::with_executable("fake-tmux", successful_runner as CommandRunner);
        let result = client.execute(["has-session", "-t", "name with spaces"]);
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn reports_non_zero_tmux_status() {
        let client = TmuxClient::with_runner(failing_runner as CommandRunner);
        let error = client
            .execute(["has-session"])
            .expect_err("fake tmux should fail");
        assert!(matches!(error, TmuxError::CommandFailed { .. }));
    }

    #[cfg(unix)]
    fn successful_interactive_runner(program: &OsStr, args: &[OsString]) -> io::Result<ExitStatus> {
        assert_eq!(program, OsStr::new("fake-tmux"));
        assert_eq!(
            args,
            [
                OsString::from("attach-session"),
                OsString::from("-t"),
                OsString::from("target")
            ]
        );
        Ok(success_status())
    }

    #[cfg(unix)]
    fn failing_interactive_runner(_: &OsStr, _: &[OsString]) -> io::Result<ExitStatus> {
        Ok(failure_status())
    }

    #[cfg(unix)]
    #[test]
    fn attach_invokes_attach_session_with_the_target_argument() {
        let client = TmuxClient::with_runners(
            "fake-tmux",
            successful_runner as CommandRunner,
            successful_interactive_runner as super::InteractiveRunner,
        );
        assert!(client.attach("target").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn attach_reports_a_non_zero_status_without_captured_stderr() {
        let client = TmuxClient::with_runners(
            "fake-tmux",
            successful_runner as CommandRunner,
            failing_interactive_runner as super::InteractiveRunner,
        );
        let error = client
            .attach("target")
            .expect_err("fake attach should fail");
        assert!(matches!(error, TmuxError::InteractiveCommandFailed { .. }));
    }

    #[cfg(unix)]
    fn current_session_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        assert_eq!(
            args,
            [
                OsString::from("display-message"),
                OsString::from("-p"),
                OsString::from("#S")
            ]
        );
        Ok(Output {
            status: success_status(),
            stdout: b"SESSION_ONE\n".to_vec(),
            stderr: Vec::new(),
        })
    }

    #[cfg(unix)]
    #[test]
    fn current_session_trims_the_reported_name() {
        let client = TmuxClient::with_runner(current_session_runner as CommandRunner);
        assert_eq!(
            client.current_session().expect("fake tmux should succeed"),
            "SESSION_ONE"
        );
    }
}
