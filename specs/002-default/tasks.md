# Tasks: Default Workspace File

**Input**: Design documents from `specs/002-default/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Required (constitution IV). Each test name follows
`given_<context>_when_<action>_then_<outcome>`. Each scenario test has `// Scenario: US<n>-AS<m>`
as the first line of its body. Each scenario id has exactly one owning test. Write each test before
its implementation task and make sure that it fails first.

**References**: `R<n>` = research.md item. Exact stderr lines = contracts/cli.md. States and types
= data-model.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no open dependencies)
- **[Story]**: User story from spec.md

---

## Phase 1: Setup

- [X] T001 Create branch `feature/default-workspace-file` from `develop` (constitution, Gitflow)
- [X] T002 Create empty modules `src/lifecycle/setup_file.rs`, `src/lifecycle/prepared_setup.rs`, `src/lifecycle/setup_source.rs`, and `src/lifecycle/setup_notice.rs` with a `//!` doc line each; register all four in `src/lifecycle/mod.rs`

---

## Phase 2: Foundational (blocks all stories)

- [X] T003 Unit test in `src/config/default_content.rs`: `DEFAULT_CONTENT` ends with `\n`, `parse` + `validate` accept it, and the result has windows `root`, `file`, `git`, `claude` in this order, each with path `.`, commands `None`, `yazi`, `lazygit`, `claude` (FR-002, FR-003)
- [X] T004 Create `pub const DEFAULT_CONTENT: &str` in `src/config/default_content.rs` with the text byte-identical to "Default Content (Normative)" in `specs/002-default/spec.md`; add `pub mod default_content;` and `pub use default_content::DEFAULT_CONTENT;` in `src/config/mod.rs` (R1)
- [X] T005 In `src/config/parser.rs` `parse()`: for a whitespace-only source, parse `DEFAULT_CONTENT` with the same `source_path` instead of `WorkspaceDefinition::default_for`; remove `default_for` from `src/config/workspace_definition.rs` (R1)
- [X] T006 In `src/config/parser.rs` `load()`: remove the `NotFound → String::new()` branch so that every read error, `NotFound` included, gives `ConfigError::Io` (R2)
- [X] T007 Update `tests/config_language.rs`: `given_an_empty_document_when_loaded_then_the_documented_default_is_returned` asserts the four default windows; replace `given_no_ws_file_when_loaded_then_the_documented_default_is_returned` (marker `// Scenario: US4-AS1` of 001) with `given_no_ws_file_when_loaded_then_an_io_error_names_the_path`, without a scenario marker
- [X] T008 [P] Create `SetupSource` (`File(PathBuf)`, `InMemoryDefault { source_path: PathBuf }`) in `src/lifecycle/setup_source.rs`; `SetupNotice` (`Created`, `Overwritten`, `WriteFailed { cause: io::ErrorKind }`, `EmptySymlink`, each with `path`) with `Display` per contracts/cli.md in `src/lifecycle/setup_notice.rs`; `PreparedSetup { source, notice }` in `src/lifecycle/prepared_setup.rs`. The `Display` text has no `ws: ` prefix; `StderrNotifier` adds it. Unit tests for the four texts in `src/lifecycle/setup_notice.rs`
- [X] T009 Run `cargo test --all-targets --all-features`. Expected failures: only tests that need a missing `.ws` to start a session (`tests/lifecycle.rs` 001 US4-AS1/AS2 tests, `src/lifecycle/up.rs` unit tests without `.ws`, and any process test that runs `ws up` in a folder without `.ws`). List them in a note under this phase. Fix any other failure

Note (T001, T009): the branch `feature/default-workspace-file` already existed at `develop`; it was
used as is. T009 found 9 expected failures, all from `ws up` in a folder without `.ws`:
`src/lifecycle/up.rs` (2 unit tests), `tests/default_session_name.rs` (5), `tests/lifecycle.rs`
(`given_no_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window`,
`given_a_non_credential_command_when_up_is_invoked_then_pass_is_never_contacted`). No other failure.

**Checkpoint**: `DEFAULT_CONTENT` is the only default. A missing `.ws` now fails in `load`. Phase 3 restores the missing-file path.

---

## Phase 3: User Story 1 – Start the default workspace without configuration (P1) 🎯 MVP

**Goal**: `ws up [SESSION_NAME]` with `.ws` missing creates `.ws` and a session with the four default windows.

