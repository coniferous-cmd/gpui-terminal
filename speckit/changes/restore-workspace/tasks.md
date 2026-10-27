---
create-time: 2026-10-23 17:32:47
---

# Execution Contract

Read proposal, spec, and design before editing code. Execute unchecked tasks in numerical order. Do not launch parallel implementations. Each task requires a file/change summary and actual verification results before checking it off. Existing checked tasks must not be repeated without a regression reason.

P1 is OPEN: the owner has not approved refusing a second ordinary instance. Task 1.1 is a blocking decision, not an invitation for the implementation model to choose. Planning completeness does not mean implementation approval.

The file names below are planned targets. If the repository has changed, inspect it and report mismatches before relocating responsibilities. Never silently substitute home, discard failed tabs, serialize terminal bytes, patch dependency code, or use `unwrap` to bypass runtime failure. Keep handwritten files at or below 500 lines.

For automated tests use these proposed integration targets: `tests/workspace_state.rs`, `tests/workspace_writer.rs`, and `tests/workspace_restore.rs`. Add narrow test seams for pure DTO, scheduler, and restore-plan logic; do not make GPUI entities serializable. UI behavior that cannot be covered automatically needs a recorded manual result.

## 1. Resolve gates and establish baseline

- [ ] 1.1 Resolve P1 with the owner before implementation.
  - Read: proposal Execution Boundary and spec Review Gate.
  - Output: record the owner's decision in those sections and the ownership section of design; keep all three consistent.
  - Gate: if no answer is available, STOP. Do not treat a request to implement the feature as approval of a particular second-instance behavior.
- [ ] 1.2 Verify the checkout and available tooling without changing application code.
  - Read: `src/main.rs`, `src/app.rs`, `src/menus.rs`, `src/workspace.rs`, `src/workspace/*.rs`, `src/sessions.rs`, `src/sessions/model.rs`, `scripts/check.sh`.
  - Run: `git status --short`, `cargo check --locked`, and inspect existing tests before choosing fixtures.
  - Gate: report pre-existing changes/errors separately. Never reset user changes or fix unrelated failures.
- [ ] 1.3 Confirm dependency API and platform assumptions against design.
  - Verify installed `gpui-pre-0.3.8` `on_window_should_close` and `on_app_quit`; the latter CANNOT veto quit.
  - Verify pinned-toolchain OS file locking and atomic replacement on the current platform. Record unavailable platforms as untested.
  - Gate: stop if assumptions differ; do not invent hooks, add dependencies, or change GPUI/Ely to continue.

## 2. Build pure storage first

- [ ] 2.1 Implement DTOs and validation in `src/workspace/state.rs` and register the module.
  - Contract: use the exact snapshot fields, format value, empty-state rules, and SavedPath tags in design. No GPUI types and no I/O yet.
  - Test: empty, two sessions with multiple tabs, invalid active indices, empty session, unknown nested field, unsupported format, and UTF-8/native path round trips.
  - Gate: `cargo test --locked --test workspace_state` passes; snapshot contains no process/output/environment fields.
- [ ] 2.2 Implement strict `load` separately from shell availability checks.
  - Modify: state storage module and `tests/workspace_state.rs` only, apart from required module exports.
  - Test: missing returns None; valid empty returns Some(empty); corrupt, trailing JSON, and unreadable files return contextual errors without changing bytes.
  - Gate: missing directory INSIDE a valid snapshot is not rejected as corrupt. Do not create a shell in this task.
- [ ] 2.3 Implement atomic `save` with injected write/replace failure coverage.
  - Behavior: same-directory unique temp, write and flush, replace target; clean up failed temp where possible and log cleanup failure.
  - Test: initial creation, replacement, and failure preserving exact old bytes. Test a destination that exists on Windows.
  - Gate: no delete-target-then-rename gap and no synchronous save in a UI render path.
- [ ] 2.4 Implement lifetime exclusive ownership on an adjacent lock file.
  - Precondition: P1 settled; use the reviewed primitive from 1.3.
  - Test with child processes: second ownership fails while held; normal release and killed owner permit acquisition; lock marker need not be deleted.
  - Gate: lock is acquired before reading or spawning restored shells. Do not wire ordinary startup yet.

## 3. Implement and test the writer without UI

- [ ] 3.1 Implement pure revision/scheduling state in `src/workspace/persistence.rs`.
  - Contract: one in-flight job, latest pending snapshot, committed revision; deduplicate equal requests.
  - Test: immediate structural requests; selection/directory request becomes due 500 ms after FIRST pending request; repeated requests do not postpone forever.
  - Gate: `cargo test --locked --test workspace_writer` passes using a controllable clock, not sleep-sensitive assertions.
