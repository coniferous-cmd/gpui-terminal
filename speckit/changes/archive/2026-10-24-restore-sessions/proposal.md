---
create-time: 2026-10-24 12:04:52
---

# Proposal

## Why

luna-terminal forgets its workspace when it quits. Every launch opens exactly one
session with one tab in the home folder, because `main.rs` builds a single
`Launch` and `Workspace::new` makes one tab from it. A user who works in four
sessions of three tabs each, each in its own folder, rebuilds that by hand at
every start.

`PRD.md` lists session restore under Deferred. The owner asked for it: keep the
sessions and tabs that were open, and the folder each tab was in, without
keeping any terminal history.

## What Changes

- A workspace snapshot is written to its own file beside `settings.json`:
  every session in column order with its name and selected tab, and every tab
  in tab-bar order with its folder. No scrollback, no screen content, no
  running program.
- The snapshot is written only when the layout changes: a session or tab opens
  or closes, tabs are reordered, or a tab reports a different folder. Switching
  a session or a tab writes nothing; the selection is written once at quit.
  Renaming a session writes once.
- Start restores the snapshot: one shell is spawned per saved tab in its saved
  folder, the saved session and tab are selected, and each session keeps its
  saved name. A folder that no longer exists falls back to its nearest existing
  ancestor, then home, logged. A tab whose program cannot start is skipped, and
  a snapshot that is corrupt, out of range or unreadable is dropped with a log
  rather than stopping the start.
- With no snapshot, or none that loads, the app opens the one home session it
  opens today.
- `PRD.md` moves session restore out of Deferred and states what is kept and
  what is not.

## Impact

- Affected specs: `session-restore` (new capability)
- Affected code:
  - `src/sessions/state.rs` (new): the snapshot, its checks, `SessionStore`,
    the file's atomic write.
  - `src/workspace/restore.rs` (new): snapshot to launches, and the model
    rebuilt from them.
  - `src/app.rs`: a start that restores, sharing the window code with
    `open`, and the quit-time flush.
  - `src/workspace.rs`, `workspace/tabs.rs`, `workspace/events.rs`,
    `workspace/rename.rs`, `workspace/closing.rs`: note each layout change.
  - `src/settings/mod.rs`: nothing in the settings file changes; the snapshot
    is not a setting.
  - `src/main.rs`: start from the snapshot.
  - `PRD.md`, `TASKS.md`, `tasks/acceptance.md`.
