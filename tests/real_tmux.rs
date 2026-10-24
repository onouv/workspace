//! Given/When/Then process-level tests against a real tmux server on a private socket.
//!
//! The fake tmux cannot prove how the real tmux parses targets and formats. These tests start a
//! disposable server: `TMUX` names a socket in a temporary folder, so the tests never touch the
//! user's own server. A test returns early with a note when tmux is not installed.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

/// A tmux server on a private socket that is killed when the guard drops, also after a panic.
struct PrivateTmuxServer {
    socket: PathBuf,
}

impl PrivateTmuxServer {
    fn new(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// The `TMUX` value that points every tmux client of `ws` at this server.
    fn tmux_env(&self) -> String {
        format!("{},0,0", self.socket.display())
    }

    fn list_sessions(&self, format: &str) -> String {
        let output = Command::new("tmux")
            .arg("-S")
            .arg(&self.socket)
            .args(["list-sessions", "-F", format])
            .output()
            .expect("tmux list-sessions should run");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for PrivateTmuxServer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .arg("-S")
            .arg(&self.socket)
            .arg("kill-server")
            .output();
    }
}

fn tmux_is_installed() -> bool {
    Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn up_without_a_name(folder: &Path, server: &PrivateTmuxServer, launcher_log: &Path) {
    support::ws_command()
        .arg("up")
        .current_dir(folder)
        .env("PWD", folder)
        .env("TMUX", server.tmux_env())
        .env(
            "WS_TERMINAL_LAUNCHER",
            support::fake_launcher_path().display().to_string(),
        )
        .env("WS_FAKE_LAUNCHER_LOG", launcher_log)
        .assert()
        .success();
}

#[test]
fn given_a_first_window_in_a_subfolder_when_up_runs_twice_without_a_name_then_the_second_run_connects_to_the_same_session()
 {
    // Covers 003 research R4/R5 on real tmux. No acceptance scenario owns this check.
    if !tmux_is_installed() {
        eprintln!("skipped: tmux is not installed");
        return;
    }
    let root = support::temporary_project();
    let folder = root.path().join("alpha");
    std::fs::create_dir_all(folder.join("sub")).expect("the project folders should be creatable");
    std::fs::write(
        folder.join(".ws"),
        "version: 1\nwindows:\n  - name: root\n    path: ./sub\n",
    )
    .expect("the .ws fixture should be writable");
    let server = PrivateTmuxServer::new(root.path().join("tmux.sock"));
    let launcher_log = root.path().join("launcher.log");

    up_without_a_name(&folder, &server, &launcher_log);
    up_without_a_name(&folder, &server, &launcher_log);

    let canonical_folder = std::fs::canonicalize(&folder).expect("the folder should resolve");
    let sessions = server.list_sessions("#{session_name}\t#{@ws-project-folder}");
    assert_eq!(
        sessions,
        format!("alpha\t{}\n", canonical_folder.display()),
        "one session `alpha` that records the project folder, not `./sub`"
    );
}
