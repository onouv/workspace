//! Given/When/Then tests for the versioned `.ws` YAML language.

use std::fs;
use std::path::Path;

use tempfile::{TempDir, tempdir};
use ws::config::{ConfigError, PanePosition, load, parse};

const CANONICAL_DOCUMENT: &str = r#"version: 1
windows:
  - name: root
    path: .
    command: bash ./scripts/start-root.sh
  - name: development
    path: ./app
    panes:
      - pos: left
        id: git
        command: git status
      - pos: right
        id: services
        panes:
          - pos: top
            id: logs
            command: bash ./scripts/observe.sh
            env:
              APP_ENV: development
          - pos: bottom
            id: shell
"#;

fn project_with_directories() -> TempDir {
    let project = tempdir().expect("a temporary project directory should be available");
    fs::create_dir(project.path().join("app")).expect("the app directory should be available");
    fs::create_dir_all(project.path().join("scripts"))
        .expect("the scripts directory should be available");
    project
}

fn write_workspace(project: &TempDir, source: &str) -> std::path::PathBuf {
    let path = project.path().join(".ws");
    fs::write(&path, source).expect("the .ws fixture should be writable");
    path
}

fn assert_invalid(source: &str, expected: &str) {
    let project = project_with_directories();
    let path = write_workspace(&project, source);
    let error = load(&path).expect_err("the invalid workspace should be rejected");
    assert!(error.to_string().contains(expected), "{error}");
}

/// Builds a flow-style pane nested `levels` deep, e.g. `nested_panes_flow(2)` is
/// `{pos: top, panes: [{pos: top}]}`, which has pane-nesting depth 2.
fn nested_panes_flow(levels: usize) -> String {
    let mut pane = String::from("{pos: top}");
    for _ in 1..levels {
        pane = format!("{{pos: top, panes: [{pane}]}}");
    }
    pane
}

fn document_with_nested_panes(levels: usize) -> String {
    format!(
        "version: 1\nwindows:\n  - name: root\n    path: .\n    panes: [{}]\n",
        nested_panes_flow(levels)
    )
}

#[test]
fn given_the_canonical_document_when_loaded_then_windows_panes_and_commands_are_preserved() {
    let project = project_with_directories();
    let path = write_workspace(&project, CANONICAL_DOCUMENT);

    let definition = load(&path).expect("the canonical document should be valid");

    assert_eq!(definition.version, 1);
    assert_eq!(definition.windows.len(), 2);
    assert_eq!(definition.windows[0].name, "root");
    assert_eq!(
        definition.windows[0].command.as_deref(),
        Some("bash ./scripts/start-root.sh")
    );
    assert_eq!(
        definition.windows[0].path,
        project.path().canonicalize().unwrap()
    );
    assert_eq!(definition.windows[1].name, "development");
    assert_eq!(definition.windows[1].panes.len(), 2);
    assert_eq!(definition.windows[1].panes[0].pos, PanePosition::Left);
    assert_eq!(definition.windows[1].panes[0].id.as_deref(), Some("git"));
    assert_eq!(
        definition.windows[1].panes[0].command.as_deref(),
        Some("git status")
    );

    assert_eq!(definition.windows[1].panes[1].panes.len(), 2);
    assert_eq!(
        definition.windows[1].panes[1].panes[0].pos,
        PanePosition::Top
    );
    assert_eq!(
        definition.windows[1].panes[1].panes[1].pos,
        PanePosition::Bottom
    );
}

#[test]
fn given_window_and_pane_environment_values_when_loaded_then_child_values_are_inherited_and_overridden()
 {
    let project = project_with_directories();
    let path = write_workspace(
        &project,
        "version: 1\nwindows:\n  - name: root\n    path: .\n    env:\n      WINDOW_VALUE: inherited\n      OVERRIDE: window\n    panes:\n      - pos: right\n        env:\n          OVERRIDE: pane\n          PANE_VALUE: local\n        panes:\n          - pos: bottom\n",
    );

    let definition = load(&path).expect("the environment document should be valid");
    let child = &definition.windows[0].panes[0].panes[0];

    assert_eq!(child.env["WINDOW_VALUE"], "inherited");
    assert_eq!(child.env["OVERRIDE"], "pane");
    assert_eq!(child.env["PANE_VALUE"], "local");
    assert_eq!(child.path, project.path().canonicalize().unwrap());
}

