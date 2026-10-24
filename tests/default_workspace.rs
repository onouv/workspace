//! Given/When/Then process-level tests for the default `.ws` file (feature 002), using a fake
//! `tmux` and a fake separate-terminal launcher.

mod support;

use std::path::{Path, PathBuf};

use assert_cmd::assert::Assert;
use tempfile::TempDir;

/// One project folder, a fake tmux on `PATH`, and the fake tmux state folder.
struct Fixture {
    _parent: TempDir,
    project: PathBuf,
    path_dir: TempDir,
    state_dir: TempDir,
}

impl Fixture {
    /// A fixture whose project folder has the name `folder_name`.
    fn in_folder(folder_name: &str) -> Self {
        let parent = support::temporary_project();
        let project = parent.path().join(folder_name);
        std::fs::create_dir(&project).expect("the project folder should be creatable");
        Self {
            _parent: parent,
            project,
            path_dir: support::fake_tmux_path_dir(),
            state_dir: support::temporary_project(),
        }
    }

    fn new() -> Self {
        Self::in_folder("project")
    }

    fn ws_file(&self) -> PathBuf {
        self.project.join(".ws")
    }

    fn canonical_project(&self) -> PathBuf {
        std::fs::canonicalize(&self.project).expect("the project folder should resolve")
    }

    /// `ws` with the given arguments in the project folder, outside tmux.
    fn ws(&self, args: &[&str]) -> assert_cmd::Command {
        let mut command = support::ws_command();
        command
            .args(args)
            .current_dir(&self.project)
            .env("PWD", &self.project)
            .env("PATH", self.path_dir.path())
            .env("WS_FAKE_TMUX_STATE", self.state_dir.path())
            .env_remove("TMUX");
        command
    }

    /// `ws up SESSION_NAME` outside tmux.
    fn up(&self, session_name: &str) -> Assert {
        self.ws(&["up", session_name]).assert()
    }

    fn calls(&self) -> String {
        std::fs::read_to_string(self.state_dir.path().join("calls.log")).unwrap_or_default()
    }

    /// `(window name, start folder)` of each `new-session`/`new-window` call, in call order.
    fn windows(&self) -> Vec<(String, PathBuf)> {
        self.calls()
            .lines()
            .filter(|line| line.starts_with("new-session") || line.starts_with("new-window"))
            .map(|line| {
                let words: Vec<_> = line.split(' ').collect();
                (
                    value_after(&words, "-n").to_owned(),
                    PathBuf::from(value_after(&words, "-c")),
                )
            })
            .collect()
    }

    /// The literal text of each `send-keys -l` call, in call order.
    fn typed_commands(&self) -> Vec<String> {
        self.calls()
            .lines()
            .filter(|line| line.starts_with("send-keys") && line.contains(" -l -- "))
            .map(|line| {
                line.split_once(" -l -- ")
                    .map(|(_, text)| text.to_owned())
                    .unwrap_or_default()
            })
            .collect()
    }
}

fn value_after<'a>(words: &[&'a str], flag: &str) -> &'a str {
    words
        .iter()
        .position(|word| *word == flag)
        .and_then(|index| words.get(index + 1))
        .copied()
        .unwrap_or_default()
}

