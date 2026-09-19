# Feature Specification: Configured tmux Workspace CLI

**Feature Branch**: `001-tmux-workspace`

**Created**: 2026-08-23

**Status**: Draft

**Input**: User description: Create `ws`, a CLI that starts or reconnects to a configured tmux workspace from a local `.ws` setup file, with secure access to the `pass` password store for future interactive commands.

## Existing Implementation Review

A local executable at `/home/ono/bin/ws` was inspected for interface ideas. Its help text establishes
these useful command concepts:

The new CLI adopts the explicit `up`, `down`, and `clip` command namespaces from the legacy help,
with this normalized interface:

```text
ws
ws --help
ws help config
ws up SESSION_NAME
ws change SESSION_NAME
ws down [SESSION_NAME] [--yes]
ws clip NAMESPACE ITEM
ws exit
```

The contract improves the legacy behavior in the following ways:

- `ws --help`, `ws help`, and `ws --version` MUST be handled before tmux or password-store access.
- `ws up SESSION_NAME` MUST take the session name as a positional argument instead of prompting for
  it. This makes the command scriptable and prevents a hidden prompt in CI or redirected input.
- When `ws up SESSION_NAME` is invoked from inside tmux, it MUST preserve the current client and open
  the target workspace in a separate terminal client/window. `ws change SESSION_NAME` is the explicit
  operation for switching the current client.
- The project-local `.ws` file replaces hard-coded windows and infrastructure-specific startup
  switches. Project services and integrations are represented by ordinary configured commands,
  working directories, environment values, and optional custom scripts instead of built-in product
  flags.
- In the legacy help, `<workspace-name>` for `ws down` means the target tmux session name. The new
  interface consistently calls this value `SESSION_NAME`.
- Workspace startup MUST NOT eagerly retrieve and export every password-store value into tmux panes.
  Credential-dependent commands MUST use `pass` on demand through the user’s existing GPG-agent flow.
- `ws exit` is retained as a possible convenience alias for detaching, but is not required for the
  workspace-layout MVP; normal tmux detach behavior remains available.

## .ws Configuration Language (Normative MVP Reference)

The `.ws` file is a UTF-8 YAML 1.2 document validated against the versioned schema below. This
section is the normative user-facing configuration contract for the MVP and MUST be published
through `ws help config`. The implementation MAY use a different internal representation, but its
accepted YAML syntax, schema validation, and semantic behavior MUST match this reference.

A `.ws` file contains one YAML document with a top-level mapping. YAML comments, quoted and plain
scalar styles, sequences, and mappings MAY be used as defined by YAML 1.2. Custom YAML tags,
multi-document streams, and duplicate mapping keys MUST be rejected. The schema is expressed below
as YAML using JSON Schema 2020-12:

```yaml
$schema: https://json-schema.org/draft/2020-12/schema
$id: urn:ws:workspace-schema:v1
title: ws workspace definition
type: object
required: [version, windows]
additionalProperties: false
properties:
  version:
    const: 1
  windows:
    type: array
    minItems: 1
    items:
      $ref: '#/$defs/window'
$defs:
  window:
    type: object
    required: [name, path]
    additionalProperties: false
    properties:
      name:
        type: string
        minLength: 1
      path:
        type: string
        minLength: 1
      command:
        type: string
        minLength: 1
      env:
        type: object
        propertyNames:
          pattern: '^[A-Za-z_][A-Za-z0-9_]*$'
        additionalProperties:
          type: string
      panes:
        type: array
        items:
          $ref: '#/$defs/pane'
  pane:
    type: object
    required: [pos]
    additionalProperties: false
    properties:
      pos:
        enum: [left, right, top, bottom]
      id:
        type: string
        minLength: 1
      path:
        type: string
        minLength: 1
      command:
        type: string
        minLength: 1
      env:
        type: object
        propertyNames:
          pattern: '^[A-Za-z_][A-Za-z0-9_]*$'
        additionalProperties:
          type: string
      panes:
        type: array
        items:
          $ref: '#/$defs/pane'
```

The schema and these semantic rules are normative:

- `version` MUST be `1`. A future incompatible syntax or semantic change MUST use a new language
  version rather than silently changing the meaning of version 1 files.
- Each window MUST have `name` and `path`. `command`, `env`, and `panes` are optional. A missing
  command starts the user’s normal shell.
- Each pane MUST have `pos`. `id`, `path`, `command`, `env`, and `panes` are optional. A pane without
  a command starts the normal shell; a pane without a path inherits the containing window or pane
  path.
- `windows` and `panes` are ordered YAML sequences. Window order determines tmux window order, and
  pane nesting determines the split hierarchy. Window names MUST be unique. Pane identifiers, when
  supplied, MUST be unique within their containing window.
- Recursive pane nesting MUST NOT exceed a depth of 32 levels beneath a window. A document nesting
  panes deeper than 32 levels MUST be rejected with a source-oriented validation error rather than
  risking unbounded recursion.
- `env` is a mapping from environment-variable names to string values. Window values are inherited
  by all descendant panes; pane values override inherited values. Values that look like YAML booleans,
  numbers, or null SHOULD be quoted so they remain strings.
- Relative paths resolve against the directory containing the `.ws` file. `left` and `right` create
  horizontal splits; `top` and `bottom` create vertical splits relative to the containing pane. Pane
  dimensions use tmux defaults unless a later language version adds sizing.
- `command` is a shell command string interpreted by the user’s configured shell. It MAY invoke an
  executable, custom Bash script, pipeline, or other shell operation. Generated tmux arguments such
  as session names and paths MUST still be passed without unsafe shell interpolation.
- Unknown properties, wrong YAML types, invalid positions, duplicate keys, missing required values,
  and invalid environment names MUST be rejected with a source location when available. An empty
  file is treated as the documented default workspace; a non-empty file MUST validate against the
  schema.

A canonical valid document is:

```yaml
version: 1
windows:
  - name: root
    path: .
    command: bash ./scripts/start-root.sh
  - name: development
    path: ./app
    panes:
      - pos: left
        id: git
        command: git status
      - pos: right
        id: services
        panes:
          - pos: top
            id: logs
            command: bash ./scripts/observe.sh
            env:
              APP_ENV: development
          - pos: bottom
            id: shell
```

The reference schema and example MUST remain synchronized across this specification, `ws help config`,
parser tests, and user documentation.

## Clip Credential Mapping Language (Normative Reference)

User Story 8's security design decision deferred the detailed `ws clip NAMESPACE ITEM` entry
mapping to a separate specification. This section is that specification: it defines the mapping
configuration format, its file locations, and its merge rule. It MUST be published through
`ws help clip`, mirroring how the `.ws` reference is published through `ws help config`.

Two optional UTF-8 YAML 1.2 documents supply the mapping, both using the schema below:

