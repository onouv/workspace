# CLI Contract Delta: `ws up [SESSION_NAME]`

This file changes the `ws up` section of `specs/001-tmux-workspace/contracts/cli.md`. All other
sections stay in force.

## Synopsis

```text
ws up [SESSION_NAME]
```

## Behavior

- `SESSION_NAME` given: behavior of 001, without change. No message. No folder check.
- `SESSION_NAME` not given:
  1. `ws` makes the derived name from the last component of the project folder (logical path).
     Each `.` and `:` becomes `_`.
  2. `ws` writes one line to stderr:
     `ws: using session name \`NAME\` from the current folder`
  3. Session missing: `ws` creates it as in 001/002 and records the project folder in the tmux
     session option `@ws-project-folder`.
  4. Session exists: `ws` compares the session folder with the project folder (symlinks resolved).
     Same folder: connect. Different or unreadable: fail.

## Diagnostics (stderr)

| Case | Exit | Message (shape) |
|------|------|-----------------|
| No folder name, not text, blank, control character | 2 | `error: invalid invocation: cannot use the current folder name as a session name (<cause>); run \`ws up SESSION_NAME\` with an explicit name` |
| Session folder differs | 5 | `error: workspace operation failed: session \`NAME\` belongs to <session folder>; run \`ws up SESSION_NAME\` with a different name` |
| Session folder unreadable | 5 | `error: workspace operation failed: session \`NAME\` has a folder that cannot be read; run \`ws up SESSION_NAME\` with a different name` |

## stdout

No change. `ws` writes nothing about the derived name to stdout.

## Help text

- `ws up --help`: `SESSION_NAME` is shown as optional. The help says: "Default: the current folder
  name, with `.` and `:` replaced by `_`."
- `ws --help` examples add `ws up`.
