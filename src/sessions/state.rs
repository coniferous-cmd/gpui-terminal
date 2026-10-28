//! The workspace's layout on disk: the sessions and tabs that were open, and the folder of each.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use gpui::{App, Global};
use serde::{Deserialize, Serialize};

/// The most sessions a snapshot may hold, and the most tabs one session may hold.
const SESSIONS: usize = 256;
const TABS: usize = 2048;

/// The file's name, in the platform's config folder; the settings file lives under the home folder.
pub const FILE: &str = "sessions.json";

/// The workspace as it stood: the session shown, and the sessions themselves.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub active: usize,
    pub sessions: Vec<SavedSession>,
}

/// One session: what it was called, the tab it showed, and its tabs.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSession {
    pub cwd: String,
    /// Empty while the session takes its program's title.
    #[serde(default)]
    pub name: String,
    pub active: usize,
    pub tabs: Vec<SavedTab>,
}

/// One tab: the folder its shell stood in.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedTab {
    #[serde(default)]
    pub name: String,
    pub folder: String,
}

impl SavedSession {
    pub fn new(cwd: &Path, name: String, active: usize, tabs: Vec<SavedTab>) -> Option<Self> {
        let Some(cwd) = cwd.to_str() else {
            log::error!(
                "sessions: session {name:?} cwd {} is not text; session is not kept",
                cwd.display()
            );
            return None;
        };
        Some(Self {
            cwd: cwd.to_owned(),
            name,
            active,
            tabs,
        })
    }
}

impl SavedTab {
    /// A tab standing in `folder`; none when that folder is not text, which the file holds.
    pub fn new(folder: &Path) -> Option<Self> {
        match folder.to_str() {
            Some(folder) => Some(Self {
                name: String::new(),
                folder: folder.to_string(),
            }),
            None => {
                log::info!(
                    "sessions: {} is not text, so its tab is not kept",
                    folder.display()
                );
                None
            }
        }
    }
}

impl Snapshot {
    /// Every value one the app can rebuild; the error says which is not.
    pub fn check(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.sessions.len() <= SESSIONS,
            "{} sessions is past {SESSIONS}",
            self.sessions.len()
        );
        if self.sessions.is_empty() {
            anyhow::ensure!(self.active == 0, "no session, but {} is shown", self.active);
            return Ok(());
        }
        anyhow::ensure!(
            self.active < self.sessions.len(),
            "session {} is shown of {}",
            self.active,
            self.sessions.len()
        );
        for (index, session) in self.sessions.iter().enumerate() {
            let at = index + 1;
            anyhow::ensure!(
                !session.cwd.is_empty() && Path::new(&session.cwd).has_root(),
                "session {at} has invalid cwd: {:?}",
                session.cwd
            );
            anyhow::ensure!(!session.tabs.is_empty(), "session {at} holds no tab");
            anyhow::ensure!(
                session.tabs.len() <= TABS,
                "session {at} holds {} tabs, past {TABS}",
                session.tabs.len()
            );
            anyhow::ensure!(
                session.active < session.tabs.len(),
                "session {at} shows tab {} of {}",
                session.active,
                session.tabs.len()
            );
            for tab in &session.tabs {
                anyhow::ensure!(
                    !tab.folder.is_empty(),
                    "session {at} has a tab in no folder"
                );
                // Rooted, not absolute: on Windows a shell may report `/c/Users`, which runs.
                anyhow::ensure!(
                    Path::new(&tab.folder).has_root(),
                    "session {at} has a tab in {}, which is no folder from the root",
                    tab.folder
                );
            }
        }
        Ok(())
    }
}

