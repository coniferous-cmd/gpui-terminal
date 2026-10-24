---
create-time: 2026-10-24 12:05:24
---

# Design

## Context

`main.rs` builds one `Launch` in the home folder, `app::open` spawns it and
opens the window, and `Workspace::new(pty, events, launch, …)` makes the one
session and the one tab from it. `Workspace` holds `Vec<Session>` with
`Session { name, active, tabs: Vec<Tab> }`, and `Tab` holds the terminal entity,
the program's title, and the folder the shell last reported through OSC 7.

Three facts shape the design:

- Shell integration reports OSC 7 at every prompt, so a folder event arrives
  once per command even when nothing changed.
- `SettingsStore` already owns an atomic temp-file-and-rename save, and
  `settings.json` refuses unknown fields and stops the start on a bad value.
- `on_app_quit` handlers run synchronously when they are registered as futures,
  before `App::shutdown` clears the windows, so a quit callback can read state
  and write a file.

## Decisions

- **The snapshot is its own file.** `sessions.json` sits beside `settings.json`
  under `dirs::config_dir()/ELYTerminal/`. It is machine-written and changes for
  reasons settings do not: a bad snapshot must never stop the start, while a bad
  `settings.json` must. Session layout is also not a user-editable setting, and
  `settings.json` stays small.
- **The atomic write moves to `src/atomic.rs`.** One function, temp file beside
  the target then a rename, with the per-process counter. `settings::save` and
  the new store both call it.
- **The snapshot holds only the layout.** `Snapshot { active, sessions }`,
  `SavedSession { name, active, tabs }`, `SavedTab { folder }`. Every field but
  `name` is required; `name` defaults to empty so a snapshot written without one
  still reads. `deny_unknown_fields` everywhere, so a later version's field
  drops the file rather than half-reading it.
- **Limits are checked, not clamped.** 256 sessions and 2048 tabs per session; a
  session with no tab, a selected index outside its range, an empty folder, a
  folder from no root, or an unknown field drops the whole snapshot. Clamping
  would invent a layout nobody asked for. The limits bound a file that lies, not
  what the app may hold — the app caps nothing — so they sit far past what
  anyone opens by hand. The folder rule is rootedness, not absoluteness: on
  Windows a shell can report `/c/Users`, which runs but is no absolute path, and
  the file must not refuse what the app itself holds.
- **A folder is stored as text.** `PathBuf` does not serialize portably. A
  folder that is not UTF-8 leaves its tab out of the snapshot, logged; in
  practice folders arrive from OSC 7, which is UTF-8 by construction.
- **Writes follow a comparison, not an event.** `SessionStore` keeps `current`,
  `saved` and `loaded`: the layout the workspace has now, what the file holds,
  and what the file held at start for the window to restore. Every change calls
  `note(snapshot, write)`: `current` always follows the workspace, and the file
  is written only when `write` is set and `current != saved`. The folder event
  calls `note(…, true)`, so a prompt that reports the folder it already had
  writes nothing; a real `cd` writes once. Selection changes call
  `note(…, false)`: no write, but the in-memory snapshot follows, so the quit
  flush carries the selection.
- **The quit flush is one compare and one write.** `app::start` registers
  `on_app_quit` to flush, and the existing last-window `on_window_closed`
  observer flushes before `cx.quit()`. Both are compare-first, so the usual quit
  writes once, and a quit with no selection change writes nothing. Because
  `current` is always in memory, the handler never reads a window, which
  `on_window_closed` could not do anyway.
- **Restore spawns every tab before the window opens.** `app::open_saved` turns
  the snapshot into one `Launch` per tab, spawns them, and passes the started
  tabs to `Workspace::new`. Keeping the spawn before `open_window` preserves
  today's fail-fast path when no shell can start. A tab whose folder is gone
  walks its ancestors for the nearest one that is a folder, then home, logged.
  A tab whose program fails to start is skipped, logged. A session left with no
  tab is dropped; the saved selection is clamped into what survives. If nothing
  survives, one home launch is tried, and only its failure fails the start.
- **`Workspace::new` takes openings, not a pty.** `Opening { name, active,
  tabs: Vec<Started> }`, `Started { pty, events, launch }`. `app::open` keeps
  its signature for `examples/shot/main.rs` and wraps its one launch in one
  opening; `shot` therefore never restores, which is what it wants, since its
  `HOME` is a scratch folder.
- **Notes anchor at the mutation points.** `Workspace::note(write, cx)` builds
  the snapshot from the model and calls the store. It is called at the end of
  `new_terminal`, `close_tab`, `close`, the tab reorder handler, `select`,
  `end_rename`, and the `Folder` arm of `session_event`. `select_tab` needs no
  note of its own: it always ends in `select`. `close` notes before
  `window.remove_window()` so closing the last session writes the empty
  snapshot.
- **No setting turns it on or off.** The owner asked for it always; an empty
  snapshot is the way to opt out, by closing every session before quitting.

## Risks / Trade-offs

- **A restored shell that exits at once closes its tab, then its session, then
  the window.** This is today's behavior for a broken shell, and restore does
  not change it; skipping spawn failures covers the common case. If it bites,
  the log shows each close before the window goes.
- **A `cd` loop writes as fast as the loop turns.** Accepted: each write is a
  few hundred bytes, and the alternative, debouncing, would lose the last
  change on a force quit.
- **The selection is only as fresh as the last note.** If the user switches tabs
  and the process is killed, the file holds the previous selection. Accepted:
  the layout, folders included, is still right.
- **`src/workspace.rs` is 452 lines.** The changes add a few lines to `new` and
  to `select`; if the file passes 500, `column_toggle` and `show_tab` move to
  `src/workspace/chrome.rs` rather than the restore code being squeezed.
- **No history is kept.** Restored tabs show a fresh prompt in the right folder.
  This is the owner's stated scope, and the PRD's Deferred entry is corrected to
  say layout only.
- **A layout the app could not finish restoring is the layout that gets saved.**
  A tab whose shell would not start, and a folder that fell back to an ancestor,
  stand that way in the file after the next write. Keeping the intended folder
  beside the actual one would mean the file no longer describes the window.
- **A window opened without a snapshot keeps none.** `app::open` calls
  `SessionStore::forget`, so `shot` leaves the owner's `sessions.json` alone
  whatever its `HOME` is, though it still writes `settings.json` there.
- **Past the file's limits nothing is saved until the layout comes back under
  them.** 256 sessions and 2048 tabs a session are bounds on what the file
  reads; a layout over them is refused whole, logged, and the last good file
  stands.
