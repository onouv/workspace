//! Application orchestration for parsed commands.

use std::env;
use std::path::PathBuf;

use crate::cli::{Cli, Command, HelpTopic};
use crate::credentials::clip_mapping::{self, ClipMappingError};
use crate::credentials::clipboard::ClipboardProvider;
use crate::credentials::pass_provider::PassProvider;
use crate::error::AppError;
use crate::help;
use crate::lifecycle::{change, clip, down, exit, up};
use crate::terminal::TerminalContext;
use crate::terminal::confirm::Confirm;
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
    /// Confirmation prompt used by destructive commands.
    pub confirm: Box<dyn Confirm>,
    /// On-demand password-store provider used by `ws clip`.
    pub pass_provider: PassProvider,
    /// Clipboard destination used by `ws clip`.
    pub clipboard_provider: Box<dyn ClipboardProvider>,
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
    /// password store, prompting, or opening a terminal.
    pub fn execute(&self, cli: Cli) -> Result<String, AppError> {
        match cli.command {
            None | Some(Command::Help(crate::cli::HelpArgs { topic: None })) => Ok(help::general()),
            Some(Command::Help(crate::cli::HelpArgs {
                topic: Some(HelpTopic::Config),
            })) => Ok(help::config().to_owned()),
            Some(Command::Help(crate::cli::HelpArgs {
                topic: Some(HelpTopic::Clip),
            })) => Ok(help::clip().to_owned()),
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
            Some(Command::Change(crate::cli::SessionArgs { session_name })) => change::execute(
                &self.dependencies.tmux,
                self.dependencies.terminal,
                &session_name,
            ),
            Some(Command::Exit) => {
                exit::execute(&self.dependencies.tmux, self.dependencies.terminal)
            }
            Some(Command::Down(crate::cli::DownArgs { session_name, yes })) => down::execute(
                &self.dependencies.tmux,
                self.dependencies.terminal,
                self.dependencies.confirm.as_ref(),
                session_name.as_deref(),
                yes,
            ),
            Some(Command::Clip(crate::cli::ClipArgs { namespace, item })) => {
                let project_dir = current_project_dir();
                let mapping = clip_mapping::load(&project_dir).map_err(map_clip_mapping_error)?;
                clip::execute(
                    &self.dependencies.pass_provider,
                    self.dependencies.clipboard_provider.as_ref(),
                    &mapping,
                    &namespace,
                    &item,
                )
            }
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

fn map_clip_mapping_error(error: ClipMappingError) -> AppError {
    AppError::InvalidConfiguration {
        message: error.to_string(),
    }
}
