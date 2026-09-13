//! YAML loading and source-aware parse errors.

use std::fs;
use std::path::{Path, PathBuf};

use super::validator;
use super::workspace_definition::{ValidatedWorkspaceDefinition, WorkspaceDefinition};
use yaml_serde::{Mapping, Value};

/// One-based source location reported by the YAML parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, " at line {}, column {}", self.line, self.column)
    }
}

/// Errors produced while loading, parsing, or validating a `.ws` document.
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read.
    Io {
        /// Configuration path.
        path: PathBuf,
        /// Operating-system error.
        source: std::io::Error,
    },
    /// The document is not valid YAML or does not match the typed schema.
    Parse {
        /// Configuration path.
        path: PathBuf,
        /// Parser or Serde message.
        message: String,
        /// Optional YAML source location.
        location: Option<SourceLocation>,
    },
    /// The document is structurally valid but violates a semantic rule.
    Validation {
        /// Configuration path.
        path: PathBuf,
        /// Nearest declaration context.
        context: String,
        /// Semantic validation message.
        message: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "{}: could not read configuration: {source}",
                    path.display()
                )
            }
            Self::Parse {
                path,
                message,
                location,
            } => {
                let location = location
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string);
                write!(formatter, "{}: {message}{location}", path.display())
            }
            Self::Validation {
                path,
                context,
                message,
            } => write!(formatter, "{}: {context}: {message}", path.display()),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { .. } | Self::Validation { .. } => None,
        }
    }
}

/// Maximum recursive pane nesting depth beneath a window, per the constitution's bounded-recursion
/// rule for externally supplied structure. Chosen well above any practical workspace layout while
/// keeping the recursive descent below in `validate_pane_types` and in
/// [`super::validator::validate`] bounded.
pub(crate) const MAX_PANE_NESTING_DEPTH: usize = 32;

/// Parse one `.ws` YAML document without performing external workspace effects.
pub fn parse(source: &str, source_path: &Path) -> Result<WorkspaceDefinition, ConfigError> {
    if source.trim().is_empty() {
        return Ok(WorkspaceDefinition::default_for(source_path));
    }

    // The untyped `Value` tree is validated (including pane-nesting depth) before the typed,
    // recursively-deserializing parse below runs, so that an over-deep document is rejected here
    // rather than risking unbounded recursion during typed deserialization.
    let value = yaml_serde::from_str::<Value>(source).map_err(|error| ConfigError::Parse {
        path: source_path.to_path_buf(),
        message: error.to_string(),
        location: error.location().map(|location| SourceLocation {
            line: location.line(),
            column: location.column(),
        }),
    })?;
    validate_yaml_types(&value, source_path, "document")?;

    yaml_serde::from_str(source).map_err(|error| ConfigError::Parse {
        path: source_path.to_path_buf(),
        message: error.to_string(),
        location: error.location().map(|location| SourceLocation {
            line: location.line(),
            column: location.column(),
        }),
    })
}

fn validate_yaml_types(
    value: &Value,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    reject_tag(value, source_path, context)?;
    let root = value.as_mapping().ok_or_else(|| {
        validation_error(source_path, context, "must be a YAML mapping".to_owned())
    })?;
    validate_mapping_keys(root, source_path, context)?;
    if let Some(version) = root.get("version") {
        require_number(version, source_path, context, "version")?;
    }
    if let Some(windows) = root.get("windows") {
        reject_tag(windows, source_path, context)?;
        let windows = windows.as_sequence().ok_or_else(|| {
            validation_error(
                source_path,
                context,
                "windows must be a YAML sequence".to_owned(),
            )
        })?;
        for (index, window) in windows.iter().enumerate() {
            validate_window_types(window, source_path, &format!("windows[{index}]"))?;
        }
    }
    Ok(())
}

fn validate_window_types(
    value: &Value,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    reject_tag(value, source_path, context)?;
    let window = value.as_mapping().ok_or_else(|| {
        validation_error(source_path, context, "must be a YAML mapping".to_owned())
    })?;
    validate_mapping_keys(window, source_path, context)?;
    require_string_field(window, source_path, context, "name")?;
    require_string_field(window, source_path, context, "path")?;
    require_string_field_if_present(window, source_path, context, "command")?;
    validate_environment_field(window, source_path, context)?;
    if let Some(panes) = window.get("panes") {
        reject_tag(panes, source_path, context)?;
        let panes = panes.as_sequence().ok_or_else(|| {
            validation_error(
                source_path,
                context,
                "panes must be a YAML sequence".to_owned(),
            )
        })?;
        for (index, pane) in panes.iter().enumerate() {
            validate_pane_types(pane, source_path, &format!("{context}.panes[{index}]"), 1)?;
        }
    }
    Ok(())
}

