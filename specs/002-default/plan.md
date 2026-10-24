# Implementation Plan: Default Workspace File

**Branch**: `002-default` (Gitflow: `feature/default-workspace-file` from `develop`) |
**Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/002-default/spec.md`

## Summary

When `ws up [SESSION_NAME]` creates a new session and `.ws` is missing or empty, `ws` writes the
fixed default content to `.ws`. Then `ws` loads the file through the normal loader. The default
workspace has four windows: `root`, `file` (`yazi`), `git` (`lazygit`), `claude` (`claude`). A new
module prepares the setup file before the load. It uses an exclusive create for a missing file and
a checked in-place write for an empty file. If a write is not possible, `ws` uses the same content
from memory and shows a warning. The default content is one compiled-in constant. It replaces the
old one-window default. Details: [research.md](./research.md).

## Technical Context

**Language/Version**: Rust stable, edition 2024

**Primary Dependencies**: `clap` (derive), `thiserror`, `yaml_serde`. No new crates.

**Storage**: The project file `.ws`. A transient temporary file in the project folder during an
exclusive create (R3).

**Testing**: `cargo test`; unit tests for the setup-file module with `tempfile`; `assert_cmd`
tests with `tests/fixtures/fake_tmux.rs` in a new `tests/default_workspace.rs`; one test against a
real tmux server in `tests/real_tmux.rs`.

**Target Platform**: Linux and macOS with tmux 3.x

**Project Type**: CLI

**Performance Goals**: At most one extra filesystem write on the create path. No extra tmux call.

**Constraints**: Never write through a symlink. Never overwrite a non-empty file. No prompts. All
file messages and warnings on stderr. A write failure does not change the exit status (FR-009).

**Scale/Scope**: About 6 source files changed, 4 new modules, 1 new test file, docs updated.

All items are known. No NEEDS CLARIFICATION.

## Constitution Check

| Principle | Status | Note |
|-----------|--------|------|
| I. CLI-first | Pass | No new argument. `ws help config` shows the default content (FR-011). File message and warnings on stderr. |
| II. Testable core | Pass | File preparation returns an outcome value. `up.rs` sends the line through the `Notifier`. No terminal writes in the module. |
| III. Safe failure | Pass | Exclusive create; overwrite only of a regular, still-empty file; no write through symlinks. A write failure gives a warning, not an error. Causes show the OS error kind only. |
| IV. Tests | Pass | One test per scenario id (17 ids), `given_when_then` names, `// Scenario:` markers. Retire the two 001 US4-AS1/AS2 tests in `tests/lifecycle.rs`. |
| V. Portable | Pass | Same behavior with and without a TTY. Unix-only metadata checks are behind `cfg(unix)`, the target platforms. |
| VI. Bounded agent | Pass | Scope is this spec. |
| VII. Idiomatic Rust | Pass | One type per file. `up.rs` does not grow much: the new logic is in its own module. |
| VIII. Secrets | N/A | No secrets. The path in a file message is not secret. |
| IX. Bounded/immutable | Pass | No retry loop: one create attempt, one overwrite attempt, at most one re-classification, then a fixed fallback. |
| X. Writing style | Pass | STE style in docs and stderr lines. FR-011 requests the doc change. |
| Gitflow | Note | Current branch is `develop`. Create `feature/default-workspace-file` from `develop` before implementation. |

Post-design re-check: Pass. One documented exception (Complexity Tracking): the residual race in
R4 (no atomic compare-and-write on POSIX).

## Project Structure

### Documentation (this feature)

```text
specs/002-default/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/cli.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── config/
│   ├── default_content.rs      # NEW: DEFAULT_CONTENT constant (the normative text)
│   ├── parser.rs               # empty source parses DEFAULT_CONTENT; load() reports NotFound as Io
│   ├── workspace_definition.rs # remove default_for (old one-window default)
│   └── mod.rs                  # export DEFAULT_CONTENT
├── lifecycle/
│   ├── setup_file.rs           # NEW: prepare(project_dir) -> PreparedSetup
│   ├── prepared_setup.rs       # NEW: PreparedSetup { source, notice }
│   ├── setup_source.rs         # NEW: SetupSource (File | InMemoryDefault)
│   ├── setup_notice.rs         # NEW: SetupNotice + Display (contract texts)
│   ├── up.rs                   # call setup_file::prepare before load; send the notice
│   └── mod.rs                  # register new modules
doc/config-lang.md              # default content, create and overwrite rules (FR-011)
README.md                       # replace the "one default window" text

tests/
├── default_workspace.rs        # NEW: US1–US4 process-level tests with fake tmux
├── lifecycle.rs                # retire 001 US4-AS1/AS2 tests
├── config_language.rs          # empty document now gives the four-window default
└── real_tmux.rs                # US1-AS1/AS3 against a real tmux server
```

**Structure Decision**: Single Rust crate. The library crate `ws::config` has no write effects; it
owns only the constant and the parse rule. The file write is a lifecycle effect in the binary
crate next to `up.rs`.

## Complexity Tracking

| Item | Why needed | Simpler alternative rejected because |
|------|------------|--------------------------------------|
| Residual race in the empty-file overwrite (R4, FR-008) | POSIX has no atomic compare-and-write; advisory locks do not bind editors | Rename-aside (R4) can lose content silently into the moved file |
