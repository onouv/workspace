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

#[test]
fn given_no_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window() {
    // Scenario: US4-AS1, US4-AS2
    let project = support::temporary_project();
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("default-missing");

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
    assert!(calls.contains("-n root"), "{calls}");
}

#[test]
fn given_an_empty_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window() {
    // Scenario: US4-AS1, US4-AS2
    let project = support::temporary_project();
    std::fs::write(project.path().join(".ws"), "")
        .expect("the empty .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("default-empty");

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
    assert!(calls.contains("-n root"), "{calls}");
}

#[test]
fn given_tmux_is_unavailable_when_up_is_invoked_then_it_reports_a_recoverable_dependency_error() {
    // Scenario: US4-AS3
    let project = support::temporary_project();

    support::ws_command()
        .args(["up", "target"])
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("tmux"));
}

#[test]
fn given_an_invalid_ws_file_when_up_is_invoked_then_no_session_is_created() {
    // Scenario: US2-AS3, US6-AS1
    let project = support::temporary_project();
    std::fs::write(project.path().join(".ws"), "not: [valid")
        .expect("the deliberately invalid .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("invalid-config");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(3);

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("has-session"), "{calls}");
    assert!(
        !calls.contains("new-session")
            && !calls.contains("new-window")
            && !calls.contains("split-window"),
        "an invalid document must not mutate tmux at all: {calls}"
    );
}

#[test]
fn given_a_referenced_directory_does_not_exist_when_up_is_invoked_then_it_reports_the_declaration_and_creates_no_session()
 {
    // Scenario: US6-AS2
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: ./does-not-exist\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("missing-dir");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("window[0]"));

    let calls = read_calls(state_dir.path());
    assert!(
        !calls.contains("new-session"),
        "an unresolvable path must not create a session: {calls}"
    );
}

#[test]
fn given_a_secret_like_environment_value_when_a_launch_step_fails_then_it_never_reaches_ws_own_output()
 {
    // Scenario: US6-AS3
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n    env:\n      API_TOKEN: sekrit-value-should-not-leak\n    panes:\n      - pos: left\n        command: git status\n      - pos: right\n        command: printenv\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("redaction");

    let assertion = support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("WS_FAKE_TMUX_FAIL_ON", "split-window")
        .env_remove("TMUX")
        .assert()
        .failure();

    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("sekrit-value-should-not-leak"), "{stdout}");
    assert!(!stderr.contains("sekrit-value-should-not-leak"), "{stderr}");
}

#[test]
fn given_an_existing_target_when_change_is_invoked_inside_tmux_then_it_switches_the_client() {
    // Scenario: US7-AS7
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("change-existing");
    support::seed_fake_tmux_session(state_dir.path(), &session_name);

    support::ws_command()
        .args(["change", &session_name])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("TMUX", "fake-tmux-socket,123,0")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(
        calls.contains(&format!("switch-client -t {session_name}")),
        "{calls}"
    );
}

#[test]
fn given_a_missing_target_when_change_is_invoked_then_it_returns_a_recoverable_error() {
    // Scenario: US7-AS8
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("change-missing");

    support::ws_command()
        .args(["change", &session_name])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("TMUX", "fake-tmux-socket,123,0")
        .assert()
        .failure()
        .code(5);

    let calls = read_calls(state_dir.path());
    assert!(!calls.contains("switch-client"), "{calls}");
}

#[test]
fn given_outside_tmux_when_change_is_invoked_then_it_fails_without_contacting_tmux() {
    // Scenario: US7-AS9
    support::ws_command()
        .args(["change", "target"])
        .env("PATH", "/path/that/does/not/exist")
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("inside tmux"));
}

#[test]
fn given_yes_when_down_is_invoked_then_it_kills_the_named_session_without_prompting() {
    // Scenario: US7-AS4
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("down-yes");
    support::seed_fake_tmux_session(state_dir.path(), &session_name);

    support::ws_command()
        .args(["down", &session_name, "--yes"])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(
        calls.contains(&format!("kill-session -t {session_name}")),
        "{calls}"
    );
}

#[test]
fn given_no_yes_and_no_tty_when_down_is_invoked_then_it_refuses() {
    // Scenario: US7-AS3
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("down-no-tty");
    support::seed_fake_tmux_session(state_dir.path(), &session_name);

    support::ws_command()
        .args(["down", &session_name])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(6)
        .stderr(predicate::str::contains("--yes"));

    let calls = read_calls(state_dir.path());
    assert!(!calls.contains("kill-session"), "{calls}");
}

#[test]
fn given_a_missing_target_when_down_is_invoked_then_it_reports_no_such_session() {
    // Scenario: US7-AS5
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("down-missing");

    support::ws_command()
        .args(["down", &session_name, "--yes"])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(5);
}

#[test]
fn given_inside_tmux_when_exit_is_invoked_then_it_detaches_without_killing_the_session() {
    // Scenario: US7-AS1
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();

    support::ws_command()
        .args(["exit"])
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("TMUX", "fake-tmux-socket,123,0")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("detach-client"), "{calls}");
    assert!(!calls.contains("kill-session"), "{calls}");
}

#[test]
fn given_outside_tmux_when_exit_is_invoked_then_it_fails_clearly() {
    support::ws_command()
        .args(["exit"])
        .env("PATH", "/path/that/does/not/exist")
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("tmux client"));
}
