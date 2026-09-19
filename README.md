# ws

`ws` is a command-line tool that creates or reconnects to a configured tmux workspace from a
project-local `.ws` file. It replaces hand-typed tmux layout commands with one command per
project, without eagerly touching a password store or nesting tmux clients inside themselves.

## Installation

Build from source with a stable Rust toolchain (edition 2024) and a tmux 3.0+ install on your
`PATH`:

```bash
cargo build --release --bin ws
cp target/release/ws /usr/local/bin/ws   # or anywhere on your PATH
```

## Quick start

From a project directory:

```bash
ws up my-project
```

- If a tmux session named `my-project` already exists, `ws` connects to it directly and never
  reads the local `.ws` file.
- Otherwise, `ws` reads `.ws` in the current directory (or falls back to one default window if
  it's missing or empty), validates it, and creates the session from it.
- Outside tmux, `ws` attaches directly. From inside tmux, it preserves your current session and
  opens the target through a separate terminal client/window instead — see
  [Separate terminal launcher](#separate-terminal-launcher).

## The `.ws` file

A `.ws` file is a small YAML document describing windows and (optionally) recursively split
panes. Run `ws help config` for the complete, authoritative schema and semantic rules — the copy
below is illustrative, not the source of truth:

```yaml
version: 1
windows:
  - name: root
    path: .
    command: bash ./scripts/start-root.sh
  - name: development
    path: ./app
    panes:
      - pos: left
        id: git
        command: git status
      - pos: right
        id: services
        panes:
          - pos: top
            id: logs
            command: bash ./scripts/observe.sh
            env:
              APP_ENV: development
          - pos: bottom
            id: shell
```

- `windows` and nested `panes` keep declaration order; the first window is selected when the
  session opens.
- `left`/`right` are horizontal splits; `top`/`bottom` are vertical splits.
- A window or pane without `command` starts your normal shell.
- `env` values are inherited by descendant panes and can be overridden per pane; they are applied
  to the spawned process only, never written into tmux's own session-level environment table.
- Relative paths resolve against the directory containing `.ws`.

## Commands

| Command | What it does |
|---|---|
| `ws up SESSION_NAME` | Create or reconnect to a workspace (see [Quick start](#quick-start)). |
| `ws change SESSION_NAME` | Switch the current tmux client to an existing session, leaving the previous one running. Requires being inside tmux already; never reads `.ws`. |
| `ws down SESSION_NAME --yes` | Kill a named session non-interactively. Without `--yes`, asks for interactive confirmation; refuses outright without a TTY. |
| `ws exit` | Detach the current tmux client without killing its session. |
| `ws help config` | Print the full `.ws` language reference. |
| `ws --version` | Print the version. |

`ws`, `ws help`, `ws --help`, `ws help config`, and `ws --version` never touch tmux or a
credential store — they work even when both are unavailable.

## Separate terminal launcher

When `ws up` is invoked from inside tmux and the target session isn't the one already attached,
`ws` opens it through a separately configured terminal launcher rather than nesting a tmux client.
Configure one with the `WS_TERMINAL_LAUNCHER` environment variable: a program followed by
space-separated arguments, for example:

```bash
export WS_TERMINAL_LAUNCHER="x-terminal-emulator -e tmux attach-session -t"
```

`ws` appends the target session name as the final argument and never shell-interprets the
configured value. Without a configured launcher, `ws up` reports a recoverable error in that
situation instead of guessing.

## Credentials

`ws` does not read, cache, or export credentials during `up`, `change`, `down`, or `exit`. A
future `ws clip NAMESPACE ITEM` utility is reserved for on-demand, single-entry access through
your existing `pass`/GPG-agent setup, but is not implemented in this MVP — see
[`specs/001-tmux-workspace/spec.md`](specs/001-tmux-workspace/spec.md) for the security design
decision behind that boundary.

## Development

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The full specification, plan, and task breakdown live under
[`specs/001-tmux-workspace/`](specs/001-tmux-workspace/), governed by the project
[constitution](.specify/memory/constitution.md).
