//! Semantic validation, path resolution, and environment inheritance.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::pane_definition::{PaneDefinition, ValidatedPaneDefinition};
use super::parser::{ConfigError, validation_error};

use super::workspace_definition::{
    CURRENT_VERSION, ValidatedWindowDefinition, ValidatedWorkspaceDefinition, WorkspaceDefinition,
};

/// Validate a deserialized workspace and produce immutable launch-ready definitions.
pub fn validate(
    definition: WorkspaceDefinition,
    source_path: &Path,
) -> Result<ValidatedWorkspaceDefinition, ConfigError> {
    let mut windows = Vec::with_capacity(definition.windows.len());
    let mut window_names = HashSet::new();
    let base_path = source_path.parent().unwrap_or_else(|| Path::new("."));

    if definition.version != CURRENT_VERSION {
        return Err(validation_error(
            source_path,
            "document",
            format!("version must be {CURRENT_VERSION}"),
        ));
    }
    if definition.windows.is_empty() {
        return Err(validation_error(
            source_path,
            "windows",
            "must contain at least one window".to_owned(),
        ));
    }

    for (index, window) in definition.windows.into_iter().enumerate() {
        let context = format!("window[{index}]");
        if window.name.trim().is_empty() {
            return Err(validation_error(
                source_path,
                &context,
                "name must not be empty".to_owned(),
            ));
        }
        if !window_names.insert(window.name.clone()) {
            return Err(validation_error(
                source_path,
                &context,
                format!("duplicate window name `{}`", window.name),
            ));
        }

        let path = resolve_directory(&window.path, base_path, source_path, &context)?;
        validate_command(window.command.as_deref(), source_path, &context)?;
        validate_environment(&window.env, source_path, &context)?;

        let mut pane_ids = HashSet::new();
        let inherited_env = window.env.clone();
        let panes = validate_panes(
            window.panes,
            &path,
            &inherited_env,
            source_path,
            &context,
            &mut pane_ids,
        )?;

        windows.push(ValidatedWindowDefinition {
            name: window.name,
            path,
            command: window.command,
            env: inherited_env,
            panes,
        });
    }

    Ok(ValidatedWorkspaceDefinition {
        version: definition.version,
        windows,
        source_path: source_path.to_path_buf(),
    })
}

fn validate_panes(
    panes: Vec<PaneDefinition>,
    parent_path: &Path,
    inherited_env: &BTreeMap<String, String>,
    source_path: &Path,
    parent_context: &str,
    pane_ids: &mut HashSet<String>,
) -> Result<Vec<ValidatedPaneDefinition>, ConfigError> {
    let mut validated = Vec::with_capacity(panes.len());

    for (index, pane) in panes.into_iter().enumerate() {
        let context = format!("{parent_context}.panes[{index}]");
        if let Some(id) = &pane.id {
            if id.trim().is_empty() {
                return Err(validation_error(
                    source_path,
                    &context,
                    "id must not be empty".to_owned(),
                ));
            }
            if !pane_ids.insert(id.clone()) {
                return Err(validation_error(
                    source_path,
                    &context,
                    format!("duplicate pane id `{id}` within window"),
                ));
            }
        }
        validate_command(pane.command.as_deref(), source_path, &context)?;
        validate_environment(&pane.env, source_path, &context)?;

        let path = match pane.path.as_deref() {
            Some(path) => resolve_directory(path, parent_path, source_path, &context)?,
            None => parent_path.to_path_buf(),
        };
        let mut effective_env = inherited_env.clone();
        effective_env.extend(pane.env.clone());
        let children = validate_panes(
            pane.panes,
            &path,
            &effective_env,
            source_path,
            &context,
            pane_ids,
        )?;

        validated.push(ValidatedPaneDefinition {
            pos: pane.pos,
            id: pane.id,
            path,
            command: pane.command,
            env: effective_env,
            panes: children,
        });
    }

    Ok(validated)
}

fn resolve_directory(
    raw_path: &str,
    base_path: &Path,
    source_path: &Path,
    context: &str,
) -> Result<PathBuf, ConfigError> {
    if raw_path.trim().is_empty() {
        return Err(validation_error(
            source_path,
            context,
            "path must not be empty".to_owned(),
        ));
    }

    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        base_path.join(raw_path)
    };
    let resolved = fs::canonicalize(&candidate).map_err(|error| {
        validation_error(
            source_path,
            context,
            format!("path `{raw_path}` cannot be resolved: {error}"),
        )
    })?;
    if !resolved.is_dir() {
        return Err(validation_error(
            source_path,
            context,
            format!("path `{raw_path}` is not a directory"),
        ));
    }
    Ok(resolved)
}

fn validate_command(
    command: Option<&str>,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    if command.is_some_and(|command| command.trim().is_empty()) {
        return Err(validation_error(
            source_path,
            context,
            "command must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_environment(
    environment: &BTreeMap<String, String>,
    source_path: &Path,
    context: &str,
) -> Result<(), ConfigError> {
    for key in environment.keys() {
        let mut characters = key.chars();
        let valid = characters
            .next()
            .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
            && characters.all(|character| character == '_' || character.is_ascii_alphanumeric());
        if !valid {
            return Err(validation_error(
                source_path,
                context,
                format!("invalid environment variable name `{key}`"),
            ));
        }
    }
    Ok(())
}