- [ ] 3.2 Connect one serialized background writer to the atomic store.
  - Test: revision A in flight, B pending, C replaces B; completion of A does not clear C. Failure keeps latest dirty state. Retry commits latest state.
  - Gate: no independent write task per mutation; older completion never marks a newer revision committed.
- [ ] 3.3 Add freeze/final-flush transitions to the pure coordinator.
  - Test: repeated close freezes only once; ordinary events cannot change frozen data; success closes only after final revision; failure retains retry data; empty final snapshot is valid.
  - Gate: lifecycle tests pass before wiring any close hook or replacing `cx.quit()`.

## 4. Migrate runtime identity and failed tabs

- [ ] 4.1 Replace terminal-derived session/tab UI identities with stable runtime IDs.
  - Modify: `sessions/model.rs`, workspace tab bar, rename, and closing references listed in design. Keep all tabs live at this step.
  - Preserve: event routing still accepts live terminal EntityId and finds its owning TabId; IDs are never saved.
  - Gate: `cargo check --locked` passes; normal create/select/reorder/rename/close behavior remains unchanged. Do not add failure UI yet.
- [ ] 4.2 Introduce `TabRuntime::Running` and `TabRuntime::Failed` and migrate all consumers together.
  - Running owns BOTH terminal entity and subscription; failed owns a contextual error. Shared directory remains on Tab.
  - Use design's consumer table: model looks, tooltip handles, selection/focus, render, tab actions, event routing, rename focus, group close, and panels.
  - Search: `rg -n '\.terminal|EntityId|sessions\[|tabs\[' src/workspace.rs src/workspace src/sessions.rs src/sessions`.
  - Gate: inspect every match; `cargo check --locked` passes. No dummy terminal, unchecked unwrap, or independent optional subscription/entity.
- [ ] 4.3 Add failed-tab rendering and Retry in `src/workspace/recovery.rs`.
  - Behavior: display intended directory and error; retry uses CURRENT Shell setting; success replaces runtime in the same stable slot.
  - Test: mixed live/failed tabs, all failed, selected failed tab, rename completion, stale events from a replaced terminal.
  - Gate: keyboard navigation/close work without a live terminal; no fabricated terminal reports or stale neighboring Git/Files content.
- [ ] 4.4 Add Choose directory and Close to recovery UI.
  - Behavior: cancellation does nothing; confirmed replacement changes intended path and retries; a second failure remains visible in the same slot.
  - Gate: manual or UI tests verify cancel, success, repeated failure, and closure without dropping neighboring tabs.

## 5. Add ordinary restoration without changing explicit launches

- [ ] 5.1 Add restore-plan construction in `src/workspace/restore.rs`.
  - Input: fully validated snapshot. Output: complete ordered session/tab slots with fresh runtime IDs and saved selection.
  - Test: every saved slot exists even when launch fails; no early write; malformed data starts zero shells.
  - Gate: `cargo test --locked --test workspace_restore` passes. Use controlled launch failures in tests rather than depending on the user's shell.
- [ ] 5.2 Start shells and finalize the restore phase.
  - Contract: current Shell setting, saved last-known directory, no replay. Queue directory/exit events during restore and apply in arrival order only after all slots exist.
  - Test: interruption after first constructed tab leaves source bytes unchanged; early shell exit cannot save a prefix; all failures preserve selections.
  - Gate: persistence activates only after the complete structure is represented.
- [ ] 5.3 Wire restore-aware ordinary startup in `main.rs` and `app.rs`.
  - Ordinary startup owns lock/store, handles missing or empty snapshot with one default home shell, and reports invalid-file errors without overwrite.
  - Preserve `app::open(Launch, ...)` as a nonpersistent explicit launch; share window construction only as necessary.
  - Gate: compare snapshot bytes before/after an explicit example launch; they are identical. Do not use the user's real config for tests.

## 6. Save mutations and show save failures

- [ ] 6.1 Add one full snapshot extractor for live AND failed entries.
  - Test: session custom name, both ordering levels, both selection levels, and intended failed-tab path; dynamic title changes do not change snapshot.
  - Gate: snapshot round trip matches the structural input and contains no runtime IDs.
