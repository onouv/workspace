# Tasks: Configured tmux Workspace CLI

**Input**: Design documents from `specs/001-tmux-workspace/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, and `quickstart.md`

**Tests**: Required by the constitution and feature specification. Tests MUST use Given/When/Then
behavior descriptions and MUST be written before the implementation tasks for each story.

**Organization**: Tasks are grouped by user story so each story can be implemented and validated as
an independently testable increment after the foundational phase.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the Rust application, dependency lockfile, repository safety, and test layout.

- [ ] T001 Add the approved Rust dependencies and test dependencies to `Cargo.toml`, resolve the maintained Serde-compatible YAML parser, and generate `Cargo.lock`.
- [ ] T002 Replace the hello-world entry point in `src/main.rs` and create the module skeleton from `plan.md`: `src/app.rs`, `src/cli.rs`, `src/error.rs`, `src/help.rs`, `src/config/mod.rs`, `src/lifecycle/mod.rs`, `src/tmux/mod.rs`, `src/terminal/mod.rs`, and `src/credentials/mod.rs`.
- [ ] T003 [P] Add repository secret exclusions for `.secrets/` and nested descendants to `.gitignore`, and verify that no existing tracked path violates the constitution’s secret-isolation rule.
- [ ] T004 [P] Create shared process-test helpers and disposable workspace utilities in `tests/support/mod.rs` without storing credentials or persistent secret fixtures.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement boundaries shared by every user story. No story implementation should begin
until this phase is complete.

- [ ] T005 Define typed application errors, stable exit-status categories, redaction helpers, and top-level error rendering in `src/error.rs`.
- [ ] T006 Define the `clap` command model for discovery, `up`, `change`, `down`, `exit`, and reserved `clip` commands in `src/cli.rs`, including global `--help` and `--version` behavior without side effects.
- [ ] T007 Define the subprocess abstraction and `TmuxClient` boundary in `src/tmux/client.rs`, including argv-safe command execution, captured stdout/stderr, status handling, and fake-runner injection for tests.
- [ ] T008 Define terminal capability detection, TTY checks, and the `TerminalLauncher` boundary in `src/terminal/mod.rs` and `src/terminal/launcher.rs`; a missing launcher must be representable as a recoverable error.
- [ ] T009 Define the application orchestration boundary and dependency injection points in `src/app.rs`, keeping command parsing, domain logic, terminal effects, and process-global state separate.
- [ ] T010 Add module-level Rustdoc and lint policy scaffolding in `src/main.rs`, documenting any intentional lint allowances and preserving one clear home for each non-trivial type.

**Checkpoint**: Command, process, terminal, error, and application boundaries compile independently and
can be exercised with fake external commands.

---

## Phase 3: User Story 1 - Discover the command interface safely (Priority: P1) 🎯 MVP

**Goal**: Make help, config reference, and version output useful and completely independent of tmux,
`pass`, and GPG availability.

**Independent Test**: With fake or unavailable `tmux`, `pass`, and GPG commands, run `ws`, `ws help`,
`ws --help`, `ws help config`, and `ws --version`; verify successful output, no prompts, no external
calls, and correct unknown-command errors.

### Tests for User Story 1

- [ ] T011 [P] [US1] Add process-level discovery tests in `tests/cli_help.rs` covering `ws`, `ws help`, `ws --help`, `ws help config`, and `ws --version` with external dependencies unavailable.
- [ ] T012 [P] [US1] Add output-contract tests in `tests/cli_help.rs` for concise usage, command listings, configuration-reference content, version output, stdout/stderr separation, and unknown-command failures.

### Implementation for User Story 1

- [ ] T013 [US1] Implement side-effect-free help and version rendering in `src/help.rs`, including the command summary and a pointer to `ws help config`.
- [ ] T014 [US1] Add the user-facing `.ws` language reference and canonical example to `doc/config-lang.md`, derived from `contracts/ws-schema.yaml`, for inclusion in `ws help config`.
- [ ] T015 [US1] Wire `ws`, `help`, `--help`, `help config`, and `--version` through `src/app.rs` so they return before tmux, pass, GPG, configuration, or terminal-launcher access.

**Checkpoint**: User Story 1 is independently usable and testable without tmux or password-store setup.

---

## Phase 4: User Story 2 - Learn and validate the YAML configuration language (Priority: P1) 🎯 MVP

**Goal**: Parse YAML 1.2 `.ws` documents against the versioned schema, validate semantics, and provide
clear source-oriented failures.

**Independent Test**: Parse the canonical example, valid environment/path/command variations, and
malformed YAML/schema/semantic cases without invoking tmux.

### Tests for User Story 2

- [ ] T016 [P] [US2] Add valid-language tests in `tests/config_language.rs` for the canonical document, recursive panes, ordered windows, window and pane environments, path inheritance, and custom Bash commands.
- [ ] T017 [P] [US2] Add invalid-language tests in `tests/config_language.rs` for YAML syntax, wrong types, missing keys, unknown keys, duplicate keys, invalid positions, invalid environment names, and invalid version values.
- [ ] T018 [P] [US2] Add source-diagnostic assertions in `tests/config_language.rs` verifying file path and line/column or nearest declaration context for parse and validation errors.

### Implementation for User Story 2

- [ ] T019 [P] [US2] Implement `WorkspaceDefinition` and its Serde representation in `src/config/workspace_definition.rs`, including version and ordered windows.
- [ ] T020 [P] [US2] Implement `WindowDefinition` in `src/config/window_definition.rs` with required name/path and optional command, environment, and pane sequence fields.
- [ ] T021 [P] [US2] Implement recursive `PaneDefinition` in `src/config/pane_definition.rs` with position, optional ID/path/command/environment, and child panes.
- [ ] T022 [US2] Implement YAML loading, source-aware parse errors, single-document handling, and duplicate-key policy in `src/config/parser.rs`.
- [ ] T023 [US2] Implement schema and semantic validation in `src/config/validator.rs`, including unknown-key/type checks, uniqueness, version, path/value rules, environment-key validation, and empty-file handling.
- [ ] T024 [US2] Implement path resolution and window/pane environment inheritance in `src/config/validator.rs`, producing immutable validated definitions without secret values.
- [ ] T025 [US2] Synchronize `contracts/ws-schema.yaml`, `doc/config-lang.md`, and the embedded config help content in `src/help.rs`, and add a test that detects drift between the published schema/example and parser fixtures.

**Checkpoint**: User Story 2 is independently testable as a YAML parser/validator and `ws help config`
publishes the same contract.

---

## Phase 5: User Story 3 - Create or reconnect to a named workspace (Priority: P1) 🎯 MVP

**Goal**: Implement `ws up SESSION_NAME` with existing-session fast path and non-disruptive in-tmux
startup.

**Independent Test**: From outside tmux and from a fake `SESSION_ONE` client, exercise missing and
existing `OTHER_SESSION` cases; verify `.ws` is read only for a missing target, the current client is
preserved in-tmux, and separate-terminal-launcher failures are recoverable.

### Tests for User Story 3

- [ ] T026 [P] [US3] Add existing-session tests in `tests/lifecycle.rs` proving `ws up SESSION_NAME` checks tmux before reading malformed `.ws` and connects to the existing target.
- [ ] T027 [P] [US3] Add in-tmux tests in `tests/lifecycle.rs` proving `ws up OTHER_SESSION` leaves `SESSION_ONE` attached and opens an existing target through a separate terminal context.
- [ ] T028 [P] [US3] Add missing-target tests in `tests/lifecycle.rs` proving `ws up OTHER_SESSION` applies `.ws`, creates a fresh target, and does not clone live panes or processes.
- [ ] T029 [P] [US3] Add race, invalid-session-name, current-session, and unavailable-launcher cases in `tests/lifecycle.rs`, asserting no duplicate session and no current-client mutation on recoverable failure.

### Implementation for User Story 3

- [ ] T030 [US3] Implement tmux session probing, session creation, attach, and target validation in `src/tmux/session.rs` using `TmuxClient` argv boundaries.
- [ ] T031 [US3] Implement current-client/session detection and safe attach/switch primitives in `src/tmux/client.rs`.
- [ ] T032 [US3] Implement the separate terminal-launcher adapter in `src/terminal/launcher.rs`, including configured launcher invocation and a clear recoverable unavailable-launcher error.
- [ ] T033 [US3] Implement `ws up` decision flow in `src/lifecycle/up.rs`: validate the session name, probe before config read, select existing-session behavior, build missing-session plans, and preserve the current in-tmux client.
- [ ] T034 [US3] Connect `up` orchestration through `src/app.rs` and `src/lifecycle/mod.rs`, ensuring no password-store access occurs during workspace startup.

**Checkpoint**: User Story 3 preserves existing sessions and supports non-disruptive creation/reconnect
behavior with fake external dependencies.

---

## Phase 6: User Story 4 - Start a default workspace (Priority: P1) 🎯 MVP

**Goal**: Create exactly one usable shell window in the invocation directory when `.ws` is missing or
empty.

**Independent Test**: Run `ws up SESSION_NAME` in a temporary directory without `.ws`, inspect the
resulting disposable tmux session, and verify one window, the resolved directory, stable name, and
normal shell.

### Tests for User Story 4

- [ ] T035 [P] [US4] Add default-workspace tests in `tests/lifecycle.rs` for missing and empty `.ws`, asserting exactly one usable window and no password-store access.
- [ ] T036 [P] [US4] Add default failure tests in `tests/lifecycle.rs` for unavailable tmux and unusable current directories, asserting actionable non-zero errors and no leaked session state.

### Implementation for User Story 4

- [ ] T037 [US4] Implement default `WorkspaceDefinition` construction in `src/config/validator.rs` or `src/config/workspace_definition.rs` for missing/empty configuration.
- [ ] T038 [US4] Implement default window creation and normal-shell selection in `src/lifecycle/up.rs` and `src/tmux/session.rs`.
- [ ] T039 [US4] Document the default workspace name, shell selection, missing/empty `.ws` behavior, and examples in `doc/config-lang.md` and `src/help.rs`.

**Checkpoint**: User Story 4 provides a complete zero-configuration workspace independently of project
configuration.

---

## Phase 7: User Story 5 - Start a configured workspace (Priority: P1) 🎯 MVP

**Goal**: Materialize validated YAML windows and recursive panes, including commands, working
 directories, positions, and inherited environment values.

**Independent Test**: Apply the canonical `.ws` example to a disposable tmux server and compare the
resulting window/pane tree and process context with the definition.

### Tests for User Story 5

- [ ] T040 [P] [US5] Add launch-plan tests in `tests/config_language.rs` or `tests/lifecycle.rs` for ordered windows, recursive panes, positions, resolved paths, and environment inheritance/override.
- [ ] T041 [P] [US5] Add command-execution tests in `tests/tmux_integration.rs` for normal commands, custom Bash scripts, shell syntax, and argv-safe generated session/path arguments.
- [ ] T042 [P] [US5] Add disposable tmux integration tests in `tests/tmux_integration.rs` that inspect window names, pane counts/relationships, working directories, commands, and environment values.

### Implementation for User Story 5

- [ ] T043 [US5] Implement immutable tmux launch-plan construction in `src/tmux/launch_plan.rs` from validated configuration, including inherited environment maps and the selected initial window.
- [ ] T044 [US5] Implement window creation, naming, working directories, and command launch in `src/tmux/window.rs`.
- [ ] T045 [US5] Implement recursive pane splitting and position mapping in `src/tmux/pane.rs`, using tmux defaults for dimensions.
- [ ] T046 [US5] Apply non-secret window and pane environments at process creation in `src/tmux/window.rs` and `src/tmux/pane.rs`; never place credential values in tmux session state.
- [ ] T047 [US5] Integrate launch-plan application and first-window selection into `src/lifecycle/up.rs`, with rollback ownership recorded for the new session.

**Checkpoint**: User Story 5 creates the configured workspace from the canonical YAML example without
manual tmux layout commands.

---

## Phase 8: User Story 6 - Diagnose invalid workspace setup (Priority: P2)

**Goal**: Fail before mutation for invalid definitions and clean up only sessions created by the
current invocation when an external setup operation fails.

**Independent Test**: Exercise malformed YAML, invalid paths, unavailable commands, and injected tmux
failures; verify source-oriented diagnostics, non-zero statuses, no partial new session, and no change
to pre-existing sessions.

### Tests for User Story 6

- [ ] T048 [P] [US6] Add pre-mutation validation tests in `tests/config_language.rs` proving invalid configuration cannot create a target session.
- [ ] T049 [P] [US6] Add rollback tests in `tests/tmux_integration.rs` with a failing fake tmux command, proving only the newly created target is cleaned up.
- [ ] T050 [P] [US6] Add redaction tests in `tests/lifecycle.rs` proving secret-like environment values and command failures never appear in stdout, stderr, logs, or tmux command strings.

### Implementation for User Story 6

- [ ] T051 [US6] Complete actionable typed diagnostics and source-context formatting in `src/error.rs` and `src/config/validator.rs`.
- [ ] T052 [US6] Implement transactional session ownership and cleanup in `src/tmux/session.rs` and `src/lifecycle/up.rs`, preserving pre-existing sessions.
- [ ] T053 [US6] Implement command-start failure reporting with window/pane identity and redaction in `src/tmux/window.rs`, `src/tmux/pane.rs`, and `src/error.rs`.

**Checkpoint**: Configuration and external-operation failures are non-destructive, diagnosable, and
safe to retry after correction.

---

## Phase 9: User Story 7 - Manage and change workspaces explicitly (Priority: P2)

**Goal**: Add `change`, `exit`, `down`, and safe `--yes` confirmation behavior.

**Independent Test**: Use disposable sessions from inside and outside tmux to verify client switching,
detach, confirmation, explicit non-TTY consent, missing-target behavior, and session isolation.

### Tests for User Story 7

- [ ] T054 [P] [US7] Add `change` tests in `tests/lifecycle.rs` proving an existing target switches the current client, leaves the previous session alive, and never reads `.ws`.
- [ ] T055 [P] [US7] Add missing-target and outside-tmux tests in `tests/lifecycle.rs` proving `ws change` returns a recoverable error and leaves the current context unchanged.
- [ ] T056 [P] [US7] Add `down` and `exit` tests in `tests/lifecycle.rs` covering interactive confirmation, `--yes`, non-TTY refusal, current-session targeting, and detach without kill.

### Implementation for User Story 7

- [ ] T057 [US7] Implement `ws change SESSION_NAME` in `src/lifecycle/change.rs` using existing-session validation and current-client switching.
- [ ] T058 [US7] Implement `ws exit` in `src/lifecycle/exit.rs` and the current-client detach operation in `src/tmux/client.rs`.
- [ ] T059 [US7] Implement interactive confirmation and `--yes` parsing in `src/lifecycle/down.rs`, using TTY detection and refusing ambiguous nameless non-TTY deletion.
- [ ] T060 [US7] Implement named/current-session termination and safe failure reporting in `src/tmux/session.rs` and `src/lifecycle/down.rs`.
- [ ] T061 [US7] Add `change`, `exit`, `down`, and `--yes` behavior to `src/help.rs` and the CLI contract documentation in `specs/001-tmux-workspace/contracts/cli.md`.

**Checkpoint**: Workspace lifecycle operations are explicit, scriptable, and do not destroy or switch
sessions unexpectedly.

---

## Phase 10: User Story 8 - Use secure future interactive commands (Priority: P3)

**Goal**: Reserve the credential utility namespace and establish a safe on-demand `pass`/GPG-agent
boundary without implementing detailed clipboard mappings.

**Independent Test**: Run help and unsupported utility commands with fake `pass`/GPG providers; verify
help never invokes them, unsupported commands do not alter tmux, and no credential is exported.

### Tests for User Story 8

- [ ] T062 [P] [US8] Add reserved-namespace tests in `tests/cli_help.rs` for `ws clip NAMESPACE ITEM`, unsupported utility errors, and help output without password-store access.
- [ ] T063 [P] [US8] Add credential-boundary tests in `tests/lifecycle.rs` using a fake `pass` executable, proving only a requested entry can be read and no value reaches stdout, stderr, tmux state, or persisted files.

### Implementation for User Story 8

- [ ] T064 [US8] Implement the reserved `clip` command parsing and not-yet-supported response in `src/cli.rs` and `src/app.rs` without changing workspace sessions.
- [ ] T065 [US8] Define the on-demand password-store provider boundary in `src/credentials/pass_provider.rs`, delegating unlock/cache behavior to `pass`/GPG-agent and never accepting or caching a master passphrase.
- [ ] T066 [US8] Add redacted credential error mapping and future-command documentation to `src/credentials/pass_provider.rs`, `src/error.rs`, and `src/help.rs`.

**Checkpoint**: The future utility boundary is safe and discoverable without prematurely implementing
credential mappings or clipboard behavior.

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Complete documentation, CI quality gates, portability checks, and final end-to-end
validation.

- [ ] T067 [P] Add CI workflow in `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features`, and secret-path checks.
- [ ] T068 [P] Add Rust API documentation and usage examples for public command/configuration types in the relevant `src/**/*.rs` modules, and ensure `cargo doc --no-deps` succeeds.
- [ ] T069 [P] Document supported tmux versions, terminal-launcher configuration, TTY behavior, exit statuses, and YAML language version in `doc/config-lang.md` and `specs/001-tmux-workspace/contracts/cli.md`.
- [ ] T070 [P] Add or update user-facing project documentation in `README.md` with installation, `ws up`, `ws change`, `ws down --yes`, `.ws`, and `ws help config` examples that use no credentials.
- [ ] T071 Run every scenario in `specs/001-tmux-workspace/quickstart.md` with disposable sessions and temporary directories, then record any compatibility findings in `specs/001-tmux-workspace/quickstart.md`.
- [ ] T072 Run the full constitution quality gates and a repository secret scan, remove dead code and unnecessary dependencies, and resolve all warnings in `src/`, `tests/`, `Cargo.toml`, and `.gitignore`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: No dependencies; establishes the Rust project, lockfile, repository safety, and test helpers.
- **Phase 2 Foundational**: Depends on Setup; blocks every user-story phase.
- **US1 and US2**: Depend on Foundation. US1 can begin once command boundaries exist; US2 can begin once configuration module boundaries exist. They may proceed in parallel after T010.
- **US3**: Depends on US1 command wiring, US2 validated configuration, and Foundation.
- **US4**: Depends on US3 `up` flow and tmux session primitives.
- **US5**: Depends on US2 configuration models and US3 session creation.
- **US6**: Depends on US2 validation and US5 launch-plan mutation paths.
- **US7**: Depends on Foundation and tmux client/session primitives from US3; it does not depend on `.ws` parsing for `change`.
- **US8**: Depends on US1 command/help boundaries and Foundation; detailed clipboard behavior remains out of scope.
- **Polish**: Depends on all desired stories, with the MVP quality gates required before release.

### User Story Dependencies

- **US1 (P1)**: Foundation only; first independently demonstrable slice.
- **US2 (P1)**: Foundation only; provides the configuration contract used by later stories.
- **US3 (P1)**: Depends on US1 and US2; adds create/reconnect behavior.
- **US4 (P1)**: Depends on US3; adds the no-configuration fallback.
- **US5 (P1)**: Depends on US2 and US3; adds full configured layouts.
- **US6 (P2)**: Depends on US2 and US5; hardens failure and rollback paths.
- **US7 (P2)**: Depends on US3 tmux primitives; adds lifecycle commands.
- **US8 (P3)**: Depends on US1 and Foundation; remains independently testable as a safe reserved namespace.

### Parallel Opportunities

- T003 and T004 can run in parallel after T001.
- T005 through T010 can be split by module boundary after Setup; T006 and T007 must be available before CLI process tests.
- T011/T012 can run in parallel with T016-T018 after Foundation because they use separate test files.
- T019-T021 are parallel model tasks; T022-T024 depend on the model shapes.
- T026-T029 are parallel test additions in separate concerns before T030-T034.
- T040-T042 can run in parallel before launch-plan implementation.
- T054-T056 can run in parallel before lifecycle implementation.
- T067-T070 are independent documentation/CI tasks after implementation stabilization.

## Parallel Example: User Story 3

```text
Task: "T026 [P] [US3] Add existing-session tests in tests/lifecycle.rs"
Task: "T027 [P] [US3] Add in-tmux separate-terminal tests in tests/lifecycle.rs"
Task: "T028 [P] [US3] Add missing-target configuration tests in tests/lifecycle.rs"
Task: "T029 [P] [US3] Add race and recoverable-failure tests in tests/lifecycle.rs"

