//! Application orchestration for parsed commands.

#![allow(dead_code)] // External dependencies are consumed by later lifecycle tasks.

use crate::cli::{Cli, Command, HelpTopic};
use crate::error::AppError;
use crate::help;
use crate::terminal::launcher::TerminalLauncher;
use crate::tmux::client::TmuxClient;

/// Injectable external-effect dependencies for the application layer.
pub struct AppDependencies {
    /// Client used for argv-safe tmux operations.
    pub tmux: TmuxClient,
    /// Launcher used to open a separate terminal context.
    pub terminal_launcher: Box<dyn TerminalLauncher>,
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

    /// Borrow the configured dependencies for command orchestration.
    pub const fn dependencies(&self) -> &AppDependencies {
        &self.dependencies
    }

    /// Execute a parsed command and return user-facing output.
    ///
    /// Discovery commands return static text without reading `.ws`, contacting tmux, invoking a
    /// password store, prompting, or opening a terminal. Workspace commands remain reserved for
    /// their later lifecycle implementation.
    pub fn execute(&self, cli: Cli) -> Result<String, AppError> {
        match cli.command {
            None | Some(Command::Help(crate::cli::HelpArgs { topic: None })) => Ok(help::general()),
            Some(Command::Help(crate::cli::HelpArgs {
                topic: Some(HelpTopic::Config),
            })) => Ok(help::config().to_owned()),
            Some(command) => Err(AppError::OperationFailed {
                message: format!("command is not implemented yet: {command:?}"),
            }),
        }
    }
}