- **User-level file**: `$XDG_CONFIG_HOME/ws/clip.yaml`, or `$HOME/.config/ws/clip.yaml` when
  `XDG_CONFIG_HOME` is unset or empty. Holds entries available to every project, such as personal
  Git and container-registry credentials.
- **Project-level file**: `.ws-clip` in the project directory, alongside `.ws`. Holds entries
  specific to one project, such as a project-scoped registry token.

Neither file is itself secret: an entry names *where* a credential lives (a `pass` store path) or
supplies a fixed non-secret value (such as a username); it MUST NOT contain a live credential
value. Both files remain outside `.secrets/` and are safe to keep in ordinary dotfiles, consistent
with the constitution's secret-isolation principle.

```yaml
$schema: https://json-schema.org/draft/2020-12/schema
$id: urn:ws:clip-mapping-schema:v1
title: ws clip credential mapping
type: object
required: [version, entries]
additionalProperties: false
properties:
  version:
    const: 1
  entries:
    type: object
    propertyNames:
      minLength: 1
    additionalProperties:
      type: object
      propertyNames:
        minLength: 1
      additionalProperties:
        $ref: '#/$defs/entry'
$defs:
  entry:
    type: object
    additionalProperties: false
    oneOf:
      - required: [pass]
        properties:
          pass:
            type: string
            minLength: 1
      - required: [literal]
        properties:
          literal:
            type: string
            minLength: 1
```

The schema and these semantic rules are normative:

- `version` MUST be `1`.
- `entries` is a mapping from namespace to a mapping from item to one entry. `NAMESPACE` and `ITEM`
  in `ws clip NAMESPACE ITEM` select `entries.NAMESPACE.ITEM`.
- Each entry MUST set exactly one of `pass` or `literal`, never both, never neither.
  - `pass` names one `pass` store entry, resolved on demand through `PassProvider::reveal` at the
    moment `ws clip` runs (FR-025 through FR-028 govern this access).
  - `literal` is a fixed, non-secret value (for example, a username) copied to the clipboard
    without contacting `pass`.
- A missing user-level or project-level file is treated as an empty mapping, the same way a
  missing `.ws` is treated as the documented default — not an error.
- The two files are merged before lookup: the project-level file's entries take precedence over
  the user-level file's entries for the same `(NAMESPACE, ITEM)` pair; entries that appear in only
  one file are kept as-is. Neither file may reference the other.
- Unknown top-level or entry properties, a wrong YAML type, an entry missing both `pass` and
  `literal` or setting both, and an unsupported `version` MUST be rejected with a source location
  when available.
- A `ws clip NAMESPACE ITEM` invocation naming a pair absent from the merged mapping MUST be
  rejected with an actionable error pointing at `ws help clip`, and MUST NOT contact `pass` or the
  clipboard provider.

A canonical valid user-level document is:

```yaml
version: 1
entries:
  git:
    ssh: {pass: repos/github/ssh/my-key}
    cli: {pass: repos/github/token}
    user: {literal: my-username}
    password: {pass: repos/github/my-account}
  docker:
    token: {pass: registries/dockerhub/tokens/build}
    user: {literal: my-username}
    password: {pass: registries/dockerhub/my-account}
```

### Clipboard provider selection

`ws clip` sends the resolved value to an external clipboard provider command, configured through
the `WS_CLIPBOARD_PROVIDER` environment variable using the same "program followed by
space-separated arguments" convention as `WS_TERMINAL_LAUNCHER`. When unset, `ws` uses
`xclip -selection clipboard`. `ws` never shell-interprets the configured value, writes the
resolved credential only to the provider's standard input, and never reads or forwards the
provider's own stdout or stderr — an unavailable or refusing provider MUST surface as a
recoverable error without falling back to printing the value.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Discover the command interface safely (Priority: P1)

As a developer, I want to view `ws` help and version information without starting tmux or touching
my password store, so that the CLI is usable for discovery even when optional runtime dependencies
are unavailable.

**Why this priority**: The reviewed implementation attempted password-store access before recognizing
help-like input. That made basic discovery fail in environments without a usable GPG agent and could
cause unexpected secret prompts.

**Independent Test**: Run `ws`, `ws help`, `ws --help`, and `ws --version` with tmux unavailable and
with a deliberately unusable password-store environment; verify each produces useful output and does
not invoke either external dependency.

**Acceptance Scenarios**:

1. **US1-AS1**. **Given** tmux and the password store are unavailable, **When** the user invokes `ws --help`,
   **Then** `ws` prints command usage, examples, and the available command groups and exits zero.
2. **US1-AS2**. **Given** no command arguments are supplied, **When** the user invokes `ws`, **Then** `ws` prints
   concise help and exits zero without prompting or changing any session.
3. **US1-AS3**. **Given** the user requests version information, **When** the user invokes `ws --version`,
   **Then** `ws` prints the version and exits zero without reading `.ws`, contacting tmux, or
   accessing `pass`.
4. **US1-AS4**. **Given** a command or option is unknown, **When** the user invokes it, **Then** `ws` reports the
   error on stderr, includes a path to help, and exits non-zero without side effects.
5. **US1-AS5**. **Given** the user wants to learn the `.ws` language, **When** the user invokes `ws help config`,
   **Then** `ws` prints the normative YAML schema, semantic rules, pane-position meanings, and a
   complete example without contacting tmux or the password store.

---

### User Story 2 - Learn the .ws configuration language (Priority: P1)

As a developer, I want a formal, locally accessible reference for `.ws` files so that I can author
and validate workspace layouts without guessing parser behavior or searching external documentation.

**Why this priority**: The configuration language is the project’s primary user interface. Defining
its YAML schema early prevents incompatible interpretations across parser, examples, help, and tests.

**Independent Test**: Run `ws help config` and verify that it contains the formal YAML schema, all
supported keys and types, constraints, pane-position semantics, custom-command guidance, and the
supplied nested-pane example; compare parser acceptance tests against the documented schema.

**Acceptance Scenarios**:

1. **US2-AS1**. **Given** the user invokes `ws help config`, **When** the command runs without tmux or `pass`,
   **Then** it prints the complete `.ws` language reference and exits zero without side effects.
2. **US2-AS2**. **Given** a `.ws` document is valid YAML and follows the schema and semantic rules, **When** the
   user invokes `ws up SESSION_NAME`, **Then** the document is accepted and produces the described
   workspace.
3. **US2-AS3**. **Given** a `.ws` document violates YAML syntax, schema types, required keys, nesting, or semantic
   rules, **When** the user invokes `ws up SESSION_NAME`, **Then** `ws` rejects it with a source
   location and does not create a new session.
4. **US2-AS4**. **Given** a window or pane uses `command: bash ./scripts/setup.sh`, **When** its command starts,
   **Then** the script runs in the declaration’s resolved working directory with the declared
   environment, allowing project-specific services and integrations without a built-in option.

