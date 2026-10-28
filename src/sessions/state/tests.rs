use super::*;

fn folder(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("luna-terminal-state-{name}-{}", std::process::id()));
    if folder.exists() {
        fs::remove_dir_all(&folder).expect("a stale test folder goes");
    }
    folder
}

fn tab(folder: &str) -> SavedTab {
    SavedTab {
        name: String::new(),
        folder: folder.into(),
    }
}

/// Two sessions in four folders, the second shown on its last tab.
fn layout() -> Snapshot {
    Snapshot {
        active: 1,
        sessions: vec![
            SavedSession {
                cwd: "/project".into(),
                name: "api".into(),
                active: 1,
                tabs: vec![tab("/src/a"), tab("/src/b")],
            },
            SavedSession {
                cwd: "/project".into(),
                name: String::new(),
                active: 0,
                tabs: vec![tab("/src/c")],
            },
        ],
    }
}

#[test]
fn a_saved_layout_loads_back_and_leaves_no_temp() {
    let folder = folder("round");
    let path = folder.join(FILE);
    save(&path, &layout()).expect("saves");
    assert_eq!(load(&path).expect("loads"), Some(layout()));
    let left: Vec<_> = fs::read_dir(&folder).expect("folder").collect();
    assert_eq!(left.len(), 1, "only sessions.json remains");
    fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn tab_names_round_trip_independently_of_session_names() {
    let mut snapshot = layout();
    snapshot.sessions[0].tabs[0].name = "服务端 API".into();
    snapshot.sessions[0].tabs[1].name = "日志".into();
    let folder = folder("tab-names");
    let path = folder.join(FILE);
    save(&path, &snapshot).expect("saves names");
    assert_eq!(load(&path).expect("loads names"), Some(snapshot));
    fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn old_tabs_without_names_use_program_titles() {
    let tab: SavedTab = serde_json::from_str(r#"{"folder":"/src"}"#).expect("old tab");
    assert!(tab.name.is_empty());
}

#[test]
fn no_file_is_no_snapshot() {
    assert_eq!(load(&folder("none").join(FILE)).expect("none"), None);
}

#[test]
fn an_empty_layout_round_trips() {
    let folder = folder("empty");
    let path = folder.join(FILE);
    save(&path, &Snapshot::default()).expect("saves");
    assert_eq!(load(&path).expect("loads"), Some(Snapshot::default()));
    fs::remove_dir_all(folder).expect("cleanup");
}

/// A file holding `json` is an error that names `said`.
fn refused(name: &str, json: &str, said: &str) {
    let folder = folder(name);
    let path = folder.join(FILE);
    fs::create_dir_all(&folder).expect("folder");
    fs::write(&path, json).expect("write");
    let error = format!("{:#}", load(&path).expect_err(json));
    assert!(error.contains(FILE), "{error}");
    assert!(error.contains(said), "{error}");
    fs::remove_dir_all(folder).expect("cleanup");
}

#[test]
fn session_cwd_is_required_for_every_session() {
    let mut json = serde_json::to_value(layout()).unwrap();
    json["sessions"][0].as_object_mut().unwrap().remove("cwd");
    refused("mixed-cwd", &json.to_string(), "cwd");
    json["sessions"][1].as_object_mut().unwrap().remove("cwd");
    refused("old-cwd", &json.to_string(), "cwd");
}

#[test]
fn invalid_session_cwd_is_rejected() {
    for (at, cwd) in [
        serde_json::json!(""),
        serde_json::json!("relative"),
        serde_json::Value::Null,
        serde_json::json!(42),
    ]
    .into_iter()
    .enumerate()
    {
        let mut json = serde_json::to_value(layout()).unwrap();
        json["sessions"][0]["cwd"] = cwd;
        refused(&format!("bad-cwd-{at}"), &json.to_string(), "cwd");
    }
    let mut json = serde_json::to_value(layout()).unwrap();
    json["sessions"][0]["extra"] = serde_json::json!(true);
    refused("unknown-session", &json.to_string(), "extra");
}

#[test]
fn cwd_is_independent_of_tab_folders_and_need_not_exist() {
    let mut snapshot = layout();
    let missing = folder("missing-cwd");
    snapshot.sessions[0].cwd = missing.to_str().unwrap().to_owned();
    snapshot.sessions[0].tabs[0].folder = "/tmp".into();
    snapshot.sessions[0].tabs[1].folder = "/var/log".into();
    let root = folder("cwd-round");
    let path = root.join(FILE);
    save(&path, &snapshot).unwrap();
    assert_eq!(load(&path).unwrap(), Some(snapshot));
    assert!(!missing.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn session_cwd_is_never_serialized_lossily() {
    use std::os::unix::ffi::OsStrExt;
    let cwd = Path::new(std::ffi::OsStr::from_bytes(b"/project/\xff"));
    assert!(SavedSession::new(cwd, "project".into(), 0, vec![tab("/tmp")]).is_none());
    let saved = SavedSession::new(
        Path::new("/project"),
        "project".into(),
        0,
        vec![tab("/tmp")],
    )
    .unwrap();
    assert_eq!(saved.cwd, "/project");
    assert_eq!(saved.tabs[0].folder, "/tmp");
}

#[test]
fn a_truncated_file_does_not_parse() {
    refused("truncated", r#"{ "active": 0, "sess"#, "does not parse");
}

#[test]
fn an_unknown_field_drops_the_file() {
    refused(
        "unknown",
        r#"{ "active": 0, "sessions": [], "scrollback": 10 }"#,
        "scrollback",
    );
    refused(
        "unknown-tab",
        r#"{ "active": 0, "sessions": [{ "cwd": "/project", "active": 0, "tabs": [{ "folder": "/a", "title": "x" }] }] }"#,
        "title",
    );
}

#[test]
fn a_selection_past_what_is_there_drops_the_file() {
    refused(
        "past-sessions",
        r#"{ "active": 1, "sessions": [{ "cwd": "/project", "active": 0, "tabs": [{ "folder": "/a" }] }] }"#,
        "session 1 is shown",
    );
    refused(
        "past-tabs",
        r#"{ "active": 0, "sessions": [{ "cwd": "/project", "active": 2, "tabs": [{ "folder": "/a" }] }] }"#,
        "shows tab 2",
    );
}

#[test]
fn a_session_with_no_tab_drops_the_file() {
    refused(
        "no-tab",
        r#"{ "active": 0, "sessions": [{ "cwd": "/project", "active": 0, "tabs": [] }] }"#,
        "holds no tab",
    );
}

#[test]
fn a_folder_from_no_root_drops_the_file() {
    refused(
        "relative",
        r#"{ "active": 0, "sessions": [{ "cwd": "/project", "active": 0, "tabs": [{ "folder": "src" }] }] }"#,
        "no folder from the root",
    );
    refused(
        "blank",
        r#"{ "active": 0, "sessions": [{ "cwd": "/project", "active": 0, "tabs": [{ "folder": "" }] }] }"#,
        "in no folder",
    );
}

#[test]
fn past_the_limits_drops_the_file() {
    let session = SavedSession {
        cwd: "/project".into(),
        name: String::new(),
        active: 0,
        tabs: vec![tab("/src")],
    };
    let many = Snapshot {
        active: 0,
        sessions: vec![session.clone(); SESSIONS + 1],
    };
    assert!(format!("{:#}", many.check().expect_err("past")).contains("past 256"));
    let deep = Snapshot {
        active: 0,
        sessions: vec![SavedSession {
            tabs: vec![tab("/src"); TABS + 1],
            ..session
        }],
    };
    assert!(format!("{:#}", deep.check().expect_err("past")).contains("past 2048"));
}

#[cfg(unix)]
#[test]
fn a_folder_that_is_not_text_leaves_its_tab_out() {
    use std::os::unix::ffi::OsStrExt;

    let bytes = std::ffi::OsStr::from_bytes(b"/src/\xff");
    assert_eq!(SavedTab::new(Path::new(bytes)), None);
    assert_eq!(SavedTab::new(Path::new("/src/a")), Some(tab("/src/a")));
}

/// A store over a fresh folder, holding `loaded`.
fn store(name: &str, loaded: Option<Snapshot>) -> (SessionStore, PathBuf) {
    let path = folder(name).join(FILE);
    let store = SessionStore {
        path: path.clone(),
        current: loaded.clone().unwrap_or_default(),
        saved: loaded.clone(),
        loaded: loaded.unwrap_or_default(),
        keeping: true,
    };
    (store, path)
}

#[test]
fn a_selection_alone_writes_nothing() {
    let (mut store, path) = store("selection", Some(layout()));
    let mut selected = layout();
    selected.active = 0;
    assert!(!store.keep(selected, false), "nothing is written");
    assert!(!path.exists(), "and no file is made");
}

#[test]
fn a_layout_change_writes_once_and_not_again() {
    let (mut store, path) = store("layout", None);
    assert!(store.keep(layout(), true), "the layout is written");
    assert_eq!(load(&path).expect("loads"), Some(layout()));
    assert!(!store.keep(layout(), true), "the same layout is no write");
    let mut moved = layout();
    moved.sessions[1].tabs[0] = tab("/var/log");
    assert!(store.keep(moved.clone(), true), "a moved tab is written");
    assert_eq!(load(&path).expect("loads"), Some(moved));
    let _ = fs::remove_dir_all(path.parent().expect("folder"));
}

#[test]
fn an_empty_layout_writes_the_file_away() {
    let (mut store, path) = store("emptied", Some(layout()));
    assert!(
        store.keep(Snapshot::default(), true),
        "the last close writes"
    );
    assert_eq!(load(&path).expect("loads"), Some(Snapshot::default()));
    let _ = fs::remove_dir_all(path.parent().expect("folder"));
}

#[test]
fn a_store_that_forgets_writes_nothing() {
    let (mut store, path) = store("forgotten", Some(layout()));
    store.keeping = false;
    store.saved = None;
    assert!(!store.keep(layout(), true), "a forgotten store writes none");
    assert!(!store.write(), "and stays that way");
    assert!(!path.exists(), "no file is made");
}
