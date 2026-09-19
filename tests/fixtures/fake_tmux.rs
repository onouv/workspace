//! A minimal, stateful stand-in for the `tmux` executable, used by process-level integration
//! tests so they can exercise the real `ws` binary without a real tmux server.
//!
//! State (which sessions "exist") is tracked as marker files under a directory named by the
//! `WS_FAKE_TMUX_STATE` environment variable, which the test harness must set. Setting
//! `WS_FAKE_TMUX_FORCE_DUPLICATE` makes every `new-session` call report a duplicate session,
//! deterministically simulating the race described in the specification. Any call passing `-P`
//! (as `ws` does for `new-session`/`new-window`/`split-window`) prints a fresh fake pane id.
//! `WS_FAKE_TMUX_FAIL_ON` names one subcommand (e.g. `split-window`) that fails every time it is
//! invoked, deterministically simulating a setup failure partway through a launch plan.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let state_dir = PathBuf::from(
        env::var_os("WS_FAKE_TMUX_STATE").expect("WS_FAKE_TMUX_STATE must be set by the test"),
    );
    let sessions_dir = state_dir.join("sessions");
    fs::create_dir_all(&sessions_dir)
        .expect("the fake tmux sessions directory should be creatable");

    let args: Vec<String> = env::args().skip(1).collect();
    log_invocation(&state_dir, &args);
    let subcommand = args.first().map(String::as_str);
    if let Ok(fail_on) = env::var("WS_FAKE_TMUX_FAIL_ON")
        && Some(fail_on.as_str()) == subcommand
    {
        return fail(&format!("fake-tmux: forced failure for {fail_on}"));
    }
    let result = match subcommand {
        Some("has-session") => has_session(&sessions_dir, &args),
        Some("new-session") => new_session(&sessions_dir, &args),
        Some("new-window") => new_window(&sessions_dir, &args),
        Some("split-window") => split_window(&args),
        Some("attach-session" | "switch-client") => attach_or_switch(&sessions_dir, &args),
        Some("kill-session") => kill_session(&sessions_dir, &args),
        Some("detach-client") => ExitCode::SUCCESS,
        Some("display-message") => display_message(),
        Some(other) => fail(&format!("fake-tmux: unsupported command: {other}")),
        None => fail("fake-tmux: no command given"),
    };
    if result == ExitCode::SUCCESS && args.contains(&"-P".to_owned()) {
        println!("%{}", next_pane_id(&state_dir));
    }
    result
}

/// Append the invocation's arguments to `<state_dir>/calls.log`, one call per line, so tests can
/// assert on exactly which tmux commands `ws` issued and in what order.
fn log_invocation(state_dir: &Path, args: &[String]) {
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(state_dir.join("calls.log"))
        .expect("the fake tmux call log should be writable");
    writeln!(log, "{}", args.join(" ")).expect("the fake tmux invocation should be recorded");
}

/// A monotonically increasing counter, persisted as a file, standing in for tmux's own `%N`
/// pane ids so successive `-P` calls in one test run return distinct values.
fn next_pane_id(state_dir: &Path) -> u32 {
    let counter_path = state_dir.join("next_pane_id");
    let current: u32 = fs::read_to_string(&counter_path)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);
    fs::write(&counter_path, (current + 1).to_string())
        .expect("the fake tmux pane-id counter should be writable");
    current
}

fn target_argument(args: &[String]) -> Option<&str> {
    let position = args.iter().position(|arg| arg == "-t")?;
    args.get(position + 1).map(String::as_str)
}

fn named_argument<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let position = args.iter().position(|arg| arg == flag)?;
    args.get(position + 1).map(String::as_str)
}

fn session_marker(sessions_dir: &Path, name: &str) -> PathBuf {
    sessions_dir.join(name)
}

/// A `-t` target may be a bare session name (`new-window`/`split-window` against the session
/// created by `new-session`) or `session:window` / `session:window.pane`; only the leading
/// session name matters for this fake's bookkeeping.
fn session_from_target(target: &str) -> &str {
    target.split([':', '.']).next().unwrap_or(target)
}

fn has_session(sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(name) = target_argument(args) else {
        return fail("fake-tmux: has-session requires -t");
    };
    if session_marker(sessions_dir, name).exists() {
        ExitCode::SUCCESS
    } else {
        fail(&format!("can't find session: {name}"))
    }
}

fn new_session(sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(name) = named_argument(args, "-s") else {
        return fail("fake-tmux: new-session requires -s");
    };
    let path = named_argument(args, "-c").unwrap_or(".");
    let window = named_argument(args, "-n").unwrap_or("");

    let forced_duplicate = env::var_os("WS_FAKE_TMUX_FORCE_DUPLICATE").is_some();
    let marker = session_marker(sessions_dir, name);
    if marker.exists() {
        return fail(&format!("duplicate session: {name}"));
    }

    // Simulating a lost race means another invocation created the session first: the marker
    // still appears (as that other invocation's `new-session` would have created it), even
    // though this call reports the same failure the real tmux would report to the loser.
    fs::write(&marker, format!("{path}\n{window}\n"))
        .expect("the fake tmux session marker should be writable");
    if forced_duplicate {
        return fail(&format!("duplicate session: {name}"));
    }
    ExitCode::SUCCESS
}

fn new_window(sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(target) = target_argument(args) else {
        return fail("fake-tmux: new-window requires -t");
    };
    let name = session_from_target(target);
    if session_marker(sessions_dir, name).exists() {
        ExitCode::SUCCESS
    } else {
        fail(&format!("can't find session: {name}"))
    }
}

/// This fake does not model individual panes, only sessions: any split target is accepted, as
/// if it always resolved to a real existing pane. Callers assert on `calls.log` for split-level
/// correctness (direction, path, env, command) rather than on any state kept here.
fn split_window(_: &[String]) -> ExitCode {
    ExitCode::SUCCESS
}

fn attach_or_switch(sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(name) = target_argument(args) else {
        return fail("fake-tmux: attach-session/switch-client requires -t");
    };
    if session_marker(sessions_dir, name).exists() {
        ExitCode::SUCCESS
    } else {
        fail(&format!("can't find session: {name}"))
    }
}

fn kill_session(sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(name) = target_argument(args) else {
        return fail("fake-tmux: kill-session requires -t");
    };
    let marker = session_marker(sessions_dir, name);
    if marker.exists() {
        fs::remove_file(&marker).expect("the fake tmux session marker should be removable");
        ExitCode::SUCCESS
    } else {
        fail(&format!("can't find session: {name}"))
    }
}

fn display_message() -> ExitCode {
    let current = env::var("WS_FAKE_TMUX_CURRENT_SESSION").unwrap_or_default();
    println!("{current}");
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}
