---
create-time: 2026-10-24 17:11:08
---

## ADDED Requirements

### Requirement: Settings File Path

The application SHALL read and write its settings at `settings.json` inside a `luna` folder under the user's home folder's `.config` folder, on macOS, Linux and Windows alike.

The path SHALL be derived from the user's home folder, not from the platform's config folder, so that every platform keeps one path.

#### Scenario: macOS keeps settings under the home folder

- **WHEN** the application starts on macOS for a user whose home folder is `/Users/alice`
- **THEN** it reads settings from `/Users/alice/.config/luna/settings.json`

#### Scenario: Windows keeps settings under the home folder

- **WHEN** the application starts on Windows for a user whose profile folder is `C:\Users\Alice`
- **THEN** it reads settings from `C:\Users\Alice\.config\luna\settings.json`

#### Scenario: The XDG config folder does not move the settings file

- **WHEN** the application starts on Linux with `XDG_CONFIG_HOME` set to a folder
- **THEN** it still reads settings from `$HOME/.config/luna/settings.json`

#### Scenario: A saved change goes to the same path

- **WHEN** a setting changes
- **THEN** the whole settings file is written to the same path it was read from

### Requirement: Settings Folder Creation

The application SHALL create the `luna` folder and the `luna` folder's parent when it first saves settings, with no folder needed beforehand.

#### Scenario: A first save makes the folders

- **WHEN** a setting changes for a user with no `~/.config/luna` folder
- **THEN** the folders are created and `settings.json` is written inside them

#### Scenario: A missing file means defaults

- **WHEN** no settings file exists at that path
- **THEN** the application starts with the default settings and writes nothing until a setting changes

#### Scenario: A bad file still stops the start

- **WHEN** the settings file at that path does not parse or holds a value out of range
- **THEN** the start fails with an error naming the file and the field, and the file is left as it was

### Requirement: Session Layout Stays in the Platform Config Folder

The workspace layout SHALL keep living in `sessions.json` inside `luna-terminal` under the platform's config folder, unchanged by this move.

The settings file and the layout file SHALL be read and written independently, neither assuming the other sits beside it.

#### Scenario: The layout file does not move

- **WHEN** the application starts on macOS and restores a workspace
- **THEN** it reads the layout from `~/Library/Application Support/luna-terminal/sessions.json`

#### Scenario: A layout reads with no settings file beside it

- **WHEN** the layout file exists at the platform config folder and no settings file exists under the home folder
- **THEN** both are used as they are, the layout restored and the default settings taken
