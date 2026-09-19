# `ws clip` Credential Mapping Reference

`ws clip NAMESPACE ITEM` sends one configured credential to the clipboard. It never prints a
value, and it resolves credentials on demand: nothing is read from the password store or written
to the clipboard until the matching `NAMESPACE ITEM` is requested. This reference is normative for
mapping-language version 1 and is also available with `ws help clip`.

## File locations

Two optional UTF-8 YAML 1.2 documents are merged to resolve an entry, both using the schema below:

- **User-level file**: `$XDG_CONFIG_HOME/ws/clip.yaml`, or `$HOME/.config/ws/clip.yaml` when
  `XDG_CONFIG_HOME` is unset or empty. Entries here are available to every project — typically
  personal Git and container-registry credentials.
- **Project-level file**: `.ws-clip` in the project directory, alongside `.ws`. Entries here are
  specific to one project, such as a project-scoped registry token.

A missing file on either side is treated as an empty mapping, the same way a missing `.ws` uses
the default workspace. When both files define the same `NAMESPACE ITEM`, the **project-level
entry wins**.

Neither file may contain a live secret value: an entry names *where* a credential lives (a `pass`
store path) or supplies a fixed, non-secret value (such as a username). Both files are safe to
keep in ordinary dotfiles — they never need `.secrets/`.

## Complete schema

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

## Rules

- The document contains one top-level mapping with required `version` and `entries` keys.
- `version` MUST be `1`. A future incompatible change uses a new version.
- `entries` is a mapping from namespace to a mapping from item to one entry. `NAMESPACE` and
  `ITEM` in `ws clip NAMESPACE ITEM` select `entries.NAMESPACE.ITEM`.
- Each entry sets exactly one of `pass` or `literal`, never both, never neither:
  - `pass` names one `pass` store entry (for example `repos/github/token`). It is resolved on
    demand via `pass show <path>` through the user's existing GPG-agent unlock flow, streamed
    directly to the clipboard provider, and never printed, logged, or cached.
  - `literal` is a fixed, non-secret value (for example a username) copied to the clipboard
    without contacting `pass` at all.
- A `NAMESPACE ITEM` pair absent from the merged mapping is rejected before contacting `pass` or
  the clipboard provider, with an error pointing back at `ws help clip`.
- Unknown properties, wrong YAML types, an entry with both or neither of `pass`/`literal`, and an
  unsupported `version` are rejected with a source location when available.

## Canonical example

A typical user-level file at `~/.config/ws/clip.yaml`:

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

A project that needs its own registry token can add a project-level `.ws-clip` file that overrides
just that one entry, without touching the user-level file:

```yaml
version: 1
entries:
  docker:
    token: {pass: registries/dockerhub/tokens/this-project}
```

## Clipboard provider

`ws clip` sends the resolved value to an external clipboard provider command, configured through
the `WS_CLIPBOARD_PROVIDER` environment variable using the same "program followed by
space-separated arguments" convention as `WS_TERMINAL_LAUNCHER` (for example `wl-copy` on Wayland,
or `pbcopy` on macOS). When unset, `ws` uses `xclip -selection clipboard`. `ws` never
shell-interprets the configured value, writes the resolved value only to the provider's standard
input, and never reads or forwards the provider's own stdout or stderr — an unavailable or
refusing provider is reported as a recoverable error, never a fallback to printing the value.
