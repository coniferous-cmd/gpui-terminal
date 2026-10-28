---
create-time: 2026-10-28 22:12:10
---

## 1. Implementation

- [x] 1.1 Add explicit session launch cwd to Session and Opening and thread it through session insertion, Workspace construction, ordinary default startup, and explicit launches. Update all constructors and fixtures; verify tests and examples compile and no constructor derives restored cwd from a live tab.
- [x] 1.2 Separate new-session directory resolution from new-tab directory selection in the shared creation flow. Preserve explicit CLI cwd precedence, relative CLI resolution, and omitted-cwd Start in behavior for new sessions; new tabs use only session cwd. Add unit tests covering both Start in choices, explicit and omitted cwd, and independent sessions.
- [ ] 1.3 Preserve spawn-before-insertion and add contextual new-tab launch errors. Verify missing paths, file paths, and access failures leave tab count, selection, and session cwd unchanged without fallback; run permission-sensitive tests as a non-root user.
- [x] 1.4 Persist required SavedSession.cwd independently of SavedTab.folder in sessions.json. Validate type, nonempty value, and rooted paths without checking existence. Reject old layouts without defaults or migration. Update unrelated test fixtures so they continue testing their intended errors; add missing-field, mixed-old/new, invalid-value, unknown-field, and valid-empty-layout tests.
- [ ] 1.5 Carry saved session cwd unchanged through restoration while retaining existing per-tab restore behavior. Add round-trip tests with distinct session and tab directories and tests where missing tab directories recover without changing session cwd, or a missing session cwd path prevents only subsequent new-tab creation.
- [x] 1.6 Handle snapshot text encoding without lossy cwd conversion: log and omit a session whose launch cwd is unrepresentable, remapping selection consistently. Add a Unix non-UTF-8 path test and keep files within 500 lines by splitting tests if needed.

## 2. Behavioral verification

- [ ] 2.1 Exercise CLI creation with an explicit cwd, cd the initial tab elsewhere, then add tabs through both the shortcut and tab-bar button. Confirm each shell starts in the retained session cwd, with Start in set to Home and to Session. Repeat after changing settings, selecting/reordering tabs, and closing the original tab.
- [ ] 2.2 Verify a second session retains its own cwd, and live directory consumers (Files, Git, sidebar description, session list) still follow the selected tab rather than the launch cwd. Verify new-session creation without --cwd retains its existing policy.
- [ ] 2.3 Quit and restart a layout with divergent session/tab directories; verify existing tab restoration and later new-tab launch independently. Load an old nonempty snapshot and confirm the diagnostic and default home session rather than partial restoration. Use isolated configuration folders, never the user's own snapshot.
- [ ] 2.4 Run scripts/check.sh and compile the relevant examples/tests; run supported cross-platform checks for path/launch behavior and explicitly record any platform not exercised. Distinguish source inspection from runtime evidence.

## 3. Documentation

- [x] 3.1 Document --cwd as the session's retained new-tab launch directory, distinguish it from live tab cwd and session list output, and state that Start in applies when creating a session, not adding tabs to it.
- [x] 3.2 Record the intentional old-layout incompatibility in release-facing notes and update project task/acceptance documentation. Keep active session-cli and restore-workspace plans in view without implementing their unrelated scope; resolve conflicts if their implementation has changed the underlying persistence model.