---

### User Story 3 - Create or reconnect to a named workspace (Priority: P1)

As a developer, I want `ws up SESSION_NAME` to either reinitialize a fresh workspace from the local
`.ws` definition or reconnect to an existing tmux session, so that I can work on another project
without disturbing the workspace I am currently using.

**Why this priority**: Reconnecting must preserve an existing session, while starting a missing target
from the local definition must provide a repeatable workspace setup. The operation must not clone live
processes or panes from the current session.

**Independent Test**: Start `SESSION_ONE`, invoke `ws up OTHER_SESSION` from inside it with both a
missing and an existing `OTHER_SESSION`, and verify that the missing case applies `.ws` while the
existing case ignores `.ws`; in both cases verify that `SESSION_ONE` remains attached and unchanged.

**Acceptance Scenarios**:

1. **US3-AS1**. **Given** a tmux session exists with the requested name and `ws up SESSION_NAME` is invoked
   outside tmux, **When** the command runs, **Then** `ws` attaches to that session without reading,
   parsing, or applying the local `.ws` file.
2. **US3-AS2**. **Given** the user is in `SESSION_ONE` and `OTHER_SESSION` does not exist, **When** the user invokes
   `ws up OTHER_SESSION`, **Then** `ws` reads and applies the local `.ws` definition, creates a
   reinitialized `OTHER_SESSION` workspace in a separate terminal client/window, and leaves
   `SESSION_ONE` attached and unchanged.
3. **US3-AS3**. **Given** the user is in `SESSION_ONE` and `OTHER_SESSION` already exists, **When** the user invokes
   `ws up OTHER_SESSION`, **Then** `ws` opens a separate terminal client/window connected to
   `OTHER_SESSION`, leaves `SESSION_ONE` attached and unchanged, and does not read, parse, or apply
   the local `.ws` file.
4. **US3-AS4**. **Given** the requested session is already attached to the current tmux client, **When** the user
   invokes `ws up SESSION_NAME`, **Then** `ws` does not switch the current client or create a
   duplicate session, and reports or opens the documented separate terminal context.
5. **US3-AS5**. **Given** the session name is missing, whitespace-only, or invalid for tmux, **When** the user
   invokes `ws up SESSION_NAME`, **Then** `ws` rejects the input with a concise diagnostic before
   changing any session.
6. **US3-AS6**. **Given** two invocations race to start the same missing session, **When** both attempt startup,
   **Then** at most one session is created and the losing invocation reconnects or reports a
   recoverable conflict.

---

### User Story 4 - Start a default workspace (Priority: P1)

As a developer, I want `ws up SESSION_NAME` to create a useful workspace even when no setup file is
present, so that the tool has a predictable zero-configuration starting point.

**Why this priority**: A single-window workspace is the minimum viable workflow and provides a safe
fallback for every project.

**Independent Test**: Invoke `ws up SESSION_NAME` from a directory without a `.ws` file and verify
that a new tmux session has exactly one window rooted at the invocation directory and running the
user’s normal shell.

**Acceptance Scenarios**:

1. **US4-AS1**. **Given** no tmux session has the requested name and no local `.ws` file exists, **When** the user
   invokes `ws up SESSION_NAME`, **Then** `ws` creates one tmux session with one usable window in
   the current project directory.
2. **US4-AS2**. **Given** the default workspace was created, **When** the user enters the session, **Then** the
   window has a stable name and provides the user’s normal interactive shell.
3. **US4-AS3**. **Given** tmux is unavailable, **When** the user invokes `ws up SESSION_NAME`, **Then** `ws` reports
   that tmux could not be started or contacted and exits non-zero without accessing or exposing
   credentials.

---

### User Story 5 - Start a configured workspace (Priority: P1)

As a developer, I want a local `.ws` file to describe windows, panes, paths, commands, and environment
variables so that one command recreates my project’s development layout.

**Why this priority**: Configuration-driven setup is the core value of `ws`; it removes repeated
manual tmux layout work while keeping each project’s setup local.

**Independent Test**: Run `ws up SESSION_NAME` both outside tmux and from a disposable
`SESSION_ONE` client in temporary projects containing a valid `.ws` file. Verify the resulting tmux
session’s windows, pane hierarchy, working directories, commands, and environment values against the
file, and verify that an in-tmux invocation preserves `SESSION_ONE`.

**Acceptance Scenarios**:

1. **US5-AS1**. **Given** a valid `.ws` file, **When** no session with the requested name exists and the user
   invokes `ws up SESSION_NAME`, **Then** `ws` creates the session from the file in declaration order.
2. **US5-AS2**. **Given** a window declaration with a name, path, and command, **When** the workspace starts,
   **Then** the corresponding window has that name, starts in that path, and runs that command.
3. **US5-AS3**. **Given** a nested pane declaration, **When** the workspace starts, **Then** `ws` creates the
   requested split direction and recursively creates its child panes in the declared hierarchy.
4. **US5-AS4**. **Given** an environment declaration in a window or pane, **When** its command starts, **Then**
   the command receives that value, with a more specific pane value overriding an inherited window
   value.
5. **US5-AS5**. **Given** a valid configuration with multiple windows and panes, **When** workspace creation
   completes, **Then** the user is connected to the new session with the first declared window selected.
6. **US5-AS6**. **Given** the user is in `SESSION_ONE` and `OTHER_SESSION` does not exist, **When** the user invokes
   `ws up OTHER_SESSION`, **Then** `ws` creates `OTHER_SESSION` from the local `.ws` file and opens it
   in a separate terminal client/window without switching or modifying `SESSION_ONE`.
7. **US5-AS7**. **Given** a configured command contains shell syntax such as a home-directory shortcut, pipe, or
   redirect, **When** the command starts, **Then** it is interpreted by the documented user shell as
   a project configuration command, while generated session names and paths are never shell-expanded
   through string interpolation.

---

### User Story 6 - Diagnose invalid workspace setup (Priority: P2)

As a developer, I want invalid setup files to fail clearly before a session is created so that
configuration mistakes do not leave behind a partially built workspace.

**Why this priority**: Configuration errors are expected during authoring. Clear, non-destructive
failures protect existing sessions and shorten feedback cycles.

**Independent Test**: Supply malformed, incomplete, or semantically invalid `.ws` files and verify
that each failure identifies the file and location when available, exits non-zero, and leaves no
newly created session with the requested name.

**Acceptance Scenarios**:

1. **US6-AS1**. **Given** a `.ws` file contains invalid syntax or unsupported fields, **When** the user invokes
   `ws up SESSION_NAME`, **Then** `ws` reports the problem with a location or relevant field and does
   not create the session.
2. **US6-AS2**. **Given** a referenced working directory does not exist, **When** the user invokes
   `ws up SESSION_NAME`, **Then** `ws` reports which declaration failed and does not silently
   substitute another directory.
