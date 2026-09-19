//! Interactive confirmation boundary for destructive operations.

use dialoguer::Confirm as DialoguerPrompt;

/// Boundary for asking the user to confirm a destructive operation.
///
/// Kept as a trait so destructive-command decision logic (see `ws down`) can be tested without a
/// real terminal; callers are responsible for checking TTY availability before using this at all
/// (see [`super::TerminalContext::is_interactive`]).
pub trait Confirm {
    /// Ask `prompt` and return whether the user confirmed. A non-answer (for example, the prompt
    /// being interrupted) is treated as a refusal, never as consent.
    fn confirm(&self, prompt: &str) -> bool;
}

/// Prompts on the real terminal using `dialoguer`.
#[derive(Debug, Default)]
pub struct TerminalConfirm;

impl Confirm for TerminalConfirm {
    fn confirm(&self, prompt: &str) -> bool {
        DialoguerPrompt::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false)
    }
}
