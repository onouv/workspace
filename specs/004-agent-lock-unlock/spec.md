# Feature Specification: Lock and Unlock Secrets

**Feature Branch**: `004-agent-lock-unlock`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: Create a feature spec for 1) `ws lock`, 2) lock before suspend, and
3) `ws unlock` as a user command to trigger the password prompt (but not automatically on every
start).

## Terms

This specification uses each term below with one meaning only.

| Term | Meaning |
|------|---------|
| agent | The GPG agent of the user. It holds the unlocked secret keys in memory. `pass` and SSH use it. |
| store key | The GPG key that encrypts the password store (named in the `.gpg-id` file of the store). |
| SSH key | A key that the agent offers to SSH (listed as enabled in the agent SSH configuration). |
| managed key | The store key or an SSH key. |
| unlocked | The agent holds the passphrase of a managed key in its cache. A use of the key needs no prompt. |
| locked | The agent holds no passphrase for any managed key. The next use of a managed key needs a prompt. |
| prompt | The passphrase dialog of the agent. The agent shows it, not `ws`. |
| lock | Make all managed keys locked. |
| unlock | Make all managed keys unlocked through prompts. |
| suspend | The computer goes to sleep or hibernation. |
| suspend lock | A lock that runs automatically before each suspend. |

## Relationship to 001-tmux-workspace

This feature adds commands. It does not replace 001 items. These 001 items apply without change:

- FR-025: `ws` uses the existing `pass`/agent flow. `ws` never receives or caches the master
  passphrase.
- FR-028: Passphrases and store entries stay out of stdout, stderr, logs, commands, and environments.
- FR-031: Stable, documented exit statuses.
- The 001 security design decision: `ws` uses the agent of the user on demand.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Lock all secrets on demand (Priority: P1)

The developer leaves the desk. The developer runs `ws lock`. After that, no secret is usable
without the passphrase.

**Why this priority**: A lock is the basic safety function. Suspend lock (US2) uses it.

**Independent Test**:

1. Unlock the managed keys of a test agent.
2. Run `ws lock`.
3. Make sure that the agent holds no passphrase for any managed key.
4. Make sure that a following use of a secret needs a prompt.

**Acceptance Scenarios**:

1. **US1-AS1**. **Given** the managed keys are unlocked, **When** the user runs `ws lock`, **Then**
   all managed keys are locked. `ws` exits with status 0 and shows one line that tells that the
   secrets are locked.
2. **US1-AS2**. **Given** the managed keys are locked, **When** the user runs `ws lock`, **Then**
   `ws` exits with status 0. Nothing changes.
3. **US1-AS3**. **Given** the agent does not run, **When** the user runs `ws lock`, **Then** `ws`
   exits with status 0. `ws` does not start the agent.
4. **US1-AS4**. **Given** the user runs `ws lock` inside a `ws` session, outside tmux, or without a
   TTY, **When** the command completes, **Then** the result is the same in each case. `ws` asks no
   question.
5. **US1-AS5**. **Given** the managed keys are unlocked and two `ws` sessions run, **When** the
   user runs `ws lock` in one session, **Then** the keys are locked for all sessions and terminals
   of the user.
6. **US1-AS6**. **Given** `ws clip` put a value on the clipboard and the clipboard still holds it,
   **When** the user runs `ws lock`, **Then** the clipboard is empty.
7. **US1-AS7**. **Given** the user copied other text after `ws clip`, **When** the user runs
   `ws lock`, **Then** the clipboard still holds that text.

---

### User Story 2 - Lock automatically before suspend (Priority: P1)

The developer closes the laptop lid. The secrets lock before the computer sleeps. After wake-up,
the first use of a secret needs the passphrase again.

**Why this priority**: The agent cache survives a suspend. Without this function, a stolen
sleeping laptop gives access to unlocked secrets until the cache time ends.

**Independent Test**:

1. Enable the suspend lock.
2. Unlock the managed keys.
3. Send the suspend signal of the system (or a test stand-in for it).
4. Make sure that all managed keys are locked before the suspend continues.
5. Disable the suspend lock and repeat. Make sure that the keys stay unlocked.

**Acceptance Scenarios**:

1. **US2-AS1**. **Given** the suspend lock is enabled and the managed keys are unlocked, **When**
   the computer starts a suspend, **Then** all managed keys are locked before the computer sleeps.
2. **US2-AS2**. **Given** the suspend lock locked the keys and the computer woke up, **When** the
   user uses a secret, **Then** the agent shows a prompt.
3. **US2-AS3**. **Given** the suspend lock is enabled and no `ws` session runs, **When** the
   computer starts a suspend, **Then** the keys are locked. The suspend lock does not depend on a
   running `ws` session.
