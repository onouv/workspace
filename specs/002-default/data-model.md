# Data Model: Default Workspace File

## DEFAULT_CONTENT (constant)

- Location: `src/config/default_content.rs`, exported from `ws::config`.
- Type: `&'static str`. UTF-8. Ends with `\n`.
- Value: byte-identical to "Default Content (Normative)" in [spec.md](./spec.md).
- Invariant (unit test): `parse` + `validate` accept it. The result has the windows `root`,
  `file`, `git`, `claude` in that order. Each has `path: .`. Only `root` has no command.
- Users: the created file (FR-001), the overwrite (FR-008), the in-memory fallback (FR-009), and
  `parse()` of an empty document.

## SetupFileState (internal enum, `setup_file.rs`)

The state of `project_dir/.ws` when `ws up [SESSION_NAME]` creates a session (research R2).

| Variant | Condition | Action |
|---------|-----------|--------|
| `Missing` | `symlink_metadata` gives `NotFound` | Exclusive create (R3) |
| `Empty` | Regular file, UTF-8, whitespace-only | Checked overwrite (R4) |
| `EmptyThroughSymlink` | Symlink to a whitespace-only file | No write; in-memory + warning |
| `Present` | Any other case | No write; normal load |

## PreparedSetup (`prepared_setup.rs`)

The result of `setup_file::prepare(project_dir: &Path) -> PreparedSetup`. Immutable.

| Field | Type | Meaning |
|-------|------|---------|
| `source` | `SetupSource` | Where the workspace comes from (see below). |
| `notice` | `Option<SetupNotice>` | Line for the `Notifier`, or none. |

Constructor: two fields of different types, so a positional constructor is allowed
(constitution IX).

## SetupSource (`setup_source.rs`)

| Variant | Meaning |
|---------|---------|
| `File(PathBuf)` | Load with `config::load`. |
| `InMemoryDefault { source_path: PathBuf }` | Parse `DEFAULT_CONTENT` with this path. |

## SetupNotice (`setup_notice.rs`)

| Variant | Fields | Spec term | When | FR |
|---------|--------|-----------|------|----|
| `Created` | `path` | file message | Exclusive create succeeded | FR-007 |
| `Overwritten` | `path` | warning | Empty file overwritten | FR-008 |
| `WriteFailed` | `path`, `cause: io::ErrorKind` | warning | Create or overwrite failed | FR-009 |
| `EmptySymlink` | `path` | warning | Symlink to an empty file | Edge case |

`Display` gives the texts in [contracts/cli.md](./contracts/cli.md). There is no variant for
"second writer won": that case loads the other file without a line (spec edge case, R9).

## State transitions

```text
Missing ──create ok──────────────▶ File(.ws)       + Created
Missing ──AlreadyExists──────────▶ re-classify once, no write ─▶ File(.ws), no line
Missing ──other error────────────▶ InMemoryDefault + WriteFailed
Empty ───still empty, write ok───▶ File(.ws)       + Overwritten
Empty ───content appeared────────▶ File(.ws)       (no line)
Empty ───entry replaced (ino)────▶ re-classify once, no write
Empty ───open/write error────────▶ InMemoryDefault + WriteFailed
EmptyThroughSymlink ─────────────▶ InMemoryDefault + EmptySymlink
Present ─────────────────────────▶ File(.ws)       (no line)
```

"Re-classify once" is a single extra pass of R2 without a write. If that pass finds an empty file
from a second process, `config::load` parses it as `DEFAULT_CONTENT`. This is the same workspace,
so the result is correct without a write.

A later failure (tmux error, lost race to a session of a different folder) does not change the
file state. `.ws` stays (spec clarification 2026-10-24).

## Changed existing items

- `config::parse`: whitespace-only source → parse `DEFAULT_CONTENT` (was: one-window default).
- `config::load`: `NotFound` → `ConfigError::Io` (was: empty document).
- `WorkspaceDefinition::default_for`: removed.