3. **US6-AS3**. **Given** a configured command cannot be started, **When** the user invokes `ws up SESSION_NAME`,
   **Then** `ws` reports the affected window or pane and returns non-zero without printing secret
   values.
4. **US6-AS4**. **Given** the file is valid but session creation fails partway through, **When** startup aborts,
   **Then** `ws` cleans up only the new session it created and leaves pre-existing sessions untouched.

---

### User Story 7 - Manage and change workspaces explicitly (Priority: P2)

As a developer, I want to detach from, change to, or take down a named tmux session through documented
commands so that workspace lifecycle operations do not require memorizing raw tmux commands.

**Why this priority**: The existing implementation demonstrates that lifecycle commands are part of
the expected user experience, while the new `up` flow provides the safer configuration behavior.

**Independent Test**: Create a disposable tmux session, invoke the lifecycle commands from inside and
outside tmux, and verify the target session state and confirmation behavior.

**Acceptance Scenarios**:

1. **US7-AS1**. **Given** the user is inside a tmux session, **When** the user invokes `ws exit`, **Then** `ws`
   detaches the client without killing the session and returns a documented status.
2. **US7-AS2**. **Given** a named session exists, **When** the user invokes `ws down SESSION_NAME`, **Then** `ws`
   requests confirmation in an interactive terminal, kills only that named session after approval,
   and reports success.
3. **US7-AS3**. **Given** `ws down SESSION_NAME` is run without a TTY, **When** `--yes` is not supplied, **Then**
   `ws` refuses the destructive action and explains how to provide explicit non-interactive consent.
4. **US7-AS4**. **Given** a named session exists and the user supplies `ws down SESSION_NAME --yes`, **When** the
   command runs, **Then** `ws` kills only that validated session without prompting and reports success.
5. **US7-AS5**. **Given** the requested session does not exist, **When** the user invokes `ws down SESSION_NAME`,
   **Then** `ws` reports that no such session exists and does not affect other sessions.
6. **US7-AS6**. **Given** no session name is supplied to `ws down` while the user is inside tmux, **When** the user
   invokes it interactively, **Then** `ws` identifies the current session and asks for explicit
   confirmation before killing it.
7. **US7-AS7**. **Given** the user is in `SESSION_ONE` and `OTHER_SESSION` exists, **When** the user invokes
   `ws change OTHER_SESSION`, **Then** `ws` switches the current tmux client to `OTHER_SESSION` and
   leaves `SESSION_ONE` running in the background.
8. **US7-AS8**. **Given** the user is in `SESSION_ONE` and `OTHER_SESSION` does not exist, **When** the user invokes
   `ws change OTHER_SESSION`, **Then** `ws` returns a non-zero recoverable error, keeps the current
   client in `SESSION_ONE`, and does not read or apply `.ws`.
9. **US7-AS9**. **Given** the user is not inside tmux, **When** the user invokes `ws change OTHER_SESSION`,
   **Then** `ws` reports that changing the current tmux client is unavailable and does not create or
   modify a session.

---

### User Story 8 - Use secure future interactive commands (Priority: P3)

As a developer, I want a consistent command namespace for utilities such as `ws clip git ssh`, so
that password-backed workflows can be added without placing secrets into every tmux pane.

**Why this priority**: The reviewed implementation shows the value of commands that copy selected Git
or Docker credentials, but the detailed command and credential mapping should be specified separately
from workspace creation.

**Independent Test**: Run help and an unsupported utility command with a controlled password-store
provider, verify that help never accesses the provider, and verify that a credential-dependent command
requests only the selected entry and never exports all entries to a session.

**Acceptance Scenarios**:

1. **US8-AS1**. **Given** the user invokes `ws clip NAMESPACE ITEM`, **When** that utility is implemented, **Then**
   it requests only the credential associated with that namespace and item and sends it to the
   selected clipboard provider without printing it.
2. **US8-AS2**. **Given** the user invokes an interactive utility that has not yet been specified, **When** `ws`
   parses it, **Then** `ws` reports that the command is unavailable or not yet supported and does not
   alter a tmux session.
3. **US8-AS3**. **Given** the local `pass` store is locked or unavailable, **When** a credential-dependent command
   runs, **Then** `ws` reports an actionable failure without showing the master passphrase, entry
   value, or provider internals containing secret material.
4. **US8-AS4**. **Given** a workspace command does not require credentials, **When** the user invokes
   `ws up SESSION_NAME`, **Then** `ws` does not access `pass`, set secret environment variables, or
   place credentials in tmux commands or shell history.

**Security design decision**: `ws` MUST use the user’s existing `pass` and GPG-agent flow on demand;
it MUST NOT receive, cache, or export the password-store master passphrase. A future command may
invoke `pass show` for one approved entry and pipe the result directly to its destination, such as a
clipboard provider. The detailed entry mapping, clipboard provider selection, clearing behavior, and
command-specific authorization remain separate specifications.

---

### User Story 9 - Configure the `ws clip` credential mapping (Priority: P3)

As a developer, I want to declare my Git and container-registry credentials once in a personal
config file and add project-specific entries only where a project needs them, so that
`ws clip git ssh` and similar commands work the same way the legacy tool's hard-coded mapping did,
without hard-coding my personal `pass` layout into the `ws` binary.

**Why this priority**: This closes the mapping gap User Story 8 deliberately deferred. It depends on
User Story 8's command namespace and security boundary, so it inherits that story's priority.

**Independent Test**: With a controlled `HOME`/`XDG_CONFIG_HOME`, a controlled project directory, a
fake `pass`, and a fake clipboard provider, populate a user-level file, a project-level file, and
run `ws clip NAMESPACE ITEM` for `pass`-backed entries, `literal` entries, project-overridden
entries, and unconfigured entries; verify the fake `pass` is invoked only for the requested `pass`
entry, the fake clipboard provider receives exactly the expected value, and no other value or file
content reaches stdout, stderr, or the clipboard.

**Acceptance Scenarios**:

1. **US9-AS1**. **Given** the user-level file defines `git ssh` as a `pass` entry, **When** the user invokes
   `ws clip git ssh`, **Then** `ws` requests that one `pass` entry and sends its value to the
   configured clipboard provider without printing it.
2. **US9-AS2**. **Given** the user-level file defines `git user` as a `literal` value, **When** the user invokes
   `ws clip git user`, **Then** `ws` sends that value to the clipboard provider without contacting
   `pass`.
3. **US9-AS3**. **Given** both the user-level and project-level files define an entry for the same
   `NAMESPACE ITEM`, **When** the user invokes `ws clip NAMESPACE ITEM` from that project, **Then**
   the project-level entry is used.
