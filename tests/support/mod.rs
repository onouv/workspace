#![allow(dead_code)]

use assert_cmd::Command;

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::{TempDir, tempdir};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(0);

pub fn ws_command() -> Command {
    Command::cargo_bin("ws").expect("the ws binary should be available to integration tests")
}

pub fn temporary_project() -> TempDir {
    tempdir().expect("a temporary project directory should be available")
}

pub fn test_session_name(prefix: &str) -> String {
    let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    format!("ws-test-{prefix}-{}-{sequence}", std::process::id())
}

/// A `PATH` directory containing an executable named `tmux` that is really the `fake-tmux`
/// fixture binary, so the real `ws` binary can spawn it by its ordinary resolved name.
pub fn fake_tmux_path_dir() -> TempDir {
    let path_dir = tempdir().expect("a temporary PATH directory should be available");
    let fake_tmux = assert_cmd::cargo::cargo_bin("fake-tmux");
    symlink(&fake_tmux, path_dir.path().join("tmux"))
        .expect("the fake tmux symlink should be creatable");
    path_dir
}

/// The absolute path to the `fake-launcher` fixture binary.
pub fn fake_launcher_path() -> PathBuf {
    assert_cmd::cargo::cargo_bin("fake-launcher")
}

/// The absolute path to the `fake-pass` fixture binary.
pub fn fake_pass_path() -> PathBuf {
    assert_cmd::cargo::cargo_bin("fake-pass")
}

/// The absolute path to the `fake-clipboard` fixture binary.
pub fn fake_clipboard_path() -> PathBuf {
    assert_cmd::cargo::cargo_bin("fake-clipboard")
}

/// A `PATH` directory containing executables named `pass` and `xclip` that are really the
/// `fake-pass` and `fake-clipboard` fixture binaries, so `ws clip` can spawn them by their
/// ordinary resolved names when no explicit `WS_CLIPBOARD_PROVIDER` override is set.
pub fn fake_credential_path_dir() -> TempDir {
    let path_dir = tempdir().expect("a temporary PATH directory should be available");
    symlink(fake_pass_path(), path_dir.path().join("pass"))
        .expect("the fake pass symlink should be creatable");
    symlink(fake_clipboard_path(), path_dir.path().join("xclip"))
        .expect("the fake xclip symlink should be creatable");
    path_dir
}

/// Pre-create a fake tmux session marker, as if another invocation had already created it.
pub fn seed_fake_tmux_session(state_dir: &Path, name: &str) {
    let sessions_dir = state_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)
        .expect("the fake tmux sessions directory should be creatable");
    std::fs::write(sessions_dir.join(name), "")
        .expect("the fake tmux session marker should be writable");
}

/// Write a user-level `ws clip` mapping file under `home_dir/.config/ws/clip.yaml`.
pub fn write_user_clip_mapping(home_dir: &Path, yaml: &str) {
    let config_dir = home_dir.join(".config").join("ws");
    std::fs::create_dir_all(&config_dir)
        .expect("the fake HOME config directory should be creatable");
    std::fs::write(config_dir.join("clip.yaml"), yaml)
        .expect("the user-level clip mapping file should be writable");
}

/// Write a project-level `ws clip` mapping file (`.ws-clip`) into `project_dir`.
pub fn write_project_clip_mapping(project_dir: &Path, yaml: &str) {
    std::fs::write(project_dir.join(".ws-clip"), yaml)
        .expect("the project-level clip mapping file should be writable");
}
