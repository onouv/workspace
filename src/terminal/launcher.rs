#![allow(dead_code)] // Concrete launch behavior is implemented by the lifecycle tasks.

use thiserror::Error;

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

/// Explicit placeholder used until a platform launcher is configured.
#[derive(Debug, Default)]
pub struct UnavailableTerminalLauncher;

impl TerminalLauncher for UnavailableTerminalLauncher {
    fn launch(&self, _: &str) -> Result<(), TerminalLauncherError> {
        Err(TerminalLauncherError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminalLauncher, TerminalLauncherError, UnavailableTerminalLauncher};

    #[test]
    fn missing_launcher_is_recoverable() {
        let launcher = UnavailableTerminalLauncher;
        assert!(matches!(
            launcher.launch("session"),
            Err(TerminalLauncherError::Unavailable)
        ));
    }
}
