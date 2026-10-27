use super::*;
use crate::cli::unix::bootstrap_lock;
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    sync::atomic::{AtomicU64, Ordering},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "luna-client-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn info(name: &str) -> SessionInfo {
    SessionInfo {
        id: "instance-s1".into(),
        window_id: "instance-w1".into(),
        name: name.into(),
        cwd: "/tmp".into(),
        tab_count: 2,
        selected: true,
    }
}

#[test]
fn tables_escape_controls_and_json_preserves_original_strings() {
    let sessions = vec![info("name\t\n\x1b[0m\u{202e}")];
    let table = listing(&sessions, false).unwrap();
    assert_eq!(table.lines().count(), 2);
    assert!(!table.contains('\x1b'));
    assert!(!table.contains('\u{202e}'));
    assert!(table.contains("\\u{9}"));
    let json = listing(&sessions, true).unwrap();
    let decoded: Vec<SessionInfo> = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, sessions);
    assert_eq!(listing(&[], true).unwrap(), "[]\n");
}

#[test]
fn missing_gui_fails_without_startup_for_non_create_commands() {
    let root = Directory::new();
    for command in [
        SessionCommand::List { json: true },
        SessionCommand::Rename {
            id: "i-s1".into(),
            name: "new".into(),
        },
        SessionCommand::Focus { id: "i-s1".into() },
        SessionCommand::Close {
            id: "i-s1".into(),
            force: true,
        },
        SessionCommand::Create {
            window: Some("i-w1".into()),
            name: None,
            cwd: None,
        },
    ] {
        assert!(
            execute(command, &root.0)
                .unwrap_err()
                .to_string()
                .contains("not running")
        );
        assert!(fs::read_dir(&root.0).unwrap().next().is_none());
    }
}

#[test]
fn runtime_ids_route_to_the_correct_instance() {
    let first = Live {
        endpoint: "one.sock".into(),
        sessions: vec![info("duplicate")],
    };
    let mut second_session = info("duplicate");
    second_session.id = "other-s1".into();
    let second = Live {
        endpoint: "two.sock".into(),
        sessions: vec![second_session],
    };
    let instances = vec![first, second];
    assert_eq!(
        owner(&instances, "other-s1").unwrap(),
        PathBuf::from("two.sock")
    );
    assert!(owner(&instances, "stale-s1").is_err());
}

#[test]
fn startup_lock_has_a_deadline_and_releases_on_drop() {
    let root = Directory::new();
    let held = bootstrap_lock(&root.0, Instant::now() + Duration::from_secs(1)).unwrap();
    assert!(bootstrap_lock(&root.0, Instant::now()).is_err());
    drop(held);
    assert!(bootstrap_lock(&root.0, Instant::now() + Duration::from_secs(1)).is_ok());
}

#[test]
fn list_uses_a_live_endpoint_instead_of_saved_state() {
    let root = Directory::new();
    let listener = platform::Listener::bind(&root.0, "instance").unwrap();
    let worker = std::thread::spawn(move || {
        loop {
            if let Some(mut connection) = listener.accept().unwrap() {
                let request: Request = read_frame(&mut connection).unwrap();
                assert!(matches!(request.operation, Operation::List {}));
                write_frame(
                    &mut connection,
                    &Response::Listed {
                        sessions: vec![info("live")],
                    },
                )
                .unwrap();
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let output = execute(SessionCommand::List { json: true }, &root.0).unwrap();
    let sessions: Vec<SessionInfo> = serde_json::from_str(&output).unwrap();
    assert_eq!(sessions[0].name, "live");
    worker.join().unwrap();
}
