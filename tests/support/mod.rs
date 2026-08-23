#![allow(dead_code)]

use assert_cmd::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::{tempdir, TempDir};

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
