# Implementation verification

## Completed checks

- `cargo test --locked --lib cli::`: 27 tests passed on macOS.
- `cargo test --locked --test cli`: 3 subprocess tests passed on macOS.
- macOS isolated live smoke: list, create with name/cwd, rename, empty-name reset, focus, close, and final-session close all returned success; GUI exited zero and removed its socket.
- `cargo clippy --locked --all-targets -- -D warnings`: passed on macOS.
- Windows protocol/client/pipe/security/console/launcher sources were checked in a temporary standalone harness targeting `x86_64-pc-windows-msvc`; this does not prove the complete app builds or runs on Windows.

## Paused checks

- `LUNA_CLI_LIVE=1 cargo test --locked --test cli_live`: failed waiting for a GUI-dispatched tab close after creating a second tab. The guard correctly rejected multi-tab CLI close without `--force`, but subsequent GUI selection/close actions did not take effect. Investigate frame/focus/action dispatch before treating this harness as acceptance evidence. The harness is opt-in and remains incomplete.
- `scripts/check.sh`: fmt and clippy passed; library tests reported 336 passed and 1 failed because `pwsh` is not installed (`term::shell::tests::powershell_reports_its_folder_byte_for_byte_and_keeps_the_status`). The gate stops there; later checks must be rerun after the prerequisite is available.
- Linux headless/live verification has not run.
- Windows native security, pipe cancellation, release console/redirection behavior, and `.com` launcher packaging have not run.
- Cold-start races/restoration and multi-instance end-to-end acceptance are not yet verified.

## Lifecycle detail

GPUI limits quit hooks to 200 ms. The implementation therefore defers last-window application quit until outstanding CLI replies are acknowledged or a bounded deadline expires, instead of relying on the quit hook alone. The quit hook is used only for final service cleanup.

Only tasks 1.1, 1.3, 3.1, and 3.2 are checked off. Other code is work in progress until each task's verification criteria pass.
