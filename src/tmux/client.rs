#![allow(dead_code)] // Tmux operations are consumed by lifecycle tasks after this boundary.

use std::ffi::{OsStr, OsString};
use std::io;
use std::process::{Command, Output};

use thiserror::Error;

/// Function signature used to execute an external command.
///
/// Keeping command execution injectable lets tests verify argv boundaries without starting a
/// real tmux server.
pub type CommandRunner = fn(&OsStr, &[OsString]) -> io::Result<Output>;

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
}

/// A small argv-safe client for the tmux executable.
#[derive(Debug)]
pub struct TmuxClient {
    runner: CommandRunner,
    executable: OsString,
}

impl TmuxClient {
    /// Construct a client that invokes `tmux` through the operating system.
    pub fn new() -> Self {
        Self::with_runner(run_system_command)
    }

    /// Construct a client with an injected command runner for tests.
    pub const fn with_runner(runner: CommandRunner) -> Self {
        Self {
            runner,
            executable: OsString::new(),
        }
    }

    /// Construct a client with an explicit executable path and command runner.
    pub fn with_executable(executable: impl Into<OsString>, runner: CommandRunner) -> Self {
        Self {
            runner,
            executable: executable.into(),
        }
    }

    /// Execute one tmux command using separate argv values.
    ///
    /// Session names, paths, and other generated values are never assembled into a shell string.
    pub fn execute<I, S>(&self, args: I) -> Result<Output, TmuxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let executable = if self.executable.is_empty() {
            OsStr::new("tmux")
        } else {
            &self.executable
        };
        let output =
            (self.runner)(executable, &args).map_err(|source| TmuxError::Invocation { source })?;

        if output.status.success() {
            Ok(output)
        } else {
            let status = output.status.code().map_or_else(
                || "terminated by signal".to_owned(),
                |code| code.to_string(),
            );
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(TmuxError::CommandFailed { status, stderr })
        }
    }
}

impl Default for TmuxClient {
    fn default() -> Self {
        Self::new()
    }
}

fn run_system_command(program: &OsStr, args: &[OsString]) -> io::Result<Output> {
    Command::new(program).args(args).output()
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
}
