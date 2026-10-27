#[cfg(windows)]
fn main() -> std::process::ExitCode {
    fn run() -> anyhow::Result<i32> {
        use anyhow::Context;
        let executable = std::env::current_exe()
            .context("cli launcher: locating launcher")?
            .with_file_name("luna-terminal.exe");
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        let mut command = std::process::Command::new(executable);
        command.args(&arguments);
        if arguments.is_empty() {
            command.spawn().context("cli launcher: starting GUI")?;
            Ok(0)
        } else {
            let status = command.status().context("cli launcher: running command")?;
            status
                .code()
                .context("cli launcher: command ended without an exit code")
        }
    }
    match run() {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {}
