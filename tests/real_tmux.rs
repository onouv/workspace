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

    fn capture_window(&self, session_name: &str, window_name: &str) -> String {
        let output = Command::new("tmux")
            .arg("-S")
            .arg(&self.socket)
            .args(["capture-pane", "-p", "-t"])
            .arg(format!("={session_name}:{window_name}"))
            .output()
            .expect("tmux capture-pane should run");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn list_windows(&self, session_name: &str, format: &str) -> String {
        let output = Command::new("tmux")
            .arg("-S")
            .arg(&self.socket)
            .args([
                "list-windows",
                "-t",
                &format!("={session_name}"),
                "-F",
                format,
            ])
            .output()
            .expect("tmux list-windows should run");
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

/// A `PATH` folder with only a link to the real `tmux`, so the pane shells cannot find `yazi`,
/// `lazygit`, or `claude` (002 default windows). `HOME` and `SHELL` keep user profiles out.
fn isolated_environment(root: &Path) -> [(&'static str, PathBuf); 3] {
    let tmux = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("tmux"))
        .find(|candidate| candidate.is_file())
        .expect("tmux should be on PATH");
    let path_dir = root.join("path");
    let home = root.join("home");
    std::fs::create_dir_all(&path_dir).expect("the PATH folder should be creatable");
    std::fs::create_dir_all(&home).expect("the HOME folder should be creatable");
    if !path_dir.join("tmux").exists() {
        std::os::unix::fs::symlink(tmux, path_dir.join("tmux"))
            .expect("the tmux link should be creatable");
    }
    [
        ("PATH", path_dir),
        ("HOME", home),
        ("SHELL", PathBuf::from("/bin/sh")),
    ]
}

fn up_without_a_name(folder: &Path, server: &PrivateTmuxServer, launcher_log: &Path) {
    let root = folder.parent().expect("the project folder has a parent");
    support::ws_command()
        .arg("up")
        .current_dir(folder)
        .env("PWD", folder)
        .envs(isolated_environment(root))
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

#[test]
fn given_no_ws_file_when_up_runs_on_real_tmux_then_window_root_is_active() {
    // Scenario: US1-AS3
    if !tmux_is_installed() {
        eprintln!("skipped: tmux is not installed");
        return;
    }
    let root = support::temporary_project();
    let folder = root.path().join("alpha");
    std::fs::create_dir(&folder).expect("the project folder should be creatable");
    let server = PrivateTmuxServer::new(root.path().join("tmux.sock"));
    let launcher_log = root.path().join("launcher.log");

    up_without_a_name(&folder, &server, &launcher_log);

    let windows = server.list_windows("alpha", "#{window_active} #{window_name}");
    assert_eq!(windows, "1 root\n0 file\n0 git\n0 claude\n");
}

#[test]
fn given_the_default_programs_are_missing_when_up_runs_on_real_tmux_then_all_four_windows_exist() {
    // Scenario: US4-AS2
    if !tmux_is_installed() {
        eprintln!("skipped: tmux is not installed");
        return;
    }
    let root = support::temporary_project();
    let folder = root.path().join("alpha");
    std::fs::create_dir(&folder).expect("the project folder should be creatable");
    let server = PrivateTmuxServer::new(root.path().join("tmux.sock"));
    let launcher_log = root.path().join("launcher.log");

    up_without_a_name(&folder, &server, &launcher_log);

    // The shell needs a moment to report the missing program; wait at most 5 seconds.
    let mut file_window = String::new();
    for _ in 0..50 {
        file_window = server.capture_window("alpha", "file");
        if file_window.contains("not found") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let windows = server.list_windows("alpha", "#{window_name}");
    assert_eq!(
        (windows.as_str(), file_window.contains("not found")),
        ("root\nfile\ngit\nclaude\n", true),
        "file window: {file_window}"
    );
}
