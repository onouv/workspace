<!--
Sync Impact Report
- Version change: 1.4.0 -> 1.5.0
- Modified principles:
  - III. Explicit Errors and Safe Failure — added a rule against leaking internal module
    paths, Rust type names, or other implementation detail into user-facing messages.
  - IV. Behavior Is Defined by Tests — added stable `US<story>-AS<n>` acceptance-scenario
    ids, a `// Scenario: US<story>-AS<n>` test marker requirement, one-scenario-one-test
    ownership, and a formal `given_<context>_when_<action>_then_<outcome>` test-naming rule
    (nested `mod given_<context>` blocks permitted, not required).
  - VII. Idiomatic Rust, Readability, and Maintainability — added non-binding size guidance
    for functions and files alongside the existing "readable size" requirement.
- Added principles:
  - IX. Bounded Execution and Immutable Validated State — explicit iteration caps for
    runtime-conditioned loops, a documented maximum recursion depth for externally supplied
    nested structure (the `.ws` pane hierarchy), immutable validated configuration/launch-plan
    types with invariants enforced at construction, and a builder-pattern requirement for
    constructors with more than two arguments or repeated argument types.
- Added sections: none (new content placed within existing principles and one new principle).
- Removed sections: none.
- Follow-up TODOs:
  - Retrofit `specs/001-tmux-workspace/spec.md` acceptance scenarios with `US<story>-AS<n>`
    ids and add matching `// Scenario:` markers to the existing tests in `tests/cli_help.rs`
    and `tests/config_language.rs`.
  - Confirm and document the maximum pane-nesting depth in `src/config/validator.rs` and
    `doc/config-lang.md`.
-->

# Interactive CLI Constitution

## Core Principles

### I. CLI-First, Discoverable User Experience
The command-line interface MUST be specified through `clap`'s derive API and remain the
single source of truth for commands, arguments, options, defaults, help text, and shell
completion metadata. Every user-facing operation MUST have a discoverable `--help` path,
clear validation messages, and stable exit-status semantics. Interactive prompts MUST be
implemented explicitly with a terminal-aware crate such as `dialoguer`; prompts MUST NOT
appear when input is redirected or when an explicit non-interactive mode is selected. The
CLI MUST keep normal results on stdout and diagnostics, progress, and logging on stderr so
that output can be safely piped and scripted.

### II. Testable Core, Thin Adapters
Business rules and application behavior MUST be separated from `clap`, terminal rendering,
and process-global state. Command handlers MUST translate parsed arguments into calls to
testable application services, and domain code MUST NOT call `std::process::exit`, read
from global stdin, or write directly to terminal streams. Terminal concerns MAY use
`console`, `indicatif`, `dialoguer`, or `crossterm` when the interaction requires them, but
those concerns MUST remain at the CLI boundary. This separation keeps interactive behavior
usable from automation and allows core behavior to be tested without a real terminal.

### III. Explicit Errors and Safe Failure
Expected failures MUST be represented as `Result` values and MUST produce actionable,
non-sensitive diagnostics without panicking. Domain and reusable modules SHOULD define
specific errors with `thiserror`; the binary boundary MAY use `anyhow` for context-rich
reporting and for mapping failures to documented exit codes. User input, filesystem,
configuration, and external-process failures MUST identify the relevant operation and
suggest a corrective action where practical. Secrets, tokens, and private input MUST NOT
be emitted in errors, logs, traces, or shell commands. Destructive operations MUST require
an explicit confirmation or an equivalent non-interactive opt-in flag. User-facing error
messages MUST NOT expose internal module paths, Rust type names, or other implementation
detail that does not help correct the problem; such detail belongs in a debug
representation or an opt-in verbose mode, not the default message.

### IV. Behavior Is Defined by Tests
Every new command, option, prompt flow, output contract, and error-path change MUST include
or update automated tests that describe its observable behavior. Unit tests MUST cover
pure application and domain logic; CLI integration tests MUST use `assert_cmd` and
`predicates` or equivalent process-level tools to verify arguments, stdout, stderr, and
exit codes. Temporary files and directories MUST use `tempfile`, and tests MUST avoid
network access, wall-clock assumptions, and shared mutable state unless the dependency is
explicitly isolated. Snapshot testing with `insta` MAY be used for deliberately stable,
substantial output, but snapshots MUST be reviewed as part of the change.
Tests MUST be built according to behavior driven testing, i.e. the MUST have clauses "Given ...", "When ...", "Then ...", in that order. This structure MUST show up in the implementtaion as well as the outputs.
Test names MUST follow the pattern `given_<context>_when_<action>_then_<outcome>` (appending
`_and_<clause>` for an additional clause), and each test MUST assert a single outcome; tests
MAY additionally group shared context under nested `mod given_<context>` blocks when that
improves readability. Every acceptance scenario in a feature specification MUST carry a
stable identifier of the form `US<story>-AS<n>`. The test verifying that scenario's
observable behavior MUST reference it with a `// Scenario: US<story>-AS<n>` comment as the
first line of the test body; no such marker MUST exist without a matching specification id.
Each scenario MUST be owned by exactly one test — do not duplicate the same scenario id
across multiple tests. A feature's `tasks.md` or `spec.md` SHOULD maintain a traceability
note mapping each scenario id to its owning test.