**Independent test**: Temp folder without `.ws`, fake tmux; `calls.log` shows `new-session -n root`, then `new-window -n file`, `-n git`, `-n claude`, each with `-c <project folder>`.

### Tests

- [X] T010 [P] [US1] Unit tests in `src/lifecycle/setup_file.rs`: missing `.ws` → `SetupSource::File`, notice `Created`, file bytes equal `DEFAULT_CONTENT`, no `.ws.*.tmp` file left in the folder, also when the link step fails (R3, FR-012)
- [X] T011 [P] [US1] Create `tests/default_workspace.rs` with helpers (reuse `tests/support/mod.rs`: `temporary_project`, `fake_tmux_path_dir`, `test_session_name`; local `read_calls(state_dir)` as in `tests/lifecycle.rs`)
- [X] T012 [US1] Test `given_no_ws_file_when_up_runs_then_the_session_has_windows_root_file_git_claude_in_order_in_the_project_folder` (US1-AS1) in `tests/default_workspace.rs`
- [X] T013 [US1] Test `given_no_ws_file_when_up_runs_then_file_git_and_claude_get_their_commands_and_root_gets_none` (US1-AS2) in `tests/default_workspace.rs`: `calls.log` has `send-keys … -l -- yazi`, `-- lazygit`, `-- claude`, and exactly three `send-keys … -l` lines
- [X] T014 [US1] Test `given_no_ws_file_when_up_runs_on_real_tmux_then_window_root_is_active` (US1-AS3) in `tests/real_tmux.rs`: private server, fake launcher as in the existing test, `list-windows -F '#{window_active} #{window_name}'` shows `1 root`; skip when tmux is missing
- [X] T015 [US1] Test `given_the_user_is_inside_tmux_and_no_ws_file_when_up_runs_with_another_name_then_the_default_workspace_opens_in_a_separate_terminal` (US1-AS4) in `tests/default_workspace.rs`: `TMUX` set, fake launcher; launcher log has the name; `calls.log` has no `attach-session` or `switch-client`
- [X] T016 [US1] Test `given_no_ws_file_when_up_runs_without_a_name_then_stderr_shows_the_name_message_then_the_file_message` (US1-AS5) in `tests/default_workspace.rs`: folder `alpha`, `PWD` set; stderr lines in this order

### Implementation

- [X] T017 [US1] Implement `classify(path) -> SetupFileState` (private enum in `src/lifecycle/setup_file.rs`) with `fs::symlink_metadata` per R2 and data-model "SetupFileState"
- [X] T018 [US1] Implement `create_missing(project_dir, path) -> PreparedSetup` in `src/lifecycle/setup_file.rs`: temp file `.ws.<pid>.tmp` with `create_new`, `fs::hard_link` to `.ws`, remove the temp file in all cases; `Unsupported` → `create_new` on `.ws` and write. Until US3/US4, any other error returns `SetupSource::File(path)` without a notice, so the loader reports it (R3)
- [X] T019 [US1] Implement `pub fn prepare(project_dir: &Path) -> PreparedSetup` in `src/lifecycle/setup_file.rs`: `Missing` → `create_missing`; `Present` → `File` without notice; other states return `File` without notice until US3/US4
- [X] T020 [US1] In `src/lifecycle/up.rs`: after `session::exists` is false, call `setup_file::prepare(project_dir)`, send `notice` through `notifier` (R6), then load: `File(path)` → `ws::config::load`; `InMemoryDefault { source_path }` → `ws::config::parse(DEFAULT_CONTENT, &source_path)` + `validator::validate` (expose a `ws::config::load_default(source_path)` function in `src/config/parser.rs` if `validate` is not public). Update the `load_definition` doc comment
- [X] T021 [US1] Update the unit tests in `src/lifecycle/up.rs`: `missing_then_created_runner` accepts `new-window` and `send-keys` (print a pane id for `-P`); `given_a_derived_name_when_up_runs_then_the_notifier_shows_the_name_once` asserts the name message first and the file message second
- [X] T022 [US1] Retire the 001 tests `given_no_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window` and `given_an_empty_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window` in `tests/lifecycle.rs` (spec "Traceability"); in `specs/001-tmux-workspace` traceability, mark US4-AS1/AS2 as replaced by 002

