use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{Context, bail, ensure};

use super::{
    SessionCommand, platform,
    protocol::{NativePath, Operation, Request, Response, SessionInfo, read_frame, write_frame},
};

struct Live {
    endpoint: PathBuf,
    sessions: Vec<SessionInfo>,
}

pub fn run(command: SessionCommand) -> anyhow::Result<String> {
    let root = platform::registry()?;
    execute(command, &root)
}

fn execute(command: SessionCommand, root: &Path) -> anyhow::Result<String> {
    let mut live = list(root)?;
    if let SessionCommand::Create { window: None, .. } = &command
        && live.is_empty()
    {
        live = bootstrap(root)?;
    }
    ensure!(!live.is_empty(), "cli: luna-terminal is not running");
    if let SessionCommand::List { json } = command {
        let sessions: Vec<_> = live
            .into_iter()
            .flat_map(|instance| instance.sessions)
            .collect();
        return listing(&sessions, json);
    }
    let (endpoint, operation) = match command {
        SessionCommand::Create { name, cwd, window } => {
            let windows: BTreeSet<_> = live
                .iter()
                .flat_map(|instance| {
                    instance
                        .sessions
                        .iter()
                        .map(|session| session.window_id.clone())
                })
                .collect();
            let window = match window {
                Some(window) => {
                    ensure!(
                        windows.contains(&window),
                        "cli: window {window} no longer exists"
                    );
                    window
                }
                None => {
                    ensure!(
                        windows.len() == 1,
                        "cli: multiple terminal windows; use --window with an ID from session list"
                    );
                    windows
                        .into_iter()
                        .next()
                        .context("cli: no terminal window available")?
                }
            };
            let endpoint = live
                .iter()
                .find(|instance| {
                    instance
                        .sessions
                        .iter()
                        .any(|session| session.window_id == window)
                })
                .context("cli: target window disappeared")?
                .endpoint
                .clone();
            let cwd = cwd
                .map(|path| -> anyhow::Result<_> {
                    let path = if path.is_absolute() {
                        path
                    } else {
                        std::env::current_dir()
                            .context("cli: resolving working directory")?
                            .join(path)
                    };
                    Ok(NativePath::from_path(&path))
                })
                .transpose()?;
            (endpoint, Operation::Create { window, name, cwd })
        }
        SessionCommand::Rename { id, name } => (owner(&live, &id)?, Operation::Rename { id, name }),
        SessionCommand::Focus { id } => (owner(&live, &id)?, Operation::Focus { id }),
        SessionCommand::Close { id, force } => (owner(&live, &id)?, Operation::Close { id, force }),
        SessionCommand::List { .. } => unreachable!("listing returned above"),
    };
    match request(&endpoint, operation)?
        .context("cli: target application exited before command dispatch")?
    {
        Response::Created { id } => Ok(format!("{id}\n")),
        Response::Applied {} => Ok(String::new()),
        Response::Error { message, .. } => bail!("{message}"),
        Response::Listed { .. } => bail!("cli: unexpected list response to mutation"),
    }
}

fn owner(live: &[Live], id: &str) -> anyhow::Result<PathBuf> {
    live.iter()
        .find(|instance| instance.sessions.iter().any(|session| session.id == id))
        .map(|instance| instance.endpoint.clone())
        .with_context(|| format!("cli: session {id} no longer exists"))
}

fn list(root: &Path) -> anyhow::Result<Vec<Live>> {
    let mut live = Vec::new();
    for endpoint in platform::discover(root)? {
        match request(&endpoint, Operation::List {})? {
            Some(Response::Listed { sessions }) => live.push(Live { endpoint, sessions }),
            Some(Response::Error { message, .. }) => bail!("{message}"),
            Some(_) => bail!("cli: unexpected response while listing sessions"),
            None => {}
        }
    }
    Ok(live)
}

fn request(endpoint: &Path, operation: Operation) -> anyhow::Result<Option<Response>> {
    let request = Request::new(operation);
    request.validate()?;
    let Some(mut connection) = platform::connect(endpoint)? else {
        return Ok(None);
    };
    write_frame(&mut connection, &request)
        .context("cli: sending command; operation outcome may be unknown; not retried")?;
    let response = read_frame(&mut connection).context(
        "cli: receiving command response; operation outcome may be unknown; not retried",
    )?;
    if let Err(error) = connection.write_all(&[1]) {
        log::warn!("cli: acknowledging delivered response: {error}");
    }
    Ok(Some(response))
}

fn bootstrap(root: &Path) -> anyhow::Result<Vec<Live>> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let lock = platform::bootstrap_lock(root, deadline)?;
    let existing = list(root)?;
    if !existing.is_empty() {
        return Ok(existing);
    }
    log::info!("cli: no GUI running; starting luna-terminal");
    let mut child =
        Command::new(std::env::current_exe().context("cli: locating application executable")?)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .context("cli: starting GUI")?;
    loop {
        check_child(&mut child)?;
        let live = list(root)?;
        if !live.is_empty() {
            drop(lock);
            return Ok(live);
        }
        ensure!(
            Instant::now() < deadline,
            "cli: GUI readiness timed out; the GUI may still start"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn check_child(child: &mut Child) -> anyhow::Result<()> {
    if let Some(status) = child.try_wait().context("cli: checking GUI startup")? {
        bail!("cli: GUI exited before becoming ready: {status}");
    }
    Ok(())
}

pub fn listing(sessions: &[SessionInfo], json: bool) -> anyhow::Result<String> {
    if json {
        return Ok(format!("{}\n", serde_json::to_string(sessions)?));
    }
    let mut output = String::from("ID\tWINDOW\tNAME\tCWD\tTABS\tSELECTED\n");
    for session in sessions {
        use std::fmt::Write;
        writeln!(
            &mut output,
            "{}\t{}\t{}\t{}\t{}\t{}",
            plain(&session.id),
            plain(&session.window_id),
            plain(&session.name),
            plain(&session.cwd),
            session.tab_count,
            session.selected
        )?;
    }
    Ok(output)
}

fn plain(value: &str) -> String {
    value.chars().flat_map(|c| {
        if c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            c.escape_unicode().collect::<Vec<_>>()
        } else { vec![c] }
    }).collect()
}

#[cfg(all(test, unix))]
mod tests;
