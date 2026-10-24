//! The window a snapshot asks for: its sessions and tabs, each shell started before it opens.

use std::path::{Path, PathBuf};

use gpui::App;

use crate::{
    sessions::state::Snapshot,
    settings::SettingsStore,
    term::{
        Events, Launch, Pty, config,
        shell::{ShellIntegration, launch},
        spawn,
    },
};

/// A tab's running shell, its reports, and what it runs, before the window takes them.
pub struct Started {
    pub pty: Pty,
    pub events: Events,
    pub launch: Launch,
}

/// A session to open: its name, the tab it shows, and its tabs.
pub struct Opening {
    pub name: String,
    pub active: usize,
    pub tabs: Vec<Started>,
}

impl Opening {
    /// One session holding `tabs`, `active` shown.
    pub fn of(name: String, active: usize, tabs: Vec<Started>) -> Self {
        Self { name, active, tabs }
    }
}

/// The openings `snapshot` asks for and which was shown; a failed tab goes, and a session left empty.
pub fn open(snapshot: &Snapshot, cx: &App) -> (Vec<Opening>, usize) {
    let home = dirs::home_dir();
    let mut openings = Vec::with_capacity(snapshot.sessions.len());
    let mut shown = 0;
    for (index, saved) in snapshot.sessions.iter().enumerate() {
        let mut tabs = Vec::with_capacity(saved.tabs.len());
        let mut active = 0;
        for (at, tab) in saved.tabs.iter().enumerate() {
            let folder = existing(Path::new(&tab.folder), home.as_deref());
            let Some(started) = start(&folder, cx) else {
                continue;
            };
            // A tab that went before the shown one must not shift the one that was shown.
            if at == saved.active {
                active = tabs.len();
            }
            tabs.push(started);
        }
        if tabs.is_empty() {
            log::error!(
                "sessions: session {:?} starts no tab, so the session goes",
                saved.name
            );
            continue;
        }
        if active != saved.active {
            log::info!(
                "sessions: session {:?} shows tab {} instead",
                saved.name,
                active + 1
            );
        }
        if index == snapshot.active {
            shown = openings.len();
        }
        openings.push(Opening {
            name: saved.name.clone(),
            active,
            tabs,
        });
    }
    (openings, shown)
}

/// `folder`'s shell started, or none when it cannot start.
fn start(folder: &Path, cx: &App) -> Option<Started> {
    let settings = SettingsStore::get(cx);
    let started = launch(
        &settings.shell,
        folder.to_path_buf(),
        &cx.global::<ShellIntegration>().0,
    )
    .and_then(|launch| {
        let (pty, events) = spawn(&launch, (80, 24), (8, 16), config(settings))?;
        Ok(Started {
            pty,
            events,
            launch,
        })
    });
    match started {
        Ok(started) => Some(started),
        Err(error) => {
            log::error!("sessions: no tab in {}: {error:#}", folder.display());
            None
        }
    }
}

/// `want` when it is a folder, else its nearest ancestor that is, else `home`.
fn existing(want: &Path, home: Option<&Path>) -> PathBuf {
    if let Some(found) = want.ancestors().find(|step| step.is_dir()) {
        if found != want {
            log::info!(
                "sessions: {} is gone; the tab starts in {}",
                want.display(),
                found.display()
            );
        }
        return found.to_path_buf();
    }
    match home.filter(|home| home.is_dir()) {
        Some(home) => {
            log::info!(
                "sessions: nothing of {} is there; the tab starts in {}",
                want.display(),
                home.display()
            );
            home.to_path_buf()
        }
        None => {
            log::error!(
                "sessions: neither {} nor a home folder is there",
                want.display()
            );
            want.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A folder of this test's own, made under the temp folder.
    fn made(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "luna-terminal-restore-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&folder).expect("folder");
        folder
    }

    /// A path nothing holds: relative, and named as no folder beside the tests is.
    fn nowhere() -> PathBuf {
        PathBuf::from(format!("no-such-place-{}", std::process::id())).join("anywhere")
    }

    #[test]
    fn a_folder_that_is_there_is_taken_as_it_is() {
        let folder = made("there");
        assert_eq!(existing(&folder, None), folder);
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn a_deleted_leaf_takes_its_nearest_ancestor() {
        let folder = made("leaf");
        let gone = folder.join("deep/gone");
        assert_eq!(existing(&gone, None), folder);
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn a_path_with_no_ancestor_takes_home() {
        let home = made("home");
        assert_eq!(existing(&nowhere(), Some(&home)), home);
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn with_no_ancestor_and_no_home_the_path_stands() {
        let gone = nowhere();
        assert_eq!(existing(&gone, None), gone);
    }
}
