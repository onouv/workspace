//! Typed recursive representation of configured tmux panes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Direction of a tmux split relative to its containing pane.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PanePosition {
    /// Split to the left, creating a horizontal split.
    Left,
    /// Split to the right, creating a horizontal split.
    Right,
    /// Split above, creating a vertical split.
    Top,
    /// Split below, creating a vertical split.
    Bottom,
}

/// A deserialized, not-yet-validated pane definition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PaneDefinition {
    /// Required relative split position.
    pub pos: PanePosition,
    /// Optional identifier unique within the containing window.
    #[serde(default)]
    pub id: Option<String>,
    /// Working directory inherited from the containing window or pane when absent.
    #[serde(default)]
    pub path: Option<String>,
    /// Optional shell command.
    #[serde(default)]
    pub command: Option<String>,
    /// Environment overrides.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Ordered recursive child panes.
    #[serde(default)]
    pub panes: Vec<PaneDefinition>,
}

/// A validated pane with resolved path and inherited environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPaneDefinition {
    /// Split direction.
    pub pos: PanePosition,
    /// Optional pane identifier.
    pub id: Option<String>,
    /// Resolved working directory.
    pub path: PathBuf,
    /// Optional shell command.
    pub command: Option<String>,
    /// Effective environment after parent inheritance and local overrides.
    pub env: BTreeMap<String, String>,
    /// Validated recursive child panes.
    pub panes: Vec<ValidatedPaneDefinition>,
}
