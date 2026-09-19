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
- A window or pane without `command` starts your normal shell. One with `command` also starts your
  normal shell, then types `command` into it followed by Enter, so aliases and functions from your
  shell's startup files are available and the pane is still there, with a fresh prompt, once the
  command finishes.
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
| `ws clip NAMESPACE ITEM` | Send one configured credential to the clipboard (see [Credentials](#credentials)). |
| `ws help config` | Print the full `.ws` language reference. |
| `ws help clip` | Print the full `ws clip` credential-mapping reference. |
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

`ws` does not read, cache, or export credentials during `up`, `change`, `down`, or `exit`.
`ws clip NAMESPACE ITEM` is the on-demand, single-entry exception: it resolves exactly one
configured credential through your existing `pass`/GPG-agent setup and sends it straight to the
clipboard, never printing it — see
[`specs/001-tmux-workspace/spec.md`](specs/001-tmux-workspace/spec.md) for the full security
design decision.

`NAMESPACE ITEM` (for example `git ssh` or `docker token`) is looked up in a mapping merged from
two optional YAML files — run `ws help clip` for the complete schema, this is a summary:

- **User-level** (available to every project): `$XDG_CONFIG_HOME/ws/clip.yaml`, or
  `$HOME/.config/ws/clip.yaml` when `XDG_CONFIG_HOME` is unset.
- **Project-level** (specific to one project, alongside `.ws`): `.ws-clip` in the project
  directory. A project-level entry overrides a user-level entry for the same `NAMESPACE ITEM`.

Either file may be absent — a missing file is treated as an empty mapping, not an error. Each
entry sets exactly one of `pass` (a `pass` store path, resolved on demand) or `literal` (a fixed,
non-secret value such as a username, copied without contacting `pass`). Neither file may contain a
live credential value, so both are safe to keep in ordinary dotfiles:

```yaml
# ~/.config/ws/clip.yaml
version: 1
entries:
  git:
    ssh: {pass: repos/github/ssh/my-key}
    cli: {pass: repos/github/token}
    user: {literal: my-username}
    password: {pass: repos/github/my-account}
  docker:
    token: {pass: registries/dockerhub/tokens/build}
    user: {literal: my-username}
    password: {pass: registries/dockerhub/my-account}
```

`ws clip` sends the resolved value to an external clipboard provider, configured through
`WS_CLIPBOARD_PROVIDER` using the same "program followed by space-separated arguments" convention
as `WS_TERMINAL_LAUNCHER` (for example `wl-copy` on Wayland, or `pbcopy` on macOS). When unset,
`ws` uses `xclip -selection clipboard`.

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