4. **US9-AS4**. **Given** neither file defines the requested `NAMESPACE ITEM`, **When** the user invokes
   `ws clip NAMESPACE ITEM`, **Then** `ws` reports that the entry is not configured, points at
   `ws help clip`, and does not contact `pass` or the clipboard provider.
5. **US9-AS5**. **Given** a mapping file is malformed, has an unsupported `version`, or an entry sets both
   or neither of `pass`/`literal`, **When** `ws clip` loads it, **Then** `ws` reports the problem
   with the file path and a location when available, and does not contact `pass` or the clipboard
   provider.
6. **US9-AS6**. **Given** neither the user-level nor the project-level file exists, **When** the user invokes
   any `ws clip NAMESPACE ITEM`, **Then** `ws` treats the mapping as empty and reports the entry as
   not configured, the same as US9-AS4, rather than failing on the missing files.
7. **US9-AS7**. **Given** `WS_CLIPBOARD_PROVIDER` is unset, **When** a `ws clip` entry resolves successfully,
   **Then** `ws` sends the value to `xclip -selection clipboard`.
8. **US9-AS8**. **Given** `WS_CLIPBOARD_PROVIDER` is set, **When** a `ws clip` entry resolves successfully,
   **Then** `ws` uses the configured program and arguments instead of the default, appending
   nothing beyond the resolved value on its standard input.
9. **US9-AS9**. **Given** the user wants to learn the mapping file format, **When** the user invokes
   `ws help clip`, **Then** `ws` prints the normative mapping schema, both file locations, the merge
   rule, and a complete example without contacting tmux, `pass`, or the clipboard provider.

---

### Edge Cases

- A help or version request is made while tmux is missing, `pass` is locked, or GPG is unavailable:
  help and version still succeed without touching those dependencies.
- A session with the requested name exists while the `.ws` file is missing, malformed, or changed:
  reconnect and ignore the file.
- Two invocations race to create the same session: one must win cleanly and the other must reconnect
  or report a recoverable conflict without producing a second session.
- The `.ws` file is empty: use the default single-window workspace or report a documented equivalent,
  but never create zero usable windows.
- The `.ws` file uses tabs, inconsistent indentation, duplicate window names, duplicate pane
  identifiers, unsupported pane positions, or an invalid environment assignment.
- The `.ws` file nests panes deeper than the documented maximum of 32 levels: the declaration that
  exceeds the limit is rejected with a validation error instead of risking unbounded recursion.
- A relative path, command, or environment value contains spaces, `=`, `:`, shell metacharacters, or
  a newline; values must not be silently truncated or reinterpreted.
- A configured path is outside the project directory, is inaccessible, or disappears between
  validation and pane creation.
- A configured command exits immediately, prompts for input, or is unavailable on the host: its
  pane remains (per FR-045), with its shell's own prompt back, showing the command's output rather
  than disappearing.
- The requested session name contains characters that tmux accepts but that could be unsafe when
  passed through a shell; generated arguments must be passed without shell interpolation.
- The invocation runs inside tmux, outside tmux, with stdin redirected, or without color/interactive
  terminal capabilities.
- `ws up OTHER_SESSION` is invoked from inside tmux when a separate terminal client/window launcher is
  unavailable; the command must fail recoverably without switching or modifying the current session.
- `ws change` is invoked outside tmux, targets a missing session, or targets the current session.
- `ws down` is requested without a session name inside or outside tmux, with and without a TTY.
- The password store is locked, missing, configured for a different user, or returns a failing status;
  diagnostics must not include secret material.
- A configured environment value contains a secret: it must not be printed by diagnostics or
  persisted outside the child process that needs it.
- The clipboard provider is unavailable, supports a different terminal environment, or exits before
  accepting the selected value; the secret must not fall back to stdout or an error message.
- Both the user-level and project-level clip mapping files are absent, empty, or one is present and
  the other absent; each case resolves without error, per US9-AS6.
- A mapping file exists but is unreadable (permissions) rather than missing: `ws clip` reports it
  the same way an unreadable `.ws` file would, without exposing filesystem detail beyond the path.
- `HOME` is unset and `XDG_CONFIG_HOME` is unset: the user-level file is treated as absent rather
  than causing a panic or an unrelated path.
- A project directory contains `.ws-clip` but no `.ws`: the project-level clip mapping still loads
  independently of workspace-layout configuration.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST expose these top-level command modes: help/version discovery, `up`,
  `change`, `down`, `exit`, and a reserved `clip` namespace.
- **FR-002**: The workspace-start command MUST be `ws up SESSION_NAME`, with exactly one required
  positional session name; it MUST NOT prompt for the name.
- **FR-003**: The system MUST provide `ws`, `ws help`, and `ws --help` as side-effect-free help paths,
  `ws help config` as the side-effect-free `.ws` language reference, and `ws --version` as a
  side-effect-free version path.
- **FR-004**: Help, configuration-reference, and version paths MUST NOT read project `.ws` files,
  contact tmux, invoke `pass`, access GPG, prompt for input, or write credentials anywhere.
- **FR-005**: The system MUST reject missing, empty, whitespace-only, or invalid session names with a
  non-zero exit status and actionable help.
- **FR-006**: `ws up` MUST check for an existing tmux session by the requested name before reading or
  parsing the local `.ws` file.
- **FR-007**: If the requested session exists, `ws up` MUST connect to that session and MUST ignore
  the `.ws` file, including its syntax and filesystem references. Outside tmux it attaches directly;
  inside tmux it uses a separate terminal client/window without switching the current client.
- **FR-008**: If the requested session does not exist and no `.ws` file is present in the current
  project directory, the system MUST create exactly one usable tmux window in that directory using
  the user’s normal interactive shell.
- **FR-009**: The system MUST locate the default setup file as `.ws` in the current project directory,
  and all relative paths in that file MUST resolve relative to the setup file’s directory.
- **FR-010**: The system MUST parse a YAML `.ws` document conforming to the versioned schema,
  including window declarations, window names, paths, commands, nested pane declarations, pane
  positions, pane identifiers, and environment mappings.
- **FR-011**: The system MUST preserve declaration order for windows and nested panes, and MUST select
  the first declared window when the new session is opened.
- **FR-012**: The system MUST support pane positions `left`, `right`, `top`, and `bottom`, applying
  each position to the split containing that pane.
- **FR-013**: Configured command text MUST use documented user-shell semantics so commands such as
  `~/.bin/service_observe` or `bash ./scripts/setup.sh` work, while session names, paths, and
  generated tmux arguments MUST never be constructed through unsafe shell interpolation.
- **FR-014**: The system MUST validate the complete setup before creating a new session, and invalid
  configuration MUST NOT leave a newly created session behind.
- **FR-015**: The system MUST report configuration errors with the setup file path and a line or field
  location whenever available, and MUST identify the affected window or pane for runtime failures.
