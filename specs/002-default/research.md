# Research: Default Workspace File

Each item gives the decision, the rationale, and the alternatives.

## R1. One source for the default content

- **Decision**: Add `src/config/default_content.rs` with `pub const DEFAULT_CONTENT: &str`. The
  text is byte-identical to the spec block and ends with `\n`. `parse()` of a whitespace-only
  source parses `DEFAULT_CONTENT`. Remove `WorkspaceDefinition::default_for`.
- **Rationale**: The created file, the in-memory fallback (FR-009), and an empty document all use
  the same text (Key Entity "Default Content"). A unit test proves that the constant passes the
  normal loader and has four windows (FR-002, FR-003).
- **Alternatives**: Build the default as a Rust value and serialize it. Rejected: the comment
  lines are part of the contract, and serialization drops them.

## R2. Classify `.ws` before the load

- **Decision**: Use `fs::symlink_metadata(project_dir/.ws)`:
  - `NotFound` → **Missing**.
  - Symlink → **Present**. Load it. If the target is empty, use the in-memory default and warn
    (spec edge case). Never write.
  - Regular file → read it. Whitespace-only UTF-8 → **Empty**. Otherwise → **Present**.
  - Directory, other type, or read error → **Present**. The loader reports it (001 FR-015).
- **Rationale**: `fs::read_to_string` follows symlinks, so today a broken symlink loads as the
  default. `symlink_metadata` gives the spec meaning of "missing". Change `config::load` so that
  `NotFound` is an `Io` error, not an empty document. A broken symlink then fails with a clear
  message (exit 3).
- **Alternatives**: Keep `NotFound → empty` in `load`. Rejected: it hides a broken symlink.

## R3. Exclusive create of a missing file

- **Decision**: Write `DEFAULT_CONTENT` to a temporary file `.ws.<pid>.tmp` in the project folder
  (`create_new`). Then `fs::hard_link(temp, .ws)`. Remove the temporary file in all cases. If
  `hard_link` gives `AlreadyExists`, a second process won: re-classify once (R2) and load its file
  (FR-005). If it gives `Unsupported` (file system without hard links), use
  `OpenOptions::create_new` on `.ws` and write directly. Any other error → fallback (R5).
- **Rationale**: `hard_link` fails when the target exists, also for a dangling symlink. The file
  appears with its full content in one step. A concurrent `ws up` never reads a partial file.
- **Alternatives**: `create_new` + `write_all` only. Rejected as the main path: a second process
  can read the file between create and write and see an empty or partial file. `rename`.
  Rejected: it replaces an existing target.

## R4. Overwrite of an empty file

- **Decision**: Open `.ws` read-write without truncate. Compare `dev`/`ino` of the handle with the
  earlier `symlink_metadata` (`std::os::unix::fs::MetadataExt`). If they differ, the entry changed:
  re-classify once (R2) without a second write attempt. Read the content through the handle. If
  it is still whitespace-only, write `DEFAULT_CONTENT` at offset 0 and call `set_len`. Otherwise
  load the new content (FR-008).
- **Rationale**: The handle check stops a write through a symlink that replaced the file. A
  read-only file or folder fails at open, which gives the FR-009 fallback without a change.
- **Residual risk (accepted)**: POSIX has no atomic compare-and-write. A writer that writes in the
  microseconds between the check read and the write can lose its content. Advisory locks do not
  help, because editors do not take them. A save into an empty file at the same moment that
  `ws up` runs is improbable.
- **Alternatives**: Rename the empty file aside, then create (R3). Rejected: a writer that holds
  the old handle writes into the moved file, and the user loses that content silently.

## R5. Fallback when a write fails

- **Decision**: On a create or overwrite failure that is not a second writer, return an in-memory
  source with a warning. The cause is the `io::ErrorKind` text, for example "permission denied"
  or "read-only file system". `up.rs` parses `DEFAULT_CONTENT` with `source_path =
  project_dir/.ws`, so `path: .` resolves to the project folder.
- **Rationale**: FR-009: the exit status does not change, and the cause contains no sensitive
  data. The error kind names the problem without internals (constitution III).
- **Alternatives**: Fail the command. Rejected by FR-009.

## R6. Order of the stderr lines

- **Decision**: `setup_file::prepare` returns an optional `SetupNotice`. `up.rs` sends it through
  the existing `Notifier` (`ws: ` prefix, stderr) right after the file step and before any tmux
  window creation. Without a name, the name message (003 FR-006) comes first, because `up.rs`
  sends it before `session::exists`. Texts: [contracts/cli.md](./contracts/cli.md).
- **Rationale**: Outside tmux, attach blocks until the user detaches. A line after attach is seen
  too late (same reason as 003 FR-006). The file step is complete at this point, so the line is
  true even if tmux fails later.
- **Alternatives**: Send the line after `connect`. Rejected for the reason above.

## R7. Where the file step runs

- **Decision**: In `up::execute`, after `session::exists` returns false and before
  `load_definition`. Nothing else calls `setup_file`.
- **Rationale**: FR-006 and US3-AS5. Help, version, `change`, `down`, `exit`, `clip`, an invalid
  explicit name (clap rejects it), an unusable folder name (003 FR-004, in `App::execute`), and an
  existing session (including a session of a different folder, 003 FR-011) never reach this
  point.
- **Alternatives**: None.

## R8. Failure after the file step (clarification 2026-10-24)

- **Decision**: No cleanup of `.ws`. The existing rollback in `up.rs` removes only the partial
  tmux session. On a lost creation race to a session of a different folder, `up.rs` already runs
  `ensure_session_belongs_to_folder` and fails with exit 5. `.ws` stays.
- **Rationale**: The spec keeps `.ws` in these cases (Edge Cases). The file has valid content, and
  the user owns it.
- **Alternatives**: Remove the created file. Rejected by the clarification.

## R9. Two `ws up` without a name in the same folder (clarification 2026-10-24)

- **Decision**: No new code. R3 gives one creator. The loser of the file race re-classifies and
  loads the file without a file message. The existing duplicate-session handling in `up.rs`
  (001 US3-AS6) and the folder check (003 FR-011) handle the session race.
- **Rationale**: The two existing mechanisms are independent and both bounded.
- **Alternatives**: A lock file. Rejected: it adds a second file (FR-012).

## R10. Missing default programs

- **Decision**: No change. `ws` types each command into the interactive shell of the pane
  (001 FR-045, `src/tmux/pane.rs`). A missing program gives "command not found" in that shell.
- **Rationale**: FR-010 forbids a check before start. The existing behavior meets US4-AS2 and
  US4-AS3.
- **Alternatives**: A `which` check. Rejected by FR-010.

## R11. Tests for a read-only folder

- **Decision**: Set the project folder to mode `0o555` in the test. Skip with a note when a probe
  file can still be created (the test runs as root).
- **Rationale**: Root ignores mode bits. A false pass is worse than a skip.
- **Alternatives**: A read-only bind mount. Rejected: needs privileges.
