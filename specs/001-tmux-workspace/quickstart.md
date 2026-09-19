# Quickstart: Validate `ws`

This guide is for implementation and acceptance testing. It assumes Rust, Cargo, and tmux are
installed. Tests MUST use temporary directories and disposable tmux sessions; do not use personal
sessions or real credentials.

## 1. Build and quality gates

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## 2. Default workspace

From a temporary directory without `.ws`:

```bash
ws up ws-quickstart-default
```

Verify that exactly one tmux window exists, its working directory is the temporary directory, and
its command is the normal shell. Tear it down after testing:

```bash
ws down ws-quickstart-default --yes
```

## 3. Configured workspace

Create a temporary project with `.ws`:

```yaml
version: 1
windows:
  - name: root
    path: .
    command: bash ./scripts/start.sh
  - name: development
    path: ./app
    env:
      APP_ENV: test
    panes:
      - pos: left
        id: git
        command: git status
      - pos: right
        id: shell
```

Add a harmless executable `scripts/start.sh`, create `app`, then run:

```bash
ws up ws-quickstart-configured
```

Verify window order, pane positions, resolved paths, inherited environment, and commands. Use
`ws help config` to compare the local file with the authoritative language reference.

## 4. Existing-session fast path

Create or start a disposable session, then place malformed YAML in `.ws` and run:

```bash
ws up ws-quickstart-existing
```

Verify that the command reconnects successfully and does not parse `.ws`.

## 5. `change` behavior

From inside `SESSION_ONE`, with an existing `OTHER_SESSION`:

```bash
ws change OTHER_SESSION
```

Verify that the current client switches to `OTHER_SESSION` and `SESSION_ONE` remains alive. Repeat
with a missing target and verify that the current client remains unchanged.

## 6. Help and safety checks

Run the following with tmux and `pass` unavailable or mocked:

```bash
ws
ws help
ws --help
ws help config
ws --version
```

All must complete without prompts or external dependency access. Test `ws down` through both an
interactive terminal and redirected/non-TTY input; only `ws down SESSION_NAME --yes` may bypass the
non-TTY confirmation refusal.

Credential tests MUST use a fake `pass` executable and generated values under temporary directories.
No real password-store entries, `.secrets` files, tokens, or private keys belong in fixtures.

## Compatibility findings (recorded during implementation)

Verified with tmux 3.2a on Linux, both through the automated suite (`cargo test --all-targets
--all-features`, which fakes `tmux` and the terminal launcher for deterministic process-level
coverage) and manual runs against a real tmux server with disposable, uniquely named sessions:

- Session, window, and pane creation (scenarios 2 and 3) match the `.ws` file exactly: window
  names, pane counts and split geometry, working directories, commands, and per-pane environment
  values, including the canonical example's `services` junction collapsing into `logs`/`shell`
  with no extra pane. Confirmed via `/proc/<pid>/environ` for a spawned pane process, not just
  `tmux show-environment` (which does not reflect `-e`-scoped process environment).
- `ws down SESSION_NAME --yes` (scenario 6) reliably kills only the named session and leaves
  others running, confirmed against a real tmux server.
- `ws up` outside tmux (scenario 2) and `ws change` (scenario 5) both depend on tmux's own
  `attach-session`/`switch-client` requiring a real, currently-attached terminal client; neither
  can be fully exercised end-to-end from a non-interactive automation shell (no controlling
  terminal), which is a property of tmux itself rather than of `ws`. The automated suite covers
  their decision logic and generated tmux arguments through injected fake runners instead (see
  `src/lifecycle/up.rs` and `src/lifecycle/change.rs` unit tests); a from-a-real-terminal pass of
  scenarios 2 and 5 is still recommended before a release.
- `tmux new-session`/`new-window`/`split-window` all accept `-e KEY=VALUE` and `-P -F
  '#{pane_id}'` on tmux 3.2a as used by `src/tmux/window.rs` and `src/tmux/pane.rs`; no
  minimum-version gate beyond "a tmux new enough to support `-e` on session/window/pane creation
  (3.0+)" is currently enforced or needed.
