use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_luna-terminal"))
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .output()
        .unwrap()
}

#[test]
fn help_and_version_work_without_a_display() {
    for args in [
        vec!["--help"],
        vec!["session", "--help"],
        vec!["session", "close", "--help"],
        vec!["--version"],
    ] {
        let output = cli(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn invalid_commands_are_usage_errors_not_gui_launches() {
    for args in [
        vec!["not-a-command"],
        vec!["session", "rename", "missing-name"],
        vec!["session", "close", "id", "--json"],
    ] {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
    }
}

#[cfg(unix)]
#[test]
fn absent_gui_returns_nonzero_with_stderr_only() {
    let root = std::env::temp_dir().join(format!("luna-cli-absent-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna-terminal"))
        .args(["session", "list", "--json"])
        .env("TMPDIR", &root)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not running"));
    std::fs::remove_dir_all(root).unwrap();
}
