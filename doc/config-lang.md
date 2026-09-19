# `.ws` Configuration Language Reference

`ws` configuration files are UTF-8 YAML 1.2 documents. The file is named `.ws` and is read from
the project directory when `ws up SESSION_NAME` creates a missing workspace. This reference is
normative for schema version 1 and is also available with `ws help config`.

## Complete schema

The following JSON Schema 2020-12 document describes the accepted YAML mappings and values:

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

## Rules

- The document contains one top-level mapping with required `version` and `windows` keys.
- `version` MUST be `1`. A future incompatible syntax or semantic change uses a new version.
- `windows` is an ordered, non-empty sequence. Each window requires a unique, non-empty `name` and
  a non-empty `path`.
- A window may define `command`, `env`, and an ordered `panes` sequence. If `command` is omitted,
  the user's normal shell is started.
- Each pane requires `pos`, which must be `left`, `right`, `top`, or `bottom`. A pane may define a
  unique `id` within its containing window, `path`, `command`, `env`, and nested `panes`.
- A pane without `path` inherits the path of its containing window or pane. Relative paths resolve
  against the directory containing `.ws`.
- `env` maps environment-variable names to string values. Names match
  `^[A-Za-z_][A-Za-z0-9_]*$`. Window values are inherited by descendant panes, and pane values
  override inherited values. Quote values that look like YAML booleans, numbers, or null when they
  must remain strings.
- Windows and panes retain declaration order. Window order determines tmux window order; nested pane
  order determines the split hierarchy.
- Recursive pane nesting MUST NOT exceed a depth of 32 levels beneath a window. A document nesting
  panes deeper than 32 levels is rejected with a source-oriented validation error rather than risking
  unbounded recursion.
- `left` and `right` create horizontal splits. `top` and `bottom` create vertical splits relative to
  the containing pane. Tmux's default pane dimensions are used.
- `command` is a shell command string and may invoke an executable, custom Bash script, pipeline, or
  other shell operation. For example, `command: bash ./scripts/setup.sh` runs the script from the
  declaration's resolved working directory with the declared environment.
- Unknown properties, wrong YAML types, duplicate mapping keys, missing required values, invalid
  positions, invalid environment names, and invalid version values are rejected with a source
  location when available.
- YAML comments, quoted and plain scalar styles, sequences, and mappings are supported. Custom YAML
  tags and multi-document streams are rejected.
- A missing or empty `.ws` file uses the default single-window workspace: one window named
  `root`, rooted at the project directory, running the user's normal interactive shell. A
  non-empty file must follow this schema.

## Canonical example

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

The `.ws` file contains project layout and non-secret runtime configuration only. Credentials must be
retrieved on demand from the approved external password store; they must not be placed in this file,
tmux environment state, command arguments, logs, or repository fixtures.
