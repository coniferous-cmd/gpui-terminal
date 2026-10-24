---
create-time: 2026-10-24 09:50:14
---

## ADDED Requirements

### Requirement: Independent interface font selection

The system SHALL provide a searchable installed-font-family picker in an Interface font section of Settings › Appearance. It SHALL identify the existing terminal font section as Terminal font. Interface font selection SHALL NOT be restricted to monospace families or introduce interface size, spacing, scale or fallback-list settings.

#### Scenario: Choose a favorite font

- **WHEN** the user selects a loadable family in the interface font picker
- **THEN** ordinary application interface text uses that family immediately without restarting or an additional Apply action
- **AND** the selection is saved for subsequent launches

### Requirement: Typography boundaries

The interface font SHALL apply to ordinary application UI, including session names, application-drawn titles, settings, buttons, panel labels, ordinary text inputs and tooltips. It SHALL NOT replace code-area monospace fonts, terminal text and its cursor preview, icon glyphs, font-family demonstration samples, native menus or system notifications. Existing size and layout rules SHALL remain unchanged.

#### Scenario: Change font with panels open

- **WHEN** the user changes the interface font while the Git panel or a dialog is open
- **THEN** ordinary UI text updates, including cached content and subsequently revealed panels
- **AND** code-area and terminal font families and terminal cell metrics remain unchanged

#### Scenario: Change the terminal font independently

- **WHEN** the user changes the terminal family or its fallbacks
- **THEN** the interface font selection remains unchanged

### Requirement: Default and configuration compatibility

The system SHALL represent the override as optional `ui_font_family` in settings. Missing or null values SHALL preserve Ely's default interface family, currently Inter. Restore default SHALL clear the override, update the picker and interface immediately, and persist that choice.

#### Scenario: Load an older settings file

- **WHEN** settings contain no `ui_font_family`
- **THEN** startup succeeds with the existing default interface font and unchanged terminal settings

#### Scenario: Restore the default

- **WHEN** the user restores the default after selecting a custom interface family
- **THEN** the interface and picker show the default family
- **AND** restarting continues to use the default rather than the previous override

### Requirement: Validated changes and explicit failure

The system SHALL validate that a named interface family is nonblank and actually loadable rather than silently accepting another family. A failed selection or save SHALL leave the stored and effective font unchanged, restore the picker to the saved choice and display an error identifying the failure. Glyphs missing from a valid family SHALL continue to use platform fallback.

#### Scenario: Refuse an invalid saved family at startup

- **WHEN** settings name an empty, whitespace-only, missing or unloadable interface family
- **THEN** startup fails with an error identifying `ui_font_family`
- **AND** the settings file is not overwritten

#### Scenario: Refuse a selection or save

- **WHEN** validation or persistence fails for a newly selected interface font
- **THEN** the prior saved font remains effective and selected
- **AND** the settings page shows the error

### Requirement: Stable application across appearance changes

The selected interface family SHALL apply before application windows first render and remain in force across theme changes, system appearance changes, supported system-theme following and window creation. Successful interface-font changes SHALL be logged.

#### Scenario: Change color theme after choosing a font

- **WHEN** the color theme or system appearance changes with a custom interface family selected
- **THEN** the chosen interface family remains in use

#### Scenario: Open settings after restart

- **WHEN** the application restarts with a valid saved override and the settings window opens
- **THEN** both the main interface and settings use the saved family without requiring another selection