4. **US2-AS4**. **Given** the suspend lock is disabled, **When** the user runs
   `ws lock --on-suspend enable`, **Then** the suspend lock is enabled and
   `ws lock --on-suspend status` shows `enabled`. After `ws lock --on-suspend disable`, the status
   shows `disabled` and no part of the suspend lock remains on the system.
5. **US2-AS5**. **Given** the lock fails during a suspend (for example, the agent does not answer),
   **When** the computer starts a suspend, **Then** the suspend continues. The failure does not
   stop or delay the suspend for more than 5 seconds.

---

### User Story 3 - Unlock on purpose (Priority: P2)

The developer starts the day. The developer runs `ws unlock` and enters the passphrase once. After
that, `pass`, `ws clip`, `ssh`, and `git push` do not ask until the cache time ends or a lock
occurs.

**Why this priority**: Without this command, the first prompt comes at the first use of a secret,
for example in the middle of a push. This command is a convenience.

**Independent Test**:

1. Lock the managed keys of a test agent.
2. Run `ws unlock` with a test prompt that gives the correct passphrase.
3. Make sure that all managed keys are unlocked.
4. Make sure that `ws` did not see the passphrase.

**Acceptance Scenarios**:

1. **US3-AS1**. **Given** the managed keys are locked, **When** the user runs `ws unlock`, **Then**
   the agent shows a prompt. After the user enters the correct passphrase, all managed keys are
   unlocked. `ws` exits with status 0 and shows one line that tells that the secrets are unlocked.
2. **US3-AS2**. **Given** all managed keys are unlocked, **When** the user runs `ws unlock`,
   **Then** no prompt appears. `ws` exits with status 0 and tells that the secrets are already
   unlocked.
3. **US3-AS3**. **Given** the prompt appears, **When** the user cancels it or the agent rejects the
   passphrase, **Then** `ws` exits with a non-zero status. The managed keys that were locked stay
   locked.
4. **US3-AS4**. **Given** the user runs `ws up`, `ws change`, or any other command, **When** the
   command runs, **Then** `ws` does not unlock and does not show a prompt for this feature. Only
   `ws unlock` starts an unlock.
5. **US3-AS5**. **Given** no display and no TTY are available for a prompt, **When** the user runs
   `ws unlock`, **Then** `ws` exits with a non-zero status. The message tells that no prompt can
   appear and how to continue.

---

### Edge Cases

- The user has more than one SSH key in the agent. `ws unlock` unlocks each of them (FR-005).
- The store key and an SSH key are subkeys of the same GPG key with the same passphrase. The agent
  still keeps one cache entry per key. See FR-006 for the number of prompts.
- No password store exists, or it has no `.gpg-id` file. `ws unlock` unlocks only the SSH keys.
  If no managed key exists at all, `ws unlock` exits with status 0 and tells that there is nothing
  to unlock.
- GnuPG is not installed. `ws lock` exits with status 0 (nothing to lock). `ws unlock` fails with
  the 001 status for an unavailable dependency.
- The cache time ends while the user works. The agent shows a prompt at the next use. `ws` does not
  change this behavior.
- The computer hibernates instead of sleeping. The suspend lock applies (the term "suspend"
  includes hibernation).
- The user locks the screen without a suspend. This feature does not lock the keys (see
  Assumptions).
- A `ws clip` value is on the clipboard at the time of a lock. `ws lock` clears it (FR-004).
- The user copied other text after `ws clip`. `ws lock` does not touch the clipboard (FR-004).
- `ws lock --on-suspend enable` runs twice. The second run changes nothing and succeeds.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide the command `ws lock`. It MUST make all managed keys locked
  for the user. The effect MUST apply to all sessions and terminals of the user.
- **FR-002**: `ws lock` MUST succeed (status 0) when the keys are already locked, when the agent
  does not run, and when GnuPG is not installed. It MUST NOT start the agent. It MUST NOT ask for
  confirmation, because a lock loses no data.
- **FR-003**: `ws lock` MUST NOT stop running programs, tmux sessions, or SSH connections that are
  already open.
- **FR-004**: When `ws lock` runs (also as suspend lock) and the clipboard still holds the value
  that `ws clip` put there, the system MUST clear the clipboard. The system MUST NOT clear other
  clipboard content. The system MUST NOT keep a copy of the value to do this check (001 FR-028).
  If the system cannot tell whether the clipboard holds a `ws clip` value, it MUST leave the
  clipboard unchanged.
- **FR-005**: The system MUST provide the command `ws unlock`. It MUST unlock the store key and each
  SSH key through the prompt of the agent. The system MUST NOT read, receive, store, or pass the
  passphrase itself (001 FR-025).
- **FR-006**: `ws unlock` MUST show no more than one prompt per locked managed key. A managed key
  that is already unlocked MUST NOT cause a prompt.
- **FR-007**: The system MUST NOT unlock and MUST NOT show a prompt for this feature in any command
  other than `ws unlock`. This includes `ws up` and the start of a session.
