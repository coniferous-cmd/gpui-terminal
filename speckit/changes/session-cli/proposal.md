---
create-time: 2026-10-27 21:17:21
---

# Proposal

## Why

Users need to inspect and manage live luna-terminal sessions from scripts and other terminals without navigating the GUI. Startup arguments alone cannot manipulate the state owned by an already running GUI.

## What Changes

- Add `session list`, `session create`, `session rename`, `session focus`, and `session close` subcommands, plus help and version output without initializing GPUI.
- Treat a session as one left-sidebar entry, not an individual terminal tab; a newly created session starts with one tab.
- Return runtime session IDs, offer JSON listing, and target mutations by ID rather than potentially duplicate names.
- Close single-tab sessions directly; reject multi-tab closure unless `--force` is supplied, without opening a GUI confirmation dialog.
- Introduce current-user-only local IPC: Unix sockets on macOS/Linux and named pipes on Windows; execute mutations on the GUI thread.
- Automatically start the GUI for `session create` when absent; other session commands fail clearly when no GUI is running.
- Preserve ordinary no-argument startup and session restoration. Operate on live state rather than editing the saved layout from the CLI.
- Exclude tab management, terminal input/output control, arbitrary command execution, and appearance overrides from this change.

## Impact

- Affected specs: new `session-cli` capability; existing `session-restore` behavior must remain intact.
- Affected code: `src/main.rs`, `src/app.rs`, session models, workspace lifecycle/rename/closing, and new CLI/IPC modules and tests.
- Platform impact: endpoint ownership, instance discovery, startup races, and Windows GUI-subsystem command output require explicit cross-platform verification.
- Existing change `restore-workspace` remains separate; integrate with the session/tab model already present without duplicating its work.
