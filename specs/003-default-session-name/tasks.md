# Tasks: Default Session Name

**Input**: Design documents from `specs/003-default-session-name/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md

**Tests**: Required (constitution IV). Each test name follows `given_<context>_when_<action>_then_<outcome>`.
Each scenario test has `// Scenario: US<n>-AS<m>` as the first line of its body. Write each test
before its implementation task and make sure that it fails first.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no open dependencies)
- **[Story]**: User story from spec.md

---

## Phase 1: Setup

- [X] T001 Create branch `feature/default-session-name` from `develop` (constitution, Gitflow)
- [X] T002 Create module skeleton `src/session_name/mod.rs` and register it in `src/main.rs`

---

## Phase 2: Foundational (blocks all stories)

- [X] T003 Move the session-name validity rule from `validate_session_name` in `src/cli.rs` to a public function in `src/session_name/mod.rs`; `src/cli.rs` calls it (R3)
- [X] T004 [P] Add `Notifier` trait, `StderrNotifier`, and a test `RecordingNotifier` in `src/terminal/notifier.rs`; export from `src/terminal/mod.rs` (R8)
- [X] T005 Add `notifier: Box<dyn Notifier>` to `AppDependencies` in `src/app.rs` and pass `StderrNotifier` in `src/main.rs`
- [X] T006 [P] Extend fake tmux in `tests/fixtures/fake_tmux.rs`: strip a leading `=` from targets, store the `-c` path per session, support `list-sessions -F` with `#{session_name}`, `#{@ws-project-folder}`, `#{session_path}`, support `set-option -t S @ws-project-folder V`, and split one invocation at `;` arguments (R4, R5, R7)
- [X] T007 [P] Add `seed_fake_tmux_session_with_folder(state_dir, name, start_dir, recorded)` in `tests/support/mod.rs`
- [X] T008 Unit test then implement exact target `=NAME` in `session::exists` in `src/tmux/session.rs` and in `attach`/`switch-client` in `src/tmux/client.rs`; update argv assertions in `src/lifecycle/change.rs`, `src/lifecycle/down.rs`, `src/tmux/client.rs` tests (R7)
- [X] T009 Unit test then implement chained `; set-option -t =NAME @ws-project-folder <canonical project folder>` on `new-session` in `src/tmux/window.rs` (R5)
- [X] T010 Run the existing suite: `cargo test --all-targets --all-features`; fix regressions from T006–T009

**Checkpoint**: Existing behavior is unchanged except exact targets and the recorded folder.

---

## Phase 3: User Story 1 – Start a workspace named after the folder (P1) 🎯 MVP

**Goal**: `ws up` without a name creates the session named after the folder.

**Independent test**: Run `ws up` in temp folder `alpha` with fake tmux; session `alpha` exists.

### Tests

- [X] T011 [P] [US1] Unit tests for `derive()` (plain name, `.`→`_`, `:`→`_`, spaces kept) in `src/session_name/derived_session_name.rs`
- [X] T012 [P] [US1] Unit tests for logical folder resolution (`$PWD` matches, `$PWD` stale, `$PWD` relative) in `src/session_name/logical_folder.rs`
- [X] T013 [P] [US1] Test `given_no_session_alpha_when_up_runs_without_a_name_in_alpha_then_it_creates_session_alpha` (US1-AS1) in `tests/default_session_name.rs`
- [X] T014 [P] [US1] Test `given_no_session_when_up_runs_without_a_name_then_stderr_names_the_derived_session_and_stdout_does_not` (US1-AS2) in `tests/default_session_name.rs`
- [X] T015 [P] [US1] Test `given_the_user_is_inside_tmux_when_up_runs_without_a_name_then_it_opens_a_separate_terminal` (US1-AS3) in `tests/default_session_name.rs`

### Implementation

- [X] T016 [P] [US1] Create `DeriveError` (thiserror) in `src/session_name/derive_error.rs` (data-model)
- [X] T017 [US1] Implement `DerivedSessionName` and `derive()` in `src/session_name/derived_session_name.rs` (R3)
- [X] T018 [US1] Implement logical folder resolution in `src/session_name/logical_folder.rs` (R2)
- [X] T019 [US1] Change `UpArgs::session_name` to `Option<String>` in `src/cli.rs` (R1)
- [X] T020 [US1] Add `SessionNameSource` in `src/lifecycle/session_name_source.rs`; in `src/app.rs` build it from the argument or from `derive()`; map `DeriveError` to `AppError::InvalidInvocation`
- [X] T021 [US1] In `src/lifecycle/up.rs` take `SessionNameSource`; call the notifier for `Derived` before any connect (FR-006)

**Checkpoint**: US1 tests pass.

Note: `DerivedSessionName::project_folder` (data-model) is deferred to T027, where the folder
check first uses it. The project folder is recorded for every new session, also for an explicit
name, so a later `ws up` without a name can match it.

---

## Phase 4: User Story 2 – Connect to the running workspace of the folder (P1)

**Goal**: Connect when the session folder matches; refuse when it differs.

**Independent test**: Seed session `alpha` with folder A; `ws up` in A attaches; in B fails with 5.

### Tests

