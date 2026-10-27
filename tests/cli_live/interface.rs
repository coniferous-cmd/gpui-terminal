use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Output, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use anyhow::{Context, ensure};
use futures::{StreamExt, channel::mpsc as channel};
use gpui::App;
use luna_terminal::{
    app,
    cli::{self, protocol::SessionInfo},
    settings::{SettingsStore, StartIn},
    workspace::{CloseTab, NewTab, SelectTab},
};

enum Step {
    NewTab,
    SelectFirstTab,
    CloseFirstTab,
    InSession,
    Quit,
}

struct Gui {
    child: Child,
    input: ChildStdin,
    replies: mpsc::Receiver<String>,
}
impl Drop for Gui {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub fn run() {
    if std::env::var_os("LUNA_CLI_LIVE_CHILD").is_some() {
        gui();
        return;
    }
    let root = PathBuf::from(format!("/tmp/luna-live-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let home = root.join("home");
    let temporary = root.join("tmp");
    fs::create_dir_all(home.join(".config/luna")).unwrap();
    fs::create_dir(&temporary).unwrap();
    let shell = root.join("shell");
    fs::write(&shell, "#!/bin/sh\nexec /bin/sh\n").unwrap();
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        home.join(".config/luna/settings.json"),
        serde_json::to_vec(&serde_json::json!({
            "ely": false, "check_updates": false, "shell": shell,
        }))
        .unwrap(),
    )
    .unwrap();
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::set_var("TMPDIR", &temporary);
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .env("LUNA_CLI_LIVE_CHILD", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (send, replies) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            if send.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    let mut gui = Gui {
        child,
        input,
        replies,
    };
    let result = check(&root, &mut gui);
    if result.is_ok() {
        assert!(
            gui.child.wait().unwrap().success(),
            "GUI did not exit successfully"
        );
        fs::remove_dir_all(&root).unwrap();
    }
    result.unwrap_or_else(|error| panic!("cli_live: {error:#}; artifacts in {}", root.display()));
    println!("cli_live: create/cwd/rename/focus/guarded-close/force/identity/shutdown passed");
}

fn gui() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("luna_terminal=info"),
    )
    .init();
    let (sender, mut receiver) = channel::unbounded::<(Step, mpsc::SyncSender<()>)>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let action = match line.unwrap().as_str() {
                "new" => Step::NewTab,
                "select-first" => Step::SelectFirstTab,
                "close-first" => Step::CloseFirstTab,
                "in-session" => Step::InSession,
                "quit" => Step::Quit,
                other => panic!("unknown GUI test action: {other}"),
            };
            let (reply, receive) = mpsc::sync_channel(1);
            sender.unbounded_send((action, reply)).unwrap();
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
            println!("applied");
            std::io::stdout().flush().unwrap();
        }
    });
    #[cfg(target_os = "linux")]
    let application =
        gpui_platform::linux(gpui::WindowingModes::WAYLAND | gpui::WindowingModes::X11);
    #[cfg(not(target_os = "linux"))]
    let application = gpui_platform::application();
    application
        .with_assets(ely_gpui_component::Assets)
        .run(move |cx: &mut App| {
            ely_gpui_component::init(cx).unwrap();
            app::start(cx).unwrap();
            let window = app::open_saved(cx).unwrap();
            cli::service::start(cx).unwrap();
            cx.activate(true);
            cx.spawn(async move |cx| {
                while let Some((step, reply)) = receiver.next().await {
                    match step {
                        Step::Quit => cx.update(|cx| cx.quit()),
                        Step::InSession => cx.update(|cx| {
                            SettingsStore::update(cx, |settings| {
                                settings.start_in = StartIn::Session
                            })
                            .unwrap();
                        }),
                        Step::NewTab => {
                            window
                                .update(cx, |_, window, cx| {
                                    window.dispatch_action(Box::new(NewTab), cx)
                                })
                                .unwrap();
                        }
                        Step::SelectFirstTab => {
                            window
                                .update(cx, |_, window, cx| {
                                    window.dispatch_action(Box::new(SelectTab(0)), cx)
                                })
                                .unwrap();
                        }
                        Step::CloseFirstTab => {
                            window
                                .update(cx, |_, window, cx| {
                                    window.dispatch_action(Box::new(CloseTab), cx)
                                })
                                .unwrap();
                        }
                    }
                    let _ = reply.send(());
                }
            })
            .detach();
        });
}

fn step(gui: &mut Gui, step: Step) -> anyhow::Result<()> {
    let action = match step {
        Step::NewTab => "new",
        Step::SelectFirstTab => "select-first",
        Step::CloseFirstTab => "close-first",
        Step::InSession => "in-session",
        Step::Quit => "quit",
    };
    writeln!(gui.input, "{action}")?;
    gui.input.flush()?;
    ensure!(
        gui.replies.recv_timeout(Duration::from_secs(5))? == "applied",
        "GUI action was not applied"
    );
    Ok(())
}

