# Data Model: Configured tmux Workspace CLI

## WorkspaceDefinition

Represents one validated `.ws` document.

| Field | Type | Rules |
|---|---|---|
| `version` | integer | Required; must be `1`. |
| `windows` | ordered list of WindowDefinition | Required for non-empty files; at least one item. |
| source path | filesystem path | The `.ws` file location; relative paths resolve from its parent directory. |

## WindowDefinition

Represents one ordered tmux window.

| Field | Type | Rules |
|---|---|---|
| `name` | string | Required, non-empty, unique within the document. |
| `path` | string | Required, non-empty; resolved and verified before mutation. |
| `command` | optional string | Shell command; missing means the user’s normal shell. |
| `env` | ordered mapping of string to string | Optional; inherited by descendant panes. |
| `panes` | ordered list of PaneDefinition | Optional; child split tree. |

## PaneDefinition

Represents a recursively nested pane.

| Field | Type | Rules |
|---|---|---|
| `pos` | enum | Required; one of `left`, `right`, `top`, `bottom`. |
| `id` | optional string | Non-empty and unique within the containing window when supplied. |
| `path` | optional string | Inherits the parent path when absent. |
| `command` | optional string | Shell command; missing means the normal shell. |
| `env` | optional mapping of string to string | Overrides inherited values by key. |
| `panes` | ordered list of PaneDefinition | Optional; recursively creates child splits. |

## LaunchPlan

An immutable, validated set of effects derived from a WorkspaceDefinition and invocation context.
It contains the target session name, resolved directories, inherited environment maps, ordered
window/pane operations, and selected initial window. It MUST contain no password-store values.

## SessionTarget

A validated tmux session name plus the operation context:

- `UpExisting`: target exists; connect without reading `.ws`.
- `UpCreate`: target is missing; apply `.ws` or the default workspace.
- `ChangeExisting`: target exists; switch the current tmux client without reading `.ws`.
- `DownNamed`: explicitly named target subject to confirmation.
- `DownCurrent`: current tmux session, only when inside tmux and interactively confirmed.

## TerminalContext

Describes whether the process is inside tmux, whether stdin is interactive, and whether a separate
terminal launcher is available. It controls attach, switch, prompt, and recoverable-failure behavior;
it does not change the workspace definition.

## CredentialRequest

A future, narrowly scoped request for one password-store entry. It contains an approved logical
name and destination operation, never the master passphrase or a collection of entries. Credential
requests are outside workspace creation and are fulfilled through `pass`/GPG-agent on demand.

## State transitions

```text
Missing session --ws up--> Validated launch plan --create--> Created session --attach--> Connected
Existing session --ws up--> Separate terminal attach (or recoverable launcher error)
Current session --ws change existing--> Current client switched; previous session remains running
Current session --ws change missing--> Recoverable error; current client unchanged
Any session --ws down confirmed--> Session terminated
Any session --ws down refused--> No mutation
```