- [ ] 6.2 Hook structural mutations after they commit.
  - Sites: successful create, close, confirmed session-group close, committed rename, and reorder in workspace modules.
  - Test: rename Escape and close confirmation Cancel do not save unintended changes; closing a tab removes only it.
  - Gate: one logical mutation requests one complete snapshot, never a midway snapshot.
- [ ] 6.3 Hook selection and directory reports using bounded coalescing.
  - Sites: `workspace.rs::select`, `tabs.rs::select_tab`, `events.rs` Folder handling, recovery directory replacement.
  - Test: no integration retains launch directory; hidden-tab Folder updates its own entry; repeated identical values do not write.
  - Gate: continuous updates do not starve saving; foreground UI remains responsive during a slow write.
- [ ] 6.4 Add visible runtime save error and Retry.
  - Contract: retain dirty state; report storage path and failure; clear only after successful latest commit.
  - Gate: injected storage failure preserves old file and shows error; retry writes newest snapshot, not the failed older one.

## 7. Integrate shutdown last

- [ ] 7.1 Add the main-window close interceptor in `src/workspace/shutdown.rs`.
  - Use verified `on_window_should_close`: veto normal close, freeze final snapshot, disable mutation/input, wait for writer, explicitly remove only on success.
  - Test: repeated close plus incoming terminal exits cannot change the frozen snapshot or spawn a second writer.
  - Gate: no PTY teardown before final commit; nonpersistent windows retain existing close behavior.
- [ ] 7.2 Route application's Quit action through the coordinator.
  - Modify `src/menus.rs` and ordinary workspace ownership access; find workspace even if Settings is active.
  - Preserve `app.rs` post-close cleanup; it is NOT where the removed workspace is saved.
  - Gate: main-window button and Quit key/menu restore identical structures; no direct early `cx.quit()` remains in the owned Quit path.
- [ ] 7.3 Handle explicit last-tab removal separately.
  - Modify `Workspace::close`/tab-close route: remove entry first, freeze empty snapshot, then flush and close. Group confirmation stays intact.
  - Gate: after user closes last tab or shell exits normally, restart produces only one default terminal, not the deleted session set.
- [ ] 7.4 Add final-save failure UI, including an empty-workspace view.
  - Actions: Retry frozen revision; Quit without saving explicitly accepts old snapshot. No Cancel-close or workspace mutation while frozen.
  - Test: zero sessions render safely; repeated retries work; explicit unsaved quit warns saved tabs may return.
  - Gate: no unchecked `sessions[active]` on empty close-failure paths; no false success message.
- [ ] 7.5 Register best-effort external quit flushing through `on_app_quit`.
  - Contract: keep subscription alive, use the SAME ordered writer, log failure. This callback CANNOT veto exit or promise Retry UI.
  - Gate: no racing emergency write; record external OS quit and forced termination limitations honestly.

## 8. Acceptance and handoff

- [ ] 8.1 Run the automated targets and map each spec scenario to a test or manual result.
  - Commands: `cargo test --locked --test workspace_state`, `cargo test --locked --test workspace_writer`, `cargo test --locked --test workspace_restore`.
  - Gate: attach actual results; a skipped test is not a pass.
- [ ] 8.2 Run isolated Windows restart acceptance with two named sessions and at least two tabs per session.
  - Verify different directories, reordered tabs, inactive-session selected tab, selected session, changed Shell setting, and fresh shell without old output.
  - Then verify individual close, group close cancellation/confirmation, last shell exit, close window, Quit while Settings is focused, and failed-tab repair.
  - Gate: record steps and observed results; never overwrite the user's real workspace/settings.
- [ ] 8.3 Exercise failures and other platforms.
  - Verify invalid JSON, unavailable directory, unwritable snapshot, failed replacement, interrupted restore, forced process termination, ownership contention, and crash lock release.
  - Run macOS/Linux equivalents where available. Explicitly list untested platform behavior; do not claim source inspection as runtime coverage.
- [ ] 8.4 Run repository checks and review diff scope.
  - Commands: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`; use `scripts/check.sh` for its additional configured checks including the line limit when supported.
  - Gate: no unrelated formatting, dependency updates, output persistence, program replay, or changed explicit-launch behavior. Report environmental blockers separately.
- [ ] 8.5 Update user documentation and project acceptance/task tracking.
  - Explain structure-only restoration, file location, approved ownership policy, failed-tab recovery, and abnormal-exit save limits.
  - Final handoff lists changed files, checked tasks, real tests, outstanding risks, and any unavailable platform verification. Do not claim the feature complete with blocking tasks unchecked.
