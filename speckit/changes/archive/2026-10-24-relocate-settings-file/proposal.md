---
create-time: 2026-10-24 17:10:48
---

# Proposal

## Why

`settings.json` follows the platform's config folder, so the same settings file sits in three different places: `~/Library/Application Support/luna-terminal` on macOS, `$XDG_CONFIG_HOME/luna-terminal` on Linux and `%APPDATA%\luna-terminal` on Windows. A user who keeps dotfiles under `~/.config` cannot manage or version the file there, and cannot name one place that holds the app's configuration on every machine.

## What Changes

- `settings.json` reads and writes at `~/.config/luna/settings.json` on all three platforms, derived from `dirs::home_dir()` rather than `dirs::config_dir()`.
- The folder and the file are created on the first save, as the atomic writer already creates a file's folder.
- `sessions.json` stays where it is, at `dirs::config_dir()/luna-terminal/sessions.json`. Only the settings file moves.
- Any statement that the two files sit together is corrected: the doc comment in `sessions/state.rs`, `PRD.md`, `TASKS.md`, the acceptance table and the README's list of per-platform paths.
- The path derivation is the same on every platform, so `XDG_CONFIG_HOME` no longer moves the settings file on Linux; the PRD says so.

## Impact

- Affected specs: new `settings-location` capability.
- Affected code: `src/settings/store.rs` (`SettingsStore::init`), whose path is the only code change; `src/sessions/state.rs`'s doc comment; `tests/fonts/interface.rs`, which builds the settings path by hand and would otherwise write where nothing reads; and the documents that name the paths (`PRD.md`, `TASKS.md`, `tasks/acceptance.md`, `README.md`, `AGENTS.md`).
- No dependency, setting, range, theme or file format changes: `settings.json` keeps every field and default it has.
- The `settings::store` unit tests keep working, since they pass the path in rather than deriving it.