fn stderr_lines(assert: &Assert) -> Vec<String> {
    String::from_utf8_lossy(&assert.get_output().stderr)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn file_message(ws_file: &Path) -> String {
    format!(
        "ws: created {} with the default workspace",
        ws_file.display()
    )
}

#[test]
fn given_no_ws_file_when_up_runs_then_the_session_has_windows_root_file_git_claude_in_order_in_the_project_folder()
 {
    // Scenario: US1-AS1
    let fixture = Fixture::new();

    fixture
        .up(&support::test_session_name("default-order"))
        .success();

    let project = fixture.canonical_project();
    assert_eq!(
        fixture.windows(),
        [
            ("root".to_owned(), project.clone()),
            ("file".to_owned(), project.clone()),
            ("git".to_owned(), project.clone()),
            ("claude".to_owned(), project),
        ]
    );
}

#[test]
fn given_no_ws_file_when_up_runs_then_file_git_and_claude_get_their_commands_and_root_gets_none() {
    // Scenario: US1-AS2
    let fixture = Fixture::new();

    fixture
        .up(&support::test_session_name("default-commands"))
        .success();

    assert_eq!(fixture.typed_commands(), ["yazi", "lazygit", "claude"]);
}

#[test]
fn given_the_user_is_inside_tmux_and_no_ws_file_when_up_runs_with_another_name_then_the_default_workspace_opens_in_a_separate_terminal()
 {
    // Scenario: US1-AS4
    let fixture = Fixture::new();
    let launcher_log = fixture.state_dir.path().join("launcher.log");
    let session_name = support::test_session_name("default-inside");

    fixture
        .ws(&["up", &session_name])
        .env("TMUX", "fake-tmux-socket,123,0")
        .env(
            "WS_TERMINAL_LAUNCHER",
            support::fake_launcher_path().display().to_string(),
        )
        .env("WS_FAKE_LAUNCHER_LOG", &launcher_log)
        .assert()
        .success();

    let calls = fixture.calls();
    let launcher_calls =
        std::fs::read_to_string(&launcher_log).expect("the launcher log should exist");
    assert!(
        launcher_calls.contains(&session_name)
            && fixture.windows().len() == 4
            && !calls.contains("attach-session")
            && !calls.contains("switch-client"),
        "launcher: {launcher_calls}\ncalls: {calls}"
    );
}

#[test]
fn given_no_ws_file_when_up_runs_without_a_name_then_stderr_shows_the_name_message_then_the_file_message()
 {
    // Scenario: US1-AS5
    let fixture = Fixture::in_folder("alpha");

    let assert = fixture.ws(&["up"]).assert().success();

    assert_eq!(
        stderr_lines(&assert),
        [
            "ws: using session name `alpha` from the current folder".to_owned(),
            file_message(&fixture.ws_file()),
        ]
    );
}

#[test]
fn given_no_ws_file_when_up_runs_then_ws_contains_exactly_the_default_content() {
    // Scenario: US2-AS1
    let fixture = Fixture::new();

    fixture
        .up(&support::test_session_name("default-content"))
        .success();

    let content = std::fs::read(fixture.ws_file()).expect("the created .ws should be readable");
    assert_eq!(content, ws::config::DEFAULT_CONTENT.as_bytes());
}

#[test]
fn given_ws_created_the_file_when_the_file_is_loaded_then_the_normal_loader_gives_the_session_windows()
 {
    // Scenario: US2-AS2
    let fixture = Fixture::new();
    fixture
        .up(&support::test_session_name("default-loader"))
        .success();

    let definition =
        ws::config::load(&fixture.ws_file()).expect("the created .ws should pass the loader");

    let loaded: Vec<_> = definition
        .windows
        .into_iter()
        .map(|window| (window.name, window.path))
        .collect();
    assert_eq!(loaded, fixture.windows());
}

#[test]
fn given_no_ws_file_when_up_runs_then_stderr_has_one_file_message_with_the_path_and_stdout_is_empty()
 {
    // Scenario: US2-AS3
    let fixture = Fixture::new();

    let assert = fixture
        .up(&support::test_session_name("default-message"))
        .success();

    assert_eq!(
        (
            stderr_lines(&assert),
            String::from_utf8_lossy(&assert.get_output().stdout).into_owned(),
        ),
        (vec![file_message(&fixture.ws_file())], String::new())
    );
}

#[test]
fn given_the_user_changed_the_created_file_when_up_runs_with_a_new_name_then_ws_uses_the_change_and_does_not_write_the_file()
 {
    // Scenario: US2-AS4
    let fixture = Fixture::new();
    fixture
        .up(&support::test_session_name("default-first"))
        .success();
    let changed = ws::config::DEFAULT_CONTENT.replace("name: claude", "name: ai");
    std::fs::write(fixture.ws_file(), &changed).expect("the .ws edit should be writable");
    let modified = std::fs::metadata(fixture.ws_file())
        .and_then(|metadata| metadata.modified())
        .expect("the .ws modification time should be readable");

    let assert = fixture
        .up(&support::test_session_name("default-second"))
        .success();

    let last_window = fixture.windows().pop().map(|(name, _)| name);
    let content = std::fs::read_to_string(fixture.ws_file()).expect(".ws should be readable");
    let modified_after = std::fs::metadata(fixture.ws_file())
        .and_then(|metadata| metadata.modified())
        .expect("the .ws modification time should be readable");
    assert_eq!(
        (
            last_window.as_deref(),
            content == changed,
            modified_after == modified,
            stderr_lines(&assert)
        ),
        (Some("ai"), true, true, Vec::<String>::new())
    );
}

fn content_and_modified(path: &Path) -> (Vec<u8>, std::time::SystemTime) {
    let content = std::fs::read(path).expect("the file should be readable");
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .expect("the modification time should be readable");
    (content, modified)
}

#[test]
fn given_the_session_exists_and_ws_is_missing_or_empty_when_up_runs_then_ws_does_not_change() {
    // Scenario: US3-AS1
    let missing = Fixture::new();
    let empty = Fixture::new();
    std::fs::write(empty.ws_file(), "").expect("the empty .ws fixture should be writable");
    let session_name = support::test_session_name("default-existing");
    support::seed_fake_tmux_session(missing.state_dir.path(), &session_name);
    support::seed_fake_tmux_session(empty.state_dir.path(), &session_name);

    missing.up(&session_name).success();
    empty.up(&session_name).success();

    assert_eq!(
        (
            missing.ws_file().exists(),
            std::fs::read_to_string(empty.ws_file()).expect(".ws should be readable"),
        ),
        (false, String::new())
    );
}

#[test]
fn given_a_valid_or_invalid_non_empty_ws_when_up_runs_then_the_content_and_modification_time_do_not_change()
 {
    // Scenario: US3-AS2
    let valid = Fixture::new();
    let invalid = Fixture::new();
    std::fs::write(
        valid.ws_file(),
        "version: 1\nwindows:\n  - name: main\n    path: .\n",
    )
    .expect("the valid .ws fixture should be writable");
    std::fs::write(invalid.ws_file(), "not: [valid")
        .expect("the invalid fixture should be writable");
    let valid_before = content_and_modified(&valid.ws_file());
    let invalid_before = content_and_modified(&invalid.ws_file());

    valid
        .up(&support::test_session_name("default-valid"))
        .success();
    invalid
        .up(&support::test_session_name("default-invalid"))
        .failure()
        .code(3);

    assert_eq!(
        (
            content_and_modified(&valid.ws_file()),
            content_and_modified(&invalid.ws_file())
        ),
        (valid_before, invalid_before)
    );
}

#[test]
fn given_an_empty_ws_when_up_runs_then_ws_overwrites_it_starts_the_default_workspace_and_shows_one_warning()
 {
    // Scenario: US3-AS3
    let fixture = Fixture::new();
    std::fs::write(fixture.ws_file(), "  \n").expect("the empty .ws fixture should be writable");

    let assert = fixture
        .up(&support::test_session_name("default-empty"))
        .success();

    let names: Vec<_> = fixture
        .windows()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        (
            std::fs::read_to_string(fixture.ws_file()).expect(".ws should be readable"),
            names,
            stderr_lines(&assert),
            String::from_utf8_lossy(&assert.get_output().stdout).into_owned(),
        ),
        (
            ws::config::DEFAULT_CONTENT.to_owned(),
            vec![
                "root".to_owned(),
                "file".to_owned(),
                "git".to_owned(),
                "claude".to_owned()
            ],
            vec![format!(
                "ws: warning: replaced the empty content of {} with the default workspace",
                fixture.ws_file().display()
            )],
            String::new(),
        )
    );
}