- [X] T022 [P] [US2] Unit tests for the folder check (match, mismatch, unreadable, recorded beats start dir, symlink resolves) in `src/tmux/session_folder.rs`
- [X] T023 [P] [US2] Test `given_session_alpha_belongs_to_this_folder_when_up_runs_without_a_name_outside_tmux_then_it_attaches_without_reading_ws` (US2-AS1) in `tests/default_session_name.rs`
- [X] T024 [P] [US2] Test `given_session_alpha_belongs_to_this_folder_when_up_runs_without_a_name_inside_tmux_then_it_opens_a_separate_terminal` (US2-AS2) in `tests/default_session_name.rs`
- [X] T025 [P] [US2] Test `given_session_alpha_belongs_to_another_folder_when_up_runs_without_a_name_then_it_exits_5_and_does_not_connect` (US2-AS3) in `tests/default_session_name.rs`
- [X] T026 [P] [US2] Test `given_plain_tmux_made_session_alpha_in_this_folder_when_up_runs_without_a_name_then_it_attaches` (US2-AS4) in `tests/default_session_name.rs`

### Implementation

- [X] T027 [US2] Implement `SessionFolder` and `check(project_folder)` in `src/tmux/session_folder.rs` (R4, R6)
- [X] T028 [US2] Implement `session::folder(tmux, name)` with `list-sessions -F` and exact name match in `src/tmux/session.rs` (R4)
- [X] T029 [US2] In `src/lifecycle/up.rs` run the folder check for `Derived` when the session exists and after a duplicate-session race; map failures to `AppError::OperationFailed` with the contract message (FR-011, R9, R10)
- [X] T030 [US2] Unit test in `src/lifecycle/up.rs`: duplicate-session race with a different folder fails (R10)

**Checkpoint**: US1 and US2 tests pass.

Note: verified against real tmux 3.5a. `set-option -t` takes a pane target, so the chained call
uses `=NAME:` (not `=NAME`). The fake tmux now rejects a `set-option` target without `:`.

---

## Phase 5: User Story 3 – Keep the explicit name (P1)

**Goal**: Explicit names act as in 001.

### Tests

- [ ] T031 [P] [US3] Test `given_folder_alpha_when_up_runs_with_beta_then_it_uses_beta_and_shows_no_message` (US3-AS1) in `tests/default_session_name.rs`
- [ ] T032 [P] [US3] Test `given_an_empty_or_blank_explicit_name_when_up_runs_then_it_exits_2_and_does_not_derive` (US3-AS2) in `tests/default_session_name.rs`

### Implementation

- [ ] T033 [US3] Make sure that `Explicit` skips the notifier and the folder check in `src/lifecycle/up.rs`
- [ ] T034 [US3] Update the 001 US3-AS5 test in `tests/lifecycle.rs`: keep the invalid-name case; make sure that no test treats a missing name as an error

---

## Phase 6: User Story 4 – Fail clearly when the folder name is not usable (P2)

**Goal**: Unusable folder names fail with exit 2 before tmux; `.`/`:` are replaced.

### Tests

- [ ] T035 [P] [US4] Test `given_the_filesystem_root_when_up_runs_without_a_name_then_it_exits_2_before_contacting_tmux` (US4-AS1) in `tests/default_session_name.rs`
- [ ] T036 [P] [US4] Test `given_folder_my_project_with_a_dot_when_up_runs_without_a_name_then_it_creates_session_my_underscore_project` (US4-AS2) in `tests/default_session_name.rs`
- [ ] T037 [P] [US4] Test `given_a_blank_or_control_character_folder_name_when_up_runs_without_a_name_then_it_exits_2_before_contacting_tmux` (US4-AS3) in `tests/default_session_name.rs` (non-UTF-8 case as a unit test in `src/session_name/derived_session_name.rs`)

### Implementation

- [ ] T038 [US4] Complete `DeriveError` messages per `contracts/cli.md` in `src/session_name/derive_error.rs`; map `current_dir()` failure to `CurrentFolderUnreadable` in `src/session_name/logical_folder.rs`

---

## Phase 7: Polish

- [ ] T039 [P] Update `ws up` help and the examples in `src/cli.rs` (FR-009); add a help test in `tests/cli_help.rs`
- [ ] T040 [P] Update `README.md` (quick start, command table) for `ws up [SESSION_NAME]`
- [ ] T041 [P] Disposable-tmux test in `tests/tmux_integration.rs`: a `.ws` with first window `path: ./sub`, then `ws up` twice without a name, connects (R4)
- [ ] T042 Fill the Traceability table in `specs/003-default-session-name/spec.md` with the owning tests
- [ ] T043 Run gates: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`
- [ ] T044 Run the manual checks in `specs/003-default-session-name/quickstart.md`

---

## Dependencies

```text
Phase 1 → Phase 2 → US1 → US2
                     └──→ US3 (after T019–T021)
                     └──→ US4 (after T017–T018)
US1..US4 → Polish
```

- US2 needs `SessionNameSource` from US1 (T020, T021).
- US3 and US4 can run in parallel with US2 after US1.

## Parallel Examples

- Phase 2: T004, T006, T007 together.
- US1: T011, T012, T013, T014, T015 together; then T016.
- US2: T022–T026 together.
- US4: T035–T037 together.

## Implementation Strategy

1. MVP: Phase 1, Phase 2, US1. `ws up` without a name works for new sessions.
2. Add US2 before any release. Without it, a reconnect can join the wrong project.
3. Add US3 and US4. Then Polish.

## Notes

- The terminal launcher (`WS_TERMINAL_LAUNCHER`) gets the bare session name. Its own tmux command
  is outside `ws` control, so exact matching (R7) does not apply to it.
