---
create-time: 2026-10-24 17:11:39
---

## 1. Implementation

- [x] 1.1 In `src/settings/store.rs`, derive the settings path from `dirs::home_dir()` joined with `.config`, `luna` and `settings.json`, keeping the error's wording when the platform names no home folder. Verify: `cargo check` passes and `SettingsStore::init` no longer calls `dirs::config_dir()`.
- [x] 1.2 Extract the path as a small function of the home folder, and add a test in `settings/store.rs` asserting it ends in `.config/luna/settings.json` under a given home. Verify: `cargo test settings::store` passes.
- [x] 1.3 In `src/sessions/state.rs`, replace the `beside settings.json` doc comment on `FILE` with one naming the platform config folder and saying the settings file lives elsewhere. Verify: no source comment claims the two files share a folder.
- [x] 1.4 `tests/fonts/interface.rs` built the settings path by hand from `dirs::config_dir()`, so `SettingsStore::init` would read defaults instead of the file the test writes. Point it at `$HOME/.config/luna/settings.json`. Verify: the `fonts` test passes on macOS.

## 2. Documentation

- [x] 2.1 Correct `PRD.md:27` to name `~/.config/luna/settings.json` and say `XDG_CONFIG_HOME` does not move it.
- [x] 2.2 Correct `PRD.md:87` and `TASKS.md:174`, which say `sessions.json` sits beside `settings.json`, to name the platform config folder instead.
- [x] 2.3 Correct `tasks/acceptance.md:33` to assert the new path, pointing the evidence at the `settings::store` test.
- [x] 2.4 Replace the README's three per-platform paths with the single `~/.config/luna/settings.json`, keeping the note that a bad file stops the start and is never overwritten.
- [x] 2.5 Correct `AGENTS.md`'s description of `dirs` to say which file takes the home folder and which takes the platform config folder.

## 3. Verification

- [ ] 3.1 Run `scripts/check.sh`; fmt, clippy `-D warnings`, the tests and the 500-line rule all pass. (fmt, clippy, the comment check and the 500-line check pass. 306 tests pass; `fish_loads_its_script_and_restores_its_data_dirs` and `powershell_reports_its_folder_byte_for_byte_and_keeps_the_status` fail the same way with this change stashed, so they are the machine's fish and pwsh, not this change.)
- [x] 3.2 Run `HOME=$(mktemp -d) cargo run`, open Settings, change the theme, and confirm `$HOME/.config/luna/settings.json` appears with the change and nothing new lands in the platform config folder.
