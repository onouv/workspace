# Feature Specification: Default Session Name

**Feature Branch**: `003-default-session-name`

**Created**: 2026-10-23

**Status**: Draft

**Input**: User description: When the user starts `ws up` without a workspace name, the system
shall pick the current folder name (not the full path) as the workspace name. If a workspace with
this name is already running in the background, the system shall connect to that tmux session.
Otherwise the system shall create a new workspace under that name.

## Terms

This specification uses each term below with one meaning only.

| Term | Meaning |
|------|---------|
| project folder | The current working directory of the `ws up` command (001 FR-009). |
| folder name | The last component of the project folder path. It is not the full path. |
| explicit name | A `SESSION_NAME` that the user types after `ws up`. |
| derived name | The session name that `ws` makes from the folder name. |
| message | One line on stderr that tells the user which derived name `ws` uses. |
| session folder | For a session that `ws` made: the project folder that `ws` recorded when it made the session. For any other session: the start directory that tmux records for it. Every tmux session has a start directory, also a session that plain `tmux` made. |
| folder check | The comparison of the session folder with the project folder (FR-011). |

## Relationship to 001-tmux-workspace

This feature replaces the items below in `specs/001-tmux-workspace`. All other items in 001 stay
in force.

| 001 item | Old behavior | New behavior |
|----------|--------------|--------------|
| FR-002 | `ws up` has exactly one required positional session name. | The session name is optional. FR-001 of this specification applies when it is not given. |
| FR-005 (missing name only) | `ws up` without a name fails. | `ws up` without a name uses the derived name. `ws` still rejects an empty, whitespace-only, or invalid explicit name. |
| FR-007 (derived name only) | An existing session gets a connect. | An existing session gets a connect only after the folder check passes (FR-011). |
| US3-AS5 (missing name only) | A missing name is rejected. | A missing name gives the derived name. An empty, whitespace-only, or invalid explicit name is still rejected. |

These 001 items stay in force without change, and apply to the derived name as to an explicit
name:

- FR-006: `ws up` checks for an existing session before it reads `.ws`.
- FR-007: An existing session ignores `.ws`. For the derived name, the folder check comes first
  (FR-011).
- FR-019: Inside tmux, `ws up` opens a separate terminal.
- US3-AS4: `ws up` for the session of the current client does not switch the client.
- US3-AS6: Two commands that start the same missing session make at most one session.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a workspace named after the folder (Priority: P1)

The developer is in a project folder. The developer runs `ws up` without a name. The developer gets
a workspace with the name of the folder.

**Why this priority**: This is the main function of the feature. Most developers name a workspace
after the project folder.

**Independent Test**:

1. Make a temporary project folder with the name `alpha`.
2. Run `ws up` in that folder against a fake tmux or a disposable tmux server.
3. Make sure that `ws` created a session with the name `alpha`.
4. Make sure that the session uses the `.ws` rules of 001 and 002.

**Acceptance Scenarios**:

1. **US1-AS1**. **Given** the project folder is `/tmp/work/alpha` and no session `alpha` exists,
   **When** the user runs `ws up`, **Then** `ws` creates the session `alpha`. The result is the same
   as for `ws up alpha`.
2. **US1-AS2**. **Given** no session with the derived name exists, **When** the user runs `ws up`,
   **Then** `ws` shows a message with the derived name. `ws` writes nothing about the name to
   stdout.
3. **US1-AS3**. **Given** the user is in `SESSION_ONE` inside tmux and no session with the derived
   name exists, **When** the user runs `ws up`, **Then** `ws` creates the session in a separate
   terminal. `SESSION_ONE` does not change (001 FR-019).

---

### User Story 2 - Connect to the running workspace of the folder (Priority: P1)

The developer started a workspace for the folder before. The workspace runs in the background. The
developer runs `ws up` without a name in the same folder. The developer connects to the running
workspace. If the running workspace belongs to a different folder with the same name, `ws`
refuses to connect.

**Why this priority**: The user asked for this. A second `ws up` must not make a second workspace.
The developer must not get the workspace of a different project by accident.

**Independent Test**:

1. Start a session with the name `alpha` in the folder `/tmp/work/alpha`.
2. Run `ws up` in `/tmp/work/alpha`.
3. Make sure that `ws` connected to the session `alpha` and did not read `.ws`.
4. Run `ws up` in `/tmp/other/alpha`.
5. Make sure that `ws` exits non-zero and does not connect.

**Acceptance Scenarios**:

1. **US2-AS1**. **Given** the session `alpha` exists, its session folder is `/tmp/work/alpha`, and
   the project folder is `/tmp/work/alpha`, **When** the user runs `ws up` outside tmux, **Then** `ws` attaches to `alpha`. `ws` does not read
   `.ws` (001 FR-006, FR-007).
