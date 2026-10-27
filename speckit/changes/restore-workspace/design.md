---
create-time: 2026-10-23 17:29:35
---

# Design

## Context

`src/main.rs::run` currently builds a launch in home and passes it to `app::open`. That function spawns one PTY before constructing `Workspace`. The working tree already groups multiple tabs under each session in `src/workspace/tabs.rs`. Tabs currently require a live terminal entity; selection, status, notification routing, and rendering assume that entity exists.

`src/workspace/events.rs` receives directory reports and closes tabs when their programs exit. This is suitable for ordinary shell exit, but shutdown must not interpret teardown as deliberate workspace removal. `src/settings/store.rs` demonstrates strict loading and temporary-file replacement. Workspace writes must have their own owner and lifecycle rather than modifying SettingsStore on each terminal event.

This change restores structure only. No terminal grid import, byte-stream replay, process reattachment, transcript storage, or multiplexer integration is needed.

## Decisions

### Persist a small independent snapshot

Store `workspace.json` next to `settings.json`, with a format discriminator, optional active-session index, and an ordered sessions array. Each session has its explicit name, active-tab index, and ordered tabs containing the last known directory. An empty workspace has no active index; every nonempty session has at least one tab and an in-range selection.

Use strict serde parsing, reject unknown fields and unsupported formats, and validate the whole document before launching any restored shell. Separate shape validation from directory availability: a missing directory is a recoverable tab failure, not a corrupt document. Preserve platform-native path data losslessly, using a tagged native representation if ordinary JSON path serialization cannot represent it; never convert paths lossily. Log read, validation, save, restore, and recovery failures with workspace ownership and path context.

Do not persist runtime entity IDs, terminal titles, marks, bells, process arguments, environment variables, or program state. New shells use the current Shell setting. No preference toggle or session-history archive is introduced in this change.

### Separate ordinary startup from explicit launches

Add an ordinary restore-aware opening path used by `main.rs`. Keep `app::open(Launch, ...)` explicit and nonpersistent for examples and program capture callers. Shared window construction can be factored without changing those callers' behavior.

Ordinary startup acquires snapshot ownership, loads and validates state, constructs all session/tab entries, then restores selection and focus. A missing file or valid empty snapshot requests the existing default home shell. All restored entries, including failed entries, must exist before persistence becomes active. Buffer directory and exit events arriving during construction, then apply them after the structure is complete; never let an early shell event save a partially built workspace.

### Model restored launch failures explicitly

Represent a tab as a running terminal or a failed launch with intended directory and contextual error. Keep stable in-memory tab identity independent of the terminal entity so retry does not change its slot. Existing event routing can map a live entity back to that tab.

A failed tab shows an Ely-styled error view with Retry, Choose directory, and Close actions. Retry uses the current configured shell and intended directory. Choosing a replacement directory updates the intended directory and attempts launch. Neither failure nor retry deletes neighboring tabs or changes their order. Failed tabs have no terminal focus, terminal input handler, or fabricated process status; session selection and tab-close controls remain functional even when every tab failed.

Audit session rendering, title handling, rename, Git/Files behavior, notification handling, orb/pet status, and terminal actions for assumptions of a live terminal. Failed tabs show their intended directory and error rather than another tab's stale panel data. This is a typed state, not a dummy terminal or silent fallback.

### Centralize mutation snapshots and ordered writes

Use a workspace persistence module responsible for validation, loading, locking, and atomic saving, with a lifecycle coordinator for UI integration. At each committed workspace mutation, derive one complete immutable snapshot. Deduplicate equal snapshots.

Structural mutations request an immediate queued save. Selection and directory updates coalesce over a bounded 500 ms window measured from the first pending change, not reset indefinitely by later events. One serialized writer commits increasing revisions; while a write runs, retain only the newest pending revision. No filesystem write should block terminal event rendering during ordinary use.

Write a unique same-directory temporary file, flush it, then atomically replace the destination. Verify replacement behavior on Windows as well as Unix and use the required platform primitive where necessary. Failed writes retain the prior snapshot and the dirty revision. Show a persistent error with retry and clear it only when a newer snapshot commits successfully. Do not promise recovery of an uncommitted revision after power loss.

### Make shutdown a lifecycle transition

Use explicit phases:

```text
Restoring -> Running -> ShuttingDown -> Closed
                  \-> save failure (dirty state retained)
```

The ordinary quit command and main-window close request both freeze the current snapshot and enter ShuttingDown before any PTYs are torn down. Drain writes and commit the frozen final revision before allowing normal close. During this phase, teardown events cannot mutate persistent structure. A failed final save must keep the window open with Retry and Quit without saving choices; the latter explicitly accepts restoration from the last committed snapshot. Keep mutation controls disabled while that decision is pending; cancelling shutdown and returning to use is outside this change.

