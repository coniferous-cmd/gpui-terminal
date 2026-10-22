# Evidence

Recorded 2026-10-22 on the owner's Windows 10 PC (10.0.19045), debug build of the working tree.

## 1.1 Baseline (before the old table was deleted)

A one-off `#[ignore]`d test in the old `rig.rs` wrote, straight from `PARTS` and its helpers (nothing hand-copied):

- `baseline/layers.json` — 50 layers: name, color, alpha, frame, pivot, bone, plus the view and the head's pivot.
- `baseline/poses.json` — 14 fixed-pose draw plans (origin 0, scale 1): calm, calm mid-glance, work, wait, fret, cheer, early doze, settled doze, mid-wink, wink from sleep, entry rise, entry wait, and two mid-ease blends (done→work, wink→wait). Together they cover every binding (body, head, lids, irises, arcs, lines, brows, blush, all five mouths, locks, ahoge, both hands, all seven marks), every expression, the entry, state transitions and the click overlay.
- `baseline/svg/<name>.svg` — the old `svg()` wrapper's exact output per layer.

Reproducible image-comparison method: each layer's new file is byte-identical to its baseline twin, so any rasterizer (gpui's or resvg's) produces identical pixels for every layer; with identical draw plans, colors and opacities, the composed character is identical. No tolerance is needed or used.

## 2.3 Baseline comparison (after extraction)

A second one-off `#[ignore]`d test (deleted after it ran, so no second asset implementation remains) compared the loaded rig against the baseline:

- Metadata: view, head pivot, and all 50 layers' name, color, alpha, frame, pivot and bone — exact.
- Artwork: all 50 embedded SVGs byte-identical to `baseline/svg/`.
- Draw plans: all 14 samples × 50 layers — same order, same visibility, same opacity, same rotation/translation matrices. Comparison ran at f32, the matrices' true precision: serde_json stores numbers as f64, and a widened f32's printed decimal can differ by one f64 ulp from the re-widened value; that wrinkle is a comparison-method artifact, checked, not a data difference. Zero unexplained differences.

## 2.4 Checks (Windows equivalents of scripts/check.sh)

- `cargo fmt --check` — clean (after one `cargo fmt` pass).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo test --quiet` — 263 passed, 0 failed (3 suites).
- Multi-line-comment rule — clean.
- 500-line rule — clean after the owner chose to exempt generated data in `scripts/check.sh` (`Cargo.lock`, `assets/pet/ely/rig.json`, `speckit/changes/*/baseline/*`); every hand-written file is within 500 lines (`rig.rs` 238, `assets.rs` 311, `assets/tests.rs` 221).

## 3.1 Runs outside the checkout

`target/debug/elyterminal.exe` copied to `target/pet-verify/` (no `assets/` there) and launched from that folder with `RUST_LOG=elyterminal=debug`:

- `pet: Ely peeks over the main window, a window kept there, on the pane's top edge, feeling Calm` — she showed from embedded resources alone.
- The user's existing `settings.json` was used unchanged; no settings migration.

## 3.2 Interaction (Windows only; macOS and Linux untested this change — their platform modules are untouched by the diff)

Script `target/pet-verify/verify.ps1` (kept out of the tree) drove the live app; settings were backed up and restored:

| Check | How | Result |
| --- | --- | --- |
| Embedded show | pet window enumerated by her 112×~123 popup size | pass |
| No focus steal | `GetForegroundWindow` never her window | pass |
| Click | `SetCursorPos` + click at her centre → log `pet: Ely is pressed` | pass |
| Drag + save | press, move +240 px, release → log `pet: Ely stands N points in`; `ely_left` updated (restored after) | pass |
| Gaze | log `pet: Ely watches the pointer in Main` / `pet: Ely returns to her own gaze` while the pointer moved | pass |
| Top/bottom | `ShowWindow(SW_MAXIMIZE)` → `pet: Ely moves to the pane's bottom edge`; `SW_RESTORE` → back on the top edge | pass |
| Show/hide | with `ely: false` in settings, zero pet windows at start; `ely: true` (default) shows her | pass |

## 3.3 Scope review

Diff: `src/pet.rs` +5/−1 (mod line, rig loaded before the window opens), `src/pet/tests.rs` 1 line, `src/pet/rig.rs` rewritten around `assets::Layer`, new `src/pet/assets.rs` + `src/pet/assets/tests.rs`, new `assets/pet/ely/`, `scripts/check.sh` generated-data exemption, AGENTS.md maintenance note. Untouched: `look.rs`, `curves`, `gaze.rs`, `view.rs`, all platform modules, settings, themes, icons, window sizes, placement algorithms. No character selection, import, hot reload or new setting. The one-off generator and comparison tests were deleted; no fallback copy of the old table remains.
