//! Decision flow for the reserved `ws clip NAMESPACE ITEM` utility namespace.
//!
//! The detailed credential-to-clipboard mapping is out of scope for the workspace-layout MVP
//! (see the specification's "Security design decision"); this only reserves the command shape
//! and reports it as not yet supported, without touching tmux or the password store.

use crate::error::AppError;

/// Report that no `clip` utility is implemented yet (FR-029, US8-AS2).
pub fn execute() -> Result<String, AppError> {
    Err(AppError::OperationFailed {
        message: "the `clip` utility is not yet supported".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::execute;
    use crate::error::AppError;

    #[test]
    fn given_the_reserved_clip_command_when_invoked_then_it_reports_not_yet_supported() {
        let error = execute().expect_err("clip should not be implemented yet");
        assert!(matches!(error, AppError::OperationFailed { .. }));
    }
}
