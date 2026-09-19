use thiserror::Error;

/// Stable process exit categories exposed by the `ws` command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ExitStatus {
    /// The command completed successfully.
    ///
    /// Never constructed directly: a successful run falls through `main` and exits 0 by the
    /// platform's own default. This variant exists so the enum documents the complete,
    /// published exit-status contract (see `specs/001-tmux-workspace/contracts/cli.md`).
    #[allow(dead_code)]
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

    // Reserved for the credential-provider integration deferred by User Story 8; see
    // `src/credentials/pass_provider.rs`, which is not yet wired into any command.
    #[allow(dead_code)]
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

/// Render an application error for stderr without exposing an underlying secret.
pub fn render_error(error: &AppError) -> String {
    format!("error: {}", error)
}
