//! Given/When/Then process-level tests for side-effect-free CLI discovery.

mod support;

use predicates::prelude::*;

fn temporary_project_with_invalid_config() -> tempfile::TempDir {
    let project = support::temporary_project();
    std::fs::write(project.path().join(".ws"), "not: [valid")
        .expect("the deliberately invalid .ws fixture should be writable");
    project
}

#[test]
fn given_missing_dependencies_when_running_without_arguments_then_prints_help() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: ws"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("help"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn given_missing_dependencies_when_running_help_then_prints_command_interface() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .args(["help"])
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: ws"))
        .stdout(predicate::str::contains("ws up SESSION_NAME"))
        .stdout(predicate::str::contains("ws help config"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn given_missing_dependencies_when_running_global_help_then_uses_stdout() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .arg("--help")
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: ws"))
        .stdout(predicate::str::contains(
            "Start and manage configured tmux workspaces",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn given_missing_dependencies_when_requesting_config_help_then_prints_normative_reference() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .args(["help", "config"])
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration Language Reference"))
        .stdout(predicate::str::contains("version: 1"))
        .stdout(predicate::str::contains("windows:"))
        .stdout(predicate::str::contains("env:"))
        .stdout(predicate::str::contains("left"))
        .stdout(predicate::str::contains("bash ./scripts/start-root.sh"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn given_missing_dependencies_when_requesting_version_then_prints_only_version() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .arg("--version")
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("ws "))
        .stdout(predicate::str::ends_with("\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn given_an_unknown_command_when_invoked_then_returns_helpful_stderr_error() {
    let project = temporary_project_with_invalid_config();

    support::ws_command()
        .args(["not-a-command"])
        .current_dir(project.path())
        .env("PATH", "/path/that/does/not/exist")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("Usage: ws"))
        .stderr(predicate::str::contains("--help"))
        .stdout(predicate::str::is_empty());
}
