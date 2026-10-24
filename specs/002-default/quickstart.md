# Quickstart: Default Workspace File

Validation guide. Exact stderr lines: [contracts/cli.md](./contracts/cli.md). States:
[data-model.md](./data-model.md).

## Prerequisites

- Rust stable, `cargo build --bin ws`.
- tmux 3.x for the manual and real-tmux steps.
- Optional: `yazi`, `lazygit`, `claude` on `PATH`. Without them, the windows show "command not
  found".

## Automated gates

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --test default_workspace
cargo test --test real_tmux
```

Expected: all pass. `real_tmux` prints a skip note when tmux is not installed.

## Manual scenarios

The steps make `demo-*` sessions on your default tmux server. "Clean up" removes them.

```bash
export WS=$PWD/target/debug/ws
```

### 1. Missing file, explicit name (US1, US2)

```bash
cd "$(mktemp -d)"
TMUX= $WS up demo-missing      # attaches; detach with C-b d
cat .ws
tmux list-windows -t demo-missing -F '#{window_index}:#{window_name}'
```

Expected: the file message on stderr. `.ws` equals the spec text. Windows:
`0:root 1:file 2:git 3:claude`. Window `root` is selected on attach.

### 2. Missing file, no name (US1-AS5)

```bash
mkdir -p "$(mktemp -d)/demo-noname" && cd "$_"
TMUX= $WS up
```

Expected: the name message, then the file message. Session `demo-noname` has the four windows.

### 3. Edited file is kept (US2-AS4)

```bash
sed -i 's/name: claude/name: ai/' .ws
TMUX= $WS up demo-edited
```

Expected: no file message. Window 3 is named `ai`. `.ws` keeps the change.

### 4. Empty file (US3-AS3)

```bash
cd "$(mktemp -d)"; printf '  \n' > .ws
TMUX= $WS up demo-empty
```

Expected: one overwrite warning. `.ws` now has the default content.

### 5. Existing session (US3-AS1)

```bash
rm .ws
TMUX= $WS up demo-empty
ls -a
```

Expected: attach to the existing session. No `.ws` is created.

### 6. Read-only folder (US4-AS1)

```bash
d=$(mktemp -d); chmod 555 "$d"; cd "$d"
TMUX= $WS up demo-ro; echo "exit=$?"
```

Expected: the write-failed warning with `permission denied`. Four windows. `exit=0` after detach.
No `.ws` in the folder.

### 7. No file for other commands (US3-AS5)

```bash
cd "$(mktemp -d)"
$WS; $WS help; $WS --help; $WS help config; $WS --version; $WS up 'bad:name'
cd /; $WS up; cd - >/dev/null
ls -a
```

Expected: no `.ws`. `ws up` in `/` fails with exit 2 (003 FR-004). `ws help config` shows the
default content.

## Clean up

```bash
for s in demo-missing demo-noname demo-edited demo-empty demo-ro; do tmux kill-session -t "$s"; done
```
