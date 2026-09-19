//! Given/When/Then process-level tests for `ws clip NAMESPACE ITEM`.

mod support;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn given_a_pass_backed_entry_when_clip_is_invoked_then_only_that_entry_is_requested_and_copied() {
    // Scenario: US8-AS1, US9-AS1
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let pass_store = tempdir().expect("a fake pass store directory should be available");
    let pass_log = tempdir().expect("a pass invocation log directory should be available");
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    ssh: {pass: repos/github/ssh}\n    cli: {pass: repos/github/token}\n",
    );
    fs::write(
        pass_store.path().join("repos_github_ssh"),
        "the-ssh-passphrase",
    )
    .expect("the seeded pass entry should be writable");

    support::ws_command()
        .args(["clip", "git", "ssh"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .env("WS_FAKE_PASS_STORE", pass_store.path())
        .env("WS_FAKE_PASS_LOG", pass_log.path().join("invocations"))
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .success()
        .stdout(predicate::str::contains("copied"))
        .stdout(predicate::str::contains("the-ssh-passphrase").not())
        .stderr(predicate::str::is_empty());

    let copied = fs::read_to_string(clip_log.path().join("copied"))
        .expect("the fake clipboard provider should have recorded the copied value");
    assert_eq!(copied, "the-ssh-passphrase");

    let pass_invocations = fs::read_to_string(pass_log.path().join("invocations"))
        .expect("the fake pass log should have recorded exactly one invocation");
    assert_eq!(pass_invocations.trim(), "show repos/github/ssh");
}

#[test]
fn given_a_literal_entry_when_clip_is_invoked_then_pass_is_never_contacted() {
    // Scenario: US9-AS2
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let pass_log = tempdir().expect("a pass invocation log directory should be available");
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    user: {literal: octocat}\n",
    );

    support::ws_command()
        .args(["clip", "git", "user"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        // No WS_FAKE_PASS_STORE is set: if `pass` were contacted, fake-pass would panic reading
        // an unset environment variable and this invocation would fail.
        .env("WS_FAKE_PASS_LOG", pass_log.path().join("invocations"))
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let copied = fs::read_to_string(clip_log.path().join("copied"))
        .expect("the fake clipboard provider should have recorded the copied value");
    assert_eq!(copied, "octocat");
    assert!(
        !pass_log.path().join("invocations").exists(),
        "a literal entry must never contact pass"
    );
}

#[test]
fn given_project_and_user_entries_collide_when_clip_is_invoked_then_the_project_entry_wins() {
    // Scenario: US9-AS3
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  docker:\n    user: {literal: user-level-value}\n",
    );
    support::write_project_clip_mapping(
        project.path(),
        "version: 1\nentries:\n  docker:\n    user: {literal: project-level-value}\n",
    );

    support::ws_command()
        .args(["clip", "docker", "user"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .success();

    let copied = fs::read_to_string(clip_log.path().join("copied"))
        .expect("the fake clipboard provider should have recorded the copied value");
    assert_eq!(copied, "project-level-value");
}

#[test]
fn given_an_unconfigured_entry_when_clip_is_invoked_then_it_reports_not_configured_without_contacting_dependencies()
 {
    // Scenario: US8-AS2, US9-AS4
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    user: {literal: octocat}\n",
    );

    support::ws_command()
        .args(["clip", "docker", "token"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("docker token"))
        .stderr(predicate::str::contains("ws help clip"));

    assert!(
        !clip_log.path().join("copied").exists(),
        "an unconfigured entry must never reach the clipboard provider"
    );
}

#[test]
fn given_no_mapping_files_when_clip_is_invoked_then_it_reports_not_configured() {
    // Scenario: US9-AS6
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();

    support::ws_command()
        .args(["clip", "git", "ssh"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no clip entry configured"))
        .stderr(predicate::str::contains("ws help clip"));
}

#[test]
fn given_a_malformed_mapping_file_when_clip_is_invoked_then_it_reports_the_problem_without_contacting_dependencies()
 {
    // Scenario: US9-AS5
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    ssh: {pass: a, literal: b}\n",
    );

    support::ws_command()
        .args(["clip", "git", "ssh"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("clip.yaml"));

    assert!(
        !clip_log.path().join("copied").exists(),
        "a malformed mapping file must never reach the clipboard provider"
    );
}

#[test]
fn given_no_clipboard_provider_configured_when_clip_is_invoked_then_it_uses_xclip_by_default() {
    // Scenario: US9-AS7
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let path_dir = support::fake_credential_path_dir();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    user: {literal: octocat}\n",
    );

    support::ws_command()
        .args(["clip", "git", "user"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", path_dir.path())
        .env_remove("WS_CLIPBOARD_PROVIDER")
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .success();

    let copied = fs::read_to_string(clip_log.path().join("copied"))
        .expect("the default `xclip` provider (resolved from PATH) should have recorded the value");
    assert_eq!(copied, "octocat");
}

#[test]
fn given_a_configured_clipboard_provider_when_clip_is_invoked_then_it_is_used_instead_of_the_default()
 {
    // Scenario: US9-AS8
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    user: {literal: octocat}\n",
    );

    support::ws_command()
        .args(["clip", "git", "user"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        // No `xclip` on PATH at all: only the explicitly configured provider can succeed.
        .env("PATH", "/path/that/does/not/exist")
        .env(
            "WS_CLIPBOARD_PROVIDER",
            support::fake_clipboard_path().display().to_string(),
        )
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .assert()
        .success();

    let copied = fs::read_to_string(clip_log.path().join("copied"))
        .expect("the configured provider should have recorded the value");
    assert_eq!(copied, "octocat");
}

#[test]
fn given_a_refusing_clipboard_provider_when_clip_is_invoked_then_the_value_never_reaches_stdout_or_stderr()
 {
    let home = tempdir().expect("a fake HOME directory should be available");
    let project = support::temporary_project();
    let clip_log = tempdir().expect("a clipboard log directory should be available");

    support::write_user_clip_mapping(
        home.path(),
        "version: 1\nentries:\n  git:\n    user: {literal: super-secret-value}\n",
    );

    support::ws_command()
        .args(["clip", "git", "user"])
        .current_dir(project.path())
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("PATH", "/path/that/does/not/exist")
        .env(
            "WS_CLIPBOARD_PROVIDER",
            support::fake_clipboard_path().display().to_string(),
        )
        .env("WS_FAKE_CLIPBOARD_LOG", clip_log.path().join("copied"))
        .env("WS_FAKE_CLIPBOARD_FAIL", "1")
        .assert()
        .failure()
        .stdout(predicate::str::contains("super-secret-value").not())
        .stderr(predicate::str::contains("super-secret-value").not());
}
