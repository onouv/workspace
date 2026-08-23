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