- **FR-016**: The system MUST make each declared path the working directory for its window or pane
  command and MUST fail clearly when the path cannot be used.
- **FR-017**: The system MUST apply environment values to the declared command scope, with pane values
  overriding inherited window values, and MUST split assignments only at the first `=` character.
- **FR-018**: The system MUST create a nested pane hierarchy without requiring users to encode tmux
  implementation details in the setup file.
- **FR-019**: When invoked outside tmux, `ws up` MUST attach to the newly created or existing workspace.
  When invoked inside tmux, `ws up` MUST preserve the current client and open the newly created or
  existing target session in a separate terminal client/window; it MUST NOT silently nest a tmux
  client. When the target is missing, `ws up` MUST apply the local `.ws` definition before opening
  the target, creating a fresh workspace rather than cloning live panes or processes.
- **FR-020**: `ws change SESSION_NAME` MUST require an existing target session, switch the current
  tmux client to it, and leave the previously selected session running in the background. It MUST
  never read or apply `.ws` while changing sessions.
- **FR-021**: `ws change SESSION_NAME` MUST return a recoverable non-zero error and leave the current
  client unchanged when the target does not exist or when invoked outside tmux.
- **FR-022**: `ws exit` MUST detach the current client without killing the session and MUST fail clearly
  when no tmux client is attached.
- **FR-023**: `ws down SESSION_NAME` MUST require explicit confirmation before killing a session when
  interactive, and MUST require the `--yes` flag as explicit non-interactive consent when no TTY is
  available. `--yes` MUST be scoped to `down` and MUST NOT bypass session-name validation.
- **FR-024**: `ws down` without a name MAY target the current tmux session only when invoked inside
  tmux and after interactive confirmation; `--yes` MUST NOT enable nameless destructive targeting,
  and the command MUST not guess a target outside tmux.
- **FR-025**: The system MUST establish password-store access only for a command that needs it and
  MUST use the existing `pass`/GPG-agent flow rather than receiving or caching the master passphrase.
- **FR-026**: The system MUST NOT eagerly retrieve or export all password-store entries into the tmux
  server, pane environments, shell startup files, command arguments, or shell history.
- **FR-027**: A credential-dependent utility MUST request only the selected entry and MUST stream the
  selected value directly to its approved destination without emitting it on stdout or stderr.
- **FR-028**: The system MUST keep master passwords and password-store entries out of stdout, stderr,
  logs, shell commands, tmux command strings, persisted configuration, and unrelated environments.
- **FR-029**: The system MUST provide discoverable help for `up`, `change`, `down`, `exit`, and
  `clip`, including `ws help clip` as the side-effect-free clip credential-mapping reference.
- **FR-030**: The system MUST keep normal results on stdout and diagnostics, progress, and logging on
  stderr; secret values MUST be emitted to neither stream.
- **FR-031**: The system MUST return stable, documented non-zero statuses for invalid CLI input,
  invalid setup, unavailable tmux, failed command setup, destructive-action refusal, and credential
  failures.
- **FR-032**: The system MUST support non-TTY invocation without hanging on an unexpected prompt;
  interactive behavior MUST be capability-aware and have a documented non-interactive path.
- **FR-033**: The system MUST expose the normative `.ws` YAML schema and semantic reference through
  `ws help config`, including every key, type, required-key rule, nesting rule, position meaning,
  path-resolution rule, command behavior, and a complete example.
- **FR-034**: The implementation’s parser, examples, help reference, and acceptance tests MUST use
  the same `.ws` language contract; changes to syntax MUST be documented as a language-versioned
  compatibility change.
- **FR-035**: The system MUST allow a `command:` field on every window and pane to execute an arbitrary
  project-provided command, including a custom Bash script, in the declaration’s resolved working
  directory and environment.
- **FR-036**: The workspace-layout MVP MUST NOT provide built-in Kubernetes, OpenShift, or NATS
  startup options. Project-specific integrations MUST be expressed through `.ws` windows, panes,
  environment values, and custom commands or scripts.
- **FR-037**: Any `workspace-name` terminology retained for compatibility or documentation MUST be
  defined as the tmux session name, and all new help and requirements MUST use `SESSION_NAME`.
- **FR-038**: The help output MUST explain that a terminal window/client is separate from a tmux
  window, and MUST document the behavior and recoverable failure mode when `ws up` cannot launch a
  separate terminal client from inside tmux.
- **FR-039**: `ws clip NAMESPACE ITEM` MUST resolve its credential source from a merged mapping built
  from an optional user-level file (`$XDG_CONFIG_HOME/ws/clip.yaml`, falling back to
  `$HOME/.config/ws/clip.yaml`) and an optional project-level file (`.ws-clip` in the project
  directory), with project-level entries overriding user-level entries for the same pair. A missing
  file on either side MUST be treated as an empty mapping, not an error.
- **FR-040**: Each mapping entry MUST resolve to exactly one of a `pass` store path or a fixed
  non-secret `literal` value; the mapping file MUST NOT itself contain a live credential value.
- **FR-041**: A `NAMESPACE ITEM` pair absent from the merged mapping MUST be rejected before any
  `pass` or clipboard-provider access, with an actionable error directing the user to
  `ws help clip`.
- **FR-042**: The system MUST expose the normative clip credential-mapping YAML schema, file
  locations, merge rule, and a complete example through `ws help clip`, mirroring `ws help config`
  for `.ws`.
- **FR-043**: The clipboard destination MUST be configurable through the `WS_CLIPBOARD_PROVIDER`
  environment variable, using the same program-plus-arguments convention as
  `WS_TERMINAL_LAUNCHER`, and MUST default to `xclip -selection clipboard` when unset. `ws` MUST
  write the resolved value only to the provider's standard input and MUST NOT read or forward the
  provider's own stdout or stderr.
- **FR-044**: A `pass`-backed entry MUST be resolved through the existing `PassProvider` boundary
  (FR-025 through FR-028) at the moment `ws clip` runs; it MUST NOT be resolved, cached, or
  exported during `ws up`, `ws change`, `ws down`, or `ws exit`.
- **FR-045**: A window or pane created with a configured `command` MUST start the user's normal
  interactive shell and type `command` into it (followed by Enter), rather than running `command`
  as the pane's own process, so the pane's aliases, functions, and other shell startup-file state
  are available to `command`; remains after that command exits, including a command that exits
  before the workspace finishes materializing, with a usable shell prompt and its final output both
  visible; and never has its early exit destroy panes chained off it, because the pane's shell —
  not `command` — is what tmux is watching. A pane running the user's normal shell (no configured
  `command`) is unaffected and closes as tmux normally would.

### Key Entities *(include if data involved)*

- **Workspace Session**: A named tmux session that can be newly created from a definition or resumed
  without reapplying configuration.
- **Workspace Definition**: The project-local `.ws` document containing ordered window and pane
  declarations.
