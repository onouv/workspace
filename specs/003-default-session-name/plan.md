# Implementation Plan: Default Session Name

**Branch**: `003-default-session-name` (Gitflow: `feature/default-session-name` from `develop`) |
**Date**: 2026-10-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-default-session-name/spec.md`

## Summary

`ws up` without `SESSION_NAME` derives the session name from the current folder name (`.` and `:`
become `_`). If the session exists and belongs to the same folder, `ws` connects. If it belongs to
a different folder, `ws` refuses. Otherwise `ws` creates it and records the project folder in a
tmux session option. A new `Notifier` dependency writes the one-line message to stderr before
attach. Details: [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable, edition 2024

**Primary Dependencies**: `clap` (derive), `thiserror`, `anyhow` (binary edge). No new crates.

**Storage**: tmux session user option `@ws-project-folder`. No files.

**Testing**: `cargo test`; unit tests for name derivation and folder check; `assert_cmd` tests
with `tests/fixtures/fake_tmux.rs`; disposable tmux tests in `tests/tmux_integration.rs`.

**Target Platform**: Linux and macOS with tmux 3.x

**Project Type**: CLI

**Performance Goals**: At most one extra tmux call (`list-sessions`) on the reconnect path.

**Constraints**: No shell interpolation of the name (001 FR-013). No prompts. Message on stderr
only.

**Scale/Scope**: About 5 source files changed, 2 new modules, 1 new test file.

## Constitution Check

| Principle | Status | Note |
|-----------|--------|------|
| I. CLI-first | Pass | Optional arg in clap; help text updated (FR-009). |
| II. Testable core | Pass | Pure `derive` function; `Notifier` trait at the boundary; tmux through `TmuxClient`. |
| III. Safe failure | Pass | Typed `DeriveError`; refusal instead of a wrong connect; messages without internals. |
| IV. Tests | Pass | One test per scenario id, `given_when_then` names, `// Scenario:` markers. Update 001 US3-AS5 test. |
| V. Portable | Pass | Non-TTY: same behavior, no prompt. Non-UTF-8 names fail clearly. |
| VI. Bounded agent | Pass | Scope is this spec. |
| VII. Idiomatic Rust | Pass | One type per file; modules under 200 lines. |
| VIII. Secrets | N/A | No secrets. Folder path in a diagnostic is not secret. |
| IX. Bounded/immutable | Pass | `DerivedSessionName` validated at construction; no loops with runtime conditions. |
| X. Writing style | Pass | STE style in docs and messages. |
| Gitflow | Note | Current branch is `main`. Create `feature/default-session-name` from `develop` before implementation. |

Post-design re-check: Pass. No violations.

## Project Structure

### Documentation (this feature)

```text
specs/003-default-session-name/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/cli.md
└── tasks.md             # /speckit.tasks
```

### Source Code (repository root)

```text
src/
├── cli.rs                         # UpArgs::session_name -> Option<String>; help text; share name rule
├── session_name/
│   ├── mod.rs                     # shared validity rule (moved from cli.rs)
│   ├── derived_session_name.rs    # DerivedSessionName + derive()
│   ├── derive_error.rs            # DeriveError
│   └── logical_folder.rs          # $PWD vs current_dir resolution
├── app.rs                         # pick explicit or derived name; pass Notifier
├── main.rs                        # StderrNotifier
├── terminal/notifier.rs           # Notifier trait, StderrNotifier
├── tmux/session.rs                # exact `=NAME` target; session_folder() via list-sessions
├── tmux/session_folder.rs         # SessionFolder + folder check
├── tmux/window.rs                 # new-session ... ; set-option @ws-project-folder
└── lifecycle/up.rs                # SessionNameSource; folder check on exists and on duplicate

tests/
├── default_session_name.rs        # US1–US4 process-level tests
├── fixtures/fake_tmux.rs          # list-sessions, set-option, `=NAME`, `;` chains, session path
├── support/mod.rs                 # seed session with folder
└── lifecycle.rs                   # 001 US3-AS5 test: drop the missing-name case
```

**Structure Decision**: Single Rust CLI project. The new `session_name` module holds the name
rules so `cli.rs` stays a thin adapter.

## Complexity Tracking

None.
