---
create-time: 2026-10-24 09:51:28
---

## 1. Implementation

- [x] 1.1 Add optional `ui_font_family` to the settings model, defaulting to None; test older settings, null and named-family round trips, and refusal of blank values without changing terminal defaults.
- [x] 1.2 Validate actual interface-family resolution during settings startup and when its override changes; ensure errors name `ui_font_family`, failed validation/save preserves prior state, and invalid startup settings are never overwritten. Extend the font and store tests.
- [x] 1.3 Capture Ely's default interface family and apply the saved effective family before windows render. Observe interface-font changes using `Theme::update`, log changes, and test that mono family, terminal family, size and scale remain untouched.
- [ ] 1.4 Audit ordinary text inheritance and explicit family use across main/settings roots, inputs, labels, tooltips and dialogs. Verify cached Git content updates and add targeted invalidation if needed; preserve code, icons, font samples and terminal preview fonts.
- [x] 1.5 Add the Interface font section with an independent searchable picker input and Restore default control; rename Font to Terminal font. Extract a coherent module as needed to keep every hand-written file within 500 lines.
- [ ] 1.6 Synchronize picker state after successful selection, failed validation/save and restoring defaults, including focused drafts. Verify no selection leaks into terminal or fallback pickers.

## 2. Verification and documentation

- [x] 2.1 Add automated regression coverage for restore/persistence and theme changes preserving the interface override; cover font-role isolation through the existing GPUI test harness where practical.
- [ ] 2.2 Exercise the UI with a visibly different installed proportional family, long session names, renaming, Files, visible and hidden Git panels, open diff/dialog content, settings reopening and restart. Confirm code/terminal typography and metrics do not change.
- [ ] 2.3 Verify load failures and save failures keep the saved font and displayed picker choice, Restore default survives restart, and color/system-theme changes preserve the selected font. Record platform coverage and any unavailable macOS, Windows or Linux checks.
- [ ] 2.4 Update relevant project requirements, task tracking and acceptance documentation to describe the independent interface-font setting and its scope.
- [ ] 2.5 Run `scripts/check.sh` and applicable platform tests; report results, including any environment blockers, without claiming unrun checks passed.

## Implementation notes

- macOS: `cargo check --locked`, all 17 settings unit tests and `cargo test --locked --test fonts` passed. The font executable covers startup rejection without overwriting, actual installed proportional fonts, update rollback on resolution/save failure, theme isolation, persisted selection and reset.
- Isolated-HOME `shot` captures confirmed Georgia in ordinary settings text, separate Interface font and Terminal font sections, and Restore default changing both the picker and surrounding text to Inter. Captures are in `/tmp/ely-ui-font-settings.png` and `/tmp/ely-ui-font-reset.png`; broader interaction scenarios remain unchecked.
- `scripts/check.sh` passed fmt and clippy, then stopped with 287/289 library tests passing: the fish and PowerShell integration tests failed because `fish` and `pwsh` are not installed. Later script stages and Windows/Linux verification have not run. Implementation paused for guidance on this environment blocker.
