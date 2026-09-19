//! Project- and user-level configuration for the `ws clip NAMESPACE ITEM` credential mapping.
//!
//! Two optional YAML documents are merged to resolve a mapping entry: a user-level file shared
//! across all projects, and a project-local file that may add or override entries for one
//! project. Neither file may contain a live secret value directly — only a `pass` entry path or a
//! fixed, non-secret literal such as a username (see `ws help clip` / `doc/clip-config-lang.md`).

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// The only mapping-language version supported by this implementation.
pub const CURRENT_VERSION: u32 = 1;

/// Name of the optional project-local mapping file, read from the project directory alongside
/// `.ws`.
pub const PROJECT_FILE_NAME: &str = ".ws-clip";

/// One resolved credential source for a `NAMESPACE ITEM` pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipEntry {
    /// Resolved on demand from the password store via `pass show <path>`.
    Pass(String),
    /// A fixed, non-secret value copied as-is (for example, a username).
    Literal(String),
}

/// The merged, validated `(NAMESPACE, ITEM)` -> [`ClipEntry`] mapping used by `ws clip`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipMapping {
    entries: BTreeMap<(String, String), ClipEntry>,
}

impl ClipMapping {
    /// Look up the configured source for one namespace/item pair.
    pub fn resolve(&self, namespace: &str, item: &str) -> Option<&ClipEntry> {
        self.entries.get(&(namespace.to_owned(), item.to_owned()))
    }

    /// Construct a mapping directly from entries, for tests exercising `ws clip` without a
    /// filesystem or environment dependency.
    #[cfg(test)]
    pub(crate) fn from_entries<I>(entries: I) -> Self
    where
        I: IntoIterator<Item = ((String, String), ClipEntry)>,
    {
        Self {
            entries: entries.into_iter().collect(),
        }
    }
}

/// Errors from loading or validating a clip mapping file.
#[derive(Debug, Error)]
pub enum ClipMappingError {
    #[error("{path}: could not read configuration: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("{path}: {context}: {message}")]
    Validation {
        path: PathBuf,
        context: String,
        message: String,
    },
}

