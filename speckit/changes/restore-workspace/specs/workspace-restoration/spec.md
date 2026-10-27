---
create-time: 2026-10-23 17:29:35
---

## Review Gate

P1 (second ordinary instance behavior) remains unapproved. The refusal scenario below is the proposed acceptance contract, not authorization to implement it. Resolve P1 with the owner before implementation and revise the linked design/tasks if another policy is chosen. Other requirements are unchanged by this review gate.

## ADDED Requirements

### Requirement: Restore workspace structure

On ordinary startup the application SHALL restore saved session names and order, tab order and last known directories, the selected session, and each session's selected tab. Each restored terminal SHALL start a new shell using the current Shell setting. Restoration SHALL NOT replay commands or restore output, processes, temporary environment changes, dynamic titles, or nested multiplexer layouts.

#### Scenario: Restore several sessions

- **WHEN** the application restarts after saving several renamed sessions with multiple tabs and different directories
- **THEN** names, order, directories, and selection match the snapshot
- **THEN** each runnable tab has a fresh shell and no restored transcript

#### Scenario: No saved terminals

- **WHEN** the workspace file is absent or contains a valid empty workspace
- **THEN** startup opens one default terminal using the existing home-directory startup behavior

#### Scenario: Directory reporting is unavailable

- **WHEN** a terminal has reported no directory since launch
- **THEN** its saved directory is its launch directory, not one inferred from typed commands

### Requirement: Persist workspace changes during use

The application SHALL save structural changes, committed renames, selections, and reported directory changes independently of normal exit. Saves SHALL be ordered and atomically replace the prior snapshot, with frequent changes coalesced without indefinite postponement. Application-controlled normal shutdown SHALL flush pending state before terminal teardown. Once GPUI reports an uncancellable application quit, the application SHALL attempt the same ordered flush and log failure, but SHALL NOT claim it can keep the application open.

#### Scenario: Continued directory and selection changes

- **WHEN** directory reports or selections keep changing
- **THEN** the newest snapshot is periodically committed and an older save cannot overwrite it

#### Scenario: Abnormal termination

- **WHEN** the application terminates without completing shutdown
- **THEN** the next launch restores the last successfully committed snapshot, which may omit recent uncommitted changes

#### Scenario: Write failure

- **WHEN** a snapshot write fails
- **THEN** the previous valid snapshot remains intact, the failure is logged and visibly reported, and pending state can be retried

### Requirement: Distinguish removal from application shutdown

Explicit tab or session closure and normal shell exit during use SHALL remove the corresponding entries from future restoration. Closing the main window or quitting the application SHALL preserve the workspace existing before teardown. Teardown exit events SHALL NOT empty the saved workspace.

#### Scenario: Close one tab

- **WHEN** a user closes a tab and subsequently exits the application
- **THEN** that tab is absent on restart and the other open tabs return

#### Scenario: Close the main window

- **WHEN** the user closes the main window with terminals still open
- **THEN** restarting restores those terminals as fresh shells

#### Scenario: Close the last tab explicitly

- **WHEN** the user explicitly closes the last tab or its shell exits normally
- **THEN** the empty workspace is saved before the resulting window closure
- **THEN** the next launch opens one default terminal

#### Scenario: Final save fails during a cancellable close

- **WHEN** the user closes the main window or invokes the application's Quit action and the final save fails
- **THEN** the window remains open with Retry and Quit without saving actions
- **THEN** Quit without saving explicitly preserves the last committed snapshot instead of claiming the newest state was saved

#### Scenario: Repeated close requests

- **WHEN** another close or Quit request arrives while final saving is pending
- **THEN** the application retains the same frozen snapshot and does not launch another writer or tear down terminals early

#### Scenario: Last-tab removal cannot be saved

- **WHEN** removing the last tab leaves an empty workspace and its final save fails
- **THEN** the empty-window error view remains usable without indexing a nonexistent session
- **THEN** Retry persists the empty workspace before closing, while Quit without saving warns that previously saved tabs may return

### Requirement: Preserve unavailable tabs

A directory or shell launch failure during restoration SHALL retain the affected tab's position and intended directory as a visible failed entry. The application SHALL offer retry, directory replacement, and explicit closure without affecting successfully restored tabs. It SHALL NOT silently substitute the home directory.

#### Scenario: Directory was removed

- **WHEN** a saved directory cannot be opened
- **THEN** its tab displays an actionable error and remains in subsequent snapshots until explicitly closed or repaired

#### Scenario: Retry after a drive becomes available

- **WHEN** the user retries a failed tab after its directory becomes available
- **THEN** a fresh shell starts in that directory in the same tab position

#### Scenario: All restored tabs fail

- **WHEN** no restored tab can launch its shell
- **THEN** all session names, tab positions, and selections remain available with recovery actions
- **THEN** focus, rename, close, and session navigation do not access a nonexistent terminal

#### Scenario: Cancel directory replacement

- **WHEN** the user cancels choosing a replacement directory
- **THEN** the intended directory and snapshot remain unchanged

### Requirement: Protect persistent state

The application SHALL validate the complete workspace document before launching restored terminals. Malformed, unsupported, or unreadable state SHALL be reported without overwriting it. An incomplete restoration SHALL NOT replace the saved workspace. Only one ordinary application instance SHALL own writes to a given workspace file at a time; a competing ordinary instance SHALL report the ownership conflict and stop before restoring shells.

#### Scenario: Invalid snapshot

- **WHEN** a snapshot has invalid selection indices, an empty session, unknown fields, or an unsupported format
- **THEN** ordinary startup fails with an owner-qualified error and leaves the file unchanged

#### Scenario: Restoration is interrupted

- **WHEN** startup is interrupted after only some saved tabs have been constructed
- **THEN** the original full snapshot remains available for the next startup

#### Scenario: Shell exits during restoration

- **WHEN** a newly started shell exits before all saved entries have been constructed
- **THEN** the event is applied only after the full restore structure exists
- **THEN** no snapshot containing only the constructed prefix is saved

#### Scenario: Another instance owns the snapshot (P1 approval required)

- **WHEN** an ordinary second instance uses the same workspace storage while the first owns it
- **THEN** it reports the conflict without spawning restored shells or changing the snapshot

#### Scenario: Previous owner crashed

- **WHEN** the owning process has terminated abnormally
- **THEN** a later ordinary startup can acquire ownership without manual deletion of a stale lock marker

### Requirement: Isolate explicit launches

Explicit program-launch entry points used by examples and capture tools SHALL retain their requested launch behavior and SHALL neither restore nor overwrite the ordinary persisted workspace.

#### Scenario: Screenshot example launches a program

- **WHEN** an example opens a terminal for a supplied program
- **THEN** it launches that program independently and leaves `workspace.json` unchanged