### V. Portable, Accessible Terminal Behavior
The tool MUST work correctly on supported Unix and Windows terminals and in a non-TTY
context such as a pipe, CI job, or redirected input. Color, spinners, progress bars, and
interactive cursor control MUST be capability-aware and MUST have a plain-text or disabled
fallback; color SHOULD be controllable through conventional flags or environment behavior.
Human-readable output MUST be concise and legible, while machine-consumable output MUST
use an explicitly selected stable format such as JSON via `serde` and `serde_json` when
that format is required. Locale, terminal width, and ANSI support MUST NOT change the
meaning of results.

### VI. Bounded Agent Autonomy and Human Checkpoints

Agent-assisted work MUST operate within an explicit scope, finite tool or iteration budget, and clear completion criteria. If unclear Agents MUST retrieve that information from user before proceeding. Agents MUST NOT execute unbounded loops, recursively invoke agents or workflows, run indefinite processes, or repeatedly retry an unchanged failure. Agents MUST NOT work while the zed editor is closed.

Unless a task explicitly defines stricter limits, an agent MUST stop after three failed attempts at the same operation or three iterations without measurable progress.

Agents MUST pause and request human approval before performing destructive or irreversible actions, modifying files outside the declared scope, accessing external services, sending data, changing dependencies or security controls, or materially expanding the task. A human MUST remain responsible for final acceptance of changes.

When stopping, the agent MUST preserve the work completed, report the actions attempted, the reason for stopping, remaining risks or blockers, and a recommended next step. Tool budgets, retry limits, and approval checkpoints MUST be reviewable from the task record or execution log.

Agents MUST apply a brief but concise style when producing output to the prompt CLI. I do not want to be swamped in text.

### VII. Idiomatic Rust, Readability, and Maintainability
Rust code MUST follow the conventions of the Rust API Guidelines and be formatted with
`rustfmt`; formatting MUST NOT be bypassed to conceal unclear structure. Code MUST pass
Clippy with warnings treated as errors, and lint suppressions MUST be narrow, documented,
and justified at the smallest applicable scope. Names MUST describe domain intent, functions
MUST remain focused, and modules MUST have clear responsibilities. Prefer straightforward,
idiomatic Rust over clever abstractions, dense expressions, unnecessary macros, or premature
generality. Ownership, borrowing, lifetimes, and error boundaries MUST be expressed in a way
that a maintainer can understand without reconstructing hidden invariants.

Public types, functions, commands, configuration keys, and non-obvious invariants MUST have
concise Rustdoc or nearby documentation. Comments MUST explain rationale, safety constraints,
or externally imposed behavior rather than restating syntax. Implementations MUST avoid
unnecessary cloning, allocation, dynamic dispatch, and `unwrap` or `expect` on recoverable
runtime paths; when one is necessary, its invariant MUST be explicit and locally verifiable.
Unsafe code remains subject to the documentation and review requirements in this constitution.

Changes MUST preserve a coherent module and dependency structure, remove dead code, and keep
related behavior close together. A reviewer MUST be able to trace a CLI request through parsing,
validation, application logic, and external effects without relying on implicit global state.
Complexity, non-idiomatic patterns, and deviations from standard Rust style MUST have a clear
benefit documented in the change description and focused tests where behavior is affected.
Modules MUST be of a readable size. Each module is a separate file. Each non-trivial type (struct) MUST be in its own file.
As non-binding guidance, a function SHOULD stay under roughly 40-60 lines and a module file
SHOULD stay under roughly 200 lines; exceeding either is acceptable when splitting further
would harm cohesion or readability, but MUST then be a deliberate choice, not an accident.

### VIII. Secret Material Isolation and Repository Hygiene
Secrets of any kind MUST NOT be committed to the repository, Git index, Git history,
release artifacts, examples, fixtures, logs, or documentation. Secrets include passwords,
passphrases, API tokens, access keys, private keys, certificates containing private material,
cookies, session credentials, and production or personal data requiring protection. Any
secret that must exist as a project-local file MUST be stored under a project-local
`.secrets/` directory, which MAY contain further subdirectories for organization. Secrets
MAY instead remain in an approved external secret manager such as `pass` with GPG-agent, an
OS keychain, or a CI secret store; external secret-manager contents MUST NOT be copied into
the repository’s `.secrets/` directory.

The repository’s `.gitignore` MUST exclude `.secrets/` and all of its descendants at every
level, including nested `.secrets` directories, and MUST NOT contain negation rules that
re-include secret files. The exclusion MUST be present before any secret-bearing file is
created. Contributors MUST NOT use force-add, alternate Git paths, generated artifacts, or
renames to bypass these exclusions. The `.gitignore` file itself MUST remain reviewed and
tracked.

Secret values MUST NOT be placed in command-line arguments, shell history, ordinary
configuration, source code, test snapshots, tmux environment settings, process logs, or
stdout/stderr. Tests MUST use generated ephemeral values in temporary directories and MUST
prove that credentials are not persisted. Documentation and sample `.secrets` layouts MUST
contain placeholders only, never live or realistic credentials.