- **Window Declaration**: A named working context with a path, optional command, inherited environment
  values, and zero or more nested pane declarations.
- **Pane Declaration**: A recursively nestable terminal context with a split position, optional
  identifier, path/command/environment details, and child panes.
- **Credential Context**: A bounded capability represented by the existing password-store provider,
  allowing an approved utility to request one named value without exposing the master passphrase.
- **Interactive Command**: A `ws` utility command that performs a narrowly defined operation, such
  as copying a selected password-store entry.
- **Clip Mapping**: The merged, validated result of the optional user-level and project-level clip
  configuration files, keyed by `(NAMESPACE, ITEM)`.
- **Clip Entry**: One mapping value: either a `pass` store path resolved on demand, or a fixed
  non-secret `literal` value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With tmux and the password store unavailable, all five discovery/reference
  invocations—`ws`, `ws help`, `ws --help`, `ws help config`, and `ws --version`—complete successfully
  without prompting or contacting either dependency.
- **SC-002**: In an environment with tmux available, a valid setup creates a session whose windows,
  panes, names, working directories, commands, and environment values match the setup file in 100%
  of automated acceptance runs.
- **SC-003**: When a named session already exists, 100% of automated `up` tests connect to that
  session without parsing the `.ws` file or creating an additional session; in-tmux tests preserve
  the current client and use a separate terminal context.
- **SC-004**: A no-configuration invocation creates exactly one usable window in 100% of automated
  default-mode tests.
- **SC-005**: 100% of malformed setup tests fail before leaving a newly created session, and each
  diagnostic identifies the invalid declaration or its nearest available location.
- **SC-006**: In credential-handling tests, zero password or password-store entry values appear in
  captured stdout, stderr, logs, command arguments, tmux command strings, or persisted workspace
  files, and startup of a non-credential command makes zero password-store requests.
- **SC-007**: In non-TTY tests, the CLI never waits indefinitely for interactive input and exits with a
  documented result within 5 seconds after the external dependency returns.
- **SC-008**: Destructive lifecycle tests cannot kill a session without interactive confirmation or
  the explicitly scoped `ws down SESSION_NAME --yes` non-interactive consent flag.
- **SC-009**: In 100% of tests starting from `SESSION_ONE`, `ws up OTHER_SESSION` leaves
  `SESSION_ONE` attached and unchanged; a missing target applies `.ws` and opens the reinitialized
  workspace in a separate terminal client/window, while an existing target is opened without applying
  `.ws`.
- **SC-010**: In 100% of tests starting from `SESSION_ONE`, `ws change OTHER_SESSION` switches to an
  existing `OTHER_SESSION`, while a missing target leaves `SESSION_ONE` active and returns a
  recoverable error.
- **SC-011**: A new developer can create a two-window, nested-pane workspace from the example language
  using one local `.ws` file and one `ws up SESSION_NAME` invocation, without manually issuing tmux
  layout commands.
- **SC-012**: The help output describes every supported MVP command, its required positional arguments,
  its side effects, the distinction between `up` and `change`, and at least one configured-workspace
  example without relying on local credentials.
- **SC-013**: In 100% of automated `ws clip` tests, the fake `pass` provider is invoked only for the
  exact entry the resolved `NAMESPACE ITEM` names (never for any other configured entry), and the
  captured clipboard-provider input matches the expected value exactly, whether sourced from
  `pass` or a `literal`.
- **SC-014**: In 100% of automated `ws clip` tests targeting an unconfigured `NAMESPACE ITEM`, a
  malformed mapping file, or a missing mapping file, zero invocations reach the fake `pass`
  provider or the fake clipboard provider.

## Traceability

Each acceptance scenario carries a stable `US<story>-AS<n>` id per the constitution's testing
principle. "Owning test" names the automated test whose `// Scenario: US<story>-AS<n>` marker
verifies that scenario's observable behavior; "Pending" marks a scenario whose implementation and
test do not exist yet, naming the `tasks.md` phase and task ids expected to close it.

