//! Given/When/Then process-level tests for `ws up SESSION_NAME`, using a fake `tmux` and a fake
//! separate-terminal launcher so the real `ws` binary can be exercised without a real tmux server.

mod support;

use predicates::prelude::*;

fn read_calls(state_dir: &std::path::Path) -> String {
    std::fs::read_to_string(state_dir.join("calls.log")).unwrap_or_default()
}

#[test]
fn given_an_existing_session_when_up_is_invoked_outside_tmux_then_it_attaches_without_reading_the_config()
 {
    // Scenario: US3-AS1
    let project = support::temporary_project();
    std::fs::write(project.path().join(".ws"), "not: [valid")
        .expect("the deliberately invalid .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("existing-outside");
    support::seed_fake_tmux_session(state_dir.path(), &session_name);

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("has-session"), "{calls}");
    assert!(calls.contains("attach-session"), "{calls}");
}

#[test]
fn given_an_existing_session_when_up_is_invoked_inside_tmux_then_it_opens_a_separate_terminal_and_preserves_the_current_client()
 {
    // Scenario: US3-AS3, US3-AS4
    let project = support::temporary_project();
    std::fs::write(project.path().join(".ws"), "not: [valid")
        .expect("the deliberately invalid .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("existing-inside");
    support::seed_fake_tmux_session(state_dir.path(), &session_name);
    let launcher_log = state_dir.path().join("launcher.log");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
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
    assert!(calls.contains("has-session"), "{calls}");
    assert!(
        !calls.contains("attach-session") && !calls.contains("switch-client"),
        "the current client must not be attached or switched: {calls}"
    );
    let launcher_calls =
        std::fs::read_to_string(&launcher_log).expect("the launcher log should exist");
    assert!(launcher_calls.contains(&session_name), "{launcher_calls}");
}

#[test]
fn given_a_missing_session_when_up_is_invoked_inside_tmux_then_it_applies_the_config_and_opens_a_separate_terminal()
 {
    // Scenario: US3-AS2
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("missing-inside");
    let launcher_log = state_dir.path().join("launcher.log");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
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
    assert!(calls.contains("has-session"), "{calls}");
    assert!(calls.contains("new-session"), "{calls}");
    assert!(
        std::path::Path::new(state_dir.path())
            .join("sessions")
            .join(&session_name)
            .exists(),
        "the fresh target session should have been created"
    );
    let launcher_calls =
        std::fs::read_to_string(&launcher_log).expect("the launcher log should exist");
    assert!(launcher_calls.contains(&session_name), "{launcher_calls}");
}

#[test]
fn given_a_missing_session_when_up_is_invoked_outside_tmux_then_it_creates_and_attaches_to_a_fresh_session()
 {
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("missing-outside");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("new-session"), "{calls}");
    assert!(calls.contains("attach-session"), "{calls}");
}

#[test]
fn given_the_target_session_race_is_lost_when_up_is_invoked_then_it_reconnects_instead_of_failing()
{
    // Scenario: US3-AS6
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("race");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("WS_FAKE_TMUX_FORCE_DUPLICATE", "1")
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("new-session"), "{calls}");
    assert!(calls.contains("attach-session"), "{calls}");
}

#[test]
fn given_an_invalid_session_name_when_up_is_invoked_then_it_is_rejected_before_any_dependency_access()
 {
    // Scenario: US3-AS5
    let project = support::temporary_project();

    support::ws_command()
        .args(["up", "bad:name"])
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("session name"));
}

#[test]
fn given_no_terminal_launcher_is_configured_when_up_is_invoked_inside_tmux_for_a_missing_target_then_it_fails_recoverably()
 {
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("no-launcher");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("TMUX", "fake-tmux-socket,123,0")
        .env_remove("WS_TERMINAL_LAUNCHER")
        .assert()
        .failure()
        .stderr(predicate::str::contains("terminal launcher"));

    let calls = read_calls(state_dir.path());
    assert!(
        !calls.contains("attach-session") && !calls.contains("switch-client"),
        "the current client must not be attached or switched: {calls}"
    );
}
