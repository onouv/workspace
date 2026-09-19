//! Window creation, naming, working directories, and command launch.

use std::ffi::OsString;

use ws::config::ValidatedWindowDefinition;

use super::client::{TmuxClient, TmuxError};
use super::pane;

/// Materialize one window: create it (as the session's own first window when `is_first`, or an
/// additional window otherwise), then recursively materialize its pane tree.
///
/// A window with no command of its own defers to its first declared pane exactly as a
/// command-less pane defers to its own first child (see [`super::pane`]), so the window's
/// initial tmux pane becomes that pane directly instead of an empty extra pane.
pub fn materialize(
    tmux: &TmuxClient,
    session_name: &str,
    window: &ValidatedWindowDefinition,
    is_first: bool,
) -> Result<(), TmuxError> {
    if window.command.is_none()
        && let Some(resolved) = pane::resolve_root(&window.panes)
    {
        let anchor = create(
            tmux,
            session_name,
            &window.name,
            &resolved.content.path,
            resolved.content.command.as_deref(),
            &resolved.content.env,
            is_first,
        )?;
        return pane::materialize(tmux, &anchor, resolved);
    }

    let anchor = create(
        tmux,
        session_name,
        &window.name,
        &window.path,
        window.command.as_deref(),
        &window.env,
        is_first,
    )?;
    pane::materialize_group(tmux, &anchor, &window.panes)
}

