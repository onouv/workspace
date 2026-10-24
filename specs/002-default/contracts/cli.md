# CLI Contract: Default Workspace File

This contract adds to `specs/001-tmux-workspace/contracts/cli.md` and
`specs/003-default-session-name/contracts/cli.md`. It changes no argument, option, or exit code.

## Affected command

`ws up [SESSION_NAME]`, with and without a name, only on the path where the session does not
exist.

## stderr lines

Each line has the `ws: ` prefix (`StderrNotifier`). `<path>` is `project_dir/.ws` as
`Path::display` shows it.

| Spec term | Case | Exact line |
|-----------|------|------------|
| file message | Created (FR-007) | `ws: created <path> with the default workspace` |
| warning | Overwritten (FR-008) | `ws: warning: replaced the empty content of <path> with the default workspace` |
| warning | Write failed (FR-009) | `ws: warning: could not write <path> (<cause>); using the default workspace without a file` |
| warning | Symlink to an empty file | `ws: warning: <path> is a link to an empty file; using the default workspace without a change` |

`<cause>` is the `std::io::ErrorKind` display text, for example `permission denied` or
`read-only file system`. It contains no file content.

Order for `ws up` without a name: the name message (003 FR-006) first, then the file message or
warning (US1-AS5).

A command that loses the file race to a second process shows no file message (spec edge case).

## stdout

No change. Outside tmux: empty. Inside tmux: `opened `<name>` in a separate terminal`.
Nothing about `.ws` goes to stdout (FR-007, FR-008).

## Exit status

No change. A write failure (FR-009) does not change the status. A `.ws` that is present but not
usable (directory, unreadable file, broken symlink) gives exit `3`, as 001 FR-015. A failure after
the file step keeps its usual status, and `.ws` stays.

## Files

- `ws up` writes only `project_dir/.ws` (FR-012).
- During an exclusive create, `ws` makes `project_dir/.ws.<pid>.tmp` and removes it before the
  command continues. A crash between the two steps can leave this file.

## `ws help config`

Shows `doc/config-lang.md`. The rule "A missing or empty `.ws` file uses the default single-window
workspace" is replaced with a section that:

1. Shows the default content verbatim.
2. Says that `ws up` creates `.ws` when it creates a session and `.ws` is missing.
3. Says that `ws up` overwrites an empty `.ws` and shows a warning.
4. Says that `ws` never overwrites a `.ws` that is not empty.
5. Says that `ws` uses the default from memory when it cannot write the file.