2. **US2-AS2**. **Given** the user is in `SESSION_ONE` inside tmux and the session `alpha` exists
   with the session folder `/tmp/work/alpha`, **When** the user runs `ws up` in `/tmp/work/alpha`, **Then** `ws` opens a separate terminal that
   is connected to `alpha`. `SESSION_ONE` does not change.
3. **US2-AS3**. **Given** the session `alpha` exists with the session folder `/tmp/work/alpha`,
   **When** the user runs `ws up` in `/tmp/other/alpha`, **Then** `ws` fails with a non-zero exit
   status. `ws` does not connect, does not create a session, and does not read `.ws`. The diagnostic
   gives the session folder and tells the user to run `ws up SESSION_NAME` with a different name.
4. **US2-AS4**. **Given** the user made the session `alpha` with plain `tmux new -s alpha` in
   `/tmp/work/alpha`, **When** the user runs `ws up` in `/tmp/work/alpha`, **Then** `ws` connects
   to `alpha`. The folder check uses the session folder that tmux records.

---

### User Story 3 - Keep the explicit name (Priority: P1)

The developer types a name after `ws up`. The developer gets the workspace with that name, as in
001.

**Why this priority**: Existing scripts and habits use `ws up SESSION_NAME`. They must not change.

**Independent Test**:

1. Run `ws up beta` in a folder with the name `alpha`.
2. Make sure that `ws` used the name `beta`.
3. Run `ws up ""` and `ws up "   "`.
4. Make sure that `ws` rejects both and does not use the derived name.

**Acceptance Scenarios**:

1. **US3-AS1**. **Given** the project folder is `/tmp/work/alpha`, **When** the user runs
   `ws up beta`, **Then** `ws` uses the name `beta`. `ws` does not show a message.
2. **US3-AS2**. **Given** the user types an empty, whitespace-only, or invalid explicit name,
   **When** the user runs `ws up` with that name, **Then** `ws` rejects the name as 001 FR-005
   specifies. `ws` does not use the derived name.

---

### User Story 4 - Fail clearly when the folder name is not usable (Priority: P2)

The developer is in a folder whose name cannot be a session name. The developer gets a clear
diagnostic and knows how to continue.

**Why this priority**: Some folder names are not valid tmux session names. The command must not
make a session with an unexpected name or fail without a reason (constitution III).

**Independent Test**:

1. Run `ws up` in the filesystem root.
2. Run `ws up` in a folder whose name has a character that a session name must not have.
3. Make sure that `ws` fails for the root (FR-004) and replaces the character with `_` (FR-005).

**Acceptance Scenarios**:

1. **US4-AS1**. **Given** the project folder has no folder name (for example, the filesystem root),
   **When** the user runs `ws up`, **Then** `ws` fails with a non-zero exit status before it
   contacts tmux. The diagnostic tells the user to give an explicit name.
2. **US4-AS2**. **Given** the folder name contains `.` or `:` (for example, `my.project`), **When**
   the user runs `ws up`, **Then** `ws` uses `my_project` as the session name (FR-005). The message
   shows `my_project`.
3. **US4-AS3**. **Given** the folder name is not valid text (not UTF-8), whitespace-only, or contains
   a control character, **When** the user runs `ws up`, **Then** `ws` fails with a non-zero exit
   status before it contacts tmux. The diagnostic tells the user to give an explicit name.

---

### Edge Cases

- The folder name contains spaces or shell metacharacters (for example, `my project` or `a$b`).
  `ws` uses the name without change. No shell interprets the name (001 FR-013).
- The folder name has leading or trailing spaces. `ws` uses the name without change. `ws` does not
  trim it.
- The project folder is a symlink. `ws` uses the folder name that the user sees in the shell (the
  logical path). The folder check resolves symlinks, so the symlink and its target give the same
  result (FR-011).
- Two project folders with the same name in different places. The first `ws up` makes the session.
  A `ws up` without a name in the second folder fails (FR-011). The user gives an explicit name for
  the second workspace.
- `my.project` and `my_project` give the same derived name. The folder check applies (FR-011).
- The user started the session with an explicit name in a different folder. A later `ws up`
  without a name whose derived name is equal fails the folder check.
- The current session of the client has the derived name. 001 US3-AS4 applies.
- Two `ws up` commands without a name start at the same time in the same folder. 001 US3-AS6
  applies. At most one session is created.
- The folder name is the same as the name of a window in `.ws`. `ws` creates the session correctly
  (see the hotfix "works correctly when session name equals current folder name").
- Help and version paths do not read the project folder or derive a name (001 FR-004).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `ws up` MUST accept zero or one positional session name. When the user gives no
  name, the system MUST use the derived name as the session name.
- **FR-002**: The derived name MUST be the folder name of the project folder. The system MUST NOT
  use the full path or a parent folder name.
- **FR-003**: After the system has the derived name, `ws up` MUST act exactly as `ws up` with that
  name as the explicit name. FR-011 is the only exception. This includes the existing-session check (001 FR-006), the reconnect
  (001 FR-007), the separate terminal inside tmux (001 FR-019), and the `.ws` rules of 001 and 002.
