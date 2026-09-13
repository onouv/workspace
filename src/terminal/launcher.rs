use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::process::Command;

use thiserror::Error;

/// Environment variable naming the configured separate-terminal launcher command.
///
/// The value is a program followed by space-separated arguments, for example
/// `x-terminal-emulator -e tmux attach-session -t`. `ws` appends the target session name as the
/// final argument; it never shell-interprets the configured value.
pub const LAUNCHER_ENV_VAR: &str = "WS_TERMINAL_LAUNCHER";

/// Errors from opening a separate terminal client/window.
#[derive(Debug, Error)]
pub enum TerminalLauncherError {
    #[error("no separate terminal launcher is configured")]
    Unavailable,
    #[error("terminal launcher failed: {message}")]
    Failed { message: String },
}

/// Boundary for opening an attachable terminal context for a tmux session.
pub trait TerminalLauncher: Send + Sync {
    /// Open a separate terminal context attached to `session_name`.
    fn launch(&self, session_name: &str) -> Result<(), TerminalLauncherError>;
}

/// Function signature used to spawn the configured launcher command without waiting for it.
///
/// Opening a separate terminal window must not block the invoking process; kept injectable so
/// tests can observe the invocation without spawning a real terminal emulator.
pub type LauncherSpawner = fn(&OsStr, &[OsString]) -> io::Result<()>;

/// Opens a separate terminal context by spawning a user-configured launcher command.
///
/// Reports the recoverable [`TerminalLauncherError::Unavailable`] error when no launcher is
/// configured, rather than nesting a tmux client or doing nothing silently.
pub struct ConfiguredTerminalLauncher {
    command: Option<(OsString, Vec<OsString>)>,
    spawner: LauncherSpawner,
}

impl ConfiguredTerminalLauncher {
    /// Build a launcher from the [`LAUNCHER_ENV_VAR`] environment variable, if set.
    pub fn from_env() -> Self {
        let command = env::var_os(LAUNCHER_ENV_VAR).and_then(|value| parse_command(&value));
        Self {
            command,
            spawner: spawn_detached,
        }
    }

    /// Construct a launcher with an explicit command and an injected spawner, for tests.
    #[cfg(test)]
    pub(crate) fn with_command(
        program: impl Into<OsString>,
        args: Vec<OsString>,
        spawner: LauncherSpawner,
    ) -> Self {
        Self {
            command: Some((program.into(), args)),
            spawner,
        }
    }

    /// Construct a launcher with no configured command, for tests of the unavailable path.
    #[cfg(test)]
    pub(crate) const fn unconfigured(spawner: LauncherSpawner) -> Self {
        Self {
            command: None,
            spawner,
        }
    }
}

impl TerminalLauncher for ConfiguredTerminalLauncher {
    fn launch(&self, session_name: &str) -> Result<(), TerminalLauncherError> {
        let Some((program, args)) = &self.command else {
            return Err(TerminalLauncherError::Unavailable);
        };

        let mut full_args = args.clone();
        full_args.push(OsString::from(session_name));
        (self.spawner)(program, &full_args).map_err(|source| TerminalLauncherError::Failed {
            message: format!("could not start the configured terminal launcher: {source}"),
        })
    }
}

fn parse_command(value: &OsStr) -> Option<(OsString, Vec<OsString>)> {
    let value = value.to_str()?;
    let mut parts = value.split_whitespace();
    let program = OsString::from(parts.next()?);
    let args = parts.map(OsString::from).collect();
    Some((program, args))
}

fn spawn_detached(program: &OsStr, args: &[OsString]) -> io::Result<()> {
    Command::new(program).args(args).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::{ConfiguredTerminalLauncher, TerminalLauncher, TerminalLauncherError};

    static SPAWN_INVOKED: AtomicBool = AtomicBool::new(false);

    fn recording_spawner(_: &OsStr, _: &[OsString]) -> io::Result<()> {
        SPAWN_INVOKED.store(true, Ordering::SeqCst);
        Ok(())
    }

    #[test]
    fn given_no_configured_command_when_launched_then_it_reports_unavailable() {
        let launcher = ConfiguredTerminalLauncher::unconfigured(recording_spawner);

        let error = launcher
            .launch("session")
            .expect_err("an unconfigured launcher should be unavailable");

        assert!(matches!(error, TerminalLauncherError::Unavailable));
        assert!(
            !SPAWN_INVOKED.load(Ordering::SeqCst),
            "an unavailable launcher must not spawn anything"
        );
    }

    #[test]
    fn given_a_configured_command_when_launched_then_the_session_name_is_appended() {
        fn asserting_spawner(program: &OsStr, args: &[OsString]) -> io::Result<()> {
            assert_eq!(program, OsStr::new("term"));
            assert_eq!(args, [OsString::from("-e"), OsString::from("session")]);
            Ok(())
        }
        let launcher = ConfiguredTerminalLauncher::with_command(
            "term",
            vec![OsString::from("-e")],
            asserting_spawner,
        );

        launcher
            .launch("session")
            .expect("a configured launcher should succeed");
    }
}
