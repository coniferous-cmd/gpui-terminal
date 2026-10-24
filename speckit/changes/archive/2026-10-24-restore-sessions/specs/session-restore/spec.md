---
create-time: 2026-10-24 12:05:08
---

## ADDED Requirements

### Requirement: The workspace layout survives a restart

The app SHALL keep, across launches, the sessions it had open, the order they
stood in, the tabs each session held, the order those tabs stood in, the folder
each tab was in, the session and tab that were selected, and any name the owner
gave a session.

It SHALL NOT keep scrollback, screen content, a running program, a program's
title, a bell, a status mark, or whether a shell reports its folder.

#### Scenario: A user quits and starts again

- **WHEN** the app quits with three sessions, the second selected, holding two,
  three and one tabs in `/src/a`, `/src/b`, `/src/c`, and the app starts again
- **THEN** three sessions stand in the same order with the same tabs, each tab
  runs a new shell in its saved folder, and the second session is selected with
  the tab that was selected in it

#### Scenario: A session keeps its name

- **WHEN** a session was renamed to `api` before the app quit, and the app
  starts again
- **THEN** the session's row, the terminal's header and the window's title show
  `api` until it is renamed again

### Requirement: The snapshot is written only when the layout changes

The app SHALL write the snapshot when a session opens or closes, when a tab
opens or closes, when tabs are reordered, when a tab reports a different
folder, and when a session is renamed. It SHALL write nothing when a session or
tab is selected, when a program sets a title, when a bell rings, when a status
mark changes, or when a prompt reports the folder it already had. It SHALL
write once when the app quits, keeping the selection of that moment.

#### Scenario: A shell draws prompt after prompt

- **WHEN** a shell reports its folder at every prompt for an hour without the
  folder changing
- **THEN** not one write reaches the file

#### Scenario: A tab changes folder

- **WHEN** the user runs `cd /var/log` in a tab
- **THEN** the file's tab holds `/var/log` before the next prompt

#### Scenario: The user switches tabs

- **WHEN** the user selects another tab and then quits
- **THEN** nothing is written at the switch, and the file holds the selected tab
  once the app quits

### Requirement: The snapshot lives in its own file

The snapshot SHALL be stored in a file of its own beside `settings.json`, in the
platform's config folder under `ELYTerminal`, and SHALL be written whole through
a temporary file renamed over it. It SHALL NOT be a field of `settings.json`,
and no setting SHALL depend on it.

#### Scenario: The settings file is hand-edited

- **WHEN** the owner edits `settings.json` while the app runs
- **THEN** the snapshot file is untouched, and neither file depends on the
  other

### Requirement: A snapshot that cannot be used is dropped, never fatal

The app SHALL drop a snapshot that does not read, does not parse, holds a
session with no tab, names a selected session or tab outside its range, or
holds more sessions or tabs than the app allows. It SHALL log the reason and
open one session in the home folder. A snapshot SHALL NOT stop the app from
starting.

#### Scenario: The snapshot file is truncated

- **WHEN** the app starts and the snapshot file does not parse
- **THEN** the file is ignored, the log names why, and the window opens with one
  session in the home folder

#### Scenario: The snapshot was written by a later version

- **WHEN** the snapshot holds a field the app does not know, or a value outside
  its range
- **THEN** the whole snapshot is dropped, and the app starts as it does with no
  file

### Requirement: A restored tab starts in its saved folder

The app SHALL start each restored tab's shell in the folder the snapshot names.
When that folder is not there, it SHALL use the nearest ancestor that is, and
when none is, the home folder, logging which. A tab whose program fails to
start SHALL be skipped with a log, and the rest of the layout SHALL stand.

#### Scenario: A saved folder was deleted

- **WHEN** a tab's saved folder is `/src/gone/deep` and `/src/gone` is the
  nearest existing ancestor
- **THEN** the tab's shell starts in `/src/gone`, and the log names the folder
  that was taken instead

#### Scenario: A saved shell no longer runs

- **WHEN** every restored shell fails to start
- **THEN** the app opens one session in the home folder, or fails to start with
  the reason if that shell fails too

### Requirement: Closing every tab means nothing is restored

The app SHALL write a snapshot with no session when the last session closes,
and SHALL then start with one session in the home folder, as a first launch
does.

#### Scenario: The user closes the last session

- **WHEN** the user closes the last session and starts the app again
- **THEN** the window opens with one session in the home folder
