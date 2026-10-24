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

#[test]
fn given_session_alpha_belongs_to_this_folder_when_up_runs_without_a_name_outside_tmux_then_it_attaches_without_reading_ws()
 {
    // Scenario: US2-AS1
    let (_parent, folder) = project_folder_named("alpha");
    std::fs::write(folder.join(".ws"), "not: [valid")
        .expect("the deliberately invalid .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    support::seed_fake_tmux_session_with_folder(state_dir.path(), "alpha", &folder, Some(&folder));

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("list-sessions"), "{calls}");
    assert!(calls.contains("attach-session -t =alpha"), "{calls}");
    assert!(!calls.contains("new-session"), "{calls}");
}

#[test]
fn given_session_alpha_belongs_to_this_folder_when_up_runs_without_a_name_inside_tmux_then_it_opens_a_separate_terminal()
 {
    // Scenario: US2-AS2
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let launcher_log = state_dir.path().join("launcher.log");
    support::seed_fake_tmux_session_with_folder(state_dir.path(), "alpha", &folder, Some(&folder));

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
    assert!(calls.contains("list-sessions"), "{calls}");
    assert!(!calls.contains("new-session"), "{calls}");
    assert!(!calls.contains("switch-client"), "{calls}");
    let launcher_calls =
        std::fs::read_to_string(&launcher_log).expect("the launcher log should exist");
    assert!(launcher_calls.contains("alpha"), "{launcher_calls}");
}

#[test]
fn given_session_alpha_belongs_to_another_folder_when_up_runs_without_a_name_then_it_exits_5_and_does_not_connect()
 {
    // Scenario: US2-AS3
    let (_parent, folder) = project_folder_named("alpha");
    let (_other_parent, other_folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    support::seed_fake_tmux_session_with_folder(
        state_dir.path(),
        "alpha",
        &other_folder,
        Some(&other_folder),
    );

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains(other_folder.display().to_string()))
        .stderr(predicate::str::contains("ws up SESSION_NAME"));

    let calls = read_calls(state_dir.path());
    assert!(!calls.contains("attach-session"), "{calls}");
    assert!(!calls.contains("new-session"), "{calls}");
}

#[test]
fn given_plain_tmux_made_session_alpha_in_this_folder_when_up_runs_without_a_name_then_it_attaches()
{
    // Scenario: US2-AS4
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    support::seed_fake_tmux_session_with_folder(state_dir.path(), "alpha", &folder, None);

    support::ws_command()
        .arg("up")
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("list-sessions"), "{calls}");
    assert!(calls.contains("attach-session -t =alpha"), "{calls}");
}

#[test]
fn given_folder_alpha_when_up_runs_with_beta_then_it_uses_beta_and_shows_no_message() {
    // Scenario: US3-AS1
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();

    support::ws_command()
        .args(["up", "beta"])
        .current_dir(&folder)
        .env("PWD", &folder)
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success()
        .stderr(predicate::str::contains("using session name").not());

    let calls = read_calls(state_dir.path());
    assert!(session_exists(state_dir.path(), "beta"), "{calls}");
    assert!(!session_exists(state_dir.path(), "alpha"), "{calls}");
    assert!(!calls.contains("list-sessions"), "{calls}");
}

#[test]
fn given_an_empty_or_blank_explicit_name_when_up_runs_then_it_exits_2_and_does_not_derive() {
    // Scenario: US3-AS2
    let (_parent, folder) = project_folder_named("alpha");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();

    for explicit_name in ["", "   "] {
        support::ws_command()
            .args(["up", explicit_name])
            .current_dir(&folder)
            .env("PWD", &folder)
            .env("PATH", path_dir.path())
            .env("WS_FAKE_TMUX_STATE", state_dir.path())
            .env_remove("TMUX")
            .assert()
            .failure()
            .code(2)
            .stderr(predicate::str::contains("session name must not be empty"))
            .stderr(predicate::str::contains("using session name").not());
    }

    let calls = read_calls(state_dir.path());
    assert!(calls.is_empty(), "no tmux command should run: {calls}");
}
