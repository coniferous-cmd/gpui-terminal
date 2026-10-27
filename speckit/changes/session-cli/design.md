---
create-time: 2026-10-27 21:18:25
---

# Design

## Context

`src/main.rs` always initializes GPUI and calls `app::open_saved`. `Workspace` owns live sessions, each containing independent tabs. Session IDs currently derive from the first terminal's GPUI entity, and `SessionStore` holds restoration snapshots rather than an authoritative live registry. GUI creation currently logs errors instead of returning them to an external caller. `workspace/closing.rs` asks for confirmation before closing groups with multiple tabs. Windows release builds use the GUI subsystem.

The existing `restore-workspace` change is separate. CLI operations must reuse the current session/tab model and layout persistence, not implement another restore mechanism.

## Decisions

### Entry-point separation

Parse arguments before constructing a platform application. Use a typed subcommand parser (prefer clap derive, subject to the pinned toolchain and dependency review). Help/version/usage and IPC-client execution terminate without initializing fonts, settings, pet windows, updates, or GPUI. No arguments retain normal GUI startup.

On Windows, explicitly preserve redirected handles and attach to the invoking console for CLI output where needed, without allocating a console for no-argument GUI launch. Verify synchronous shell invocation and exit status against the release build; if the GUI subsystem prevents the required shell behavior, resolve packaging with a console launcher during implementation rather than claiming the debug build proves correctness.

### Runtime identity and discovery

Assign each GUI process a random instance nonce, each terminal window an instance-scoped ID, and each session an instance-scoped monotonic ID independent of its tabs. Do not expose entity IDs as durable public identifiers. Do not save CLI IDs across restarts. A session's ID stays when its first tab closes or tabs reorder.

Publish per-instance endpoints in a user-private runtime registry. Preserve existing independent GUI launches rather than enforcing a new single-instance policy. List queries every live instance; ID-addressed operations route to the owning instance. An unresponsive instance is an error, not an excuse to silently return a complete-looking partial list. Remove stale registry entries only after validating ownership and confirming the endpoint is not live; never unlink an arbitrary discovered path.

Creation accepts optional `--window`. Without it, exactly one terminal window is required across live instances. Multiple windows return an explicit ambiguity error. This avoids relying on an unreliable cross-process notion of the last active window. Record these targeting rules as implementation defaults for review.

### Transport and authorization

Use Unix-domain sockets on macOS/Linux, with an owner-only parent directory and socket, and Windows named pipes with an explicit current-user security descriptor and remote clients rejected. Validate discovery metadata ownership and reject unsafe permissions or symlinks. Authentication by a shared public pipe name alone is insufficient.

Use one length-prefixed JSON request and response per connection, followed by a client acknowledgment byte (`1`) before server teardown, protocol revision 1, strict operation DTOs, and a 1 MiB frame cap. Set finite deadlines: 2 seconds for connection, 10 seconds for ordinary responses, and 30 seconds for cold-start readiness. Validate inputs before dispatch. Respond with a tagged success or structured error; never include logs in the response payload. Do not retry a request once it may have reached the GUI.

Represent an explicit working directory losslessly over IPC using a platform-tagged native path encoding: Unix bytes or Windows UTF-16 units. Resolve relative paths in the CLI process. Listing may render paths lossily for display but must escape terminal controls in human output and use proper JSON serialization.

### GUI-thread command service

Keep blocking connection handling off the GUI thread. Bridge requests to the GPUI app executor and return results through a one-shot channel. Only the GUI thread reads or mutates workspace entities. Resolve IDs again immediately before executing a command so a session closed while a request waited cannot redirect the operation.

Factor reusable result-returning create/rename/focus/close methods from existing GUI actions. They must preserve status subscriptions, selection cleanup, title updates, notifications, state logging, and `Workspace::note` persistence. Existing GUI actions may log returned errors, but CLI callers receive them explicitly. An error during spawn must not insert an empty session.

A new session contains one configured shell with normal integration. An omitted cwd follows Start in for the selected target window. Creation selects the session within that window without requiring OS foreground activation; `focus` explicitly requests window activation. Rename affects only the session custom name, using existing trimming and empty-name reset rules.

### Closing and shutdown ordering

Check tab count on the GUI thread when executing close. Return a confirmation-required error for multiple tabs without `--force`, without opening the GUI dialog. Forced closure uses the normal whole-session close path. GUI close actions retain their existing confirmation behavior.

Arrange last-window shutdown so the close response can be flushed after successful removal and before the service is dropped. Do not acknowledge a request merely because it was queued. Use bounded shutdown coordination before calling `App::quit`; GPUI gives quit hooks only 200 ms, so a quit hook alone cannot drain replies. Await response acknowledgments asynchronously before requesting last-window application exit, and do not block the GUI thread waiting for socket writes. Test last-session closure explicitly.

### Cold start

Only create may launch the GUI when none is discoverable and no explicit window was specified. Serialize bootstrap using a user-scoped OS-backed lock; recheck discovery after obtaining it. Launch the current executable with its ordinary GUI startup behavior, wait for readiness while monitoring process exit, release the bootstrap lock after discovery is usable, and send each caller's create request once. Endpoint readiness means restoration has completed, not merely that a socket was bound.

Cold start intentionally preserves restored sessions and adds the requested session afterward; it does not clear saved layouts. If restoration yields an ambiguous target, return the same window-selection error. Startup, permissions, display, or shell failures return named errors. A lost response means an unknown operation outcome, not automatic recreation.

## Risks / Trade-offs

- Same-user IPC is not a sandbox against malicious programs running as that user. This scope deliberately excludes sending arbitrary input and reading terminal contents.
- Registry, transport, and native path encoding add cross-platform complexity; isolate transport modules and test without GPUI wherever possible.
- A GUI subsystem binary may not satisfy all Windows shell and redirection expectations; release-mode command-shell tests are a required gate, not an optional smoke test.
- Focus activation is compositor-controlled on Wayland; success means selection and activation request, not a guarantee of foreground access.
- Existing restore work may change workspace APIs; keep this change independent and run restore regression tests after integrating shared lifecycle helpers.
- No stable IDs across restart, no tab subcommands, no arbitrary executable option, and no terminal content or key injection in this iteration.
