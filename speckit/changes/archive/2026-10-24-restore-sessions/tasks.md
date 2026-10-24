---
create-time: 2026-10-24 12:06:19
---

## 1. Implementation

- [x] 1.1 Move the atomic write out of `src/settings/store.rs` into
  `src/atomic.rs`: one `write(path, text)` that makes a temp file beside the
  target, syncs it and renames it over the target, keeping the per-process
  counter. `settings::save` calls it. Verify: `cargo test` still passes the
  store's round-trip and overlapping-save tests, and `scripts/check.sh` passes.

- [x] 1.2 Add `src/sessions/state.rs` with the snapshot: `Snapshot { active,
  sessions }`, `SavedSession { name, active, tabs }`, `SavedTab { folder }`,
  `deny_unknown_fields`, `name` defaulting to empty, and `PathBuf` converted to
  and from text (a folder that is not UTF-8 leaves its tab out, logged). Add
  `load(path) -> Result<Option<Snapshot>>` (missing file is `Ok(None)`), and
  `save`. Checks: 64 sessions, 128 tabs a session, no empty session, `active`
  inside its range, every folder absolute and non-empty. Verify with unit tests:
  round trip, missing file, truncated file, unknown field, out-of-range
  `active`, empty session, over the limits, and one tab left out for a folder
  that is not UTF-8 (Unix only).

- [x] 1.3 Add `SessionStore` to `src/sessions/state.rs`: `init(cx)` loading the
  file at `dirs::config_dir()/luna-terminal/sessions.json` and logging a drop with
  its reason, `note(cx, snapshot, write)` comparing `current` with `saved`, and
  `flush(cx)`. A failed write logs and leaves `saved` behind, so the next
  `note(…, true)` tries again. Verify with unit tests: a selection-only note
  writes nothing, a layout note writes once, the same snapshot twice writes
  once, and an empty snapshot round-trips.

- [x] 1.4 Add `src/workspace/restore.rs`: turn a `Snapshot` into
  `Vec<Opening>`, one `Started` per tab, and a saved folder into one that is
  there by walking its ancestors, then home, logging what it took instead. A
  tab whose `launch` or `spawn` fails is skipped and logged; a session left
  empty is dropped; the saved selection is clamped. Verify with unit tests for
  the folder walk (an existing folder, a deleted leaf under an existing parent,
  a deleted path with no ancestor, a path outside any folder) and a manual run
  with a snapshot naming a deleted folder.

- [x] 1.5 Reshape the start: `Workspace::new(openings, active, window, cx)`
  building sessions, tabs and the first Git/Files folder from the openings;
  `app::open` keeping its signature for `examples/shot/main.rs` by wrapping one
  launch in one opening; a new `app::open_saved(cx)` that reads the store,
  restores, and falls back to one home launch when nothing survives. Verify:
  `cargo run` with no snapshot opens one home session as before; `shot` renders
  its program unchanged.

- [x] 1.6 Note every layout change: `new_terminal`, `close_tab`, `close` (before
  `remove_window`), the tab reorder handler, `end_rename` and
  `session_event`'s `Folder` arm with `write = true`; `select` and `select_tab`
  with `write = false`. Verify by hand on macOS with `HOME` pointed at a scratch
  folder: `cd` writes once and a second prompt does not; switching tabs and
  sessions writes nothing; opening, closing and reordering tabs each write once;
  renaming a session writes once; closing the last session leaves an empty
  snapshot.

- [x] 1.7 Flush on quit: `on_app_quit` in `app::start`, and the last-window
  `on_window_closed` observer flushing before `cx.quit()`. Verify: switch tabs,
  quit with Cmd-Q, and the file holds the tab that was shown; quit again without
  switching and the file's timestamp does not move.

- [x] 1.8 Switch `main.rs` to `app::open_saved`. Verify: two sessions of two and
  three tabs in four folders quit and come back with the same rows, the same tab
  bars in the same order, each shell in its own folder, the same selection, and
  each renamed session still named.

- [x] 1.9 Write the state down: `PRD.md` moves session restore out of Deferred
  into the sessions chapter, saying what is kept and what is not; `TASKS.md`
  gains the task; `tasks/acceptance.md` gains its row with what was run. Verify:
  every claim in the row names a test or a run.

## 2. Verification

Left open: 2.1 fails only its PowerShell test, since this Mac has no `pwsh` and
the Homebrew cask wants an interactive `sudo` (the fish test passes with the
`XDG_DATA_DIRS` Ghostty injects unset); 2.2 could not run because Docker Hub is
unreachable from this network, so no image could be built; 2.5 needs a push.
Fmt, clippy, 306 tests, the one-line comment rule and the 500-line rule all pass
here.

- [ ] 2.1 `scripts/check.sh` passes: fmt, clippy with `-D warnings`, the tests
  and the 500-line rule. If `src/workspace.rs` passes 500 lines, move
  `column_toggle` and `show_tab` to `src/workspace/chrome.rs` first.

- [ ] 2.2 `scripts/linux.sh test --locked` passes, so the state tests run on
  Linux as CI runs them.

- [x] 2.3 On macOS, run the app from a scratch `HOME`: build three sessions with
  tabs in different folders, quit, start again, and check every restored tab's
  folder with `pwd`; then delete one saved folder, start again, and check the
  log names the ancestor taken instead.

- [x] 2.4 Break the file on purpose, twice: truncate `sessions.json` and add an
  unknown field. Each start opens one home session and logs why, and neither
  stops the app.

- [ ] 2.5 CI green on macOS, Ubuntu and Windows.
