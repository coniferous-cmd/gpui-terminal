---
create-time: 2026-10-28 22:04:28
---

# Proposal

## Why

`session create --cwd PATH` currently starts only the first tab in PATH. The session does not retain that directory, and subsequent tabs consult Start in and may start at home or in another tab's current directory. A session created for a directory should retain that launch context for every new tab.

## What Changes

- Keep a session-owned launch cwd, initialized from the resolved directory used to create the session, including an explicit CLI `--cwd`.
- Start every newly added tab in its owning session's launch cwd, independently of Start in and of shell directory reports from existing tabs.
- Keep each tab's live folder independent for terminal reports, Files/Git, and restoration.
- Persist and restore the session launch cwd separately from individual tab folders.
- Preserve existing new-session directory selection when `--cwd` is omitted; resolve it once when creating that session.
- Refuse a new tab when the retained launch directory cannot be entered; do not substitute home or an ancestor.

## Impact

- Affected specs: new `session-tab-cwd`; additive `session-restore` requirements.
- Affected code: `src/sessions/model.rs`, `src/workspace/tabs.rs`, `src/workspace/control.rs`, `src/workspace/restore.rs`, workspace snapshot capture, `src/sessions/state.rs`, startup opening constructors, and their tests/documentation.
- Coordinate with active `session-cli` and `restore-workspace` changes; their creation and restoration paths must carry the new field without changing CLI list output or per-tab restore policy.
- Breaking snapshot change: a saved session must contain its launch cwd. Old layouts missing it are rejected whole through the existing invalid-snapshot path, logged, and replaced at startup by a default session. No migration or inferred cwd is provided, as confirmed by the owner.