After those tests exist:
Task: "T030 [US3] Implement tmux session probing and creation in src/tmux/session.rs"
Task: "T032 [US3] Implement the terminal launcher in src/terminal/launcher.rs"
Task: "T033 [US3] Implement ws up decision flow in src/lifecycle/up.rs"
```

## Implementation Strategy

### MVP First

The MVP consists of Phases 1-7 and User Stories 1-5:

1. Establish the Rust project, safety rules, and foundational boundaries.
2. Deliver side-effect-free help and `ws help config`.
3. Deliver the versioned YAML parser and validator.
4. Deliver `ws up SESSION_NAME` existing-session and missing-session behavior.
5. Deliver the default one-window workspace.
6. Deliver configured windows, recursive panes, commands, paths, and environments.
7. Stop and validate with disposable tmux sessions before adding destructive lifecycle or credential utilities.

### Incremental Delivery

1. Setup + Foundation → compilable architecture and fake external boundaries.
2. US1 → safe discovery MVP slice.
3. US2 → stable `.ws` language and parser contract.
4. US3-US5 → complete workspace-layout MVP.
5. US6 → failure/rollback hardening.
6. US7 → lifecycle operations with explicit safety controls.
7. US8 → reserved credential utility boundary.
8. Polish → CI, documentation, compatibility, and secret scans.

### Notes

- `[P]` tasks touch separate files or boundaries and have no incomplete dependency.
- Every user-story task includes a story label and an exact file path.
- Tests are written before their story’s implementation tasks and use Given/When/Then behavior descriptions.
- No task creates real credentials, persistent secret fixtures, or files outside the constitution’s allowed secret policy.
- The plan deliberately avoids built-in Kubernetes, OpenShift, and NATS options; project-specific setup uses `.ws` commands and scripts.
- Generate no `tasks.md` tasks for detailed clipboard mappings until a separate feature specification exists.
