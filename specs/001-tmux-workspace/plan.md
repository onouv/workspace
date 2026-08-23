# Implementation Plan: Configured tmux Workspace CLI

**Branch**: `001-tmux-workspace` | **Date**: 2026-08-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-tmux-workspace/spec.md`

## Summary

Implement `ws` as a Rust 2024 CLI that creates or reconnects to named tmux sessions from a
project-local, versioned YAML `.ws` document. The design separates argument parsing, YAML/schema
validation, launch-plan construction, tmux effects, terminal launching, and future credential
utilities. Existing-session detection happens before configuration loading; missing sessions are
created only after complete validation. In-tmux `up` preserves the current client through a separate
terminal launcher, while `change` explicitly switches the current tmux client.

## Technical Context

**Language/Version**: Stable Rust, Rust 2024 edition

**Primary Dependencies**: `clap` derive, `serde`, a maintained Serde-compatible YAML parser,
`thiserror`, `anyhow`, `dialoguer`; test dependencies `assert_cmd`, `predicates`, and `tempfile`

**Storage**: No persistent application database. Project-local `.ws` YAML files are read-only input;
passwords remain in external `pass`/GPG-agent or approved secret stores.

**Testing**: `cargo test`, unit tests for parser/validator/plan logic, process-level CLI tests with
`assert_cmd` and `predicates`, disposable tmux integration tests with fake external commands

**Target Platform**: Local Unix-like development environments with tmux for the MVP; terminal and
clipboard adapters remain isolated for later platform support

**Project Type**: Rust command-line application

**Performance Goals**: Help and config-reference commands complete without external dependency
access; valid workspace setup reaches tmux creation without avoidable repeated parsing; no indefinite
waits for prompts

**Constraints**: No secrets in repository or runtime output; existing sessions bypass `.ws`; complete
configuration validation precedes tmux mutation; shell-generated arguments use argv boundaries; current
in-tmux client remains unchanged by `up`; implementation follows the project constitution’s Rust
readability and quality gates

**Scale/Scope**: One local project definition at a time; recursive window/pane trees of practical
development-workspace size; no daemon or multi-user service

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **CLI-first**: PASS. `clap` derive owns commands, arguments, help, and stable output contracts.
- **Testable core**: PASS. Parser, validator, launch planner, tmux client, and terminal launcher are
  separate boundaries; domain logic does not exit the process or own global streams.
- **Errors and safety**: PASS. Typed errors map to stable statuses; no panics for user input;
  destructive `down` requires confirmation or `--yes`; secrets remain outside normal output.
- **Behavior-driven tests**: PASS. Acceptance tests use Given/When/Then cases from the spec and
  process-level assertions for stdout, stderr, status, and external effects.
- **Portability**: PASS with an explicit adapter boundary. TTY detection uses standard Rust APIs;
  terminal launching and tmux differences are isolated and report recoverable failures.
- **Bounded agent autonomy**: PASS. The plan is finite, has explicit artifacts and gates, and
  introduces no unbounded process or workflow.
- **Idiomatic Rust quality**: PASS. Formatting, Clippy, focused modules, documentation, and narrow
  lint scopes are required; every non-trivial type gets a clear home.
- **Secret isolation**: PASS. No repository secret fixtures; `.secrets` remains ignored; credential
  access is lazy through `pass`/GPG-agent and never copied into tmux environments.

## Project Structure

### Documentation (this feature)

```text
specs/001-tmux-workspace/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   └── ws-schema.yaml
└── checklists/
    └── requirements.md
```

### Source Code (repository root)

```text
src/
├── main.rs
├── app.rs
├── cli.rs
├── error.rs
├── help.rs
├── terminal/
│   ├── mod.rs
│   └── launcher.rs
├── config/
│   ├── mod.rs
│   ├── parser.rs
│   ├── validator.rs
│   ├── workspace_definition.rs
│   ├── window_definition.rs
│   └── pane_definition.rs
├── lifecycle/
│   ├── mod.rs
│   ├── up.rs
│   ├── change.rs
│   ├── down.rs
│   └── exit.rs
├── tmux/
│   ├── mod.rs
│   ├── client.rs
│   ├── session.rs
│   ├── window.rs
│   ├── pane.rs
│   └── launch_plan.rs
└── credentials/
    ├── mod.rs
    └── pass_provider.rs

tests/
├── cli_help.rs
├── config_language.rs
├── lifecycle.rs
└── tmux_integration.rs
```

**Structure Decision**: Use one Rust binary with focused modules. Configuration types and tmux
operations are separated from lifecycle command adapters. Each non-trivial struct has a dedicated
module file in accordance with the constitution. Integration tests use fake `tmux`, `pass`, and
terminal-launcher executables where behavior can be isolated; real tmux tests use disposable,
uniquely named sessions and guaranteed cleanup.

## Implementation Phases

### Phase 0: Research and dependency lock

1. Confirm the maintained Serde-compatible YAML parser and its YAML 1.2 behavior, source locations,
   duplicate-key handling, and license/security status.
2. Confirm the minimum supported tmux command forms for session probes, window creation, pane splits,
   environment setup, attach, and client switching.
3. Define the terminal-launcher adapter contract and the initial supported launcher mechanism; no
   launcher must produce a recoverable error rather than a client switch or nested tmux client.
4. Confirm Rust TTY detection and subprocess termination behavior on the MVP platform.

Output: `research.md` decisions are reflected in dependency selection and contracts.

### Phase 1: Design and core implementation

1. Add the approved dependencies and establish the module structure.
2. Implement `clap` command parsing and side-effect-free help/config-reference output.
3. Implement YAML parsing, typed schema validation, duplicate/unknown-key checks, semantic validation,
   path resolution, environment inheritance, and immutable launch-plan construction.
4. Implement the `TmuxClient` adapter and transactional session/window/pane setup with cleanup of
   only sessions created by the current invocation.
5. Implement `up`, `change`, `down`, and `exit` lifecycle behavior, TTY confirmation, `--yes`, and
   separate terminal launching.
6. Add fake-process unit/integration tests, then disposable real-tmux tests where available.
7. Re-run the constitution gates and update documentation/help from the contract artifacts.

Output: working source, tests, and synchronized help/schema documentation.

## Complexity Tracking

No constitution violations require justification. The terminal-launcher and tmux adapters are
explicit boundaries required by the requested in-tmux behavior and portability constraints, not
unnecessary domain abstractions.
