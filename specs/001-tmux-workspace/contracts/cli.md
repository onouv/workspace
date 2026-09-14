# CLI Contract

## Discovery and reference

```text
ws
ws help
ws --help
ws help config
ws --version
```

These commands are side-effect-free: they do not read `.ws`, contact tmux, invoke `pass`/GPG,
prompt, or write credentials.

## Workspace commands

```text
ws up SESSION_NAME
ws change SESSION_NAME
ws down [SESSION_NAME] [--yes]
ws exit
```

### `ws up SESSION_NAME`

- Validates `SESSION_NAME` before external effects.
- Checks for an existing tmux session before reading `.ws`.
- Existing target: connects without reading or applying `.ws`.
- Missing target: validates and applies `.ws`, or creates the default single-window workspace.
- Outside tmux: attaches to the target.
- Inside tmux: preserves the current client and uses a separate terminal client/window; failure
  to launch that context is recoverable.

### `ws change SESSION_NAME`

- Requires an existing target session and an active tmux client.
- Switches the current client to the target.
- Leaves the previous session running in the background.
- Never reads or applies `.ws`.
- A missing target or non-tmux invocation leaves the current context unchanged and returns a
  recoverable error.

### `ws down [SESSION_NAME] [--yes]`

- A named target is required outside tmux.
- Interactive use prompts for confirmation.
- Non-TTY use requires `--yes`.
- `--yes` is scoped to `down`, confirms a validated named target, and does not bypass validation.
- Without a name, only the current tmux session may be selected interactively after confirmation.

### `ws exit`

Detaches the current tmux client without killing its session. It fails clearly outside tmux.

## Reserved utility namespace

```text
ws clip NAMESPACE ITEM
```

The namespace is reserved for future credential-dependent utilities. Detailed entry mappings and
clipboard-provider behavior are specified separately. Such commands must request one entry on
demand and must never print it.

## Exit-status categories

Stable numeric values, defined in `src/error.rs`:

| Status | Code | Category |
|---|---|---|
| `Success` | 0 | The command completed successfully. |
| `InvalidInvocation` | 2 | Invalid CLI input, including an invalid session name. |
| `InvalidConfiguration` | 3 | Invalid `.ws` YAML/schema/semantic content. |
| `DependencyUnavailable` | 4 | Missing or unusable tmux, or no separate terminal launcher configured/reachable. |
| `OperationFailed` | 5 | A workspace operation (tmux command) failed. |
| `DestructiveActionRefused` | 6 | A destructive operation was refused. |
| `CredentialFailure` | 7 | An unavailable or unauthorized credential provider. |

## Separate terminal launcher

`ws up` opens a target session through a separate terminal client/window when invoked inside
tmux, configured via the `WS_TERMINAL_LAUNCHER` environment variable: a program followed by
space-separated arguments (for example, `x-terminal-emulator -e tmux attach-session -t`). `ws`
appends the target session name as the final argument and never shell-interprets the configured
value. When the variable is unset, opening a separate terminal reports the recoverable
`DependencyUnavailable` status instead of nesting a tmux client or switching the current client.

## Output rules

- Human-readable results and help are concise.
- Normal results use stdout.
- Diagnostics, progress, and logs use stderr.
- Secrets never use either stream.

## Supported tmux versions

`ws` requires a tmux new enough to support `-e KEY=VALUE` and `-P -F` on `new-session`,
`new-window`, and `split-window` (tmux 3.0 or later); verified against tmux 3.2a. No version
check is performed at runtime — an incompatible tmux surfaces as an ordinary `OperationFailed`
tmux command failure.

## TTY behavior

Interactive prompts (`ws down` confirmation) require both stdin and stdout to be a real terminal;
a redirected or piped stream on either is treated as non-interactive. Non-interactive `ws down`
without `--yes` refuses rather than hanging. All other commands are unaffected by TTY state.