#[test]
fn given_no_ws_file_when_help_version_or_an_invalid_name_runs_then_no_ws_is_created() {
    // Scenario: US3-AS5
    let fixture = Fixture::new();
    let blank_folder = Fixture::in_folder("   ");
    let foreign = Fixture::in_folder("alpha");
    let other_folder = support::temporary_project();
    support::seed_fake_tmux_session_with_folder(
        foreign.state_dir.path(),
        "alpha",
        other_folder.path(),
        Some(other_folder.path()),
    );

    for args in [
        &[][..],
        &["help"],
        &["--help"],
        &["help", "config"],
        &["--version"],
        &["up", "bad:name"],
    ] {
        // Only the effect on `.ws` matters here, not the exit status of each command.
        let _ = fixture.ws(args).output();
    }
    blank_folder.ws(&["up"]).assert().failure().code(2);
    foreign.ws(&["up"]).assert().failure().code(5);

    assert_eq!(
        [
            fixture.ws_file().exists(),
            blank_folder.ws_file().exists(),
            foreign.ws_file().exists()
        ],
        [false, false, false]
    );
}

#[test]
fn given_no_ws_file_when_change_down_exit_or_clip_runs_then_no_ws_is_created() {
    // 002 FR-006 and SC-004; no acceptance scenario owns this check.
    let fixture = Fixture::new();
    let home = support::temporary_project();
    support::seed_fake_tmux_session(fixture.state_dir.path(), "target");

    for args in [
        &["change", "target"][..],
        &["down", "target", "--yes"],
        &["exit"],
        &["clip", "namespace", "item"],
    ] {
        // Only the effect on `.ws` matters here, not the exit status of each command.
        let _ = fixture
            .ws(args)
            .env("HOME", home.path())
            .env_remove("XDG_CONFIG_HOME")
            .output();
    }

    assert!(!fixture.ws_file().exists());
}