fn validate_pane_types(
    value: &Value,
    source_path: &Path,
    context: &str,
    depth: usize,
) -> Result<(), ConfigError> {
    if depth > MAX_PANE_NESTING_DEPTH {
        return Err(validation_error(
            source_path,
            context,
            format!("pane nesting exceeds the maximum depth of {MAX_PANE_NESTING_DEPTH}"),
        ));
    }
    reject_tag(value, source_path, context)?;
    let pane = value.as_mapping().ok_or_else(|| {
        validation_error(source_path, context, "must be a YAML mapping".to_owned())
    })?;
    validate_mapping_keys(pane, source_path, context)?;
    require_string_field(pane, source_path, context, "pos")?;
    require_string_field_if_present(pane, source_path, context, "id")?;
    require_string_field_if_present(pane, source_path, context, "path")?;
    require_string_field_if_present(pane, source_path, context, "command")?;
    validate_environment_field(pane, source_path, context)?;
    if let Some(panes) = pane.get("panes") {
        reject_tag(panes, source_path, context)?;
        let panes = panes.as_sequence().ok_or_else(|| {
            validation_error(
                source_path,
                context,
                "panes must be a YAML sequence".to_owned(),
            )
        })?;
        for (index, pane) in panes.iter().enumerate() {
            validate_pane_types(
                pane,
                source_path,
                &format!("{context}.panes[{index}]"),
                depth + 1,
            )?;
        }
    }
    Ok(())
}

fn validate_environment_field(
    mapping: &Mapping,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    let Some(environment) = mapping.get("env") else {
        return Ok(());
    };
    reject_tag(environment, source_path, context)?;
    let environment = environment.as_mapping().ok_or_else(|| {
        validation_error(
            source_path,
            context,
            "env must be a YAML mapping".to_owned(),
        )
    })?;
    for (key, value) in environment {
        reject_tag(key, source_path, context)?;
        reject_tag(value, source_path, context)?;
        if !key.is_string() {
            return Err(validation_error(
                source_path,
                context,
                "environment variable names must be strings".to_owned(),
            ));
        }
        if !value.is_string() {
            return Err(validation_error(
                source_path,
                context,
                "invalid type: environment variable values must be strings".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_mapping_keys(
    mapping: &Mapping,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    for key in mapping.keys() {
        reject_tag(key, source_path, context)?;
        if !key.is_string() {
            return Err(validation_error(
                source_path,
                context,
                "mapping keys must be strings".to_owned(),
            ));
        }
    }
    Ok(())
}

fn require_string_field(
    mapping: &Mapping,
    source_path: &Path,
    context: &str,
    field: &str,
) -> Result<(), ConfigError> {
    let Some(value) = mapping.get(field) else {
        return Ok(());
    };
    require_string(value, source_path, context, field)
}

fn require_string_field_if_present(
    mapping: &Mapping,
    source_path: &Path,
    context: &str,
    field: &str,
) -> Result<(), ConfigError> {
    require_string_field(mapping, source_path, context, field)
}

fn require_string(
    value: &Value,
    source_path: &Path,
    context: &str,
    field: &str,
) -> Result<(), ConfigError> {
    reject_tag(value, source_path, context)?;
    if !value.is_string() {
        return Err(validation_error(
            source_path,
            context,
            format!("invalid type for `{field}`; expected a string"),
        ));
    }
    Ok(())
}

fn require_number(
    value: &Value,
    source_path: &Path,
    context: &str,
    field: &str,
) -> Result<(), ConfigError> {
    reject_tag(value, source_path, context)?;
    if !matches!(value, Value::Number(_)) {
        return Err(validation_error(
            source_path,
            context,
            format!("invalid type for `{field}`; expected an integer"),
        ));
    }
    Ok(())
}

fn reject_tag(value: &Value, source_path: &Path, context: &str) -> Result<(), ConfigError> {
    if matches!(value, Value::Tagged(_)) {
        return Err(validation_error(
            source_path,
            context,
            "custom YAML tags are not supported".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn validation_error(path: &Path, context: &str, message: String) -> ConfigError {
    ConfigError::Validation {
        path: path.to_path_buf(),
        context: context.to_owned(),
        message,
    }
}

/// Load and fully validate a `.ws` file.
pub fn load(source_path: &Path) -> Result<ValidatedWorkspaceDefinition, ConfigError> {
    let source = fs::read_to_string(source_path).map_err(|source| ConfigError::Io {
        path: source_path.to_path_buf(),
        source,
    })?;
    let definition = parse(&source, source_path)?;
    validator::validate(definition, source_path)
}