fn raw(arguments: &[&str]) -> anyhow::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_luna-terminal"))
        .args(["session"])
        .args(arguments)
        .output()
        .context("running session command")
}

fn command(arguments: &[&str]) -> anyhow::Result<String> {
    let output = raw(arguments)?;
    ensure!(
        output.status.success(),
        "command {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).context("decoding command output")
}

fn sessions() -> anyhow::Result<Vec<SessionInfo>> {
    serde_json::from_str(&command(&["list", "--json"])?).context("decoding live sessions")
}

#[track_caller]
fn wait_for(mut condition: impl FnMut() -> anyhow::Result<bool>) -> anyhow::Result<()> {
    let caller = std::panic::Location::caller();
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        if condition()? {
            return Ok(());
        }
        ensure!(Instant::now() < until, "live test timed out at {caller}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn check(root: &Path, gui: &mut Gui) -> anyhow::Result<()> {
    wait_for(|| Ok(raw(&["list", "--json"])?.status.success()))?;
    let initial = sessions()?;
    ensure!(initial.len() == 1, "expected one initial session");
    let first = &initial[0].id;
    step(gui, Step::NewTab)?;
    wait_for(|| Ok(sessions()?[0].tab_count == 2))?;
    let rejected = raw(&["close", first])?;
    ensure!(
        !rejected.status.success() && rejected.stdout.is_empty(),
        "multi-tab close was not rejected"
    );
    ensure!(
        String::from_utf8_lossy(&rejected.stderr).contains("--force"),
        "missing force diagnostic"
    );
    ensure!(sessions()?[0].tab_count == 2, "guarded close changed tabs");
    step(gui, Step::SelectFirstTab)?;
    std::thread::sleep(Duration::from_millis(150));
    step(gui, Step::CloseFirstTab)?;
    wait_for(|| Ok(sessions()?[0].tab_count == 1))?;
    ensure!(
        sessions()?[0].id == *first,
        "session ID changed after original tab closed"
    );
    let cwd = root.to_str().unwrap();
    let created = command(&["create", "--name", " backend ", "--cwd", cwd])?
        .trim()
        .to_string();
    let listed = sessions()?;
    let new = listed
        .iter()
        .find(|session| session.id == created)
        .context("new session missing")?;
    ensure!(
        new.name == "backend" && Path::new(&new.cwd) == root && new.selected,
        "wrong created session"
    );
    step(gui, Step::InSession)?;
    let inherited = command(&["create"])?.trim().to_string();
    ensure!(
        sessions()?
            .iter()
            .any(|session| session.id == inherited && Path::new(&session.cwd) == root),
        "Start in was ignored"
    );
    command(&["rename", &created, " renamed "])?;
    ensure!(
        sessions()?
            .iter()
            .any(|session| session.id == created && session.name == "renamed"),
        "rename failed"
    );
    command(&["rename", &created, ""])?;
    ensure!(
        sessions()?
            .iter()
            .any(|session| session.id == created && session.name != "renamed"),
        "name reset failed"
    );
    command(&["focus", first])?;
    ensure!(
        sessions()?
            .iter()
            .any(|session| session.id == *first && session.selected),
        "focus failed"
    );
    let before = sessions()?.len();
    ensure!(
        !raw(&["create", "--cwd", "/luna-cli-no-such-directory"])?
            .status
            .success(),
        "invalid cwd accepted"
    );
    fs::remove_file(root.join("shell"))?;
    ensure!(
        !raw(&["create"])?.status.success(),
        "missing shell accepted"
    );
    ensure!(
        sessions()?.len() == before,
        "failed spawn left empty session"
    );
    ensure!(
        !raw(&["rename", "stale-s0", "name"])?.status.success(),
        "stale ID accepted"
    );
    command(&["close", &created])?;
    command(&["close", &inherited])?;
    // Restore the shell so a second tab can prove forced whole-group closure.
    fs::write(root.join("shell"), "#!/bin/sh\nexec /bin/sh\n")?;
    fs::set_permissions(root.join("shell"), fs::Permissions::from_mode(0o700))?;
    step(gui, Step::NewTab)?;
    wait_for(|| Ok(sessions()?[0].tab_count == 2))?;
    command(&["close", first, "--force"])?;
    wait_for(|| Ok(!raw(&["list"])?.status.success()))?;
    ensure!(
        !fs::read_dir(cli::platform::registry()?)?.any(|entry| entry
            .is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "sock"))),
        "endpoint survived shutdown"
    );
    Ok(())
}