/// The file's snapshot; none when there is no file. A file that fails to read or check is an error.
pub fn load(path: &Path) -> anyhow::Result<Option<Snapshot>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("sessions: cannot read {}", path.display()));
        }
    };
    let mut reader = serde_json::Deserializer::from_str(&text);
    let snapshot: Snapshot = serde_path_to_error::deserialize(&mut reader).map_err(|error| {
        anyhow::anyhow!(
            "sessions: {} does not parse at `{}`: {}",
            path.display(),
            error.path(),
            error.inner()
        )
    })?;
    reader
        .end()
        .with_context(|| format!("sessions: {} does not parse", path.display()))?;
    snapshot
        .check()
        .with_context(|| format!("sessions: {} holds a bad layout", path.display()))?;
    Ok(Some(snapshot))
}

/// Writes `snapshot` whole to `path`, which its checks allow.
pub fn save(path: &Path, snapshot: &Snapshot) -> anyhow::Result<()> {
    snapshot
        .check()
        .context("sessions: this layout is not saved")?;
    let mut text = serde_json::to_string_pretty(snapshot).context("sessions: cannot encode")?;
    text.push('\n');
    crate::atomic::write(path, &text)
}

/// The layout the workspace has, the layout the file holds, and the file; its only writer.
pub struct SessionStore {
    path: PathBuf,
    /// The layout the workspace has now, whether it is written or not.
    current: Snapshot,
    /// What the file holds; none until a write lands.
    saved: Option<Snapshot>,
    /// The layout the file held at start, which the window restores.
    loaded: Snapshot,
    /// False for a window opened without a layout, which keeps none.
    keeping: bool,
}

impl Global for SessionStore {}

impl SessionStore {
    /// Loads `sessions.json` into the app; a file it cannot use is logged and dropped.
    pub fn init(cx: &mut App) -> anyhow::Result<()> {
        let folder = dirs::config_dir().context("sessions: the platform names no config folder")?;
        let path = folder.join("luna-terminal").join(FILE);
        let loaded = match load(&path) {
            Ok(Some(snapshot)) => {
                log::info!(
                    "sessions: loaded {} sessions from {}",
                    snapshot.sessions.len(),
                    path.display()
                );
                Some(snapshot)
            }
            Ok(None) => {
                log::info!("sessions: no file at {}", path.display());
                None
            }
            Err(error) => {
                log::error!("{error:#}; the layout is dropped");
                None
            }
        };
        cx.set_global(Self {
            path,
            current: loaded.clone().unwrap_or_default(),
            saved: loaded.clone(),
            loaded: loaded.unwrap_or_default(),
            keeping: true,
        });
        Ok(())
    }

    /// The layout the file held at start.
    pub fn loaded(cx: &App) -> Snapshot {
        cx.global::<Self>().loaded.clone()
    }

    /// Stops keeping the layout: a window opened without a snapshot writes nothing.
    pub fn forget(cx: &mut App) {
        let store = cx.global_mut::<Self>();
        store.keeping = false;
        store.saved = None;
        log::info!("sessions: this window keeps no layout");
    }

    /// Notes the layout the workspace has now, writing it when `write` is set and it changed.
    pub fn note(cx: &mut App, snapshot: Snapshot, write: bool) {
        cx.global_mut::<Self>().keep(snapshot, write);
    }

    /// Writes the layout the workspace has now, when it changed.
    pub fn flush(cx: &mut App) {
        cx.global_mut::<Self>().write();
    }

    /// Notes `snapshot`, then writes it when `write` is set; says whether the file was written.
    fn keep(&mut self, snapshot: Snapshot, write: bool) -> bool {
        self.current = snapshot;
        write && self.write()
    }

    /// Writes `current` over the file when it differs from what the file holds; says whether it did.
    fn write(&mut self) -> bool {
        if !self.keeping || self.saved.as_ref() == Some(&self.current) {
            return false;
        }
        match save(&self.path, &self.current) {
            Ok(()) => {
                log::info!(
                    "sessions: saved {} sessions to {}",
                    self.current.sessions.len(),
                    self.path.display()
                );
                self.saved = Some(self.current.clone());
                true
            }
            // What the file holds stays, so the next write tries again.
            Err(error) => {
                log::error!("sessions: the layout is not saved: {error:#}");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests;
