---
create-time: 2026-10-24 17:11:22
---

# Design

## Context

`SettingsStore::init` derives its path from `dirs::config_dir()`, joined with `luna-terminal` and `settings.json` (`src/settings/store.rs:23-24`). `SessionStore::init` derives its own path the same way, joined with `luna-terminal` and `sessions.json` (`src/sessions/state.rs:168-169`). The two stores hold their own `PathBuf` and never consult each other, so the paths are already independent even though today they land in one folder.

`dirs::config_dir()` gives `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` or `~/.config` on Linux, and `%APPDATA%` on Windows. `dirs::home_dir()` gives `$HOME` on macOS and Linux, and `SHGetKnownFolderPath(FOLDERID_Profile)` on Windows — it does not read `USERPROFILE`, so an environment override does not redirect it there.

The settings file holds every field the settings window edits, including the window size, the column widths and Ely's place.

## Decisions

- **The settings path comes from the home folder, not the platform's config folder.** `dirs::home_dir()?.join(".config").join("luna").join("settings.json")` gives one path on all three platforms, which is the point of the move: nothing to remember per platform, and a folder that dotfile managers already track. The cost is that `XDG_CONFIG_HOME` no longer moves the file on Linux and that macOS and Windows get a folder their conventions would place elsewhere; the PRD states both.

- **The folder is named `luna`.** The owner asked for `~/.config/luna`. This is shorter than the binary's `luna-terminal`, and the inconsistency is permanent; the path is stated in full wherever it appears so no one has to guess.

- **`sessions.json` does not move.** The owner asked for it to stay. The layout is workspace state rather than configuration, and a move buys it nothing. `SessionStore::init` keeps `dirs::config_dir()` untouched.

- **No shared path helper.** Each store keeps deriving its own path, as both already do. There is exactly one caller of each, and a helper would hide which store owns which file, against `Fail fast. No silent fallbacks.` and the rule against dead abstraction.

- **A missing home folder fails the start, named.** `dirs::home_dir()` returning none is an error from `SettingsStore::init`, worded as the existing missing-config-folder error is, so the owner of the failure is the settings store.

- **Nothing about the file's contents changes.** The struct, its serde attributes, the ranges and the atomic write stay as they are; only the path handed to `load` and kept for `save` differs.

## Risks / Trade-offs

- **A Linux user with `XDG_CONFIG_HOME` set finds the settings file somewhere new.** The PRD and README name the single path and say the variable does not move it, so the behavior is stated rather than discovered.
- **macOS and Windows keep two folders for one app**: `~/.config/luna` for settings, and the platform config folder for the layout. Both doc comments say which file lives where and why, so the split reads as a decision rather than an oversight.
- **`shot`'s isolation relies on `HOME`.** On macOS and Linux both `home_dir()` and `config_dir()` read `$HOME`, so `HOME=/tmp/x` still isolates both files. On Windows `home_dir()` ignores the environment, so that isolation does not exist there; no test depends on it.
- **The acceptance table asserts the old path.** `tasks/acceptance.md:33` fails as a claim the moment the code changes, so it is corrected in the same change rather than left to drift.
