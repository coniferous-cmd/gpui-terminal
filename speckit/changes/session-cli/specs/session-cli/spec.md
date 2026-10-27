---
create-time: 2026-10-27 21:18:25
---

## ADDED Requirements

### Requirement: Command-line entry point

The application SHALL support `session list`, `session create`, `session rename <id> <name>`, `session focus <id>`, and `session close <id> [--force]`. Help, version, parsing errors, and commands targeting an existing GUI SHALL not initialize a second GUI. No-argument startup SHALL retain ordinary startup and restoration behavior.

#### Scenario: Help without a display

- **WHEN** a user requests `--help`, `--version`, or subcommand help without a display server
- **THEN** the command prints the requested information and exits successfully without initializing GPUI.

#### Scenario: Invalid arguments

- **WHEN** arguments contain an unknown command, unsupported option, or missing required operand
- **THEN** the command reports a usage error to stderr, exits nonzero, and performs no GUI operation.

### Requirement: Live session listing and identity

`session list` SHALL list live sidebar sessions across discoverable application instances, with opaque session ID, window ID, display name, active-tab folder, tab count, and whether selected within its window. Default output SHALL be a table; `--json` SHALL return a JSON array with `id`, `window_id`, `name`, `cwd`, `tab_count`, and `selected` fields. IDs SHALL remain stable for the lifetime of a session, including when its original tab closes, and SHALL not target a different session after restart. Names MAY repeat. Folder text SHALL be display-only, not an assertion of lossless filesystem encoding.

#### Scenario: Listing reflects live state

- **WHEN** sessions have been created, renamed, reordered, or closed since the last saved snapshot
- **THEN** listing reflects the current GUI state rather than reading `sessions.json`.

#### Scenario: Duplicate names and stale IDs

- **WHEN** two sessions share a name and a mutation specifies one session ID
- **THEN** only that session is targeted; an unknown or expired ID produces a nonzero error without changing another session.

### Requirement: Session creation

`session create [--name <name>] [--cwd <directory>] [--window <window-id>]` SHALL create a sidebar session containing one tab running the configured shell with normal integration. It SHALL select the new session within its window and print its ID only after successful creation. Explicit relative directories SHALL resolve against the CLI process's working directory; an omitted directory SHALL follow the normal Start in setting in the target window. Names SHALL follow GUI rename trimming semantics.

#### Scenario: Explicit working directory

- **WHEN** creation specifies an existing accessible directory
- **THEN** the shell starts there and the command returns the new session ID.

#### Scenario: Invalid directory or failed spawn

- **WHEN** the directory is invalid or the configured shell cannot start
- **THEN** the command returns nonzero with the failure and leaves no empty session.

#### Scenario: Multiple possible target windows

- **WHEN** creation omits `--window` and multiple live terminal windows exist
- **THEN** it fails with an ambiguity error directing the user to select a window ID from `session list`.

### Requirement: Rename and focus

Rename SHALL update the session's custom name using GUI semantics without changing individual tab names; an empty trimmed name SHALL restore the current tab's normal display name. Focus SHALL select the session, retain its selected tab, focus its terminal, and request that its window be shown and activated, subject to platform focus restrictions.

#### Scenario: Reset a custom name

- **WHEN** `session rename <id> ""` succeeds
- **THEN** the session resumes its ordinary tab-derived title and normal layout persistence records the change.

#### Scenario: Focus a hidden session

- **WHEN** focus targets a session not currently selected in its window
- **THEN** the GUI selects that session and requests window activation without creating another tab or session.

### Requirement: Explicit destructive closure

`session close <id>` SHALL directly close a single-tab session. A session containing multiple tabs SHALL require `--force`. CLI closure SHALL never wait for a GUI confirmation dialog. Forced closure SHALL close the whole group and its programs. Normal last-session window closure and application exit behavior SHALL remain intact.

#### Scenario: Multi-tab safety

- **WHEN** a multi-tab session is closed without `--force`
- **THEN** the command fails nonzero with an explanation and leaves all tabs running without opening a dialog.

#### Scenario: Forced or single-tab closure

- **WHEN** a single-tab session is closed, or any session is closed with `--force`
- **THEN** the addressed session is removed and the CLI receives success before transport teardown caused by last-window shutdown.

### Requirement: GUI discovery and automatic startup

Commands SHALL use current-user-only local IPC. With no running GUI, only `session create` SHALL launch one automatically, wait for readiness within a finite timeout, and perform the requested creation once. Other session commands SHALL fail with a clear not-running error. An explicit unavailable window SHALL fail rather than trigger startup. Concurrent creation requests SHALL not cause duplicate bootstrap GUIs. Existing no-argument multi-instance behavior SHALL not be silently replaced with single-instance forwarding.

#### Scenario: Cold creation

- **WHEN** creation is requested with no GUI running and no explicit window ID
- **THEN** one GUI starts using normal restoration, and the requested session is added once after startup; failure or ambiguity is reported rather than guessing a target.

#### Scenario: GUI unavailable

- **WHEN** list, rename, focus, or close is requested with no running GUI
- **THEN** the CLI exits nonzero and explains that luna-terminal is not running.

#### Scenario: Unauthorized local peer

- **WHEN** another operating-system user attempts to reach the control endpoint
- **THEN** endpoint permissions or peer authorization deny access before session operations execute.

### Requirement: Scriptable outcomes and bounded failures

Successful commands SHALL exit zero; failed commands SHALL exit nonzero with a useful stderr diagnostic and no success output. Logs SHALL not pollute stdout. List JSON SHALL contain no human-readable preamble. IPC SHALL reject malformed, unsupported, or oversized messages and impose finite connection/readiness/response waits. A timeout after dispatch SHALL report an unknown outcome and SHALL NOT automatically retry a mutation. GUI mutations SHALL use normal notifications, logging, focus, lifecycle, and persistence paths.

#### Scenario: Response lost after mutation

- **WHEN** the GUI may have applied a create or close request but its reply is lost
- **THEN** the CLI reports the uncertain result and does not blindly resend the operation.

#### Scenario: Windows release invocation

- **WHEN** a release executable is invoked from a command shell or with redirected output on Windows
- **THEN** CLI output and exit status are usable without spawning an unwanted console for ordinary GUI startup.