Explicit tab closure and normal shell exit in Running still remove tabs. If the last tab disappears, commit the empty snapshot through the same shutdown coordinator before the existing automatic window close. Session group confirmation continues to apply before removal.

Verified against installed `gpui-pre-0.3.8`: `Window::on_window_should_close` (`src/window.rs:6732`) accepts `Fn(&mut Window, &mut App) -> bool`; returning false vetoes the platform close. Register once for the ordinary workspace, queue the coordinator request, and return false while saving. After success, explicitly remove the window. `App::on_app_quit` (`src/app.rs:2596`) returns a subscription and accepts a future, but explicitly cannot cancel quitting. It is only a best-effort final flush for external quit, never the place to implement Retry UI. Retain/detach subscriptions appropriately. These are source-verified API facts, not cross-platform runtime test results.

Route `src/menus.rs`'s Quit action through the ordinary workspace coordinator instead of calling `cx.quit()` first; locate the ordinary workspace even when Settings has focus. `src/workspace.rs::close` currently calls `window.remove_window()` immediately for the last session: replace that route with empty-snapshot flush for persistent workspaces only. `src/app.rs`'s on-window-closed observer runs too late to save the removed workspace; keep its final app cleanup only after the coordinator allowed closure. Nonpersistent launches retain their old closure behavior. Forced OS termination is abnormal exit.

### Own the file for the lifetime of the ordinary instance

Hold an OS-managed exclusive lock on a dedicated adjacent lock file from before reading until shutdown finishes. Use the pinned Rust toolchain's file-lock APIs if supported on all targets; confirm API behavior before selecting an alternative. The lock file is not deleted to signal release, avoiding inode replacement races. OS handle release makes crashes recoverable without stale PID cleanup.

The proposed conflict policy is to refuse a second ordinary instance using the same configuration directory with an explicit error before it starts shells. No merge, last-writer-wins, or IPC forwarding is included. Explicit nonpersistent launches need no workspace lock. P1 is OPEN: this policy has not been approved by the owner and tightens concurrent-instance behavior. Stop before wiring the conflict branch until the owner confirms it. Do not infer approval from this design or invent IPC forwarding.

### Concrete implementation map

The following module names and contracts are proposed implementation targets, not APIs claimed to already exist. Keep hand-written files under 500 lines. Do not put filesystem logic in render methods.

| Target | Responsibility | Must not own |
| --- | --- | --- |
| `src/workspace/state.rs` | DTOs, strict validation, native path encoding, load/save, lock owner; split storage into `state/store.rs` if needed | GPUI entities or terminal handles |
| `src/workspace/persistence.rs` | Pure revision/scheduling state and one writer; snapshot requests and completion results | Reconstructing sessions or replaying output |
| `src/workspace/restore.rs` | Convert a validated snapshot into all session/tab slots, launch shells, finalize selection | Saving a partial restore |
| `src/workspace/recovery.rs` | Failed-tab view and retry/directory actions | Silently replacing a path |
| `src/workspace/shutdown.rs` | Close request, frozen final revision, flush completion and failure UI | Direct unsynchronized file writes |
| `src/sessions/model.rs` | Stable runtime IDs and running/failed tab state | Persistent file IDs or paths to storage |

Model contract:

- `WorkspaceSnapshot`: `format: u32` (initial value 1), `active: Option<usize>`, `sessions: Vec<SessionSnapshot>`.
- `SessionSnapshot`: `name: String` (empty means no custom name), `active: usize`, `tabs: Vec<TabSnapshot>`.
- `TabSnapshot`: `directory: SavedPath`; require an absolute native path. Existence is checked only during launch.
- `SavedPath`: tagged `Utf8(String)`, `UnixBytes(Vec<u8>)`, or `WindowsWide(Vec<u16>)`. Prefer UTF-8 when lossless; reject foreign native encodings. Decode to `PathBuf` without replacement characters. Directory existence is not schema validation.
- Empty snapshot: `active = None`, `sessions = []`. Nonempty snapshot: active index present and in range. Every session has a nonempty tabs list and a valid active index. Apply strict unknown-field rejection to every DTO.
- Use fresh monotonic in-memory `SessionId` and `TabId` newtypes, not `EntityId` or persisted IDs. Session ID currently comes from its first terminal; this must change too, otherwise an all-failed session cannot exist. Convert UI element keys to an accepted numeric ID representation explicitly.
- Keep title, folder, reports, bell, and waiting on `Tab`. Move terminal entity AND subscription into `TabRuntime::Running`; `TabRuntime::Failed` owns the launch error. Never use independent optional terminal/subscription fields, which permit inconsistent combinations.
- Pure storage contracts: `load(path) -> Result<Option<WorkspaceSnapshot>>`, `validate(snapshot) -> Result<()>`, and `save(path, snapshot) -> Result<()>`. `None` means only NotFound, never permission failure or malformed data.
- Snapshot extraction reads intended directories from failed tabs as well as live tabs; it never serializes runtime state.