/// Create one tmux window and return its initial pane's tmux id.
///
/// The session's first window is created together with the session itself (`new-session`);
/// every later window is created with `new-window`. Both forms accept the same `-c`/`-e`/
/// trailing-command shape, so only the leading tmux subcommand differs.
fn create(
    tmux: &TmuxClient,
    session_name: &str,
    window_name: &str,
    path: &std::path::Path,
    command: Option<&str>,
    env: &std::collections::BTreeMap<String, String>,
    is_first: bool,
) -> Result<String, TmuxError> {
    let mut args: Vec<OsString> = if is_first {
        vec![
            "new-session".into(),
            "-d".into(),
            "-s".into(),
            session_name.into(),
        ]
    } else {
        vec!["new-window".into(), "-t".into(), session_name.into()]
    };
    args.push("-n".into());
    args.push(window_name.into());
    args.push("-c".into());
    args.push(path.to_string_lossy().into_owned().into());
    for (key, value) in env {
        args.push("-e".into());
        args.push(format!("{key}={value}").into());
    }
    args.push("-P".into());
    args.push("-F".into());
    args.push("#{pane_id}".into());
    let output = tmux.execute(args)?;
    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if let Some(command) = command {
        pane::send_command(tmux, &pane_id, command)?;
    }
    Ok(pane_id)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::process::{ExitStatus, Output};

    use ws::config::{PanePosition, ValidatedPaneDefinition, ValidatedWindowDefinition};

    use super::materialize;
    use crate::tmux::client::{CommandRunner, TmuxClient};

    thread_local! {
        static CALLS: RefCell<Vec<Vec<String>>> = const { RefCell::new(Vec::new()) };
    }

    fn success_status() -> ExitStatus {
        std::os::unix::process::ExitStatusExt::from_raw(0)
    }

    /// Records each call's argv and hands back a fresh, incrementing fake pane id, so a test can
    /// assert both on what was invoked and on how later calls chain off earlier ones.
    fn recording_runner(_: &OsStr, args: &[OsString]) -> io::Result<Output> {
        let id = CALLS.with(|calls| {
            let mut calls = calls.borrow_mut();
            let id = calls.len();
            calls.push(
                args.iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect(),
            );
            id
        });
        Ok(Output {
            status: success_status(),
            stdout: format!("%{id}\n").into_bytes(),
            stderr: Vec::new(),
        })
    }

    fn calls() -> Vec<Vec<String>> {
        CALLS.with(|calls| calls.borrow().clone())
    }

    fn leaf(pos: PanePosition, command: Option<&str>) -> ValidatedPaneDefinition {
        ValidatedPaneDefinition {
            pos,
            id: None,
            path: "/project".into(),
            command: command.map(str::to_owned),
            env: BTreeMap::new(),
            panes: Vec::new(),
        }
    }

    fn junction(pos: PanePosition, panes: Vec<ValidatedPaneDefinition>) -> ValidatedPaneDefinition {
        ValidatedPaneDefinition {
            pos,
            id: None,
            path: "/project".into(),
            command: None,
            env: BTreeMap::new(),
            panes,
        }
    }

    #[test]
    fn given_a_window_with_no_command_or_panes_when_materialized_then_it_uses_new_session() {
        let tmux = TmuxClient::with_runner(recording_runner as CommandRunner);
        let window = ValidatedWindowDefinition {
            name: "root".to_owned(),
            path: "/project".into(),
            command: None,
            env: BTreeMap::new(),
            panes: Vec::new(),
        };

        materialize(&tmux, "target", &window, true).expect("materialization should succeed");

        let calls = calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0][0], "new-session");
        assert!(calls[0].contains(&"target".to_owned()));
        assert!(calls[0].contains(&"root".to_owned()));
    }

    #[test]
    fn given_an_additional_window_when_materialized_then_it_uses_new_window() {
        let tmux = TmuxClient::with_runner(recording_runner as CommandRunner);
        let window = ValidatedWindowDefinition {
            name: "development".to_owned(),
            path: "/project/app".into(),
            command: None,
            env: BTreeMap::new(),
            panes: Vec::new(),
        };

        materialize(&tmux, "target", &window, false).expect("materialization should succeed");

        assert_eq!(calls()[0][0], "new-window");
    }

    #[test]
    fn given_the_canonical_pane_tree_when_materialized_then_no_extra_pane_is_created_for_the_junction()
     {
        // Mirrors the canonical `.ws` example's "development" window: `git` (a leaf) and
        // `services` (a command-less junction deferring to `logs`, then `shell`).
        let tmux = TmuxClient::with_runner(recording_runner as CommandRunner);
        let window = ValidatedWindowDefinition {
            name: "development".to_owned(),
            path: "/project/app".into(),
            command: None,
            env: BTreeMap::new(),
            panes: vec![
                leaf(PanePosition::Left, Some("git status")),
                junction(
                    PanePosition::Right,
                    vec![
                        leaf(PanePosition::Top, Some("bash ./scripts/observe.sh")),
                        leaf(PanePosition::Bottom, None),
                    ],
                ),
            ],
        };

        materialize(&tmux, "target", &window, true).expect("materialization should succeed");

        let calls = calls();
        // Exactly 3 panes are created: the window itself (= `git`), then `logs`, then `shell`.
        // `services` never gets its own tmux pane. A pane's own command follows its creation call
        // as two `send-keys` calls (the literal text, then Enter), rather than being embedded in
        // the creation call itself.
        assert_eq!(calls.len(), 7, "{calls:#?}");
        assert_eq!(calls[0][0], "new-session");
        assert!(!calls[0].iter().any(|arg| arg.contains("git status")));
        assert_eq!(
            calls[1],
            ["send-keys", "-t", "%0", "-l", "--", "git status"]
        );
        assert_eq!(calls[2], ["send-keys", "-t", "%0", "Enter"]);
        assert_eq!(calls[3][0], "split-window");
        assert!(calls[3].contains(&"-h".to_owned()) && !calls[3].contains(&"-b".to_owned()));
        assert_eq!(
            calls[4],
            [
                "send-keys",
                "-t",
                "%3",
                "-l",
                "--",
                "bash ./scripts/observe.sh"
            ]
        );
        assert_eq!(calls[5], ["send-keys", "-t", "%3", "Enter"]);
        assert_eq!(calls[6][0], "split-window");
        assert!(calls[6].contains(&"-v".to_owned()));
    }
}
