# Verification

## Passed on macOS

- `cargo check --all-targets --locked`: application, tests and examples compile.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `git diff --check`.
- `cargo test --lib --locked -- --skip term::shell::tests`: 332 passed; 12 shell integration tests explicitly filtered, not counted as passes.
- `cargo test --test cli --locked`: 3 passed.
- Focused directory policy tests: 3 passed. Snapshot tests: 20 passed, including old/mixed snapshot rejection, invalid cwd, distinct session/tab folders, nonexistent retained path and non-UTF-8 rejection.
- Repository text-file 500-line scan: no violations. Snapshot tests were moved to `src/sessions/state/tests.rs`.
- Original `LUNA_CLI_LIVE=1 cargo test --test cli_live --locked` passed before adding the new cwd regression.

## Partial desktop evidence

The added `tests/cli_live/cwd.rs` uses isolated HOME/TMPDIR and a shell that records its actual starting cwd, changes directory and reports OSC 7. Its first run observed four launches in the session's retained project directory despite changed live folders, a Start in change and original-tab closure. The assertion initially treated macOS `/tmp` and `/private/tmp` aliases as different; filesystem identity comparison now canonicalizes test observations only, not production paths.

Subsequent runs timed out in the preexisting initial-session close-first check, before reaching the new regression. An explicit focus retry did not resolve it and was removed. The full new desktop regression has NOT passed. Its missing/file/permission-denied launch checks remain unverified. Do not infer correctness of GUI state preservation from unit-test success.

## Blockers and remaining coverage

- `scripts/check.sh`: fmt/clippy passed, PowerShell integration failed, and the run timed out at 200 seconds. Isolated rerun reports `pwsh is not installed` at `src/term/shell/tests.rs:37`. The owner chose not to install PowerShell and to continue other work; full checks remain incomplete.
- The other shell integration tests were not included in the successful 332-test run; filtering them is not a suite pass.
- Full desktop regression, actual shortcut/button/reorder interactions, restart round-trip and old-layout startup behavior still need verification. Persistence unit tests alone do not complete these tasks.
- Windows and Linux were not exercised for this change.
- No task with outstanding required verification has been marked complete. Tasks 1.3, 1.5, 2.1–2.4 remain open.
