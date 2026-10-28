---
create-time: 2026-10-28 22:08:34
---

## ADDED Requirements

### Requirement: A session retains its creation directory

Each session SHALL retain a launch cwd resolved once at creation. An explicit `session create --cwd` SHALL supply that cwd using the existing CLI path-resolution rules. Without an explicit cwd, creation SHALL retain the directory selected by the existing new-session policy. The initial tab SHALL launch in that directory.

#### Scenario: Explicit CLI directory

- **WHEN** `session create --cwd /projects/luna` succeeds
- **THEN** the session retains `/projects/luna` as its launch cwd and its first tab starts there.

#### Scenario: Omitted directory

- **WHEN** a session is created without an explicit cwd and the existing Start in policy resolves to directory P
- **THEN** P is retained as that session's launch cwd, regardless of later setting changes.

### Requirement: New tabs use their owning session's launch cwd

Every newly added tab SHALL start in its owning session's retained launch cwd, regardless of the selected tab's current folder or the current Start in setting. Directory reports, tab selection, tab reordering, renaming, and closing the original tab SHALL NOT change that retained cwd. Existing tabs SHALL NOT be moved when another tab is created.

#### Scenario: Shell cd does not redirect new tabs

- **WHEN** a session created with cwd `/projects/luna` has its selected tab report `/tmp` and a new tab is added
- **THEN** the new tab starts in `/projects/luna` and the existing tab remains in `/tmp`.

#### Scenario: Home setting does not override a session

- **WHEN** Start in is Home and a new tab is added to a session whose launch cwd is `/projects/luna`
- **THEN** the tab starts in `/projects/luna`, not home.

#### Scenario: Ownership survives tab changes

- **WHEN** the original tab has been closed and other tabs reordered or selected before a new tab is added
- **THEN** the new tab still uses the owning session's retained launch cwd.

#### Scenario: Sessions have independent directories

- **WHEN** sessions A and B retain different launch directories and each receives a new tab
- **THEN** each new tab uses its own session's launch directory, not the other session's directory.

### Requirement: Launch failures do not substitute another directory

If a session's retained cwd is missing, is not a directory, or cannot be entered, new-tab creation SHALL fail with a diagnostic naming the directory and operation. It SHALL NOT launch in home, an ancestor, or an existing tab's folder, and SHALL leave session cwd, tab count, and selection unchanged.

#### Scenario: Directory deleted after session creation

- **WHEN** a session's retained launch directory is deleted and a new tab is requested
- **THEN** creation fails, the failure is logged, and no tab is added or selected.

### Requirement: Live folder consumers remain tab-based

Tab directory reports SHALL continue updating each tab's live folder. Files, Git, the sidebar directory description, and `session list`'s existing cwd field SHALL retain their current active-tab-folder semantics. This change SHALL NOT introduce a command for editing session launch cwd.

#### Scenario: A tab reports a different directory

- **WHEN** the selected tab reports `/tmp` in a session whose launch cwd is `/projects/luna`
- **THEN** live folder consumers show `/tmp` while new tabs still start in `/projects/luna`.
