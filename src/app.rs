//! Application orchestration for parsed commands.

use std::env;
use std::path::PathBuf;

use crate::cli::{Cli, Command, HelpTopic};
use crate::error::AppError;
use crate::help;
use crate::lifecycle::up;
use crate::terminal::TerminalContext;
use crate::terminal::launcher::TerminalLauncher;
use crate::tmux::client::TmuxClient;

/// Injectable external-effect dependencies for the application layer.
pub struct AppDependencies {
    /// Client used for argv-safe tmux operations.
    pub tmux: TmuxClient,
    /// Launcher used to open a separate terminal context.
    pub terminal_launcher: Box<dyn TerminalLauncher>,
    /// Terminal capabilities detected for the current process.
    pub terminal: TerminalContext,
}

/// Application orchestration boundary.
///
/// Help and version behavior is deliberately handled before any dependency operation. Later
/// lifecycle tasks will add the workspace command branches to this same boundary.
pub struct App {
    dependencies: AppDependencies,
}

impl App {
    /// Create an application with explicitly supplied external dependencies.
    pub const fn new(dependencies: AppDependencies) -> Self {
        Self { dependencies }
    }

    /// Execute a parsed command and return user-facing output.
    ///
    /// Discovery commands return static text without reading `.ws`, contacting tmux, invoking a
    /// password store, prompting, or opening a terminal. Remaining lifecycle commands (`change`,
    /// `down`, `exit`, `clip`) are reserved for later implementation.
    pub fn execute(&self, cli: Cli) -> Result<String, AppError> {
        match cli.command {
            None | Some(Command::Help(crate::cli::HelpArgs { topic: None })) => Ok(help::general()),
            Some(Command::Help(crate::cli::HelpArgs {
                topic: Some(HelpTopic::Config),
            })) => Ok(help::config().to_owned()),
            Some(Command::Up(crate::cli::UpArgs { session_name })) => {
                let project_dir = current_project_dir();
                up::execute(
                    &self.dependencies.tmux,
                    self.dependencies.terminal_launcher.as_ref(),
                    self.dependencies.terminal,
                    &project_dir,
                    &session_name,
                )
            }
            Some(command) => Err(AppError::OperationFailed {
                message: format!("command is not implemented yet: {command:?}"),
            }),
        }
    }
}

/// Resolve the current project directory for workspace commands.
///
/// Discovery commands never call this, so a rare failure to read the current directory does not
/// block them. When a lifecycle command does need it, an unusable directory still surfaces as a
/// clear error further downstream (for example, `.ws` failing to resolve or tmux failing to
/// start), rather than being reported here as a misleadingly specific cause.
fn current_project_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
