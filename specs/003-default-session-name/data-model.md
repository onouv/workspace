# Data Model: Default Session Name

## SessionNameSource (enum)

How `ws up` got the session name.

| Variant | Data | Folder check | Message |
|---------|------|--------------|---------|
| `Explicit` | validated name | no | no |
| `Derived` | validated derived name | yes (FR-011) | yes (FR-006) |

## DerivedSessionName (immutable value)

| Field | Type | Rule |
|-------|------|------|
| `name` | `String` | Folder name, `.` and `:` replaced with `_`, then valid under the shared session-name rule. |
| `project_folder` | `PathBuf` | Canonical project folder. Used for the folder check and for `@ws-project-folder`. |

Construction validates all rules (constitution IX). Two fields of different types: a plain
constructor is allowed.

## DeriveError (thiserror)

| Variant | Spec | Exit status |
|---------|------|-------------|
| `NoFolderName` | FR-004 (root) | 2 |
| `NotText` | FR-004 | 2 |
| `Blank` | FR-004 | 2 |
| `ControlCharacter` | FR-004 | 2 |
| `CurrentFolderUnreadable` | FR-004 | 2 |

Each message ends with: `run \`ws up SESSION_NAME\` with an explicit name`.

## SessionFolder (read from tmux)

| Field | Source | Rule |
|-------|--------|------|
| `recorded` | `#{@ws-project-folder}` | Used when not empty. |
| `start` | `#{session_path}` | Used when `recorded` is empty. |

## FolderCheck result

| Result | Condition | Action |
|--------|-----------|--------|
| `Match` | canonical(session folder) == canonical(project folder) | connect |
| `Mismatch { session_folder }` | different | fail, exit 5 |
| `Unreadable` | no row, empty folder, or canonicalize fails | fail, exit 5 |

## State flow of `ws up` without a name

```text
derive name ──fail──▶ exit 2 (no tmux contact)
     │
     ▼
notify "using session name `NAME` (from folder)"
     │
     ▼
session exists? ──yes──▶ folder check ──Match──▶ connect
     │                        └─Mismatch/Unreadable──▶ exit 5
     no
     ▼
load .ws, new-session ; set-option @ws-project-folder
     ├─ok──────────────▶ connect
     └─duplicate──────▶ folder check (as above)
```