Notes:
- T014 found a 001 defect: `new-window` had no `-d`, so the last window, not `root`, was active
  (001 FR-011). Fixed in `src/tmux/window.rs` with the unit test
  `given_an_additional_window_when_materialized_then_it_does_not_become_the_current_window`. The
  "Notes" section below said `-d` was already there; that was wrong.
- T018: when `hard_link` fails for a reason other than `AlreadyExists` (not only `Unsupported`),
  `ws` tries a direct `create_new`. Some file systems report a missing hard-link feature as
  "permission denied". A real permission problem fails the same way in the direct create.
- T020: `ws::config::validator::validate` is public, so no `load_default` function was needed.
- T012–T016 passed at once after T017–T020. Mutation checks: no notice → T016 fails; no `-d` on
  `new-window` → T014 fails on tmux 3.5a.

**Checkpoint**: US1 tests pass. `cargo test --all-targets --all-features` passes.

---

## Phase 4: User Story 2 – Keep the default as a project file (P1)

**Goal**: The created `.ws` is the exact default content, loads through the normal loader, and later edits are used.

**Independent test**: Run `ws up` in a temp folder; `.ws` equals `ws::config::DEFAULT_CONTENT`; edit it; a second `ws up` with a new name uses the edit.

### Tests

- [X] T023 [P] [US2] Test `given_no_ws_file_when_up_runs_then_ws_contains_exactly_the_default_content` (US2-AS1) in `tests/default_workspace.rs`: compare bytes with `ws::config::DEFAULT_CONTENT`
- [X] T024 [P] [US2] Test `given_ws_created_the_file_when_the_file_is_loaded_then_the_normal_loader_gives_the_session_windows` (US2-AS2) in `tests/default_workspace.rs`: `ws::config::load(project/.ws)` window names and paths match the `new-session`/`new-window` arguments in `calls.log`
- [X] T025 [P] [US2] Test `given_no_ws_file_when_up_runs_then_stderr_has_one_file_message_with_the_path_and_stdout_is_empty` (US2-AS3) in `tests/default_workspace.rs`: exact line per contracts/cli.md
- [X] T026 [P] [US2] Test `given_the_user_changed_the_created_file_when_up_runs_with_a_new_name_then_ws_uses_the_change_and_does_not_write_the_file` (US2-AS4) in `tests/default_workspace.rs`: rename `claude` to `ai`; `calls.log` has `-n ai`; bytes and modification time of `.ws` do not change; no file message

### Implementation

- [X] T027 [US2] Fix any failure from T023–T026 in `src/lifecycle/setup_file.rs` or `src/lifecycle/up.rs`. If all pass at once, add a note under this phase with a mutation check (for example, drop the comment lines from the written content) that makes T023 fail

Note (T027): T023–T026 passed at once. Mutation check: writing the content without the two comment
lines makes only T023 fail.

**Checkpoint**: US1 and US2 tests pass.

---

## Phase 5: User Story 3 – Change files only when necessary (P1)

**Goal**: `ws` writes `.ws` only when it is missing or empty and the session does not exist; never over content.

**Independent test**: Existing session, valid/invalid/empty `.ws`, second writer, and help/version/invalid-name runs; only the allowed cases change the file.

### Tests

- [X] T028 [P] [US3] Unit tests in `src/lifecycle/setup_file.rs`: empty regular file → `Overwritten`, content equals `DEFAULT_CONTENT`; whitespace-only file → `Overwritten`; empty file that gets content before the write step → no write, `File`, no notice; inode changed between classify and open → re-classify once, no write (R4)
- [X] T029 [P] [US3] Two unit tests in `src/lifecycle/setup_file.rs`: (a) `given_another_writer_created_ws_when_create_missing_runs_then_the_other_content_is_kept` → `File`, no notice, content unchanged; (b) the same with an empty other file → no write (R3, R9)
- [X] T030 [P] [US3] Test `given_the_session_exists_and_ws_is_missing_or_empty_when_up_runs_then_ws_does_not_change` (US3-AS1) in `tests/default_workspace.rs`: seed the session (`support::seed_fake_tmux_session`); missing stays missing, empty stays empty
- [X] T031 [P] [US3] Test `given_a_valid_or_invalid_non_empty_ws_when_up_runs_then_the_content_and_modification_time_do_not_change` (US3-AS2) in `tests/default_workspace.rs`: invalid file gives exit 3
- [X] T032 [P] [US3] Test `given_an_empty_ws_when_up_runs_then_ws_overwrites_it_starts_the_default_workspace_and_shows_one_warning` (US3-AS3) in `tests/default_workspace.rs`: exact warning per contracts/cli.md, nothing about `.ws` on stdout
- [X] T033 [US3] Owning test for US3-AS4: mark unit test (a) of T029 with `// Scenario: US3-AS4`; no other test carries this id
- [X] T034 [P] [US3] Test `given_no_ws_file_when_help_version_or_an_invalid_name_runs_then_no_ws_is_created` (US3-AS5) in `tests/default_workspace.rs`: `ws`, `ws help`, `ws --help`, `ws help config`, `ws --version`, `ws up bad:name`, `ws up` in `/`-like unusable folder (use `PWD=/` as in `tests/default_session_name.rs`), and `ws up` without a name with a seeded session of another folder (`seed_fake_tmux_session_with_folder`)
- [X] T035 [P] [US3] Test `given_no_ws_file_when_change_down_exit_or_clip_runs_then_no_ws_is_created` in `tests/default_workspace.rs`, without a scenario marker (FR-006, SC-004)

