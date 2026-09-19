//! Typed representation of a `.ws` workspace document.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::window_definition::WindowDefinition;

/// The only configuration language version supported by the MVP.
pub const CURRENT_VERSION: u32 = 1;

/// A deserialized, not-yet-validated workspace document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDefinition {
    /// Configuration language version.
    pub version: u32,
    /// Ordered tmux windows.
    pub windows: Vec<WindowDefinition>,
}

impl WorkspaceDefinition {
    /// Construct the documented one-window definition used by an empty `.ws` file.
    pub fn default_for(_source_path: &Path) -> Self {
        Self {
            version: CURRENT_VERSION,
            windows: vec![WindowDefinition {
                name: "root".to_owned(),
                path: ".".to_owned(),
                command: None,
                env: Default::default(),
                panes: Vec::new(),
            }],
        }
    }
}

/// A fully validated workspace with resolved paths and inherited environments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWorkspaceDefinition {
    /// Configuration language version.
    pub version: u32,
    /// Ordered, immutable validated windows.
    pub windows: Vec<ValidatedWindowDefinition>,
    /// Configuration source path used to resolve relative paths.
    pub source_path: PathBuf,
}

/// A validated window with a resolved working directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedWindowDefinition {
    /// Unique tmux window name.
    pub name: String,
    /// Existing resolved working directory.
    pub path: PathBuf,
    /// Optional shell command.
    pub command: Option<String>,
    /// Window environment values.
    pub env: std::collections::BTreeMap<String, String>,
    /// Validated panes in declaration order.
    pub panes: Vec<super::pane_definition::ValidatedPaneDefinition>,
}
