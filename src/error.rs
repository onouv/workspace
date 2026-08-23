#![allow(dead_code)] // Public error boundary is wired by the application layer in T009.

use std::fmt;

use thiserror::Error;

/// Stable process exit categories exposed by the `ws` command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ExitStatus {
    /// The command completed successfully.
    Success = 0,
    /// The command line could not be parsed or validated.
    InvalidInvocation = 2,
    /// The workspace definition is invalid.
    InvalidConfiguration = 3,
    /// An external dependency such as tmux is unavailable.
    DependencyUnavailable = 4,
    /// A workspace operation failed.
    OperationFailed = 5,
    /// A destructive operation was refused.
    DestructiveActionRefused = 6,
    /// A credential provider operation failed.
    CredentialFailure = 7,
}

impl ExitStatus {
    /// Return the numeric process status associated with this category.
    pub const fn code(self) -> i32 {
        self as i32
    }
}

/// Errors that may be shown at the application boundary.
///
/// Callers should pass safe, user-facing context to these variants. Secret values and raw
/// external-command output must never be used as error messages.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid invocation: {message}")]
    InvalidInvocation { message: String },

    #[error("invalid workspace configuration: {message}")]
    InvalidConfiguration { message: String },

    #[error("dependency unavailable: {message}")]
    DependencyUnavailable { message: String },

    #[error("workspace operation failed: {message}")]
    OperationFailed { message: String },

    #[error("destructive operation refused: {message}")]
    DestructiveActionRefused { message: String },

    #[error("credential operation failed: {message}")]
    CredentialFailure { message: String },
}

impl AppError {
    /// Return the stable exit category for this error.
    pub const fn exit_status(&self) -> ExitStatus {
        match self {
            Self::InvalidInvocation { .. } => ExitStatus::InvalidInvocation,
            Self::InvalidConfiguration { .. } => ExitStatus::InvalidConfiguration,
            Self::DependencyUnavailable { .. } => ExitStatus::DependencyUnavailable,
            Self::OperationFailed { .. } => ExitStatus::OperationFailed,
            Self::DestructiveActionRefused { .. } => ExitStatus::DestructiveActionRefused,
            Self::CredentialFailure { .. } => ExitStatus::CredentialFailure,
        }
    }
}

/// A display-only marker that prevents a secret value from being rendered accidentally.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Redacted;

impl fmt::Display for Redacted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Discard a sensitive value and return a safe display marker.
///
/// The value is deliberately not retained by the returned marker.
pub fn redact<T>(_: T) -> Redacted {
    Redacted
}

/// Render an application error for stderr without exposing an underlying secret.
pub fn render_error(error: &AppError) -> String {
    format!("error: {}", error)
}
