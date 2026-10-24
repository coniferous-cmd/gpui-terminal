---
create-time: 2026-10-24 09:49:53
---

# Proposal

## Why

Users can customize terminal text but cannot choose the font used by the application's ordinary interface. Users should be able to use a favorite installed family to personalize the interface without changing terminal text or the monospace font used for code.

## What Changes

- Add an independent Interface font section to Settings › Appearance with a searchable installed-family picker and a Restore default control.
- Apply a selected interface family immediately across ordinary application UI and persist it across launches.
- Preserve the existing Ely default, currently Inter, when no override is selected; old settings remain valid.
- Rename the existing Font section to Terminal font to make the two settings unambiguous.
- Keep code-area monospace fonts, terminal fonts, font size, spacing rules, icons, native menus and system notifications outside this setting.
- Reject families that cannot load, retain the saved selection on failure, and show the error. Missing glyphs continue to use platform fallback.
- Do not add interface font size, interface fallback lists or a separate large preview; the live interface provides the preview.

## Impact

- Affected specs: new `interface-font` capability.
- Affected code: settings model, validation and persistence; Appearance controls and their input state; startup and settings observation; Ely theme font integration; cached UI invalidation where needed; settings/font tests and acceptance documentation.
- Existing configuration gains an optional `ui_font_family` field. No dependency or component-library update is expected; Ely already exposes `Theme.font_family` and `Theme::update`.
