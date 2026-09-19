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

/// Pre-create a fake tmux session marker, as if another invocation had already created it.
pub fn seed_fake_tmux_session(state_dir: &Path, name: &str) {
    let sessions_dir = state_dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir)
        .expect("the fake tmux sessions directory should be creatable");
    std::fs::write(sessions_dir.join(name), "")
        .expect("the fake tmux session marker should be writable");
}