### Implementation

- [X] T036 [US3] Implement `overwrite_empty(path, earlier_metadata) -> PreparedSetup` in `src/lifecycle/setup_file.rs`: open read-write without truncate, compare `dev`/`ino` (`std::os::unix::fs::MetadataExt`), read through the handle, write `DEFAULT_CONTENT` at offset 0 and `set_len` only if still whitespace-only; else `File` without notice (R4). Document the residual risk of R4 in a comment
- [X] T037 [US3] In `create_missing`, handle `AlreadyExists`: re-classify once and return `File` without notice and without a write (R3, R9)
- [X] T038 [US3] In `prepare`, route `Empty` → `overwrite_empty`; at most one re-classification per call, never a second write attempt (constitution IX)

Notes:
- T028–T029, T039 were written first and failed to compile; T036–T038 and T043–T044 were then
  done together in `src/lifecycle/setup_file.rs`, because both phases change the same functions.
- Deviation from data-model "Re-classify once": after a lost create race, `ws` classifies once
  more and overwrites an empty file of the second process, as spec FR-005 requires ("FR-008
  applies if that file is empty"). After a failed same-entry or still-empty check in the
  overwrite, `ws` loads the file as it is, without a second classification. Both paths make at
  most one write attempt.
- The replaced-entry unit test keeps the old file under another name, because a new file can
  reuse the inode number of a removed one. Mutation check: no same-entry check → that test fails.

**Checkpoint**: US1–US3 tests pass.

---

## Phase 6: User Story 4 – Continue when a write fails or a program is missing (P2)

**Goal**: Read-only folder or missing program still gives four windows and exit 0.

**Independent test**: Folder mode `0o555`, then `PATH` without `yazi`/`lazygit`/`claude`; four windows each time.

### Tests

- [X] T039 [P] [US4] Unit tests in `src/lifecycle/setup_file.rs`: read-only folder with `.ws` missing → `InMemoryDefault`, `WriteFailed` with `PermissionDenied`, no file; read-only empty file → `InMemoryDefault`, file unchanged; symlink to an empty file → `InMemoryDefault`, `EmptySymlink`, target unchanged (R5, R11: skip with a note when a probe file can be created)
- [X] T040 [P] [US4] Test `given_a_read_only_folder_and_no_ws_file_when_up_runs_then_it_warns_starts_four_windows_and_exits_0` (US4-AS1) in `tests/default_workspace.rs` (R11 skip rule)
- [X] T041 [P] [US4] Test `given_the_default_programs_are_missing_when_up_runs_on_real_tmux_then_all_four_windows_exist` (US4-AS2) in `tests/real_tmux.rs`: `PATH` with tmux and the shell only; four windows; the session still exists after the commands ran; skip when tmux is missing
- [X] T042 [P] [US4] Test `given_a_folder_that_is_not_a_git_repository_when_up_runs_then_the_git_window_is_created` (US4-AS3) in `tests/default_workspace.rs`

### Implementation

- [X] T043 [US4] In `src/lifecycle/setup_file.rs`, map create and overwrite errors (other than `AlreadyExists` and a second writer) to `InMemoryDefault { source_path }` + `WriteFailed { cause: error.kind() }` (R5)
- [X] T044 [US4] In `classify`/`prepare`, handle `EmptyThroughSymlink` → `InMemoryDefault` + `EmptySymlink` without a write

Note (T041): `tests/real_tmux.rs` now runs `ws` with a `PATH` that has only `tmux`, `SHELL=/bin/sh`,
and a temporary `HOME`. Without this, the real-tmux tests started the real `yazi`, `lazygit`, and
`claude` of the developer. The 003 test uses the same helper and still passes.

**Checkpoint**: All story tests pass.

---

## Phase 7: Polish

- [X] T045 [P] Edge-case tests without scenario markers in `tests/default_workspace.rs`: `.ws` is a directory → exit 3, no write; broken symlink → exit 3, no write; symlink to a non-empty file → loads, target unchanged; `.ws` kept after a forced tmux failure (`WS_FAKE_TMUX_FAIL_ON=new-window`) (spec Edge Cases, clarification 2026-10-24)
- [X] T046 [P] Update `doc/config-lang.md` per contracts/cli.md "`ws help config`": replace the "missing or empty `.ws`" rule with a section that shows `DEFAULT_CONTENT` verbatim and the create, overwrite, never-overwrite, and in-memory rules (FR-011)
- [X] T047 Drift test in `tests/config_language.rs`: `given_the_published_reference_when_checked_for_drift_then_it_contains_the_default_content` asserts that `doc/config-lang.md` contains `ws::config::DEFAULT_CONTENT` (FR-011)
- [X] T048 [P] Update `README.md`: replace "falls back to one default window" with the default `.ws` behavior and the four windows
- [X] T049 Fill the Traceability table in `specs/002-default/spec.md` with the owning tests (17 ids)
- [X] T050 Run gates: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`
- [X] T051 Run the manual checks in `specs/002-default/quickstart.md`; note results under this phase

Notes:
- T045 also covers analysis finding G1: `given_a_gitignore_when_up_creates_ws_then_only_ws_is_added_and_gitignore_does_not_change`.
- T050: all gates pass (`cargo fmt --check`, `cargo check`, `cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test --all-targets --all-features`).
- T051, first run: the script set `PATH` to a folder with only `tmux`. `mkdir` then failed, and
  later steps ran in the repository root. Step 5 (`rm .ws`) deleted the developer's own `.ws` in
  the repository root (ignored by Git, not recoverable). `ws up` then wrote the default `.ws`
  there; the developer keeps it. No other file changed.
- T051, second run: a script with `set -euo pipefail`, absolute tool paths, and guards that allow
  `ws` only in scratch folders or `/`. A private real tmux server (tmux 3.5a), the fake launcher
  instead of attach. Steps 1–7 of `quickstart.md` matched: windows `root file git claude` with
  `root` active; the name message before the file message; the edited `.ws` used and unchanged;
  the overwrite warning; no `.ws` for an existing session, a read-only folder (exit 0, four
  windows), or the other commands; `ws up` in `/` exits 2. The repo-root `.ws` did not change.
  Attach in a real terminal outside tmux is a manual check for the developer.

---

## Dependencies

```text
Phase 1 → Phase 2 → US1 → US2
                     ├──→ US3 (after T017–T020)
                     └──→ US4 (after T017–T020; T044 after T038)
US1..US4 → Polish
```

- Phase 2 breaks the missing-file path on purpose. US1 restores it. Do not merge between them.
- US2 has no new code in the normal case; it proves US1 output.
- US3 and US4 both edit `src/lifecycle/setup_file.rs`: run their implementation tasks in sequence.

## Parallel Examples

- Phase 2: T003 and T008 together.
- US1: T010 and T011 together; then T012–T016 in `tests/default_workspace.rs` one after another (same file), T014 in parallel (`tests/real_tmux.rs`).
- US2: T023–T026 (same file, write in sequence, run together).
- US3: T028, T029 (unit) in parallel with T030–T035 (process tests).
- US4: T039, T041 in parallel with T040, T042.
- Polish: T045, T046, T048 together.

## Implementation Strategy

1. MVP: Phase 1, Phase 2, US1. `ws up` in a folder without `.ws` gives the four windows and the file.
2. Add US2 (proof of the file contract). Add US3 before any release: without it, an empty `.ws` is not overwritten and race cases are not covered.
3. Add US4. Then Polish.

## Notes

- `new-window` uses `-d` (`src/tmux/window.rs`, added in Phase 3), so window `root` stays active. T014 proves it on real tmux.
- The fake tmux does not run window commands. Missing-program behavior (US4-AS2) needs real tmux.