Verified live-terminal consumers to migrate (re-run the search after edits):

| Location | Required handling |
| --- | --- |
| `sessions/model.rs::looks` | Failed tab returns no fabricated program record; recovery error belongs to its view |
| `sessions.rs::mark` | Tooltip weak handles include running tabs only |
| `workspace.rs::select` | Release old live focus if present; focus either new live terminal or workspace recovery controls |
| `workspace.rs` render | Render live entity or failed view; guard empty sessions during final-save failure |
| `workspace/tabs.rs` | Stable IDs in tab bar/reorder/close; map terminal EntityId only for running event sources |
| `workspace/events.rs` | Ignore events from removed/replaced entities; notification presses inspect running tabs only |
| `workspace/rename.rs` | On commit/cancel return focus safely even when the selected tab failed |
| `workspace/closing.rs` | Session confirmation uses stable session identity, not the first terminal entity |

### Writer and lifecycle invariants

Maintain one in-flight revision, one latest pending snapshot, a committed revision, and an optional frozen shutdown revision. Revisions increase monotonically in one process; they are not required on disk. A success advances committed only to that job's revision; newer pending work remains pending. A failure preserves pending/frozen state and reports the error. Never spawn an independent saver per event.

| Phase/event | Persistent effect | UI/next phase |
| --- | --- | --- |
| Restoring / terminal report | Queue report; no snapshot write | Stay Restoring |
| Restoring / all entries built | Apply queued reports in arrival order, then take full snapshot | Running |
| Running / mutation | Request immediate or bounded-coalesced full snapshot | Running |
| Running / close-window or Quit | Freeze current full snapshot; enqueue final revision | ShuttingDown |
| Running / remove last tab | Remove first; freeze empty snapshot | ShuttingDown |
| ShuttingDown / repeated close or terminal event | Do not change frozen snapshot or start a second writer | Stay ShuttingDown |
| ShuttingDown / final revision committed | Allow window removal once | Closed |
| ShuttingDown / save failed | Retain frozen revision; no terminal teardown | Show Retry / Quit without saving |
| External uncancellable quit | Attempt ordered flush; log failure | Cannot veto exit |

Disable workspace mutation/input while final-save UI is pending; Retry resubmits the frozen snapshot. Do not add a Cancel-close path in this change. This avoids having to reconcile mutations against a frozen snapshot. Keep the empty-workspace failure view independent of `sessions[active]`.

### Execution rules and stop conditions

Follow tasks serially. After each task, report changed files, test command/results, and unresolved issues. Do not mark a task complete because code compiles if its behavior test was not run. Build failures caused by unavailable tooling must be distinguished from code failures.

Stop for owner/reviewer input if P1 remains open, a dependency lacks a documented hook, safe final flush cannot be achieved, native path handling is incompatible with launch, or a task would change the specified behavior. Do not add a dependency or patch GPUI/Ely to work around the issue without review. Do not claim OS quit can be cancelled through `on_app_quit`.

## Risks / Trade-offs

- Failed tabs broaden the session model. Typed live/failed state and focused UI tests are required; unchecked live-terminal assumptions would cause crashes on restoration.
- GPUI close interception and uncancellable quit signatures were verified from installed source. Runtime behavior, including asynchronous flush ordering on each platform, still requires testing; source inspection is not a test result.
- A crash may lose the newest pending change; bounded coalescing limits that window without forcing a disk write for every focus event.
- Atomic replacement does not by itself guarantee survival of every storage-controller or filesystem power failure. Test process-interruption safety and avoid stronger durability claims.
- Directory reports describe the last known directory, not arbitrary foreground-process state. Herdr's last reported focused directory may be saved, but its panes are not recreated.
- A resumed shell executes its normal startup configuration. The application does not replay commands, but cannot prevent user-owned shell initialization scripts from doing work.
- Many saved tabs start many shells. Preserve all entries and avoid an arbitrary truncation policy; assess UI responsiveness with a representative large workspace.
- Native path encoding and Windows atomic replacement require cross-platform tests. Do not silently lose filenames that JSON strings cannot represent.
- Invalid snapshots stop ordinary startup and are left intact, consistent with strict settings loading. Automatic repair and backup-history management are outside this change.
- The single-owner conflict policy and failed-tab recovery UI should be reviewed before implementation; they favor preventing state loss over silently continuing.
