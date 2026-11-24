# Feature Specification: Generate PNG Images from PlantUML Files

**Feature Branch**: `005-init-puml-img`

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: new feature "init-puml-img" as follows: a cmd ws init puml goes through
the current folder and all its sub folders and generates image files (*.png) from all *.puml files
it encounters. The names must be identical, other than the .png extension.

## Terms

This specification uses each term below with one meaning only.

| Term | Meaning |
|------|---------|
| start folder | The current folder at the time the user runs `ws init puml`. |
| scan | The walk through the start folder and all its sub folders. |
| source file | A file with the extension `.puml` that the scan finds. |
| image | A PNG file that the system makes from one source file. |
| image path | The path of the source file, with the extension `.puml` replaced by `.png`. Example: `doc/arch/flow.puml` gives `doc/arch/flow.png`. |
| renderer | The installed PlantUML program that changes a source file into an image. |

## Relationship to 001-tmux-workspace

This feature adds a command. It does not replace 001 items. These 001 items apply without change:

- FR-031: Stable, documented exit statuses.
- FR-032: Non-TTY invocation does not hang on a prompt.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generate all images in a project (Priority: P1)

The developer keeps diagrams as `.puml` files in different folders of a project. The developer
runs `ws init puml` in the project root. After that, each source file has an image next to it,
with the same name and the extension `.png`.

**Why this priority**: This is the full value of the feature. Without it, the developer must
render each diagram by hand.

**Independent Test**:

1. Make a temporary folder with source files at the top level and in nested sub folders.
2. Run `ws init puml` in that folder.
3. Make sure that each source file has an image at its image path.
4. Make sure that no other new files exist.

**Acceptance Scenarios**:

1. **US1-AS1**. **Given** the start folder holds `a.puml` and `sub/deep/b.puml`, **When** the user
   runs `ws init puml`, **Then** `a.png` and `sub/deep/b.png` exist and are valid PNG images. `ws`
   exits with status 0.
2. **US1-AS2**. **Given** the scan found source files, **When** the command completes, **Then**
   stdout shows one line per image made and one summary line with the number of images made.
3. **US1-AS3**. **Given** a source file holds a diagram with its own output name (for example
   `@startuml other-name`), **When** the user runs `ws init puml`, **Then** the image is at the
   image path of the source file. No file with the other name is made.
4. **US1-AS4**. **Given** the start folder holds no source file at its top level, and
   `sub/c.puml` exists, **When** the user runs `ws init puml`, **Then** the system still scans the
   full sub folder tree and makes `sub/c.png`. `ws` exits with status 0.
5. **US1-AS5**. **Given** the start folder holds files with other extensions (for example
   `notes.txt`, `x.puml.bak`), **When** the user runs `ws init puml`, **Then** the system makes no
   image for them.
6. **US1-AS6**. **Given** the user runs `ws init puml` outside tmux, without a `.ws` file, or
   without a TTY, **When** the command completes, **Then** the result is the same in each case.
   `ws` asks no question.

---

### User Story 2 - Existing images (Priority: P1)

The developer changed some diagrams and runs `ws init puml` again. The images reflect the current
source files.

**Why this priority**: The developer runs the command again after each diagram change. A wrong
result here gives old diagrams in the documentation.

**Independent Test**:

1. Run `ws init puml` in a folder with source files.
2. Change one source file.
3. Run `ws init puml` again.
4. Make sure that the image of the changed source file shows the change.

**Acceptance Scenarios**:

1. **US2-AS1**. **Given** an image exists at the image path of a source file, **When** the user runs
   `ws init puml`, **Then** the system replaces the existing image with a new image. The system
   writes no warning and no log line about the replacement.
2. **US2-AS2**. **Given** the image path is a folder, not a file, **When** the user runs
   `ws init puml`, **Then** the system does not change that folder, reports an error for that
   source file, and continues with the other source files.

---

### User Story 3 - Errors in single diagrams (Priority: P2)

One diagram has a syntax error. The developer runs `ws init puml`. All other images are made. The
output tells which source file failed and why.

**Why this priority**: In a large project, one bad diagram must not block all others.

**Independent Test**:

1. Make a temporary folder with two valid source files and one source file with a syntax error.
2. Run `ws init puml`.
3. Make sure that the two valid images exist.
4. Make sure that stderr names the bad source file and that the exit status is not 0.

**Acceptance Scenarios**:

1. **US3-AS1**. **Given** one source file has a syntax error, **When** the user runs
   `ws init puml`, **Then** the system makes the images of all other source files. stderr names the
   bad source file with its path relative to the start folder and the error of the renderer. `ws`
   exits with a non-zero status.
2. **US3-AS2**. **Given** the renderer is not installed, **When** the user runs `ws init puml`,
   **Then** the system makes no image. `ws` exits with the 001 status for an unavailable dependency.
   The message tells the user to install PlantUML.
