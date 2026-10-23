# Quickstart: Validate Default Session Name

## Prerequisites

- Rust stable toolchain, tmux 3.x on `PATH`.
- Run from the repository root.

## Automated gates

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Expected: all pass. Each scenario `US<n>-AS<m>` in `spec.md` has one owning test.

## Manual check (real tmux, outside tmux)

1. `mkdir -p /tmp/wsq/alpha /tmp/wsq/other/alpha /tmp/wsq/my.project`
2. `cd /tmp/wsq/alpha && cargo run -q -- up`
   Expected: stderr shows ``using session name `alpha` ``. tmux attaches to `alpha`. Detach.
3. `cargo run -q -- up` again in `/tmp/wsq/alpha`.
   Expected: attaches to the same `alpha`. `tmux ls` shows one `alpha`.
4. `cd /tmp/wsq/other/alpha && cargo run -q -- up; echo $?`
   Expected: exit `5`. The message names `/tmp/wsq/alpha`. No attach.
5. `cd /tmp/wsq/my.project && cargo run -q -- up`
   Expected: session `my_project`.
6. `cd / && cargo run -q -- up; echo $?`
   Expected: exit `2`. The message tells you to run `ws up SESSION_NAME`.
7. `cd /tmp/wsq/alpha && cargo run -q -- up beta`
   Expected: session `beta`. No message about the folder name.
8. Clean up: `tmux kill-session -t =alpha; tmux kill-session -t =my_project; tmux kill-session -t =beta`

## Reference

- Contract: [contracts/cli.md](./contracts/cli.md)
- Data model: [data-model.md](./data-model.md)
