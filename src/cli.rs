use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub mod client;
mod console;
pub mod identity;
pub mod protocol;
pub mod service;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub use unix as platform;
#[cfg(windows)]
pub use windows as platform;

#[derive(Debug, Parser)]
#[command(name = "luna-terminal", version, about = "A terminal on GPUI and Ely")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage live sidebar sessions, not individual terminal tabs.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List live sessions across running luna-terminal windows.
    List {
        /// Print a JSON array instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Create a session with one tab; start the GUI if it is not running.
    Create {
        /// Set a custom session name.
        #[arg(long)]
        name: Option<String>,
        /// Retain this directory for new tabs; relative paths use the CLI's cwd.
        #[arg(long, value_name = "DIRECTORY")]
        cwd: Option<PathBuf>,
        /// Select a target when multiple terminal windows are running.
        #[arg(long, value_name = "WINDOW_ID")]
        window: Option<String>,
    },
    /// Rename a session; an empty name restores the current tab's title.
    Rename {
        #[arg(value_name = "SESSION_ID")]
        id: String,
        name: String,
    },
    /// Select a session and request activation of its window.
    Focus {
        #[arg(value_name = "SESSION_ID")]
        id: String,
    },
    /// Close a session and its programs.
    Close {
        #[arg(value_name = "SESSION_ID")]
        id: String,
        /// Allow closing all tabs in a multi-tab session without a dialog.
        #[arg(long)]
        force: bool,
    },
}

pub fn dispatch() -> Option<i32> {
    use std::io::Write;
    if std::env::args_os().len() == 1 {
        return None;
    }
    if let Err(error) = console::attach() {
        let _ = writeln!(std::io::stderr().lock(), "{error:#}");
        return Some(1);
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            return Some(if error.print().is_ok() { code } else { 1 });
        }
    };
    let Some(Command::Session { command }) = cli.command else {
        return None;
    };
    match client::run(command) {
        Ok(output) => {
            let mut stdout = std::io::stdout().lock();
            if let Err(error) = stdout
                .write_all(output.as_bytes())
                .and_then(|()| stdout.flush())
            {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "cli: writing command output: {error}"
                );
                Some(1)
            } else {
                Some(0)
            }
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "{error:#}");
            Some(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn session(args: &[&str]) -> SessionCommand {
        let cli = Cli::try_parse_from(
            ["luna-terminal", "session"]
                .into_iter()
                .chain(args.iter().copied()),
        )
        .unwrap();
        let Some(Command::Session { command }) = cli.command else {
            panic!("session command missing");
        };
        command
    }

    #[test]
    fn no_arguments_preserve_gui_startup() {
        assert!(
            Cli::try_parse_from(["luna-terminal"])
                .unwrap()
                .command
                .is_none()
        );
    }

    #[test]
    fn list_supports_table_and_json() {
        assert!(matches!(
            session(&["list"]),
            SessionCommand::List { json: false }
        ));
        assert!(matches!(
            session(&["list", "--json"]),
            SessionCommand::List { json: true }
        ));
    }

    #[test]
    fn create_preserves_names_paths_and_window_ids() {
        assert!(matches!(
            session(&["create"]),
            SessionCommand::Create {
                name: None,
                cwd: None,
                window: None,
            }
        ));
        let SessionCommand::Create { name, cwd, window } = session(&[
            "create", "--name", "backend", "--cwd", "../api", "--window", "window-1",
        ]) else {
            panic!("create command missing");
        };
        assert_eq!(name.as_deref(), Some("backend"));
        assert_eq!(cwd, Some(PathBuf::from("../api")));
        assert_eq!(window.as_deref(), Some("window-1"));
    }

    #[test]
    fn rename_accepts_an_empty_name_without_trimming_in_the_parser() {
        for name in ["", " backend ", "共享终端"] {
            let SessionCommand::Rename { id, name: parsed } = session(&["rename", "s-1", name])
            else {
                panic!("rename command missing");
            };
            assert_eq!(id, "s-1");
            assert_eq!(parsed, name);
        }
    }

    #[test]
    fn focus_and_close_target_ids() {
        assert!(matches!(session(&["focus", "s-1"]), SessionCommand::Focus { id } if id == "s-1"));
        for (args, expected) in [
            (vec!["close", "s-1"], false),
            (vec!["close", "s-1", "--force"], true),
        ] {
            assert!(matches!(session(&args), SessionCommand::Close { id, force }
                if id == "s-1" && force == expected));
        }
    }

    #[test]
    fn incomplete_unknown_and_extra_arguments_are_errors() {
        for args in [
            vec!["session"],
            vec!["session", "unknown"],
            vec!["session", "rename", "s-1"],
            vec!["session", "focus"],
            vec!["session", "close"],
            vec!["session", "list", "extra"],
            vec!["session", "create", "--cwd"],
            vec!["session", "create", "--force"],
            vec!["session", "close", "s-1", "--json"],
            vec!["--unknown"],
        ] {
            let result = Cli::try_parse_from(["luna-terminal"].into_iter().chain(args));
            assert!(result.is_err(), "invalid arguments were accepted");
        }
    }

    #[test]
    fn help_and_version_are_successful_parser_outcomes() {
        for args in [
            vec!["--help"],
            vec!["session", "--help"],
            vec!["session", "create", "--help"],
            vec!["session", "close", "--help"],
        ] {
            let error = Cli::try_parse_from(["luna-terminal"].into_iter().chain(args)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::DisplayHelp);
            assert_eq!(error.exit_code(), 0);
        }
        let error = Cli::try_parse_from(["luna-terminal", "--version"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert_eq!(error.exit_code(), 0);
    }
}
