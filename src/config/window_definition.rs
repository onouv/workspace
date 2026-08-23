//! Typed representation of a configured tmux window.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::pane_definition::PaneDefinition;

/// A deserialized, not-yet-validated window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WindowDefinition {
    /// Tmux window name.
    pub name: String,
    /// Working directory, relative to the `.ws` file unless absolute.
    pub path: String,
    /// Optional shell command.
    #[serde(default)]
    pub command: Option<String>,
    /// Environment values inherited by descendant panes.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Ordered pane definitions.
    #[serde(default)]
    pub panes: Vec<PaneDefinition>,
}