#[test]
fn given_a_read_only_folder_and_no_ws_file_when_up_runs_then_it_warns_starts_four_windows_and_exits_0()
 {
    // Scenario: US4-AS1
    let fixture = Fixture::new();
    let mut permissions = std::fs::metadata(&fixture.project)
        .expect("the project folder should exist")
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&fixture.project, permissions).expect("the mode should be settable");
    if std::fs::write(fixture.project.join("probe"), "").is_ok() {
        eprintln!("skipped: mode bits are ignored for this user");
        return;
    }

    let assert = fixture
        .up(&support::test_session_name("default-read-only"))
        .success();

    assert_eq!(
        (
            stderr_lines(&assert),
            fixture.windows().len(),
            fixture.ws_file().exists()
        ),
        (
            vec![format!(
                "ws: warning: could not write {} (permission denied); using the default workspace without a file",
                fixture.ws_file().display()
            )],
            4,
            false
        )
    );
}

#[test]
fn given_a_folder_that_is_not_a_git_repository_when_up_runs_then_the_git_window_is_created() {
    // Scenario: US4-AS3
    let fixture = Fixture::new();
    assert!(!fixture.project.join(".git").exists());

    fixture
        .up(&support::test_session_name("default-no-git"))
        .success();

    let git_window = fixture
        .windows()
        .into_iter()
        .find(|(name, _)| name == "git");
    assert_eq!(
        git_window,
        Some(("git".to_owned(), fixture.canonical_project()))
    );
}

#[test]
fn given_ws_is_a_directory_when_up_runs_then_it_exits_3_without_a_write() {
    // 002 edge case; no acceptance scenario owns this check.
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.ws_file()).expect("the .ws folder should be creatable");

    fixture
        .up(&support::test_session_name("default-dir"))
        .failure()
        .code(3);

    assert!(fixture.ws_file().is_dir());
}

#[test]
fn given_ws_is_a_broken_symlink_when_up_runs_then_it_exits_3_without_a_write() {
    // 002 edge case; no acceptance scenario owns this check.
    let fixture = Fixture::new();
    let target = fixture.project.join("missing.yaml");
    std::os::unix::fs::symlink(&target, fixture.ws_file()).expect("the link should be creatable");

    fixture
        .up(&support::test_session_name("default-broken"))
        .failure()
        .code(3);

    assert!(!target.exists());
}

#[test]
fn given_ws_is_a_symlink_to_a_non_empty_file_when_up_runs_then_it_loads_the_target_unchanged() {
    // 002 edge case; no acceptance scenario owns this check.
    let fixture = Fixture::new();
    let target = fixture.project.join("shared.yaml");
    std::fs::write(
        &target,
        "version: 1\nwindows:\n  - name: linked\n    path: .\n",
    )
    .expect("the link target should be writable");
    std::os::unix::fs::symlink(&target, fixture.ws_file()).expect("the link should be creatable");
    let before = content_and_modified(&target);

    fixture
        .up(&support::test_session_name("default-linked"))
        .success();

    let names: Vec<_> = fixture
        .windows()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        (names, content_and_modified(&target)),
        (vec!["linked".to_owned()], before)
    );
}

#[test]
fn given_ws_created_the_file_when_tmux_fails_later_then_ws_stays() {
    // 002 clarification 2026-10-24; no acceptance scenario owns this check.
    let fixture = Fixture::new();

    fixture
        .ws(&["up", &support::test_session_name("default-tmux-fails")])
        .env("WS_FAKE_TMUX_FAIL_ON", "new-window")
        .assert()
        .failure();

    let content = std::fs::read(fixture.ws_file()).expect("the created .ws should stay");
    assert_eq!(content, ws::config::DEFAULT_CONTENT.as_bytes());
}

#[test]
fn given_a_gitignore_when_up_creates_ws_then_only_ws_is_added_and_gitignore_does_not_change() {
    // 002 FR-012; no acceptance scenario owns this check.
    let fixture = Fixture::new();
    let gitignore = fixture.project.join(".gitignore");
    std::fs::write(&gitignore, "target/\n").expect("the .gitignore fixture should be writable");
    let before = content_and_modified(&gitignore);

    fixture
        .up(&support::test_session_name("default-gitignore"))
        .success();

    let mut names: Vec<_> = std::fs::read_dir(&fixture.project)
        .expect("the project folder should be readable")
        .map(|entry| entry.expect("a folder entry").file_name())
        .collect();
    names.sort();
    assert_eq!(
        (names, content_and_modified(&gitignore)),
        (vec![".gitignore".into(), ".ws".into()], before)
    );
}