Changes that add or modify secret handling MUST include a review of storage location,
permissions, lifetime, redaction, and cleanup behavior. Before review and in CI, the project
MUST verify that no secret-bearing path is tracked and SHOULD run an appropriate secret
scanner. If a secret is suspected to have entered Git history, it MUST be revoked or rotated
and removed from history through the project’s approved incident procedure; deleting the
working-tree file alone is insufficient.

### IX. Bounded Execution and Immutable Validated State
Every loop whose continuation depends on a runtime condition — a retry loop, a polling loop,
or race-recovery handling such as concurrent session-creation contention — MUST carry an
explicit iteration cap or deadline and MUST define its behavior when that limit is reached;
silent truncation is not acceptable. Iterating a finite in-memory collection satisfies this
requirement inherently and needs no added cap. An intentionally long-running top-level loop
(such as a run/serve loop) is the deliberate exception: it MUST be named to make that intent
obvious and MUST NOT hold an external resource or lock across iterations.

Recursion MUST remain the exception rather than the default. Recursive traversal of
externally supplied structure — including the recursive `.ws` pane hierarchy — MUST enforce
an explicit, documented maximum depth and MUST reject deeper input with a clear validation
error rather than risking unbounded stack growth.

Validated configuration and launch-plan types (such as `WorkspaceDefinition`,
`WindowDefinition`, `PaneDefinition`, and any tmux launch plan) MUST be immutable once
constructed and MUST validate their invariants at construction, so that any instance in
scope is known-valid for its lifetime. A constructor requiring more than two arguments, or
any two arguments of the same type, MUST use a builder rather than a positional constructor
to avoid argument-order mistakes.

## Technology and Runtime Constraints

The project MUST use the stable Rust toolchain and Rust 2024 edition unless a documented
exception is approved. `clap` with derive support is the required argument-parser and
command-modeling foundation. Crates MUST be selected by responsibility rather than added
speculatively: `thiserror` for typed domain errors, `anyhow` at the executable boundary,
`dialoguer` for simple prompts, `console` or `crossterm` for terminal capability handling,
`indicatif` for progress reporting, and `serde` with a format-specific crate for explicit
serialization needs are approved defaults. `assert_cmd`, `predicates`, and `tempfile` are
approved test dependencies; `insta` and `criterion` MAY be added when their maintenance
and review costs are justified.

Dependencies MUST be kept current within their compatible major versions, committed in
`Cargo.lock` for this executable, and reviewed for licensing, maintenance, and security
impact. The project MUST avoid unsafe code unless its necessity, invariants, and review
plan are documented. Configuration and environment behavior MUST be documented rather than
implicitly inferred from incidental crate defaults.

## Development Workflow and Quality Gates

Changes MUST state the affected CLI contract and include user-visible help or documentation
updates when commands or options change. Before review, contributors MUST run `cargo fmt
--check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo test --all-targets --all-features`; CI MUST run the same gates on every change.
Integration tests that invoke the binary MUST verify both interactive-capable and
non-interactive paths for behavior that differs by TTY availability. A change that alters
an existing command, output format, exit code, configuration key, or destructive-action
safeguard MUST document the compatibility impact and migration path.

Reviews MUST check the constitution, test coverage of the observable contract, failure
safety, stream separation, non-TTY behavior, and secret-storage rules. Reviews MUST verify
that `.gitignore` excludes `.secrets/` and descendants before any local secret path is used.
Performance work MUST include a reproducible measurement or benchmark before introducing
complexity. Documentation and examples MUST use commands that can run without undisclosed
local state or credentials.

The repository MUST follow the Gitflow branching model:
- `main` contains production-ready code.
- `develop` contains the integration state for the next release.
- Feature branches MUST be created from `develop` and named
  `feature/<short-description>`.
- Feature branches MUST merge back into `develop` through review.
- Release branches MUST use `release/<version>` and may merge into both
  `main` and `develop`.
- Hotfix branches MUST use `hotfix/<version>` and may merge into both
  `main` and `develop`.
- Direct commits to `main` and `develop` MUST NOT be made for normal development.
- Production releases MUST be tagged from `main`.
- Completed feature, release, and hotfix branches SHOULD be deleted after merging
 
## Governance

This constitution is the governing engineering standard for the project. A pull request
that conflicts with it MUST either be revised or include an explicit exception explaining
the trade-off, affected guarantees, scope, owner, and expiration or removal plan. Exceptions
require maintainer approval and MUST NOT weaken security or data-loss safeguards silently.

Amendments MUST update this file, include a Sync Impact Report, increment the semantic
version, and describe any required code, test, documentation, or migration work. Versioning
follows semantic versioning: a major version removes or materially redefines a principle; a
minor version adds a principle or materially expands requirements; a patch version clarifies
wording without changing governance intent. Maintainers MUST review compliance during code
review and may reject changes that lack the required validation evidence. When this document
conflicts with a lower-level guide, the constitution takes precedence until formally amended.

**Version**: 1.5.0 | **Ratified**: 2026-08-23 | **Last Amended**: 2026-09-13