3. **US3-AS3**. **Given** a sub folder cannot be read, **When** the user runs `ws init puml`,
   **Then** stderr names that folder. The system continues with the other folders. `ws` exits with a
   non-zero status.

---

### Edge Cases

- A sub folder is a symbolic link. The scan does not follow it (FR-004). This prevents loops and
  scans outside the start folder.
- A source file is a symbolic link to a file. The scan treats it as a source file. The image goes
  to the image path of the link, not of the target.
- The folder tree is deeper than the maximum depth (FR-005). The system does not scan below that
  depth, reports the skipped folders on stderr, and exits with a non-zero status.
- A source file has the extension `.PUML` or `.Puml`. The system does not treat it as a source file
  (FR-002).
- A source file holds more than one diagram. The image path holds the first diagram only. The
  system tells on stderr that it ignored the other diagrams.
- A folder name starts with `.` (for example `.git`). The scan goes into it, the same as into all
  other sub folders (FR-003).
- A file name has spaces or non-ASCII characters. The system handles it the same as other names.
- The start folder and all its sub folders hold no source file. `ws` exits with status 0 and tells
  that it found no `.puml` file.
- The source file is empty. The system reports an error for that source file (US3-AS1 applies).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide the command `ws init puml`. It MUST scan the start folder and
  all its sub folders.
- **FR-002**: The system MUST treat a file as a source file only when its name ends with `.puml`
  (lowercase, exact).
- **FR-003**: The scan MUST be recursive. It MUST go into each sub folder at each level, including
  folders whose name starts with `.`. Only FR-004 and FR-005 limit the scan.
- **FR-004**: The scan MUST NOT follow symbolic links to folders.
- **FR-005**: The scan MUST stop at a documented maximum folder depth below the start folder. The
  default maximum depth is 32.
- **FR-006**: For each source file, the system MUST write the image to the image path. The name of
  the image MUST be identical to the name of the source file, other than the extension `.png`. A
  name in the diagram text MUST NOT change the image path.
- **FR-007**: The system MUST NOT write, change, or delete any file other than images at image
  paths.
- **FR-008**: When an image exists at the image path, the system MUST replace it. The system MUST
  NOT write a warning or a log line about the replacement. The stdout line of FR-010 is the same as
  for a new image.
- **FR-009**: When the system cannot make the image for one source file, it MUST continue with the
  other source files. It MUST NOT leave a partial or empty image at the image path of the failed
  source file.
- **FR-010**: The system MUST write one line per image made to stdout. It MUST write errors and
  warnings to stderr. Each line MUST name the file with its path relative to the start folder.
- **FR-011**: The system MUST exit with status 0 only when it found no error. When one or more source
  files or folders failed, it MUST exit with a stable, documented non-zero status (001 FR-031).
- **FR-012**: When the renderer is not available, the system MUST stop before it makes any image,
  exit with the 001 status for an unavailable dependency, and tell the user to install PlantUML.
- **FR-013**: The command MUST work without a `.ws` file, outside tmux, and without a TTY. It MUST
  NOT ask any question.
- **FR-014**: `ws help` and `ws init puml --help` MUST describe the command, the scan rules
  (FR-002 to FR-005), and the image path rule (FR-006).

### Key Entities

- **Source file**: A `.puml` file in the scanned tree. Attributes: path relative to the start
  folder, last change time.
- **Image**: The PNG file at the image path of one source file. Attributes: path, last change
  time.
- **Run result**: The list of images made, the list of failed source files and folders with their
  reason, and the exit status.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After one run in a tree with valid source files only, 100% of the source files have an
  image at their image path, and no other file in the tree changed.
- **SC-002**: The user makes all images of a project with one command, without arguments.
- **SC-003**: In a tree with one invalid source file, all other images are made, and the user can
  find the invalid file from the output alone in under 30 seconds.
- **SC-004**: The command adds no more than 1 second of its own time for a tree of 1,000 folders,
  not counting the time of the renderer.
- **SC-005**: The command never hangs without a TTY. Each run ends with a status and output.

## Assumptions

- The renderer is an installed PlantUML program on the `PATH`. The system does not install it and
  does not download it. The system does not use a remote PlantUML server, because that sends
  diagram content to an external service.
- `ws init` is a new command group. `puml` is its first subcommand. Other `init` subcommands are out
  of scope for this feature.
- The command takes no folder argument. The user changes to the folder to scan.
- The user wants all sub folders scanned, so hidden folders are not excluded (FR-003).
- The image format is PNG only. Other formats (SVG, PDF) are out of scope.
- Images are made one after the other. Parallel rendering is out of scope for this feature.
- The maximum depth of 32 (FR-005) applies constitution principle IX to the scan.
