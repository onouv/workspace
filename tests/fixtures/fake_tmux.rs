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
//!
//! Like the real tmux, one invocation may chain several commands with a separate `;` argument,
//! and a `-t` target may carry the exact-match prefix `=`. `set-option` stores session user
//! options, and `list-sessions -F` renders `#{session_name}`, `#{session_path}`, and
//! `#{@ws-project-folder}` for each fake session.

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
    for command in args.split(|arg| arg == ";") {
        let result = run_command(&state_dir, &sessions_dir, command);
        if result != ExitCode::SUCCESS {
            return result;
        }
    }
    ExitCode::SUCCESS
}

/// Run one command of a possibly `;`-chained invocation.
fn run_command(state_dir: &Path, sessions_dir: &Path, args: &[String]) -> ExitCode {
    let subcommand = args.first().map(String::as_str);
    if let Ok(fail_on) = env::var("WS_FAKE_TMUX_FAIL_ON")
        && Some(fail_on.as_str()) == subcommand
    {
        return fail(&format!("fake-tmux: forced failure for {fail_on}"));
    }
    let result = match subcommand {
        Some("has-session") => has_session(sessions_dir, args),
        Some("new-session") => new_session(sessions_dir, args),
        Some("new-window") => new_window(sessions_dir, args),
        Some("split-window") => split_window(args),
        Some("send-keys") => ExitCode::SUCCESS,
        Some("attach-session" | "switch-client") => attach_or_switch(sessions_dir, args),
        Some("kill-session") => kill_session(sessions_dir, args),
        Some("set-option") => set_option(state_dir, sessions_dir, args),
        Some("list-sessions") => list_sessions(state_dir, sessions_dir, args),
        Some("detach-client") => ExitCode::SUCCESS,
        Some("display-message") => display_message(),
        Some(other) => fail(&format!("fake-tmux: unsupported command: {other}")),
        None => fail("fake-tmux: no command given"),
    };
    if result == ExitCode::SUCCESS && args.contains(&"-P".to_owned()) {
        println!("%{}", next_pane_id(state_dir));
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

/// The `-t` target without the exact-match prefix `=`.
fn target_argument(args: &[String]) -> Option<&str> {
    let position = args.iter().position(|arg| arg == "-t")?;
    args.get(position + 1)
        .map(|target| target.strip_prefix('=').unwrap_or(target))
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

/// Store a session user option as `<state_dir>/options/<session>/<option>`.
fn set_option(state_dir: &Path, sessions_dir: &Path, args: &[String]) -> ExitCode {
    let Some(target) = target_argument(args) else {
        return fail("fake-tmux: set-option requires -t");
    };
    // Like the real tmux, `set-option -t` takes a pane target, so a bare `NAME` without `:`
    // does not name a session.
    let Some(name) = target.strip_suffix(':').map(session_from_target) else {
        return fail(&format!("no such session: {target}"));
    };
    if !session_marker(sessions_dir, name).exists() {
        return fail(&format!("can't find session: {name}"));
    }
    let (Some(option), Some(value)) = (args.get(args.len() - 2), args.last()) else {
        return fail("fake-tmux: set-option requires an option and a value");
    };
    let options_dir = state_dir.join("options").join(name);
    fs::create_dir_all(&options_dir).expect("the fake tmux options directory should be creatable");
    fs::write(options_dir.join(option), value).expect("the fake tmux option should be writable");
    ExitCode::SUCCESS
}

/// Print one line per session, rendering the supported format variables.
fn list_sessions(state_dir: &Path, sessions_dir: &Path, args: &[String]) -> ExitCode {
    let format = named_argument(args, "-F").unwrap_or("#{session_name}");
    let mut names: Vec<String> = fs::read_dir(sessions_dir)
        .expect("the fake tmux sessions directory should be readable")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    if names.is_empty() {
        return fail("no server running");
    }
    names.sort();
    for name in names {
        let marker = fs::read_to_string(session_marker(sessions_dir, &name)).unwrap_or_default();
        let path = marker.lines().next().unwrap_or("");
        let recorded = fs::read_to_string(
            state_dir
                .join("options")
                .join(&name)
                .join("@ws-project-folder"),
        )
        .unwrap_or_default();
        println!(
            "{}",
            format
                .replace("#{session_name}", &name)
                .replace("#{session_path}", path)
                .replace("#{@ws-project-folder}", &recorded)
        );
    }
    ExitCode::SUCCESS
}