- **FR-008**: The system MUST provide `ws lock --on-suspend enable`, `ws lock --on-suspend disable`,
  and `ws lock --on-suspend status`. `enable` installs the suspend lock for the current user.
  `disable` removes it completely. `status` shows `enabled` or `disabled` on stdout. `enable` and
  `disable` MUST be idempotent. If administrator rights are necessary, the system MUST tell the
  user before it asks for them, and MUST NOT ask without a TTY; without a TTY it MUST fail with a
  documented status and tell the user to run the command in a terminal.
- **FR-009**: When the suspend lock is enabled, the system MUST lock before each suspend
  (including hibernation). The suspend lock MUST work when no `ws` session runs.
- **FR-010**: A failure of the suspend lock MUST NOT stop a suspend and MUST NOT delay it by more
  than 5 seconds. The system MUST record the failure where the user can find it later (for
  example, the system log).
- **FR-011**: `ws unlock` MUST exit with a non-zero, documented status when the user cancels the
  prompt, when the agent rejects the passphrase, or when no prompt can appear. The message MUST
  NOT contain the passphrase or a store entry.
- **FR-012**: `ws lock` and `ws unlock` MUST write their one-line result to stdout and diagnostics
  to stderr (001 FR-030). They MUST work in a non-TTY context. `ws unlock` without a display and
  without a TTY MUST fail as FR-011 specifies.
- **FR-013**: `ws help`, `ws lock --help`, and `ws unlock --help` MUST describe the commands and
  the `--on-suspend` actions. The
  help MUST tell that `ws unlock` never runs automatically and that the agent cache time controls
  how long the keys stay unlocked. The README MUST agree with this text.

### Key Entities

- **Managed Key**: A key that the agent protects and that `pass` or SSH uses. It has the state
  locked or unlocked. `ws lock` and the suspend lock change it to locked. `ws unlock` and a normal
  use with a prompt change it to unlocked. The cache time of the agent changes it to locked.
- **Suspend Lock**: A setting of the user. It is enabled or disabled.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the automated tests, all managed keys are locked after `ws lock`.
- **SC-002**: In 100% of the tests with the suspend lock enabled, all managed keys are locked before
  the suspend continues. No test suspend is delayed by more than 5 seconds.
- **SC-003**: In 100% of the automated tests, `ws unlock` shows no more than one prompt per locked
  managed key, and none when all keys are unlocked.
- **SC-004**: In 100% of the automated tests, the passphrase does not appear in the stdout,
  stderr, arguments, environment, or files of `ws`.
- **SC-005**: In 100% of the automated tests of commands other than `ws unlock`, no prompt of this
  feature appears.
- **SC-006**: A developer unlocks once at the start of the day and then uses `pass`, `ws clip`, and
  `git push` without a passphrase prompt until a lock, a suspend, or the end of the cache time.
- **SC-007**: In 100% of the automated tests, `ws lock` clears a `ws clip` value from the clipboard
  and leaves other clipboard content unchanged.

## Traceability

Each acceptance scenario has a stable `US<story>-AS<n>` id (constitution IV). All scenarios are
**Pending** until `/speckit.tasks` gives each scenario a test.

| Scenario | Owning test / status |
|----------|----------------------|
| US1-AS1 | Pending |
| US1-AS2 | Pending |
| US1-AS3 | Pending |
| US1-AS4 | Pending |
| US1-AS5 | Pending |
| US1-AS6 | Pending |
| US1-AS7 | Pending |
| US2-AS1 | Pending |
| US2-AS2 | Pending |
| US2-AS3 | Pending |
| US2-AS4 | Pending |
| US2-AS5 | Pending |
| US3-AS1 | Pending |
| US3-AS2 | Pending |
| US3-AS3 | Pending |
| US3-AS4 | Pending |
| US3-AS5 | Pending |

## Clarifications

### Session 2026-10-24

- Q: How does the user enable and disable the suspend lock? → A: With
  `ws lock --on-suspend enable|disable|status` (FR-008).
- Q: Does `ws lock` clear a `ws clip` value from the clipboard? → A: Yes, only when the clipboard
  still holds that value (FR-004).

## Assumptions

- The agent is the GnuPG agent with SSH support, as set up on 2026-10-24. Other SSH agents are out
  of scope.
- The agent shows the prompt (pinentry). `ws` only asks the agent to unlock. This keeps 001 FR-025.
- The cache time is the setting of the agent (now 8 hours). `ws` does not change it.
- A screen lock without a suspend does not lock the keys. This can be a later feature.
- A logout or reboot ends the agent and its cache. This feature does not change that.
- The suspend lock may need administrator rights once to install, because suspend events belong to
  the system, not to the user session.
- The ideal is one prompt for all managed keys when they share one passphrase. The agent keeps one
  cache entry per key, so one prompt per key is the guaranteed limit (FR-006). The plan examines
  whether one prompt is possible without `ws` handling the passphrase.
