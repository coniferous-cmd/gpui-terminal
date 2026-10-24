---
create-time: 2026-10-24 09:50:14
---

# Design

## Context

`Settings.font_family` and `font_fallbacks` drive the custom terminal engine. `src/settings/store.rs` validates their actual resolution before startup and before saving a changed family. `src/settings/window/appearance.rs` exposes those settings in a Font section.

Ely's pinned Theme separates `font_family` (Inter by default) from `mono_family` (JetBrains Mono). Its `Theme::update` edits non-color tokens and refreshes windows. Application color-theme integration is in `src/themes/chrome.rs`; `src/app.rs` observes settings for theme and opacity changes. Git panel content uses a cached entity in `src/workspace.rs`, so refreshing a window alone must not be assumed to invalidate every cached rendering.

The goal is personalizing ordinary UI typography, not changing terminal rendering, code typography, interface sizes or platform glyph fallback.

## Decisions

- Add `ui_font_family: Option<String>` with a default of None. Missing and null values mean use the component library's initial interface family. Capture that default before applying an override so Restore default cannot accidentally reuse the active custom font. Do not label this as the system font.
- Reject empty or whitespace-only overrides during model validation. Validate actual family resolution at startup and only when the override changes during an update. Reuse or extract the existing strict resolution check without exposing terminal-owned error wording to the interface setting. Keep normal glyph fallback; checking family resolution does not require complete character coverage.
- Keep `SettingsStore::update` as the only save path. Validate and persist before applying theme changes, so a failed save changes neither effective typography nor stored state. Error context names `ui_font_family` and the failing family or persistence operation.
- Apply the effective interface family at startup before windows open and observe only relevant changes thereafter. Use `Theme::update` to change `font_family`, never `mono_family`, `font_scale` or terminal settings. Keep font application separate from palette changes, or explicitly preserve it through every palette application path, including Omarchy and system appearance. Log successful effective family changes.
- Audit ordinary UI roots and controls for inherited and explicit family choices rather than assuming a theme mutation reaches everything. Preserve explicit monospace code regions, icon rendering, font demonstration samples and the terminal cursor preview. Verify cached Git content invalidation and add targeted notification only if necessary.
- Give the new picker its own persistent TextInput entity and stable identity, separate from terminal and fallback pickers. Reuse the installed-family search behavior and failure recovery pattern. A refused change restores the saved effective family and closes the rejected draft. Restore default clears the override and synchronizes the input even when it previously held focus.
- Add an Interface font section before Terminal font in Appearance. Provide Family and Restore default controls; disable Restore default when no override exists. Use existing theme layout tokens. Do not add a large preview: selection immediately updates the interface itself.
- Add automated coverage for configuration defaults, serialization, validation, family resolution and theme separation where the existing GPUI test harness allows. Manually verify interaction and cache behavior on available platforms, recording untested platforms rather than claiming coverage.
- Keep each hand-written file at or below 500 lines. Appearance already has 500 lines, so extract the new controls or a coherent existing group into a sibling module rather than appending to that file.

## Risks / Trade-offs

- Different font metrics can widen labels or change ascenders without changing the configured size. Preserve current clipping, scrolling and layout constraints; probe long session names and broad fonts, but do not introduce automatic resizing or scale changes.
- A selected installed family may disappear before the next launch. Follow the project's fail-fast policy: identify the invalid setting and do not overwrite the file, rather than silently choosing another font.
- Theme refresh may leave cached text stale. Test an already visible Git panel, hidden/revealed panels and open dialogs; ensure ordinary text updates without resetting unrelated interaction state unnecessarily.
- Picker state can retain a rejected or reset draft while focused. Use an independent input and explicitly synchronize saved state, following existing picker failure handling.
- Font resolution and aliases vary by platform. Exercise bundled/default, installed proportional and unavailable families on available macOS, Windows and Linux environments; avoid promising identical glyph fallback across operating systems.
