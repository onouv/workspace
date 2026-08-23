# Research: Configured tmux Workspace CLI

## Decision: Use versioned YAML 1.2 with a typed schema

- **Decision**: Treat `.ws` as one YAML 1.2 document with `version: 1` and a top-level
  `windows` sequence. Validate the structure with typed Rust models and a semantic validation
  pass; keep the JSON Schema in the specification and contract artifacts as the normative
  user-facing reference.
- **Rationale**: Standard YAML removes the need for a custom lexer while mappings and sequences
  naturally represent window and recursive pane trees. A version field provides an explicit
  compatibility boundary. Typed models give clear Rust APIs and source-oriented diagnostics.
- **Alternatives considered**: The original indentation DSL was rejected because it duplicated
  YAML’s structure and required custom parsing rules. A generic runtime JSON Schema validator was
  not selected for the first implementation because the small schema is easier to keep readable
  and type-safe with explicit Rust validation; the published schema remains authoritative.

## Decision: Parse and validate with Serde-compatible YAML tooling

- **Decision**: Use `serde` derive models and a maintained Serde-compatible YAML parser selected
  during dependency lockfile creation. The baseline API is `serde_yaml`-compatible; the exact
  maintained package and version MUST pass the project’s dependency, license, and security review.
- **Rationale**: The feature needs structured values, recursive panes, typed environment mappings,
  and line-aware parse errors. Keeping parsing behind a `ConfigParser` boundary prevents the rest
  of the application from depending on a parser-specific representation.
- **Alternatives considered**: Hand-written YAML parsing was rejected. A general schema engine was
  deferred until schema complexity justifies it. Duplicate-key detection and unknown-key rejection
  will be enforced at the parser/validation boundary rather than assumed from defaults.

## Decision: Model tmux as an external process boundary

- **Decision**: Invoke tmux through `std::process::Command` with argument vectors, never through an
  interpolated shell command. Encapsulate probes, session lifecycle, window creation, pane splits,
  environment setup, and attach/switch operations behind a `TmuxClient` boundary.
- **Rationale**: tmux is an installed executable with version- and platform-specific behavior.
  An adapter makes failures actionable, enables fake-command integration tests, and protects
  session names and paths from shell injection.
- **Alternatives considered**: A tmux library binding was rejected for the MVP because tmux’s CLI is
  the stable integration surface and a binding would add maintenance and portability cost.

## Decision: Validate the complete definition before mutating tmux

- **Decision**: Parse, schema-validate, resolve paths, validate names/positions/environment keys,
  and build an immutable launch plan before creating a session. If setup fails after creation,
  remove only the session created by the current invocation.
- **Rationale**: This prevents malformed `.ws` files from leaving partial workspaces and gives the
  existing-session fast path a strict guarantee that the file is never read.
- **Alternatives considered**: Incremental parse-and-create was rejected because it creates partial
  state and makes rollback difficult.

## Decision: Keep credential access lazy and delegated to pass/GPG-agent

- **Decision**: `up`, `change`, `down`, help, and config-reference commands do not retrieve secrets.
  Future credential utilities invoke `pass` for one requested entry and rely on the user’s existing
  GPG-agent cache. `ws` never receives, stores, or exports the master passphrase.
- **Rationale**: The existing implementation eagerly exported many credentials into tmux, and even
  help-like invocations could touch GPG. On-demand access minimizes exposure and preserves the
  operating-system/user-agent security boundary.
- **Alternatives considered**: Copying all values into session environment variables was rejected.
  An in-process passphrase cache was rejected because it duplicates GPG-agent responsibilities.

## Decision: Separate current-client switching from non-disruptive startup

- **Decision**: `ws change SESSION_NAME` uses the current tmux client and requires an existing
  session. `ws up SESSION_NAME` creates or reuses the target, then—when already inside tmux—uses a
  `TerminalLauncher` adapter to open a separate attachable terminal context. If no launcher is
  available, it returns a recoverable error without switching the current client.
- **Rationale**: tmux’s `switch-client` changes the current client; it cannot simultaneously preserve
  that client and present another session. A launcher boundary makes this distinction explicit and
  avoids accidental nested tmux clients.
- **Alternatives considered**: Silently nesting `tmux attach` inside a pane was rejected. Making
  `up` always switch clients was rejected because it violates the non-disruptive workflow.

## Decision: Use explicit, stable command exit categories

- **Decision**: Define internal error categories and map them to stable non-zero CLI statuses:
  invalid CLI input, invalid configuration, unavailable dependency, failed workspace operation,
  destructive-action refusal, and credential failure. Exact numeric values are documented in the
  CLI contract and tested at process level.
- **Rationale**: Scripts need reliable behavior while users need actionable diagnostics. Typed errors
  keep command handlers readable and prevent accidental secret disclosure.
- **Alternatives considered**: Returning one generic failure code was rejected because callers cannot
  distinguish safe retry, configuration correction, and authorization problems.
