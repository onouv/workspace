//! Given/When/Then process-level tests for materializing a full `.ws` workspace (windows,
//! recursive panes, commands, and environment values) against a fake tmux.

mod support;

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

fn read_calls(state_dir: &std::path::Path) -> String {
    std::fs::read_to_string(state_dir.join("calls.log")).unwrap_or_default()
}

#[test]
fn given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order()
 {
    // Scenario: US2-AS2, US2-AS4, US5-AS1, US5-AS2, US5-AS3
    let project = support::temporary_project();
    std::fs::create_dir(project.path().join("app")).expect("the app directory should exist");
    std::fs::create_dir_all(project.path().join("scripts"))
        .expect("the scripts directory should exist");
    std::fs::write(project.path().join(".ws"), CANONICAL_DOCUMENT)
        .expect("the canonical .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("canonical");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    let lines: Vec<&str> = calls.lines().collect();

    // The first window ("root") is created together with the session, and its command is typed
    // into its pane afterward rather than embedded in the creation call.
    let new_session = lines
        .iter()
        .find(|line| line.starts_with("new-session"))
        .unwrap_or_else(|| panic!("expected a new-session call: {calls}"));
    assert!(new_session.contains("-n root"), "{new_session}");
    assert!(
        !new_session.contains("bash ./scripts/start-root.sh"),
        "{new_session}"
    );

    // The second window ("development") has no command of its own: `git` (its first declared
    // pane) becomes its initial pane directly, via `new-window`.
    let new_window = lines
        .iter()
        .find(|line| line.starts_with("new-window"))
        .unwrap_or_else(|| panic!("expected a new-window call: {calls}"));
    assert!(new_window.contains("-n development"), "{new_window}");
    assert!(!new_window.contains("git status"), "{new_window}");

    // Every configured command is typed into its own pane as two `send-keys` calls: the literal
    // text, then Enter.
    let send_keys: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("send-keys"))
        .collect();
    assert_eq!(send_keys.len(), 6, "{calls}");
    assert!(
        send_keys
            .iter()
            .any(|line| line.contains("-l -- bash ./scripts/start-root.sh")),
        "{calls}"
    );
    assert!(
        send_keys
            .iter()
            .any(|line| line.contains("-l -- git status")),
        "{calls}"
    );
    assert!(
        send_keys
            .iter()
            .any(|line| line.contains("-l -- bash ./scripts/observe.sh")),
        "{calls}"
    );

    // `services` (a command-less junction) never gets its own pane: exactly two splits occur —
    // one that fuses straight to `logs` (its first child), and one for `shell`.
    let splits: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("split-window"))
        .collect();
    assert_eq!(splits.len(), 2, "{calls}");
    assert!(splits[0].contains("-h"), "{}", splits[0]);
    assert!(
        !splits[0].contains("bash ./scripts/observe.sh"),
        "{}",
        splits[0]
    );
    assert!(
        splits[0].contains("-e APP_ENV=development"),
        "{}",
        splits[0]
    );
    assert!(splits[1].contains("-v"), "{}", splits[1]);
    assert!(
        !splits[1].contains("bash ./scripts/observe.sh"),
        "{}",
        splits[1]
    );
}

#[test]
fn given_a_window_environment_when_up_is_invoked_then_it_is_inherited_by_its_pane() {
    // Scenario: US5-AS4
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n    env:\n      WINDOW_VALUE: inherited\n    panes:\n      - pos: left\n        command: git status\n      - pos: right\n        command: printenv\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("env-inherit");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    let split = calls
        .lines()
        .find(|line| line.starts_with("split-window"))
        .unwrap_or_else(|| panic!("expected a split-window call: {calls}"));
    assert!(split.contains("-e WINDOW_VALUE=inherited"), "{split}");
}

#[test]
fn given_a_pane_command_when_up_is_invoked_then_it_is_typed_into_the_pane_instead_of_replacing_its_shell()
 {
    // Scenario: FR-045
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n    command: git status\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("remain-on-exit");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    let lines: Vec<&str> = calls.lines().collect();
    let new_session = lines
        .iter()
        .find(|line| line.starts_with("new-session"))
        .unwrap_or_else(|| panic!("expected a new-session call: {calls}"));
    // The window's own pane keeps running its normal shell — the command is never embedded in
    // the creation call...
    assert!(!new_session.contains("git status"), "{new_session}");
    // ...it is instead typed into that shell as two `send-keys` calls (the literal text, then
    // Enter), so the pane is never at risk from the command exiting: its shell is what tmux is
    // actually watching, and it is still there, with a fresh prompt, once the command finishes.
    let send_keys: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("send-keys"))
        .collect();
    assert_eq!(send_keys.len(), 2, "{calls}");
    assert!(
        send_keys[0].contains("-l -- git status"),
        "{}",
        send_keys[0]
    );
    assert!(send_keys[1].ends_with("Enter"), "{}", send_keys[1]);
}

#[test]
fn given_a_command_with_shell_syntax_when_up_is_invoked_then_it_is_passed_through_as_one_argument()
{
    // Scenario: US5-AS7
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n    command: echo one | cat && echo ~two\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("shell-syntax");

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env_remove("TMUX")
        .assert()
        .success();

    let calls = read_calls(state_dir.path());
    // Generated tmux arguments (session/window/path) never interpolate the shell command string;
    // it reaches tmux as exactly one argument, to be interpreted by the user's own shell.
    assert!(calls.contains("echo one | cat && echo ~two"), "{calls}");
}

#[test]
fn given_a_pane_split_fails_partway_through_when_up_is_invoked_then_only_the_new_session_is_rolled_back()
 {
    // Scenario: US6-AS4
    let project = support::temporary_project();
    std::fs::write(
        project.path().join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: .\n    panes:\n      - pos: left\n        command: git status\n      - pos: right\n        command: printenv\n",
    )
    .expect("the .ws fixture should be writable");
    let path_dir = support::fake_tmux_path_dir();
    let state_dir = support::temporary_project();
    let session_name = support::test_session_name("rollback");
    let pre_existing = support::test_session_name("pre-existing");
    support::seed_fake_tmux_session(state_dir.path(), &pre_existing);

    support::ws_command()
        .args(["up", &session_name])
        .current_dir(project.path())
        .env("PATH", path_dir.path())
        .env("WS_FAKE_TMUX_STATE", state_dir.path())
        .env("WS_FAKE_TMUX_FAIL_ON", "split-window")
        .env_remove("TMUX")
        .assert()
        .failure()
        .code(5);

    let calls = read_calls(state_dir.path());
    assert!(calls.contains("new-session"), "{calls}");
    assert!(calls.contains("split-window"), "{calls}");
    assert!(
        calls.contains(&format!("kill-session -t {session_name}")),
        "the partially created session should be rolled back: {calls}"
    );
    assert!(
        !state_dir
            .path()
            .join("sessions")
            .join(&session_name)
            .exists(),
        "the rolled-back session should no longer exist"
    );
    assert!(
        state_dir
            .path()
            .join("sessions")
            .join(&pre_existing)
            .exists(),
        "a pre-existing, unrelated session must survive the rollback"
    );
}
