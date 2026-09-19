//! YAML configuration parsing and validation for `.ws` files.

pub mod pane_definition;
pub mod parser;
pub mod validator;
pub mod window_definition;
pub mod workspace_definition;

pub use pane_definition::{PaneDefinition, PanePosition, ValidatedPaneDefinition};
pub use parser::{ConfigError, SourceLocation, load, parse};
pub use window_definition::WindowDefinition;
pub use workspace_definition::{
    CURRENT_VERSION, ValidatedWindowDefinition, ValidatedWorkspaceDefinition, WorkspaceDefinition,
};
