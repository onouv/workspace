//! Immutable tmux launch plan derived from a validated workspace definition and target session.

use ws::config::{ValidatedWindowDefinition, ValidatedWorkspaceDefinition};

/// The complete, immutable set of tmux effects needed to materialize a workspace: the target
/// session name plus the validated, already path-resolved and environment-inherited window/pane
/// tree it was built from.
pub struct LaunchPlan {
    /// Target tmux session name.
    session_name: String,
    /// Validated, ordered window/pane tree.
    definition: ValidatedWorkspaceDefinition,
}

impl LaunchPlan {
    /// Construct a launch plan for `session_name` from a validated workspace definition.
    pub const fn new(session_name: String, definition: ValidatedWorkspaceDefinition) -> Self {
        Self {
            session_name,
            definition,
        }
    }

    /// The target tmux session name.
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Windows in declaration order; FR-011 selects the first as the initial window.
    pub fn windows(&self) -> &[ValidatedWindowDefinition] {
        &self.definition.windows
    }
}
