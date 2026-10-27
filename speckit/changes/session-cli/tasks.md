---
create-time: 2026-10-27 21:21:16
---

## 1. CLI contract and entry point

- [x] 1.1 Add typed parsing for session list/create/rename/focus/close, help/version, create name/cwd/window options, list JSON, and close force; test valid forms, empty rename, and rejected operands/options.
- [ ] 1.2 Dispatch CLI operations before GPUI construction; test help/version and malformed input on Linux without a display and verify no-argument startup remains unchanged.
- [x] 1.3 Implement stdout/stderr and exit-code contracts, including table escaping and JSON serialization; test redirected output and ensure logs never enter stdout.
- [ ] 1.4 Verify and implement Windows release console/redirected-handle behavior, synchronous command invocation, and exit codes; resolve any launcher packaging requirement and verify GUI startup opens no console.

## 2. Live identity and reusable operations

- [ ] 2.1 Introduce instance-scoped public window/session IDs independent of tabs; test stability after original-tab closure and reordering, duplicate names, and invalidation after restart.
- [ ] 2.2 Factor result-returning session creation with optional native cwd and name; verify shell integration, Start in defaults, relative cwd resolution, spawn errors, and no empty-session insertion.
- [ ] 2.3 Expose GUI-thread list/rename/focus operations through shared workspace helpers; test title reset, selected-tab retention, focus requests, and normal persistence notifications.
- [ ] 2.4 Expose close with atomic tab-count validation and explicit force; test single-tab closure, multi-tab rejection without dialogs, forced whole-group closure, and unchanged GUI confirmation.

## 3. Local protocol and transport

- [x] 3.1 Define strict revisioned request/response DTOs, structured errors, native path encoding, length framing, and frame limits; test malformed JSON, unknown operations/revisions, oversized/truncated frames, and non-UTF-8/native path round trips.
- [x] 3.2 Implement owner-only Unix socket transport and registry validation; test socket/directory permissions, unsafe paths/symlinks, stale discovery, and refusal of unauthorized peers where supported by the test environment.
- [ ] 3.3 Implement Windows named pipes with current-user ACLs and remote-client rejection; test successful same-user communication, access denial, pipe contention, and deadline cancellation.
- [ ] 3.4 Add finite connection/read/response deadlines and bounded request handling; test disconnected/slow peers and ensure uncertain mutations are reported without automatic retry.
- [ ] 3.5 Bridge transport workers to the GPUI executor with one-shot replies; verify workspace access occurs only on the GUI thread and closed IDs are revalidated at dispatch.

## 4. Discovery, startup, and lifecycle

- [ ] 4.1 Publish endpoints only when normal restore/startup is ready and clean them on exit; test safe stale-record handling without deleting unrelated paths.
- [ ] 4.2 Implement cross-instance listing and ID routing; test multiple processes/windows, explicit create window selection, ambiguity errors, unreachable-instance errors, and unknown/expired IDs.
- [ ] 4.3 Implement create-only automatic startup with an OS-backed bootstrap lock and readiness/child-exit monitoring; test concurrent creators, failed startup, and unavailable explicit windows.
- [ ] 4.4 Ensure cold creation retains restored sessions and adds each requested session once; verify no other subcommand launches a GUI or edits saved state directly.
- [ ] 4.5 Coordinate last-session shutdown and response flushing without blocking the GUI thread; test CLI success on final-session closure and bounded cleanup after client disconnect.

## 5. Verification and documentation

- [ ] 5.1 Add end-to-end coverage for list JSON, create/name/cwd, rename/reset, focus, guarded close, force close, and not-running failures against a live application.
- [ ] 5.2 Run transport and CLI tests on macOS, Linux, and Windows; record actual release-mode Windows shell/redirection checks and platform focus limitations rather than inferring them from unit tests.
- [ ] 5.3 Run existing session/tab/restore regression tests and `scripts/check.sh`; keep handwritten files within 500 lines and inspect the diff for unrelated changes.
- [ ] 5.4 Document commands, runtime-only IDs, multi-window selection, cold-start restoration, force-close risks, JSON fields, timeouts/uncertain outcomes, and Windows invocation requirements in the user-facing documentation.
