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

The implementation MUST document stable numeric values for these categories before implementation:

- invalid invocation or session name
- invalid `.ws` YAML/schema/semantic content
- missing or unusable tmux
- terminal-launcher failure
- workspace operation failure
- refused destructive operation
- unavailable or unauthorized credential provider

## Output rules

- Human-readable results and help are concise.
- Normal results use stdout.
- Diagnostics, progress, and logs use stderr.
- Secrets never use either stream.
