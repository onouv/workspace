#![allow(dead_code)] // Application wiring is completed by the later orchestration task.

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
/// Command-specific behavior is implemented by later lifecycle tasks. This type keeps external
/// effects injectable so application and domain code do not own process-global state.
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
}
