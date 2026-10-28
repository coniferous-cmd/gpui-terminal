---
create-time: 2026-10-28 22:08:34
---

# Design

## Context

`Workspace::create_terminal` in `src/workspace/tabs.rs` selects a folder before distinguishing a new session from a new tab. Without an explicit folder it consults Start in, optionally reading `session.current().folder`. `Session` has no directory field. CLI creation supplies a folder to this shared path, so the value survives only in the initial tab's mutable folder.

Persistence currently uses `SavedSession` and `SavedTab` in `src/sessions/state.rs`, captured by `src/workspace/save.rs`. `Opening` carries restored tabs into `Workspace::new`. Each restored tab uses its own saved folder with the existing ancestor/home recovery policy. Invalid snapshots are logged and dropped, not fatal. The owner explicitly chose to reject old layouts rather than migrate them.

## Decisions

### Separate session launch cwd from live tab folder

Add `cwd: PathBuf` to `Session` and `Opening`, and a required `cwd: String` to `SavedSession`. The runtime field means the directory resolved when the session was created, not the selected tab's current folder. It has no setter in this change. `Tab.folder` and all directory-report handlers retain their existing behavior.

```text
CLI --cwd /project ──> resolve new-session directory ──> Session.cwd
                                                         │
                                             every new tab launch
                                                         │
                                                         v
                                                  Tab.folder
                                                         │
                                                  shell cd reports
                                                         │
                                                         v
                                            live panels / tab snapshot
```

Keep existing CLI list `cwd`, sidebar folder text, Files and Git tied to the selected tab. This avoids an unrelated CLI output change. Documentation must distinguish session launch cwd from a tab's reported cwd.

### Resolve directory policy at the correct lifetime

For new sessions, keep explicit folder precedence and existing Start in behavior. Resolve once, validate and spawn the initial terminal, then insert the session with that exact resolved directory. Store the directory before any shell report can overwrite the initial tab's live folder.

For new tabs, read only the owning session's cwd; do not consult Start in or another tab's folder. Existing shortcut and tab-bar add paths must reach this same policy. Keep the directory selection sufficiently isolated to unit-test new-session versus new-tab decisions without a GPUI window. Do not introduce a new override for individual new tabs.

Require session cwd explicitly when constructing restored sessions and startup openings. `app::open` uses its supplied launch cwd; the ordinary default session uses the resolved home launch directory. Update constructors and callers, including tests/examples, rather than deriving session cwd inside `add_session` from whichever tab happens to be first.

### Preserve strict launch failures

Retain directory validation and spawn-before-insertion ordering. Add directory and operation context to failures so new-tab errors identify the session launch path. Directory existence is not sufficient proof of access: preserve errors from actual shell/PTY creation and do not fall back on failure. Do not change selection, append a tab, or alter session cwd unless creation succeeds. No new error-dialog framework is required; use the existing logging/CLI error paths.

### Persist the two directory meanings independently

Snapshot capture stores `Session.cwd` plus every retained tab's current folder. Require `SavedSession.cwd` without serde defaults or optional migration fields. Validate nonempty rooted paths with the same platform-aware rooted-path rule already used for saved tab folders. Do not require directory existence at snapshot load or save: a removed directory is a future launch failure, not structural corruption.

Restore passes the saved cwd unchanged through `Opening` into `Session`. Existing tabs retain their own restoration policy, including the current ancestor/home fallback and failed-tab skipping. Those per-tab recoveries must never rewrite session cwd. A restored session may thus have running tabs even though its launch cwd is unavailable; adding a new tab then fails explicitly.

The existing snapshot is text-only. Preserve native runtime paths; never serialize cwd lossily. If a session cwd cannot be represented as text, omit that session from the snapshot with an owning-session diagnostic, consistent with existing exclusion of unserializable tab folders. Remap selected indices when excluding entries. Do not introduce a path encoding overhaul in this change.

### Reject old layouts without migration

A missing cwd on any saved session fails strict deserialization of the entire snapshot. Invalid cwd values fail parsing or structural checks. `SessionStore::init` uses its existing logged invalid-layout behavior and ordinary startup opens a home session. No cwd is inferred from the selected tab, first tab, Start in, or home for an old saved session. An empty snapshot remains valid because it has no session requiring the field.

Document the breaking layout change prominently. No snapshot-format version or compatibility shim is needed to distinguish an absent required field.

### Coordinate adjacent planning without expanding scope

The active `session-cli` plan already specifies active-tab cwd in list output; leave that contract intact. The active `restore-workspace` design proposes a different persistence model and failure policy than today's code and the main `session-restore` spec. This change targets the checked-in implementation and main spec, not those unimplemented proposals. If that work lands first, carry the same required session cwd through its replacement DTOs and resolve any conflicting invalid-layout policy before implementing; do not implement its larger recovery/writer redesign here.

## Risks / Trade-offs

- The user-approved incompatible schema discards old nonempty saved layouts at startup. Release notes and a regression test must make this intentional behavior visible.
- Session launch cwd and live folder can differ. Preserve existing displays and list semantics; explain the distinction in creation help/documentation rather than adding unrelated UI controls.
- Every opening path must initialize cwd. Explicit constructor parameters and compiling tests/examples expose missed paths.
- Snapshot tests with old fixtures will otherwise fail at the new missing field before reaching their intended validations. Update ordinary fixtures; retain explicit old-layout fixtures for rejection tests.
- `src/sessions/state.rs` is already near the 500-line limit. Split tests or storage helpers when needed rather than exceeding it.
- A removed or inaccessible session cwd deliberately blocks new tabs even when an existing tab has moved somewhere usable. This is the requested ownership rule, not a reason to silently select another directory.
