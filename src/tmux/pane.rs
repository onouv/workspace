//! Recursive pane splitting, environment application, and command execution, using tmux
//! defaults for pane dimensions.
//!
//! A pane with no command of its own that only exists to describe further splits never gets its
//! own tmux pane; its position is inherited by its first declared child, recursively, so a
//! document like the canonical example's `services` junction materializes exactly the panes it
//! describes (`logs`, `shell`) rather than an extra, content-less pane in between.

use std::ffi::OsString;

use ws::config::{PanePosition, ValidatedPaneDefinition};

use super::client::{TmuxClient, TmuxError};

/// A pane position resolved through any leading chain of command-less junction panes.
pub struct ResolvedPane<'a> {
    /// The pane definition whose command/path/env become this position's actual content.
    pub content: &'a ValidatedPaneDefinition,
    /// Sibling lists to chain-split off the pane created for `content`, outermost first.
    chained: Vec<&'a [ValidatedPaneDefinition]>,
}

/// Resolve `siblings[0]` (with `siblings[1..]` as its own remaining siblings), if `siblings` is
/// non-empty.
pub fn resolve_root(siblings: &[ValidatedPaneDefinition]) -> Option<ResolvedPane<'_>> {
    let (first, rest) = siblings.split_first()?;
    Some(resolve(first, rest))
}

fn resolve<'a>(
    node: &'a ValidatedPaneDefinition,
    siblings: &'a [ValidatedPaneDefinition],
) -> ResolvedPane<'a> {
    let mut content = node;
    let mut chained = vec![siblings];
    while content.command.is_none() {
        match content.panes.split_first() {
            Some((next, rest)) => {
                chained.push(rest);
                content = next;
            }
            None => break,
        }
    }
    if !content.panes.is_empty() {
        chained.push(&content.panes);
    }
    ResolvedPane { content, chained }
}

/// Materialize everything still pending for a position already created as `anchor`, with
/// `resolved.content`'s settings already baked in by the caller (a window-creation call, for a
/// window's own pane tree; see [`super::window`]).
pub fn materialize(
    tmux: &TmuxClient,
    anchor: &str,
    resolved: ResolvedPane<'_>,
) -> Result<(), TmuxError> {
    for group in resolved.chained {
        materialize_group(tmux, anchor, group)?;
    }
    Ok(())
}

/// Chain-split `group`'s panes off `anchor`, in declared order. Also used directly for a window
/// that has its own command, so all of its declared panes are fresh splits (`group` is then the
/// window's whole pane list).
pub fn materialize_group(
    tmux: &TmuxClient,
    anchor: &str,
    group: &[ValidatedPaneDefinition],
) -> Result<(), TmuxError> {
    let Some((first, rest)) = group.split_first() else {
        return Ok(());
    };
    let resolved = resolve(first, rest);
    // The split direction comes from `first` — the position actually being materialized (for
    // example `services: right`) — never from `resolved.content`, which may be a deferred
    // descendant (`logs: top`) contributing only its path/command/env, not its own position.
    let new_anchor = split(tmux, anchor, first.pos, resolved.content)?;
    materialize(tmux, &new_anchor, resolved)
}

/// Split `anchor` to create one new pane at `pos`, with `content`'s path, command, and
/// environment. Returns the new pane's tmux id, used as the anchor for anything split off it in
/// turn.
fn split(
    tmux: &TmuxClient,
    anchor: &str,
    pos: PanePosition,
    content: &ValidatedPaneDefinition,
) -> Result<String, TmuxError> {
    let (direction, before) = split_flags(pos);
    let mut args: Vec<OsString> = vec![
        "split-window".into(),
        "-t".into(),
        anchor.into(),
        direction.into(),
    ];
    if before {
        args.push("-b".into());
    }
    args.push("-c".into());
    args.push(content.path.to_string_lossy().into_owned().into());
    for (key, value) in &content.env {
        args.push("-e".into());
        args.push(format!("{key}={value}").into());
    }
    args.push("-P".into());
    args.push("-F".into());
    args.push("#{pane_id}".into());
    if let Some(command) = &content.command {
        args.push("--".into());
        args.push(command.clone().into());
    }
    let output = tmux.execute(args)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// `left`/`right` are horizontal splits; `top`/`bottom` are vertical splits, each placing the
/// new pane before (left/top) or after (right/bottom) the pane it splits, per the specification.
const fn split_flags(pos: PanePosition) -> (&'static str, bool) {
    match pos {
        PanePosition::Left => ("-h", true),
        PanePosition::Right => ("-h", false),
        PanePosition::Top => ("-v", true),
        PanePosition::Bottom => ("-v", false),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ws::config::{PanePosition, ValidatedPaneDefinition};

    use super::resolve_root;

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
    fn given_two_leaf_siblings_when_resolved_then_the_first_is_its_own_content() {
        let siblings = vec![
            leaf(PanePosition::Left, Some("git status")),
            leaf(PanePosition::Right, None),
        ];

        let resolved = resolve_root(&siblings).expect("a non-empty list should resolve");

        assert_eq!(resolved.content.command.as_deref(), Some("git status"));
    }

    #[test]
    fn given_a_command_less_junction_when_resolved_then_it_defers_to_its_first_child() {
        // Mirrors the canonical example's `services` pane: no command of its own, so its
        // position is inherited by `logs`, its first declared child.
        let siblings = vec![junction(
            PanePosition::Right,
            vec![
                leaf(PanePosition::Top, Some("bash ./scripts/observe.sh")),
                leaf(PanePosition::Bottom, None),
            ],
        )];

        let resolved = resolve_root(&siblings).expect("a non-empty list should resolve");

        assert_eq!(
            resolved.content.command.as_deref(),
            Some("bash ./scripts/observe.sh")
        );
        // Both `shell` (the junction's second child) and the junction's own (empty) sibling
        // list are pending as further splits off the pane created for `logs`.
        assert_eq!(resolved.chained.len(), 2);
    }
}