- **FR-004**: The system MUST fail with a non-zero exit status before it contacts tmux when the
  project folder has no folder name, or the folder name is not valid text, is whitespace-only, or
  contains a control character. The diagnostic MUST tell the user to run `ws up SESSION_NAME`. The
  exit status MUST be the 001 status for invalid CLI input (001 FR-031).
- **FR-005**: When the folder name contains `.` or `:`, the system MUST replace each `.` and each
  `:` with `_` to make the derived name. Example: `my.project` gives `my_project`. The system MUST
  NOT change other characters. This rule applies only to the derived name. The system still
  rejects an explicit name with `.` or `:` (001 FR-005).
- **FR-006**: When the system uses a derived name, it MUST show exactly one message with the
  derived name on stderr. The system MUST NOT write about the name to stdout. When the user gives
  an explicit name, the system MUST NOT show this message.
- **FR-007**: The system MUST validate an explicit name as 001 FR-005 specifies. An empty or
  whitespace-only explicit name MUST NOT fall back to the derived name.
- **FR-008**: The system MUST NOT pass the derived name through a shell (001 FR-013).
- **FR-009**: `ws help up` and `ws --help` MUST show that `SESSION_NAME` is optional and that the
  default is the folder name. The README MUST agree with this text.
- **FR-010**: `ws change` and `ws down` MUST NOT change. Only `ws up` uses the derived name.
- **FR-011**: When a session with the derived name exists, the system MUST do the folder check
  before it connects. The system MUST compare the two folders after it resolves symlinks in both.
  If the folders are the same, the system MUST connect (001 FR-007). If the folders are different,
  or the system cannot read the session folder, the system MUST fail with a non-zero exit status.
  The system MUST NOT connect, create a session, or read `.ws` in this case. The diagnostic MUST
  give the session name and the session folder. The diagnostic MUST tell the user to run
  `ws up SESSION_NAME` with a different name. The exit status MUST be a stable, documented status
  (001 FR-031). The system MUST NOT do the folder check for an explicit name.

### Key Entities

- **Derived Name**: The session name that the system makes from the folder name. It has the same
  validity rules as an explicit name.
- **Session Folder**: For a session that `ws` made, the project folder that `ws` recorded. For any
  other session, the start directory that tmux records. The folder check uses it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the automated tests that run `ws up` without a name in a folder with a
  usable name, the session name is equal to the folder name (after FR-005, if it applies).
- **SC-002**: In 100% of the automated tests where a session with the derived name exists and its
  session folder is the project folder, `ws up` without a name connects to it, creates no session,
  and does not read `.ws`.
- **SC-003**: In 100% of the automated tests with an explicit name, the result is the same as
  before this feature.
- **SC-004**: In 100% of the automated tests with an unusable folder name, `ws` exits non-zero
  before it contacts tmux and the diagnostic tells the user to give an explicit name.
- **SC-005**: A developer in a project folder starts or reconnects to the workspace of that folder
  with one command that has no arguments.
- **SC-006**: In 100% of the automated tests where a session with the derived name exists and its
  session folder is a different folder, `ws up` without a name exits non-zero and does not connect.

## Traceability

Each acceptance scenario has a stable `US<story>-AS<n>` id (constitution IV). All scenarios are
**Pending** until `/speckit.tasks` gives each scenario a test. This feature changes the
missing-name part of 001 US3-AS5. When this feature is implemented, update the 001 test for that
scenario so that it does not test a missing name.

| Scenario | Owning test / status |
|----------|----------------------|
| US1-AS1 | Pending |
| US1-AS2 | Pending |
| US1-AS3 | Pending |
| US2-AS1 | Pending |
| US2-AS2 | Pending |
| US2-AS3 | Pending |
| US2-AS4 | Pending |
| US3-AS1 | Pending |
| US3-AS2 | Pending |
| US4-AS1 | Pending |
| US4-AS2 | Pending |
| US4-AS3 | Pending |

## Clarifications

### Session 2026-10-23

- Q: What does `ws up` do when the folder name contains `.` or `:`? → A: Replace each `.` and `:`
  with `_` (FR-005).
- Q: What does `ws up` without a name do when a session with the derived name exists but belongs to
  a different folder? → A: Refuse with a non-zero exit status (FR-011).
- Q: How does `ws` know the folder of a session that plain `tmux` made? → A: Use the start
  directory that tmux records for every session (FR-011). For a session that `ws` made, use the
  project folder that `ws` recorded, because the start directory can be a subfolder from `.ws`.

## Assumptions

- "Workspace name" in the request means the tmux session name (001 calls it `SESSION_NAME`).
- The folder name comes from the logical path of the project folder (the path that the shell
  shows). If the logical path is not available, the system uses the physical path.
- `ws` does not change the case of the folder name and does not shorten long names.
- Only `ws up` gets the default. `ws down` without a name keeps its 001 behavior (FR-024).
