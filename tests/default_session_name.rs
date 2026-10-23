//! Given/When/Then process-level tests for `ws up` without `SESSION_NAME` (feature 003), using a
//! fake `tmux` and a fake separate-terminal launcher.

mod support;

use std::path::{Path, PathBuf};

use predicates::prelude::*;
use tempfile::TempDir;

fn read_calls(state_dir: &Path) -> String {
    std::fs::read_to_string(state_dir.join("calls.log")).unwrap_or_default()
}

/// A temporary parent folder that holds one project folder named `folder_name`.
fn project_folder_named(folder_name: &str) -> (TempDir, PathBuf) {
    let parent = support::temporary_project();
    let folder = parent.path().join(folder_name);
    std::fs::create_dir(&folder).expect("the named project folder should be creatable");
    (parent, folder)
}

fn session_exists(state_dir: &Path, name: &str) -> bool {
    state_dir.join("sessions").join(name).exists()
}

#[test]
fn given_no_session_alpha_when_up_runs_without_a_name_in_alpha_then_it_creates_session_alpha() {
    // Scenario: US1-AS1
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    assert!(
        session_exists(state_dir.path(), "alpha"),
        "{}",
        read_calls(state_dir.path())
    );
}

#[test]
fn given_no_session_when_up_runs_without_a_name_then_stderr_names_the_derived_session_and_stdout_does_not()
 {
    // Scenario: US1-AS2
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success()
        .stderr(predicate::str::contains("using session name `alpha`").count(1))
        .stdout(predicate::str::contains("alpha").not());
}

#[test]
fn given_the_user_is_inside_tmux_when_up_runs_without_a_name_then_it_opens_a_separate_terminal() {
    // Scenario: US1-AS3
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let launcher_log = state_dir.path().join("launcher.log");

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("TMUX", "fake-tmux-socket,123,0")
        .env(
            "WS_TERMINAL_LAUNCHER",
            support::fake_launcher_path().display().to_string(),
        )
        .env("WS_FAKE_LAUNCHER_LOG", &launcher_log)
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(!calls.contains("switch-client"), "{calls}");
    let launcher_calls =
        std::fs::read_to_string(&launcher_log).expect("the launcher log should exist");
    assert!(launcher_calls.contains("alpha"), "{launcher_calls}");
}
