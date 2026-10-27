---
create-time: 2026-10-23 17:29:02
---

# Proposal

## Why

Restarting ELYTerminal currently opens a fresh shell instead of the user's previous workspace. Users must recreate named sessions, their tabs, working directories, and selection manually. Remembering this structure restores the workspace without pretending that old processes survived.

## What Changes

- Automatically persist session names and order, tab order and last known working directories, and selected session and tab positions.
- Restore the structure on ordinary application startup, launching a fresh shell from the current Shell setting in each saved directory.
- Keep workspace state in `workspace.json` beside `settings.json`, separate from user preferences.
- Save changes during use and flush before normal application shutdown; distinguish removing a tab from shutting down the application.
- Preserve failed-to-restore tabs with visible recovery actions instead of silently dropping them or substituting a directory.
- Protect the snapshot from partial writes, stale asynchronous saves, incomplete restoration, and competing application instances.
- Do not persist terminal output, dynamic program titles, process IDs, environment changes, commands, or nested multiplexer state. Do not reconnect or replay programs.

## Execution Boundary

- This plan targets a model that needs explicit, sequential instructions. Follow `tasks.md` in order; each task has a narrow scope and an observable completion gate.
- The confirmed scope is workspace structure only. Do not add transcript storage, process survival, command replay, preferences, IPC, or a general session-management framework.
- Product decision P1 is OPEN: refusing a second ordinary instance is proposed, not owner-approved. Ask the owner before implementing that behavior; do not treat completed planning artifacts as product approval.
- Storage ownership must prevent concurrent overwrites regardless of the eventual P1 choice. If P1 changes, reconcile specs, design, and tasks before coding that branch.
- Stop and report an incompatible API or failing prerequisite. Do not weaken a requirement, skip a failed tab, silently use home, or mark unrun tests as passed.

## Impact

- Affected specs: new `workspace-restoration` capability.
- Affected code: `src/main.rs`, `src/app.rs`, `src/workspace.rs`, `src/workspace/{tabs,events,rename,closing}.rs`, session models and views, application quit handling, and a new workspace persistence module.
- Existing explicit-launch callers, including examples, must remain isolated from automatic workspace restoration and snapshot writes.
- Existing installations without a workspace file retain the default single-terminal startup, then begin recording their workspace.
- Runtime failures need visible UI in addition to owner-qualified logs. Persistence adds local directory and session-name metadata, but no terminal transcript.