fn validation_error(path: &Path, context: &str, message: String) -> ClipMappingError {
    ClipMappingError::Validation {
        path: path.to_path_buf(),
        context: context.to_owned(),
        message,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDocument {
    version: u32,
    entries: BTreeMap<String, BTreeMap<String, RawEntry>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    #[serde(default)]
    pass: Option<String>,
    #[serde(default)]
    literal: Option<String>,
}

/// Parse one mapping document's text into entries, without touching the filesystem.
fn parse(
    source: &str,
    path: &Path,
) -> Result<BTreeMap<(String, String), ClipEntry>, ClipMappingError> {
    if source.trim().is_empty() {
        return Ok(BTreeMap::new());
    }

    let document: RawDocument =
        yaml_serde::from_str(source).map_err(|error| ClipMappingError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    if document.version != CURRENT_VERSION {
        return Err(validation_error(
            path,
            "document",
            format!("version must be {CURRENT_VERSION}"),
        ));
    }

    let mut entries = BTreeMap::new();
    for (namespace, items) in document.entries {
        if namespace.trim().is_empty() {
            return Err(validation_error(
                path,
                "entries",
                "namespace must not be empty".to_owned(),
            ));
        }
        for (item, raw) in items {
            let context = format!("entries.{namespace}.{item}");
            if item.trim().is_empty() {
                return Err(validation_error(
                    path,
                    &context,
                    "item must not be empty".to_owned(),
                ));
            }
            let entry = match (raw.pass, raw.literal) {
                (Some(value), None) if !value.trim().is_empty() => ClipEntry::Pass(value),
                (None, Some(value)) if !value.trim().is_empty() => ClipEntry::Literal(value),
                (Some(_), Some(_)) => {
                    return Err(validation_error(
                        path,
                        &context,
                        "must set exactly one of `pass` or `literal`, not both".to_owned(),
                    ));
                }
                _ => {
                    return Err(validation_error(
                        path,
                        &context,
                        "must set exactly one of `pass` or `literal`".to_owned(),
                    ));
                }
            };
            entries.insert((namespace.clone(), item), entry);
        }
    }
    Ok(entries)
}

/// Load one mapping file. A missing file is treated the same as an empty one, matching the `.ws`
/// convention that absence is a supported default rather than an error.
fn load_file(path: &Path) -> Result<BTreeMap<(String, String), ClipEntry>, ClipMappingError> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => {
            return Err(ClipMappingError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    parse(&source, path)
}

/// Resolve the user-level path from already-read environment values, without touching the
/// filesystem or process environment itself, so the precedence rule stays independently testable.
fn resolve_user_file_path(
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    let config_home = match xdg_config_home {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from(home?).join(".config"),
    };
    Some(config_home.join("ws").join("clip.yaml"))
}

/// The user-level mapping file: `$XDG_CONFIG_HOME/ws/clip.yaml`, falling back to
/// `$HOME/.config/ws/clip.yaml` when `XDG_CONFIG_HOME` is unset or empty. `None` when neither
/// variable is usable.
pub fn user_file_path() -> Option<PathBuf> {
    resolve_user_file_path(
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    )
}

/// The project-level mapping file: [`PROJECT_FILE_NAME`] in `project_dir`.
pub fn project_file_path(project_dir: &Path) -> PathBuf {
    project_dir.join(PROJECT_FILE_NAME)
}

/// Load and merge the user- and project-level mapping files, with project entries overriding
/// user entries for the same `(NAMESPACE, ITEM)` pair (FR-039). Missing files are treated as
/// empty; the result is an empty mapping when neither file exists.
pub fn load(project_dir: &Path) -> Result<ClipMapping, ClipMappingError> {
    let mut entries = BTreeMap::new();
    if let Some(user_path) = user_file_path() {
        entries.extend(load_file(&user_path)?);
    }
    entries.extend(load_file(&project_file_path(project_dir))?);
    Ok(ClipMapping { entries })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{ClipEntry, ClipMappingError, load_file, parse, resolve_user_file_path};

    #[test]
    fn given_an_empty_document_when_parsed_then_it_yields_no_entries() {
        let entries = parse("", Path::new("clip.yaml")).expect("an empty document should parse");
        assert!(entries.is_empty());
    }

    #[test]
    fn given_pass_and_literal_entries_when_parsed_then_both_resolve_to_their_kind() {
        let source = "version: 1\nentries:\n  git:\n    ssh: {pass: repos/github/ssh}\n    user: {literal: octocat}\n";

        let entries = parse(source, Path::new("clip.yaml")).expect("a valid document should parse");

        assert_eq!(
            entries.get(&("git".to_owned(), "ssh".to_owned())),
            Some(&ClipEntry::Pass("repos/github/ssh".to_owned()))
        );
        assert_eq!(
            entries.get(&("git".to_owned(), "user".to_owned())),
            Some(&ClipEntry::Literal("octocat".to_owned()))
        );
    }

    #[test]
    fn given_an_entry_with_both_pass_and_literal_when_parsed_then_it_is_rejected() {
        let source = "version: 1\nentries:\n  git:\n    ssh: {pass: a, literal: b}\n";

        let error = parse(source, Path::new("clip.yaml"))
            .expect_err("an entry with both sources should be rejected");

        assert!(matches!(error, ClipMappingError::Validation { .. }));
    }

    #[test]
    fn given_an_entry_with_neither_pass_nor_literal_when_parsed_then_it_is_rejected() {
        let source = "version: 1\nentries:\n  git:\n    ssh: {}\n";

        let error = parse(source, Path::new("clip.yaml"))
            .expect_err("an entry with no source should be rejected");

        assert!(matches!(error, ClipMappingError::Validation { .. }));
    }

    #[test]
    fn given_an_unsupported_version_when_parsed_then_it_is_rejected() {
        let source = "version: 2\nentries: {}\n";

        let error = parse(source, Path::new("clip.yaml"))
            .expect_err("an unsupported version should be rejected");

        assert!(matches!(error, ClipMappingError::Validation { .. }));
    }

    #[test]
    fn given_an_unknown_top_level_field_when_parsed_then_it_is_rejected() {
        let source = "version: 1\nentries: {}\nextra: true\n";

        let error =
            parse(source, Path::new("clip.yaml")).expect_err("an unknown field should be rejected");

        assert!(matches!(error, ClipMappingError::Parse { .. }));
    }

    #[test]
    fn given_a_missing_file_when_loaded_then_it_yields_no_entries() {
        let project = tempdir().expect("a temporary directory should be available");
        let missing = project.path().join("clip.yaml");

        let entries = load_file(&missing).expect("a missing file should load as empty");

        assert!(entries.is_empty());
    }

    #[test]
    fn given_xdg_config_home_set_when_resolved_then_it_takes_precedence_over_home() {
        let path = resolve_user_file_path(
            Some(OsStr::new("/xdg-config")),
            Some(OsStr::new("/home/example")),
        )
        .expect("a path should resolve when XDG_CONFIG_HOME is set");

        assert_eq!(path, Path::new("/xdg-config/ws/clip.yaml"));
    }

    #[test]
    fn given_xdg_config_home_unset_when_resolved_then_it_falls_back_to_home_dot_config() {
        let path = resolve_user_file_path(None, Some(OsStr::new("/home/example")))
            .expect("a path should resolve when only HOME is set");

        assert_eq!(path, Path::new("/home/example/.config/ws/clip.yaml"));
    }

    #[test]
    fn given_neither_variable_set_when_resolved_then_no_path_is_available() {
        let path = resolve_user_file_path(None, None);

        assert!(path.is_none());
    }
}
