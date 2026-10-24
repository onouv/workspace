# Research: Default Session Name

Each item gives the decision, the rationale, and the alternatives.

## R1. Optional positional argument

- **Decision**: Change `UpArgs::session_name` to `Option<String>`. Keep `validate_session_name` as
  the value parser. clap runs the parser only for a given value, so `ws up ""` and `ws up "   "`
  still fail (FR-007).
- **Rationale**: clap stays the single source of the CLI contract (constitution I). `DownArgs`
  already uses this pattern.
- **Alternatives**: A `--here` flag. Rejected: the user asked for `ws up` without arguments.

## R2. Folder name source (logical path)

- **Decision**: Read `$PWD`. Use it only if it is absolute and it resolves (canonical path) to the
  same folder as `std::env::current_dir()`. Otherwise use `current_dir()`. Take
  `Path::file_name()` of the result.
- **Rationale**: The shell sets `$PWD` to the logical path. The check stops a stale or false `$PWD`.
  `file_name()` is `None` for `/`, which gives FR-004.
- **Alternatives**: `current_dir()` only. Rejected: for a symlink folder it gives the target name,
  not the name that the user sees.

## R3. Derived-name rules

- **Decision**: A pure function `derive(folder_name: &OsStr) -> Result<SessionName, DeriveError>`:
  1. Not UTF-8 → `DeriveError::NotText`.
  2. Whitespace-only → `DeriveError::Blank`.
  3. Control character → `DeriveError::ControlCharacter`.
  4. Replace each `.` and `:` with `_` (FR-005).
  5. Run the result through the same check as `validate_session_name` (shared function).
- **Rationale**: Step 5 keeps one validity rule for both name sources (Key Entity "Derived Name").
  A pure function gets unit tests without a filesystem (constitution II, IV).
- **Alternatives**: Lossy UTF-8 conversion. Rejected: it makes a name that is not the folder name.

## R4. Session folder: how ws reads it

- **Decision**: Read the folder of a session with one call:
  `tmux list-sessions -F "#{session_name}\t#{@ws-project-folder}\t#{session_path}"`.
  Find the row whose name is exactly equal to the derived name. Use `@ws-project-folder` if it is
  not empty. Otherwise use `session_path`.
- **Rationale**: `session_path` is the start directory of the session (`new-session -c`). `ws`
  starts the first window with `-c <window path>`. If the first window in `.ws` has
  `path: ./sub`, `session_path` is the subfolder, and the folder check would refuse a session that
  `ws` made in this project folder. So `ws` also records the project folder in the session user
  option `@ws-project-folder`. A session from plain `tmux` has no such option and uses
  `session_path`, as the clarification says. `list-sessions` compares names in Rust, so no tmux
  target matching applies.
- **Alternatives**:
  - `session_path` only. Rejected: false refusals for `.ws` files with a subfolder first window.
  - `display-message -t =NAME: -p '#{session_path}'`. Rejected: two calls and target syntax.
- **Spec impact**: The spec term "session folder" gets one sentence: for a session that `ws`
  made, it is the project folder that `ws` recorded. This is the intent of the clarification.

## R5. Record the project folder atomically

- **Decision**: Append `; set-option -t =<session>: @ws-project-folder <canonical project folder>`
  to the same tmux invocation as `new-session` (tmux runs `;`-chained commands in one queue).
- **Rationale**: A second `ws up` cannot see the session without the option. No extra rollback
  path. `set-option -t` takes a pane target: `=NAME` alone fails with "no such session"; `=NAME:`
  names the session exactly (verified with tmux 3.5a).
- **Alternatives**: A second `set-option` call. Rejected: a short window where the option is
  missing gives a false refusal.

## R6. Folder comparison

- **Decision**: Canonicalize the project folder and the session folder with
  `std::fs::canonicalize`. Compare the results. If the session folder does not exist or cannot be
  read, the check fails (FR-011).
- **Rationale**: The clarification session decided to resolve symlinks.

## R7. Exact session-name match in `has-session`

- **Decision**: Use the exact-match target `=NAME` in `session::exists` and in `attach`/launcher
  targets for the derived name path. Apply it to all `ws up` paths.
- **Rationale**: `tmux has-session -t alpha` also matches `alphabet` when `alpha` does not exist
  (prefix match). A derived name makes this case more likely. Without `=`, `ws up` in folder
  `alpha` would attach to `alphabet`.
- **Risk**: This changes the argv of existing calls. Existing tests that assert the exact argv must
  change. The fake tmux must strip a leading `=`.

## R8. Message on stderr before attach

- **Decision**: Add a `Notifier` trait in `src/terminal/notifier.rs` with `notify(&self, line: &str)`.
  `StderrNotifier` writes one line to stderr. Tests use a recording notifier. Add it to
  `AppDependencies`. `up::execute` calls it before `connect`.
- **Rationale**: Outside tmux, `attach` blocks until detach. A message in the returned stdout
  string would come too late and would go to stdout (FR-006). Domain code must not write to
  stderr directly (constitution II). Feature 002 needs the same channel for its warnings.
- **Alternatives**: `eprintln!` in `app.rs`. Rejected: not testable and it mixes streams into the
  orchestration layer.

## R9. Exit statuses

- **Decision**:
  - Unusable folder name (FR-004): `InvalidInvocation` (2).
  - Folder check fails (FR-011): `OperationFailed` (5). The input is valid, but the tmux state
    conflicts.
- **Rationale**: Uses the existing 001 categories (FR-031). No new status.

## R10. Race between two `ws up` without a name

- **Decision**: When `new-session` reports a duplicate session for a derived name, `ws` does the
  folder check before it connects.
- **Rationale**: The winner may be in a different folder with the same name. The check keeps
  FR-011 true for the loser.