| Scenario | Owning test / status |
|----------|----------------------|
| US1-AS1 | `tests/cli_help.rs::given_missing_dependencies_when_running_global_help_then_uses_stdout` |
| US1-AS2 | `tests/cli_help.rs::given_missing_dependencies_when_running_without_arguments_then_prints_help` |
| US1-AS3 | `tests/cli_help.rs::given_missing_dependencies_when_requesting_version_then_prints_only_version` |
| US1-AS4 | `tests/cli_help.rs::given_an_unknown_command_when_invoked_then_returns_helpful_stderr_error` |
| US1-AS5 | `tests/cli_help.rs::given_missing_dependencies_when_requesting_config_help_then_prints_normative_reference` |
| US2-AS1 | `tests/cli_help.rs::given_missing_dependencies_when_requesting_config_help_then_prints_normative_reference` |
| US2-AS2 | `tests/tmux_integration.rs::given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order` |
| US2-AS3 | `tests/lifecycle.rs::given_an_invalid_ws_file_when_up_is_invoked_then_no_session_is_created` |
| US2-AS4 | `tests/tmux_integration.rs::given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order` |
| US3-AS1 | `tests/lifecycle.rs::given_an_existing_session_when_up_is_invoked_outside_tmux_then_it_attaches_without_reading_the_config` |
| US3-AS2 | `tests/lifecycle.rs::given_a_missing_session_when_up_is_invoked_inside_tmux_then_it_applies_the_config_and_opens_a_separate_terminal` |
| US3-AS3 | `tests/lifecycle.rs::given_an_existing_session_when_up_is_invoked_inside_tmux_then_it_opens_a_separate_terminal_and_preserves_the_current_client` |
| US3-AS4 | `tests/lifecycle.rs::given_an_existing_session_when_up_is_invoked_inside_tmux_then_it_opens_a_separate_terminal_and_preserves_the_current_client` |
| US3-AS5 | `tests/lifecycle.rs::given_an_invalid_session_name_when_up_is_invoked_then_it_is_rejected_before_any_dependency_access` |
| US3-AS6 | `tests/lifecycle.rs::given_the_target_session_race_is_lost_when_up_is_invoked_then_it_reconnects_instead_of_failing` |
| US4-AS1 | `tests/lifecycle.rs::given_no_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window` |
| US4-AS2 | `tests/lifecycle.rs::given_no_ws_file_when_up_is_invoked_then_it_creates_one_usable_default_window` |
| US4-AS3 | `tests/lifecycle.rs::given_tmux_is_unavailable_when_up_is_invoked_then_it_reports_a_recoverable_dependency_error` |
| US5-AS1 | `tests/tmux_integration.rs::given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order` |
| US5-AS2 | `tests/tmux_integration.rs::given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order` |
| US5-AS3 | `tests/tmux_integration.rs::given_the_canonical_document_when_up_is_invoked_then_every_window_and_pane_is_materialized_in_order` |
| US5-AS4 | `tests/tmux_integration.rs::given_a_window_environment_when_up_is_invoked_then_it_is_inherited_by_its_pane` |
| US5-AS5 | `tests/lifecycle.rs::given_a_missing_session_when_up_is_invoked_outside_tmux_then_it_creates_and_attaches_to_a_fresh_session` |
| US5-AS6 | `tests/lifecycle.rs::given_a_missing_session_when_up_is_invoked_inside_tmux_then_it_applies_the_config_and_opens_a_separate_terminal` |
| US5-AS7 | `tests/tmux_integration.rs::given_a_command_with_shell_syntax_when_up_is_invoked_then_it_is_passed_through_as_one_argument` |
| US6-AS1 | `tests/lifecycle.rs::given_an_invalid_ws_file_when_up_is_invoked_then_no_session_is_created` |
| US6-AS2 | `tests/lifecycle.rs::given_a_referenced_directory_does_not_exist_when_up_is_invoked_then_it_reports_the_declaration_and_creates_no_session` |
| US6-AS3 | `tests/lifecycle.rs::given_a_secret_like_environment_value_when_a_launch_step_fails_then_it_never_reaches_ws_own_output` |
| US6-AS4 | `tests/tmux_integration.rs::given_a_pane_split_fails_partway_through_when_up_is_invoked_then_only_the_new_session_is_rolled_back` |
| US7-AS1 | `tests/lifecycle.rs::given_inside_tmux_when_exit_is_invoked_then_it_detaches_without_killing_the_session` |
| US7-AS2 | `src/lifecycle/down.rs::tests::given_interactive_confirmation_accepted_when_down_runs_then_it_kills` |
| US7-AS3 | `tests/lifecycle.rs::given_no_yes_and_no_tty_when_down_is_invoked_then_it_refuses` |
| US7-AS4 | `tests/lifecycle.rs::given_yes_when_down_is_invoked_then_it_kills_the_named_session_without_prompting` |
| US7-AS5 | `tests/lifecycle.rs::given_a_missing_target_when_down_is_invoked_then_it_reports_no_such_session` |
| US7-AS6 | `src/lifecycle/down.rs::tests::given_no_name_inside_tmux_when_down_runs_interactively_then_it_targets_the_current_session` |
| US7-AS7 | `tests/lifecycle.rs::given_an_existing_target_when_change_is_invoked_inside_tmux_then_it_switches_the_client` |
| US7-AS8 | `tests/lifecycle.rs::given_a_missing_target_when_change_is_invoked_then_it_returns_a_recoverable_error` |
| US7-AS9 | `tests/lifecycle.rs::given_outside_tmux_when_change_is_invoked_then_it_fails_without_contacting_tmux` |
| US8-AS1 | `tests/clip.rs::given_a_pass_backed_entry_when_clip_is_invoked_then_only_that_entry_is_requested_and_copied` |
| US8-AS2 | `tests/clip.rs::given_an_unconfigured_entry_when_clip_is_invoked_then_it_reports_not_configured_without_contacting_dependencies` |
| US8-AS3 | `src/credentials/pass_provider.rs::tests::given_pass_is_locked_when_revealed_then_no_detail_is_included_in_the_error` (also exercised end-to-end by `src/lifecycle/clip.rs::tests::given_pass_is_locked_when_clip_runs_then_it_reports_a_credential_failure_without_secret_detail`). |
| US8-AS4 | `tests/lifecycle.rs::given_a_non_credential_command_when_up_is_invoked_then_pass_is_never_contacted` |
| US9-AS1 | `tests/clip.rs::given_a_pass_backed_entry_when_clip_is_invoked_then_only_that_entry_is_requested_and_copied` |
| US9-AS2 | `tests/clip.rs::given_a_literal_entry_when_clip_is_invoked_then_pass_is_never_contacted` |
| US9-AS3 | `tests/clip.rs::given_project_and_user_entries_collide_when_clip_is_invoked_then_the_project_entry_wins` |
| US9-AS4 | `tests/clip.rs::given_an_unconfigured_entry_when_clip_is_invoked_then_it_reports_not_configured_without_contacting_dependencies` |
| US9-AS5 | `tests/clip.rs::given_a_malformed_mapping_file_when_clip_is_invoked_then_it_reports_the_problem_without_contacting_dependencies` |
| US9-AS6 | `tests/clip.rs::given_no_mapping_files_when_clip_is_invoked_then_it_reports_not_configured` |
| US9-AS7 | `tests/clip.rs::given_no_clipboard_provider_configured_when_clip_is_invoked_then_it_uses_xclip_by_default` |
| US9-AS8 | `tests/clip.rs::given_a_configured_clipboard_provider_when_clip_is_invoked_then_it_is_used_instead_of_the_default` |
| US9-AS9 | `tests/cli_help.rs::given_missing_dependencies_when_requesting_clip_help_then_prints_the_mapping_reference` |

## Assumptions

- The user has tmux installed and has permission to connect to its server; supported tmux versions
  will be documented during planning.
- The current working directory is the project root for the default `.ws` lookup, and each project
  owns its own setup file.
- A missing `.ws` is a supported default case, not an error; an empty `.ws` follows the same default
  behavior unless planning establishes a stricter rule.
- The user’s normal shell is available through the host environment and is used only when a window
  or pane has no explicit command.
- The setup language is intentionally small and uses standard YAML 1.2. The JSON Schema and semantic
  rules in this specification are the normative MVP contract and are also published by `ws help
  config`; future syntax or semantic changes require a language-versioned compatibility decision.
- `pass` and its GPG-agent integration are already installed and configured for the user. `ws` uses
  that existing unlock flow and does not implement a password-store format or master-passphrase cache.
- `ws clip` maps friendly names such as `git ssh` and `docker token` to `pass` entries through the
  user- and project-level mapping files defined in "Clip Credential Mapping Language (Normative
  Reference)" and User Story 9; the specific entries a given user or project configures remain
  outside this specification, which defines only the mapping format, locations, and merge rule.
- Clipboard integration is now specified: `ws clip` defaults to `xclip -selection clipboard` and
  is overridable through `WS_CLIPBOARD_PROVIDER`. Support for clipboard providers beyond that
  program-plus-arguments convention (for example a provider requiring interactive authorization)
  remains a separate specification.
- The first implementation targets local Unix-like development environments; cross-platform terminal
  behavior, clipboard providers, separate-terminal launchers, and supported tmux versions must be
  confirmed during planning.
- A terminal window/client means a separately attachable terminal context, not an additional window in
  the current tmux session. `ws up` must use a supported launcher or return a recoverable error rather
  than silently nesting tmux.
- `ws down` is included because it is part of the established command model, but safe confirmation
  behavior is more important than preserving the old implementation’s implicit current-session target.