#[test]
fn given_an_empty_document_when_loaded_then_the_documented_default_is_returned() {
    let project = project_with_directories();
    let path = write_workspace(&project, "\n  \n");

    let definition = load(&path).expect("an empty .ws should use the default workspace");

    assert_eq!(definition.windows.len(), 1);
    assert_eq!(definition.windows[0].name, "root");
    assert_eq!(
        definition.windows[0].path,
        project.path().canonicalize().unwrap()
    );
}

#[test]
fn given_invalid_yaml_schema_or_semantics_when_loaded_then_the_document_is_rejected() {
    assert_invalid("version: [1", "line");
    assert_invalid("version: 1\nwindows: nope\n", "windows");
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: .\n    env:\n      VALUE: true\n",
        "invalid type",
    );
    assert_invalid("windows:\n  - name: root\n    path: .\n", "version");
    assert_invalid("version: 1\nwindows: []\n", "at least one window");
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: .\n    extra: true\n",
        "unknown field",
    );
    assert_invalid(
        "version: 1\nversion: 1\nwindows:\n  - name: root\n    path: .\n",
        "duplicate",
    );
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: .\n    panes:\n      - pos: diagonal\n",
        "diagonal",
    );
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: .\n    env:\n      BAD-NAME: value\n",
        "invalid environment variable name",
    );
    assert_invalid(
        "version: 2\nwindows:\n  - name: root\n    path: .\n",
        "version must be 1",
    );
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: !custom .\n",
        "tag",
    );
    assert_invalid(
        "version: 1\nwindows:\n  - name: root\n    path: .\n  - name: root\n    path: .\n",
        "duplicate window name",
    );
}

#[test]
fn given_source_errors_when_loaded_then_the_path_and_location_or_declaration_context_are_reported()
{
    let project = project_with_directories();
    let path = write_workspace(&project, "version: [1\nwindows: []\n");
    let error = load(&path).expect_err("malformed YAML should fail");

    match error {
        ConfigError::Parse {
            path: error_path,
            location,
            ..
        } => {
            assert_eq!(error_path, path);
            let location = location.expect("YAML errors should have a source location");
            assert!(location.line >= 1);
            assert!(location.column >= 1);
        }
        other => panic!("expected a source-aware parse error, got {other:?}"),
    }

    let path = write_workspace(
        &project,
        "version: 1\nwindows:\n  - name: root\n    path: missing\n",
    );
    let error = load(&path).expect_err("an unresolved path should fail");
    assert!(error.to_string().contains(path.to_string_lossy().as_ref()));
    assert!(error.to_string().contains("window[0]"));
}

#[test]
fn given_pane_nesting_at_the_maximum_depth_when_loaded_then_the_document_is_accepted() {
    let project = project_with_directories();
    let path = write_workspace(&project, &document_with_nested_panes(32));

    load(&path).expect("nesting at the documented maximum depth should be accepted");
}

#[test]
fn given_pane_nesting_beyond_the_maximum_depth_when_loaded_then_the_document_is_rejected() {
    let project = project_with_directories();
    let path = write_workspace(&project, &document_with_nested_panes(33));

    let error =
        load(&path).expect_err("nesting beyond the documented maximum depth should be rejected");
    assert!(error.to_string().contains("maximum depth of 32"), "{error}");
}

#[test]
fn given_the_published_schema_and_example_when_checked_for_drift_then_the_reference_contains_both()
{
    const SCHEMA: &str = include_str!("../specs/001-tmux-workspace/contracts/ws-schema.yaml");
    const REFERENCE: &str = include_str!("../doc/config-lang.md");

    assert!(REFERENCE.contains(SCHEMA.trim()));
    assert!(REFERENCE.contains(CANONICAL_DOCUMENT.trim()));
}

#[test]
fn given_a_parsed_document_when_parsed_without_filesystem_access_then_only_typed_values_are_returned()
 {
    let definition = parse(
        "version: 1\nwindows:\n  - name: root\n    path: .\n",
        Path::new("/nonexistent/project/.ws"),
    )
    .expect("parsing should not access the filesystem");

    assert_eq!(definition.windows[0].name, "root");
}
