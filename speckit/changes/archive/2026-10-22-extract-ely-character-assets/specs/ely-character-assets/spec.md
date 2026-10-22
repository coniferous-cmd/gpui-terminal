---
create-time: 2026-10-22 14:14:29
---

## ADDED Requirements

### Requirement: Ely assets are separate from rendering code

The system SHALL define the built-in Ely's layer artwork and static rig description under `assets/pet/ely/`, outside Rust source. The description SHALL preserve layer names, ordering, colors, opacity, bounds, pivots, view coordinates, head pivot and existing motion bindings. Rust SHALL retain animation and rendering algorithms, not a duplicate artwork or layer-description table.

#### Scenario: Built-in rig is constructed

- **WHEN** the application constructs Ely's rig
- **THEN** all layers and their static description come from the embedded asset bundle in their declared order
- **AND** the bundle contains all existing expression and effect layers

### Requirement: Assets remain embedded

The system SHALL embed Ely's description and artwork into the executable and SHALL NOT require runtime access to the source tree or a separate asset folder. Existing settings and release packaging SHALL remain compatible.

#### Scenario: Executable runs outside the checkout

- **WHEN** the built application runs from a directory without `assets/pet/ely/`
- **THEN** Ely displays and animates normally using embedded resources

### Requirement: Resource errors are explicit

The system SHALL validate the embedded description before using it to construct a drawable rig. It SHALL reject unknown fields or binding values, duplicate layer names, missing resources, invalid side indices, non-finite geometry, non-positive frame/view sizes, invalid RGB values and opacity outside zero to one. Failures SHALL identify the pet subsystem and offending resource, layer or field, and SHALL NOT silently skip layers or substitute a different rig. Tests SHALL also validate SVG syntax and bounds.

#### Scenario: Description is invalid

- **WHEN** a test provides a malformed description or unresolved SVG reference
- **THEN** loading fails with a diagnostic identifying the offending resource or field before a partial rig is used

#### Scenario: Embedded artwork is invalid

- **WHEN** a layer contains invalid SVG or geometry outside its declared frame
- **THEN** resource tests fail and identify that layer

### Requirement: Appearance and motion remain unchanged

The system SHALL preserve Ely's current geometry, color, clipping, mask rendering, dimensions, expression selection, transform order and animation timing. It SHALL preserve all current moods, transitions, idle sleep, click response, gaze tracking and the shirt/hair coverage during hops.

#### Scenario: Deterministic poses are compared

- **WHEN** pre-extraction and post-extraction rigs render identical fixed poses on the same rendering setup
- **THEN** layer order, visible layers, transforms and opacities match
- **AND** representative images for calm, working, waiting, error, cheering, sleep and wink have no unexplained visual differences

### Requirement: Window and interaction behavior remain unchanged

The system SHALL preserve the existing show/hide setting, pointer interaction, dragging, saved horizontal position, focus behavior and platform-specific edge placement. This change SHALL NOT add user-selectable roles, external loading, hot reload or new settings.

#### Scenario: Existing interaction is exercised

- **WHEN** the user clicks or drags Ely, toggles her visibility, or moves the main window between supported edge-placement conditions
- **THEN** behavior and persisted settings are unchanged from before extraction
